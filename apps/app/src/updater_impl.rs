use crate::api::Result;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::http::HeaderValue;
use tauri::http::header::ACCEPT;
use tauri::{Manager, ResourceId, Runtime, Webview};
use tauri_plugin_http::reqwest;
use tauri_plugin_http::reqwest::ClientBuilder;
use tauri_plugin_updater::{Error, Update, UpdaterExt};
use theseus::{
    LoadingBarType, emit_loading, init_loading, launcher_user_agent, settings,
};
use tokio::time::Instant;
use url::Url;

const UPDATE_SERVER_LATEST_URL: &str = "https://update.axlmc.org/latest";

// Debian and derivatives update via the apt package manager. The whole
// operation (repo setup script plus package install) runs as a single
// `pkexec` invocation so the polkit authorization prompt appears only once.
const AXOLOTL_APT_SETUP_URL: &str = "https://ppa.axlmc.org/setup.sh";
const AXOLOTL_APT_PACKAGE: &str = "axolotl-launcher";

/// SHA-256 digest of the repository setup script at `AXOLOTL_APT_SETUP_URL`.
///
/// The script is downloaded to a temporary file and handed to `pkexec sh`
/// only when its digest matches this pinned value, so unverified remote text
/// is never executed as root. Keep this in sync with the published script;
/// leaving it `None` disables first-time repository setup entirely
/// (fail-closed): the package must be installed through a pre-configured
/// repository instead.
const AXOLOTL_APT_SETUP_SHA256: Option<&str> = None;

// The updater plugin builds `Update` with no request timeout, so a stalled
// connection would hang the download forever. Bound the whole download.
const UPDATE_DOWNLOAD_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(15 * 60);

// ── Shared types ─────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadata {
    rid: ResourceId,
    current_version: String,
    version: String,
    date: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    force_update: bool,
    raw_json: serde_json::Value,
}

#[derive(Default)]
pub struct PendingUpdateData(pub Mutex<Option<(Arc<Update>, Vec<u8>)>>);

// ── Updater plugin helpers ───────────────────────────────────────

fn update_channel(channel: &str) -> Result<&str> {
    match channel {
        "release" | "beta" => Ok(channel),
        _ => Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!("Unknown update channel: {channel}"),
        ))
        .into()),
    }
}

fn update_platform() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok("windows-x86_64"),
        ("linux", "x86_64") => Ok("linux-x86_64"),
        ("linux", "aarch64") => Ok("linux-aarch64"),
        ("macos", "x86_64") => Ok("darwin-x86_64"),
        ("macos", "aarch64") => Ok("darwin-aarch64"),
        (os, arch) => {
            Err(theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Unsupported updater platform: {os}-{arch}"
            )))
            .into())
        }
    }
}

fn update_endpoint() -> Result<Url> {
    Url::parse(UPDATE_SERVER_LATEST_URL).map_err(|error| {
        theseus::Error::from(theseus::ErrorKind::OtherError(error.to_string()))
            .into()
    })
}

/// Build the platform-updater with the given endpoints and run a check.
async fn check_with_endpoints<R: Runtime>(
    webview: &Webview<R>,
    channel: &str,
) -> Result<Option<Update>> {
    let channel = update_channel(channel)?;
    let platform = update_platform()?;
    let current_version =
        webview.app_handle().package_info().version.to_string();
    let mut updater = webview
        .updater_builder()
        .endpoints(vec![update_endpoint()?])?
        .header("Accept", "application/json")?
        .header("X-Axolotl-Channel", channel)?
        .header("X-Axolotl-Platform", platform)?
        .header("X-Axolotl-Version", current_version)?;

    #[cfg(target_os = "windows")]
    {
        let install_dir = std::env::current_exe()
            .map_err(|error| {
                theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                    "Failed to resolve current executable: {error}"
                )))
            })?
            .parent()
            .ok_or_else(|| {
                theseus::Error::from(theseus::ErrorKind::OtherError(
                    "Current executable has no parent directory".to_string(),
                ))
            })?
            .to_path_buf();

        tracing::debug!(
            install_dir = %install_dir.display(),
            "Using current executable directory for Windows app updates"
        );
        updater = updater.installer_arg(format!(
            "/INSTALL_DIR=\"{}\"",
            install_dir.display()
        ));
    }

    let updater = updater.build()?;
    updater.check().await.map_err(Into::into)
}

/// Check the updater manifest through the configured Update Server endpoint.
async fn check_with_updater<R: Runtime>(
    webview: &Webview<R>,
    channel: &str,
) -> Result<Option<UpdateMetadata>> {
    let Some(mut update) = check_with_endpoints(webview, channel).await? else {
        return Ok(None);
    };
    update.timeout = Some(UPDATE_DOWNLOAD_TIMEOUT);

    let published_at = update
        .raw_json
        .get("published_at")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let force_update = update
        .raw_json
        .get("force_update")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let metadata = UpdateMetadata {
        rid: webview.resources_table().add(update.clone()),
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        date: None,
        body: update.body.clone(),
        published_at,
        force_update,
        raw_json: update.raw_json,
    };

    Ok(Some(metadata))
}

// ── Tauri commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn check_app_update<R: Runtime>(
    webview: Webview<R>,
    channel: String,
) -> Result<Option<UpdateMetadata>> {
    check_with_updater(&webview, &channel).await
}

// Reimplementation of Update::download mostly, minus the actual download part
#[tauri::command]
pub async fn get_update_size<R: Runtime>(
    webview: Webview<R>,
    rid: ResourceId,
) -> Result<Option<u64>> {
    let update = webview.resources_table().get::<Update>(rid)?;

    let mut headers = update.headers.clone();
    if !headers.contains_key(ACCEPT) {
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/octet-stream"),
        );
    }

    let mut request = ClientBuilder::new().user_agent(launcher_user_agent());
    if let Some(timeout) = update.timeout {
        request = request.timeout(timeout);
    }
    if let Some(ref proxy) = update.proxy {
        let proxy = reqwest::Proxy::all(proxy.as_str())?;
        request = request.proxy(proxy);
    }
    let response = request
        .build()?
        .head(update.download_url.clone())
        .headers(headers)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::Network(format!(
            "Download request failed with status: {}",
            response.status()
        ))
        .into());
    }

    let content_length = response
        .headers()
        .get("Content-Length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok());

    Ok(content_length)
}

#[tauri::command]
pub async fn enqueue_update_for_installation<R: Runtime>(
    webview: Webview<R>,
    rid: ResourceId,
) -> Result<()> {
    let pending_data = webview.state::<PendingUpdateData>().inner();

    let update = webview.resources_table().get::<Update>(rid)?;

    let progress = init_loading(
        LoadingBarType::LauncherUpdate {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
        },
        1.0,
        "Downloading update...",
    )
    .await?;

    let download_start = Instant::now();
    let update_data = update
        .download(
            |chunk_size, total_size| {
                let Some(total_size) = total_size else {
                    return;
                };
                if let Err(e) = emit_loading(
                    &progress,
                    chunk_size as f64 / total_size as f64,
                    None,
                ) {
                    tracing::error!(
                        "Failed to update download progress bar: {e}"
                    );
                }
            },
            || {},
        )
        .await?;
    let download_duration = download_start.elapsed();
    tracing::info!("Downloaded update in {download_duration:?}");

    pending_data
        .0
        .lock()
        .unwrap()
        .replace((update, update_data));

    Ok(())
}

#[tauri::command]
pub fn remove_enqueued_update<R: Runtime>(webview: Webview<R>) {
    let pending_data = webview.state::<PendingUpdateData>().inner();
    pending_data.0.lock().unwrap().take();
}

// ── Debian / derivatives apt update ─────────────────────────────

/// Whether this Linux system updates Axolotl through apt (Debian and its
/// derivatives) and has `pkexec` available for a single privileged prompt.
#[tauri::command]
pub fn is_apt_linux() -> bool {
    #[cfg(target_os = "linux")]
    {
        let debian_like = std::path::Path::new("/etc/debian_version").exists()
            || std::path::Path::new("/etc/apt").is_dir()
            || std::path::Path::new("/usr/bin/apt-get").exists();
        let has_pkexec = ["/usr/bin/pkexec", "/bin/pkexec"]
            .iter()
            .any(|path| std::path::Path::new(path).exists());
        debian_like && has_pkexec
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Validates an apt package version against the character set Debian accepts
/// for version strings, so it can never smuggle shell syntax or unexpected
/// arguments into a privileged command.
fn validate_apt_version(version: &str) -> Result<()> {
    if version.is_empty()
        || version.len() > 128
        || !version.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b".+-:~".contains(&byte)
        })
    {
        return Err(theseus::Error::from(theseus::ErrorKind::InputError(
            "invalid apt version".to_string(),
        ))
        .into());
    }
    Ok(())
}

/// Queries the installed version of a package through dpkg without root.
fn query_dpkg_version(package: &str) -> Result<String> {
    let output = std::process::Command::new("dpkg-query")
        .args(["-W", "-f=${Version}", package])
        .output()?;
    if !output.status.success() {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!(
                "dpkg-query failed for {package}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ))
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Detects whether the Axolotl apt repository is already configured, so a
/// first-time installation knows whether it must run the hash-verified setup
/// script. Checks the standard source-list locations and, as a fallback, the
/// dpkg status of the launcher package.
fn apt_repository_configured() -> bool {
    for path in [
        "/etc/apt/sources.list.d/axolotl.list",
        "/etc/apt/sources.list.d/axolotl.sources",
        "/etc/apt/sources.list.d/axlmc.list",
        "/etc/apt/sources.list.d/axlmc.sources",
    ] {
        if std::path::Path::new(path).exists() {
            return true;
        }
    }
    if let Ok(entries) = std::fs::read_dir("/etc/apt/sources.list.d") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.contains("axolotl") || name.contains("axlmc") {
                return true;
            }
        }
    }
    std::process::Command::new("dpkg-query")
        .args(["-W", "-f=${Status}", AXOLOTL_APT_PACKAGE])
        .output()
        .map(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .contains("install ok installed")
        })
        .unwrap_or(false)
}

/// Runs the first-time repository setup script after verifying its SHA-256
/// digest against the pinned value above. The script is written to a
/// temporary file and executed as a fixed local path via `pkexec sh`; an
/// unverified or mismatching script is never executed.
async fn run_apt_setup_script() -> Result<()> {
    let expected = AXOLOTL_APT_SETUP_SHA256.ok_or_else(|| {
        theseus::Error::from(theseus::ErrorKind::OtherError(
            "the Axolotl apt repository is not configured and no pinned setup script hash is \
             available; add the repository manually and retry"
                .to_string(),
        ))
    })?;

    let response = ClientBuilder::new()
        .user_agent(launcher_user_agent())
        .timeout(UPDATE_DOWNLOAD_TIMEOUT)
        .build()?
        .get(AXOLOTL_APT_SETUP_URL)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!(
                "failed to download the apt setup script: {}",
                response.status()
            ),
        ))
        .into());
    }
    let bytes = response.bytes().await?;

    use sha2::{Digest, Sha256};
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!(
                "apt setup script hash mismatch (expected {expected}, got {actual}); refusing to run it"
            ),
        ))
        .into());
    }

    // Persist the verified bytes in a uniquely named, exclusively created
    // temporary file (mode 0600 on Unix). The handle is moved into the
    // spawn_blocking closure and kept alive for the whole pkexec run, so no
    // other local process can predict or swap the path between our hash
    // check and the privileged execution.
    use std::io::Write;
    let mut script_file = tempfile::NamedTempFile::new().map_err(|io| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "failed to create temporary apt setup script: {io}"
        )))
    })?;
    script_file.write_all(&bytes).map_err(|io| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "failed to write temporary apt setup script: {io}"
        )))
    })?;
    script_file.flush().map_err(|io| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "failed to flush temporary apt setup script: {io}"
        )))
    })?;
    let script_path = script_file.path().to_path_buf();

    let output = tokio::task::spawn_blocking(move || {
        // `script_file` is kept alive here; dropping it would delete the file.
        let _script_file = script_file;
        std::process::Command::new("pkexec")
            .args(["sh", script_path.to_str().unwrap_or("")])
            .output()
    })
    .await
    .map_err(|join| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to run the apt setup script: {join}"
        )))
    })?
    .map_err(|io| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to start pkexec: {io}"
        )))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!("apt setup script failed: {}", stderr.trim()),
        ))
        .into());
    }
    Ok(())
}

/// Runs a pkexec command with a fixed argument list (no shell) on the blocking
/// pool, returning its output. The command line is never assembled from
/// remote or user-controlled text.
async fn run_pkexec_async(args: Vec<String>) -> Result<std::process::Output> {
    let task_result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("pkexec").args(&args).output()
    })
    .await
    .map_err(|join| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to run pkexec task: {join}"
        )))
    })?;

    let output = task_result.map_err(|io| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to start pkexec: {io}"
        )))
    })?;

    Ok(output)
}

/// Update Axolotl on Debian and its derivatives through apt, prompting for
/// root via `pkexec`. The repository is configured first (through a
/// hash-verified setup script on first use), then `apt-get update` and a
/// version-pinned `apt-get install` run with fixed arguments — never through
/// a shell that could interpolate remote or user-controlled text.
#[tauri::command]
pub async fn install_apt_update(version: String) -> Result<()> {
    if !is_apt_linux() {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            "apt updates are only supported on Debian-based Linux systems with pkexec"
                .to_string(),
        ))
        .into());
    }
    validate_apt_version(&version)?;

    // First-time installation needs the repository to exist; configure it
    // through the hash-verified setup script when it is missing.
    if !apt_repository_configured() {
        run_apt_setup_script().await?;
    }

    let update_output =
        run_pkexec_async(vec!["apt-get".to_string(), "update".to_string()])
            .await?;
    if !update_output.status.success() {
        let stderr = String::from_utf8_lossy(&update_output.stderr);
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!("apt update failed: {}", stderr.trim()),
        ))
        .into());
    }

    let install_output = run_pkexec_async(vec![
        "apt-get".to_string(),
        "install".to_string(),
        "-y".to_string(),
        format!("{AXOLOTL_APT_PACKAGE}={version}"),
    ])
    .await?;
    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!("apt install failed: {}", stderr.trim()),
        ))
        .into());
    }

    // Persist the success announcement only after the installed version is
    // verified to match the requested one.
    let installed = query_dpkg_version(AXOLOTL_APT_PACKAGE)?;
    if installed != version {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!("installed version {installed} != requested {version}"),
        ))
        .into());
    }

    let mut current = settings::get().await?;
    current.pending_update_toast_for_version = Some(version);
    settings::set(current).await?;

    Ok(())
}

#[cfg(test)]
mod apt_version_tests {
    use super::*;

    #[test]
    fn apt_version_accepts_valid_debian_versions() {
        for v in ["1.9.5", "1:2.3-4", "1.2.3~beta+git", "1.0-1~bpo11+1"] {
            assert!(validate_apt_version(v).is_ok(), "should accept {v}");
        }
    }

    #[test]
    fn apt_version_rejects_shell_injection() {
        for v in [
            "",
            "1.2;rm -rf /",
            "1.2$(id)",
            "1.2`id`",
            "1.2 | sh",
            "1.2 && apt",
            "1.2\nreboot",
            "a b",
            "1..2/..",
        ] {
            assert!(validate_apt_version(v).is_err(), "should reject {v:?}");
        }
    }
}
