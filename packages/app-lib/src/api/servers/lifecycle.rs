//! Server process control: starting, stopping, and monitoring the JVM.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use dashmap::{DashMap, DashSet};
use std::sync::LazyLock;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};

use crate::event::ServerPayloadType;
use crate::event::emit::emit_server;
use crate::state::{clear_log_buffer, get_log_buffer};
use crate::util::io::IOError;
use crate::{ErrorKind, Result};

use super::logs::{analyze_exit_reason, stream_server_output};
use super::manifest::{
    read_manifest, resolve_jar_name, server_path, write_manifest,
};

const DEFAULT_MEMORY_MB: u32 = 2048;
const STOP_TIMEOUT_SECS: u64 = 60;

struct ServerProcess {
    child: tokio::sync::Mutex<Child>,
    stdin: tokio::sync::Mutex<ChildStdin>,
    stop_requested: AtomicBool,
}

static SERVER_PROCESSES: LazyLock<DashMap<String, Arc<ServerProcess>>> =
    LazyLock::new(DashMap::new);

/// Synchronous start-in-flight guard. Reserving the slot before the first
/// `.await` prevents concurrent `start` calls (e.g. double-clicks) from both
/// passing the running check and spawning two JVMs on the same directory.
static SERVER_STARTING: LazyLock<DashSet<String>> = LazyLock::new(DashSet::new);

pub(super) fn is_running(server_id: &str) -> bool {
    SERVER_PROCESSES.contains_key(server_id)
}

pub async fn start(
    server_id: &str,
    java_path: Option<String>,
    memory_mb: Option<u32>,
    jvm_args: Option<Vec<String>>,
) -> Result<()> {
    if SERVER_PROCESSES.contains_key(server_id)
        || !SERVER_STARTING.insert(server_id.to_string())
    {
        return Err(ErrorKind::InputError(
            "Server is already running".to_string(),
        )
        .as_error());
    }
    let result = start_inner(server_id, java_path, memory_mb, jvm_args).await;
    SERVER_STARTING.remove(server_id);
    result
}

async fn start_inner(
    server_id: &str,
    java_path: Option<String>,
    memory_mb: Option<u32>,
    jvm_args: Option<Vec<String>>,
) -> Result<()> {
    let dir = server_path(server_id).await?;
    let mut manifest = read_manifest(&dir).await?;
    let launch_args = if manifest.server_type == "forge" {
        forge_launch_args(&dir)?
    } else {
        let jar_name = resolve_jar_name(&manifest);
        let jar_path = dir.join(&jar_name);
        if !jar_path.exists() {
            return Err(ErrorKind::LauncherError(format!(
                "Server jar not found: {jar_name}. Download the server files first."
            ))
            .as_error());
        }
        vec!["-jar".to_string(), jar_name, "nogui".to_string()]
    };

    let java = java_path
        .or_else(|| manifest.java_path.clone())
        .unwrap_or_else(|| "java".to_string());
    let memory = memory_mb
        .or(manifest.memory_mb)
        .unwrap_or(DEFAULT_MEMORY_MB);

    // Ensure eula.txt exists (create with eula=false if missing)
    let eula_path = dir.join("eula.txt");
    if !eula_path.exists() {
        tokio::fs::write(&eula_path, "eula=false\n")
            .await
            .map_err(|e| IOError::with_path(e, &eula_path))?;
    }

    let mut command = Command::new(&java);
    command.arg(format!("-Xmx{memory}M"));
    for arg in jvm_args.unwrap_or_else(|| manifest.jvm_args.clone()) {
        command.arg(arg);
    }
    for arg in launch_args {
        command.arg(arg);
    }
    command.current_dir(&dir);
    // Dynamic-loader injection variables (Steam overlays, debugging tools,
    // Dynamic-loader injection variables (Steam overlays, debugging tools,
    // stale shell exports) lengthen every dyld failure message the JVM
    // produces, which is exactly what trips the JNA < 5.13 macOS assertion;
    // they have no business affecting a managed server either.
    for variable in [
        "DYLD_LIBRARY_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_FALLBACK_FRAMEWORK_PATH",
        "DYLD_INSERT_LIBRARIES",
    ] {
        command.env_remove(variable);
    }
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    command.stdin(std::process::Stdio::piped());
    command.kill_on_drop(true);

    let mut child = command.spawn().map_err(|e| {
        ErrorKind::LauncherError(format!("Failed to start server process: {e}"))
            .as_error()
    })?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take();

    manifest.last_started_at = Some(Utc::now());
    manifest.last_exit_crashed = false;
    write_manifest(&dir, &manifest).await?;

    clear_log_buffer(server_id);
    let process = Arc::new(ServerProcess {
        child: tokio::sync::Mutex::new(child),
        stdin: tokio::sync::Mutex::new(stdin.ok_or_else(|| {
            ErrorKind::LauncherError(
                "Server stdin could not be captured".to_string(),
            )
            .as_error()
        })?),
        stop_requested: AtomicBool::new(false),
    });
    SERVER_PROCESSES.insert(server_id.to_string(), process.clone());

    if let Some(stdout) = stdout {
        tokio::spawn(stream_server_output(server_id.to_string(), stdout));
    }
    if let Some(stderr) = stderr {
        tokio::spawn(stream_server_output(server_id.to_string(), stderr));
    }
    tokio::spawn(monitor_server_process(server_id.to_string(), dir, process));

    emit_server(server_id, ServerPayloadType::Started)
        .await
        .ok();
    Ok(())
}

pub async fn send_command(server_id: &str, command: &str) -> Result<()> {
    let process = SERVER_PROCESSES
        .get(server_id)
        .map(|entry| entry.value().clone())
        .ok_or_else(|| {
            ErrorKind::InputError("Server is not running".to_string())
                .as_error()
        })?;
    let mut stdin = process.stdin.lock().await;
    stdin
        .write_all(format!("{command}\n").as_bytes())
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!("Failed to send command: {e}"))
                .as_error()
        })?;
    stdin.flush().await.map_err(|e| {
        ErrorKind::LauncherError(format!("Failed to send command: {e}"))
            .as_error()
    })?;
    Ok(())
}

pub async fn stop(server_id: &str) -> Result<()> {
    let process = SERVER_PROCESSES
        .get(server_id)
        .map(|entry| entry.value().clone())
        .ok_or_else(|| {
            ErrorKind::InputError("Server is not running".to_string())
                .as_error()
        })?;
    process.stop_requested.store(true, Ordering::SeqCst);
    let mut stdin = process.stdin.lock().await;
    let _ = stdin.write_all(b"stop\n").await;
    let _ = stdin.flush().await;

    let watchdog = process.clone();
    let server_id = server_id.to_string();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(STOP_TIMEOUT_SECS))
            .await;
        if let Some(current) = SERVER_PROCESSES.get(&server_id)
            && current.stop_requested.load(Ordering::SeqCst)
        {
            let _ = watchdog.child.lock().await.kill().await;
        }
    });
    Ok(())
}

pub async fn kill(server_id: &str) -> Result<()> {
    let process = SERVER_PROCESSES
        .get(server_id)
        .map(|entry| entry.value().clone())
        .ok_or_else(|| {
            ErrorKind::InputError("Server is not running".to_string())
                .as_error()
        })?;
    process.stop_requested.store(true, Ordering::SeqCst);
    let mut child = process.child.lock().await;
    child.kill().await?;
    Ok(())
}

async fn monitor_server_process(
    server_id: String,
    dir: PathBuf,
    process: Arc<ServerProcess>,
) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let exit_status = {
            let mut child = process.child.lock().await;
            match child.try_wait() {
                Ok(Some(status)) => Some(status),
                Ok(None) => continue,
                Err(_) => None,
            }
        };

        SERVER_PROCESSES.remove(&server_id);
        let stop_requested = process.stop_requested.load(Ordering::SeqCst);
        let eula_accepted = read_eula_accepted(&dir).await;
        let crashed = exit_status
            .map(|status| !status.success() && !stop_requested && eula_accepted)
            .unwrap_or(false);

        // Classify self-exits from the tail of the console output so the UI
        // can react (e.g. offer the EULA dialog). User-requested stops and
        // unmatched exits stay unclassified. The brief settle wait lets the
        // output-stream tasks flush their final lines into the buffer first.
        let reason = if stop_requested {
            None
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            analyze_exit_reason(&get_log_buffer(&server_id))
        };

        if let Ok(mut manifest) = read_manifest(&dir).await {
            manifest.last_exit_crashed = crashed;
            let _ = write_manifest(&dir, &manifest).await;
        }

        emit_server(&server_id, ServerPayloadType::Stopped { crashed, reason })
            .await
            .ok();
        return;
    }
}

/// Builds the JVM launch arguments for a Forge server. Modern Forge (1.17+)
/// ships `@args` files that enumerate the classpath and main class; legacy Forge
/// (<=1.16) produces a single runnable `forge-*.jar`.
fn forge_launch_args(dir: &Path) -> Result<Vec<String>> {
    let forge_dir = dir
        .join("libraries")
        .join("net")
        .join("minecraftforge")
        .join("forge");
    if let Ok(entries) = std::fs::read_dir(&forge_dir) {
        let args_file = if cfg!(windows) {
            "win_args.txt"
        } else {
            "unix_args.txt"
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join(args_file);
            if candidate.is_file() {
                let mut args = Vec::new();
                if dir.join("user_jvm_args.txt").exists() {
                    args.push("@user_jvm_args.txt".to_string());
                }
                args.push(format!("@{}", candidate.to_string_lossy()));
                args.push("nogui".to_string());
                return Ok(args);
            }
        }
    }
    if let Some(jar) = find_forge_jar(dir) {
        return Ok(vec!["-jar".to_string(), jar, "nogui".to_string()]);
    }
    Err(ErrorKind::LauncherError(
        "Forge server files are missing. Reinstall the server.".to_string(),
    )
    .as_error())
}

fn find_forge_jar(dir: &Path) -> Option<String> {
    let entry = std::fs::read_dir(dir).ok()?.flatten().find(|e| {
        e.file_name().to_string_lossy().starts_with("forge-")
            && e.path().extension().is_some_and(|ext| ext == "jar")
    })?;
    Some(entry.file_name().to_string_lossy().into_owned())
}

async fn read_eula_accepted(dir: &Path) -> bool {
    match tokio::fs::read_to_string(dir.join("eula.txt")).await {
        Ok(text) => text
            .lines()
            .find_map(|line| line.split_once('='))
            .filter(|(key, _)| key.trim() == "eula")
            .is_some_and(|(_, value)| {
                value.trim().eq_ignore_ascii_case("true")
            }),
        Err(_) => false,
    }
}
