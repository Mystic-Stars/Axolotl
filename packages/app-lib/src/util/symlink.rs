use std::sync::OnceLock;

#[cfg(target_os = "windows")]
use serde::{Deserialize, Serialize};
use std::path::Path;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
use tokio::fs;
use tracing::error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkCapability {
    Supported,
    RequiresAdmin,
    Unsupported,
}

static SYMLINK_CAPABILITY: OnceLock<SymlinkCapability> = OnceLock::new();

pub async fn check_symlink_capability() -> SymlinkCapability {
    if let Some(capability) = SYMLINK_CAPABILITY.get() {
        return *capability;
    }

    let capability = check_symlink_capability_internal().await;
    let _ = SYMLINK_CAPABILITY.set(capability);
    capability
}

async fn check_symlink_capability_internal() -> SymlinkCapability {
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(e) => {
            error!("failed to create temp dir — {e}");
            return SymlinkCapability::Unsupported;
        }
    };

    let target_path = temp_dir.path().join("target");
    let link_path = temp_dir.path().join("link");

    if let Err(e) = fs::create_dir(&target_path).await {
        error!("failed to create target dir — {e}");
        return SymlinkCapability::Unsupported;
    }

    #[cfg(target_os = "windows")]
    {
        let target = target_path.clone();
        let link = link_path.clone();
        let junction_result = tokio::task::spawn_blocking(move || {
            junction::create(&target, &link)
        })
        .await;

        if matches!(junction_result, Ok(Ok(()))) {
            return SymlinkCapability::Supported;
        }
        if let Err(e) = junction_result {
            error!(error = %e, "junction::create task panicked");
        }

        let target = target_path.clone();
        let link = link_path.clone();
        let symlink_result = tokio::task::spawn_blocking(move || {
            symlink_rs::symlink_dir(&target, &link)
        })
        .await;

        match symlink_result {
            Ok(Ok(_)) => SymlinkCapability::Supported,
            Ok(Err(e)) => {
                let raw_os_error = e.raw_os_error();
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || raw_os_error == Some(1314)
                {
                    error!("symlink: permission denied — {e}");
                    SymlinkCapability::RequiresAdmin
                } else {
                    error!("symlink: system does not support it — {e}");
                    SymlinkCapability::Unsupported
                }
            }
            Err(e) => {
                error!("symlink: blocking task panicked — {e}");
                SymlinkCapability::Unsupported
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let target = target_path.clone();
        let link = link_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            symlink_rs::symlink_dir(&target, &link)
        })
        .await;

        match result {
            Ok(Ok(_)) => SymlinkCapability::Supported,
            Ok(Err(e)) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    error!("symlink: permission denied — {e}");
                    SymlinkCapability::RequiresAdmin
                } else {
                    error!("symlink: system does not support it — {e}");
                    SymlinkCapability::Unsupported
                }
            }
            Err(e) => {
                error!("symlink: blocking task panicked — {e}");
                SymlinkCapability::Unsupported
            }
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Serialize, Deserialize)]
struct ElevatedLinkRequest {
    target: PathBuf,
    link: PathBuf,
    result_file: PathBuf,
}

#[cfg(target_os = "windows")]
#[derive(Serialize, Deserialize)]
struct ElevatedLinkResult {
    ok: bool,
    error: Option<String>,
}

/// Create a directory or file link synchronously without elevation.
/// Directory links prefer junctions, which do not require administrator
/// privileges on Windows, and fall back to symbolic links.
pub(crate) fn create_link_blocking(
    target: &Path,
    link: &Path,
) -> std::io::Result<()> {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if !target.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Target path does not exist: {}", target.display()),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        if target.is_dir() {
            match junction::create(target, link) {
                Ok(()) => Ok(()),
                Err(junction_error) => {
                    tracing::debug!(
                        "junction creation failed: {junction_error}; falling back to symlink"
                    );
                    symlink_rs::symlink_dir(target, link)
                }
            }
        } else {
            symlink_rs::symlink_file(target, link)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        symlink_rs::symlink_auto(target, link)
    }
}

/// Create a link through a short-lived elevated helper process. The current
/// launcher executable is re-launched with `--elevated-create-link` via UAC,
/// performs the operation, and exits immediately afterwards.
#[cfg(target_os = "windows")]
pub(crate) async fn create_link_elevated(
    target: &Path,
    link: &Path,
) -> std::io::Result<()> {
    let target = target.to_path_buf();
    let link = link.to_path_buf();
    tokio::task::spawn_blocking(move || {
        create_link_elevated_blocking(&target, &link)
    })
    .await
    .map_err(|error| {
        std::io::Error::other(format!("elevated link task panicked: {error}"))
    })?
}

#[cfg(target_os = "windows")]
fn create_link_elevated_blocking(
    target: &Path,
    link: &Path,
) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let result_file = std::env::temp_dir()
        .join(format!("axolotl-link-result-{}.json", uuid::Uuid::new_v4()));
    let request = ElevatedLinkRequest {
        target: target.to_path_buf(),
        link: link.to_path_buf(),
        result_file: result_file.clone(),
    };
    let payload = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        serde_json::to_vec(&request).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error.to_string(),
            )
        })?,
    );

    let executable = std::env::current_exe().map_err(|error| {
        std::io::Error::other(format!(
            "Could not locate launcher executable: {error}"
        ))
    })?;
    let escaped = executable.to_string_lossy().replace('\'', "''");
    let command = format!(
        "$process = Start-Process -FilePath '{escaped}' -ArgumentList '--elevated-create-link','{payload}' -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ErrorAction Stop; if ($null -eq $process) {{ exit 1 }}; exit $process.ExitCode"
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .creation_flags(0x0800_0000)
        .status()
        .map_err(|error| {
            std::io::Error::other(format!(
                "Could not request administrator permission: {error}"
            ))
        })?;

    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Administrator permission was denied",
        ));
    }

    let raw_result = std::fs::read_to_string(&result_file).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Administrator permission was denied",
        )
    })?;
    let _ = std::fs::remove_file(&result_file);

    let result: ElevatedLinkResult = serde_json::from_str(&raw_result)
        .map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error.to_string(),
            )
        })?;
    if result.ok {
        Ok(())
    } else {
        Err(std::io::Error::other(
            result
                .error
                .unwrap_or_else(|| "Link creation failed".to_string()),
        ))
    }
}

/// Entry point for the elevated link-creation helper process. Decodes the
/// request, creates the link, writes the outcome to the result file, and
/// returns an exit code. The launcher binary calls this before Tauri
/// initializes when started with `--elevated-create-link`.
#[cfg(target_os = "windows")]
pub fn create_link_elevated_helper(payload: &str) -> i32 {
    let request = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        payload.trim(),
    )
    .ok()
    .and_then(|bytes| {
        serde_json::from_slice::<ElevatedLinkRequest>(&bytes).ok()
    });

    let Some(request) = request else {
        tracing::error!("Elevated link creation: invalid request payload");
        return 1;
    };

    let result = match create_link_blocking(&request.target, &request.link) {
        Ok(()) => ElevatedLinkResult {
            ok: true,
            error: None,
        },
        Err(link_error) => ElevatedLinkResult {
            ok: false,
            error: Some(link_error.to_string()),
        },
    };

    let serialized = match serde_json::to_vec(&result) {
        Ok(serialized) => serialized,
        Err(serialize_error) => {
            tracing::error!(
                "Elevated link creation: failed to serialize result: {serialize_error}"
            );
            return 1;
        }
    };

    if let Err(write_error) = std::fs::write(&request.result_file, serialized) {
        tracing::error!(
            "Elevated link creation: failed to write result file: {write_error}"
        );
        return 1;
    }

    if result.ok { 0 } else { 1 }
}
