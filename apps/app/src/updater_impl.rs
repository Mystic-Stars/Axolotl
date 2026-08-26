use crate::api::Result;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::http::HeaderValue;
use tauri::http::header::ACCEPT;
use tauri::{Manager, ResourceId, Runtime, Webview};
use tauri_plugin_http::reqwest;
use tauri_plugin_http::reqwest::ClientBuilder;
use tauri_plugin_updater::{Error, Update, UpdaterExt};
use theseus::{
    LoadingBarType, emit_loading, init_loading, launcher_user_agent,
};
use tokio::time::Instant;
use url::Url;

const MIAWA_API_BASE: &str = "https://miawa.cn/api/v2";
const MIAWA_HOST: &str = "https://miawa.cn";
const MIAWA_API_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(15);

// The updater plugin builds `Update` with no request timeout, so a stalled
// connection would hang the download forever. Bound the whole download
// (installers can exceed 100 MB) so failures always surface and can fall
// back to another source.
const UPDATE_DOWNLOAD_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(15 * 60);

// ── Miawa API types ──────────────────────────────────────────────

#[derive(Deserialize)]
struct MiawaEnvelope<T> {
    data: T,
}

#[derive(Deserialize)]
struct MiawaLatest {
    version: String,
}

#[derive(Deserialize)]
struct MiawaPrepare {
    download_url: String,
}

// ── Shared types ─────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadata {
    rid: ResourceId,
    current_version: String,
    version: String,
    date: Option<String>,
    body: Option<String>,
    raw_json: serde_json::Value,
}

#[derive(Default)]
pub struct PendingUpdateData(pub Mutex<Option<(Arc<Update>, Vec<u8>)>>);

// ── Miawa API helpers ────────────────────────────────────────────

fn miawa_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(launcher_user_agent())
        .timeout(MIAWA_API_TIMEOUT)
        .build()
        .expect("Failed to build Miawa HTTP client")
}

async fn miawa_latest_version() -> Result<String> {
    let client = miawa_client();
    let latest: MiawaEnvelope<MiawaLatest> = client
        .get(format!("{MIAWA_API_BASE}/latest/axolotl"))
        .send()
        .await
        .map_err(|e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Miawa latest version request failed: {e}"
            )))
        })?
        .json()
        .await
        .map_err(|e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Failed to parse Miawa latest version response: {e}"
            )))
        })?;

    Ok(latest.data.version)
}

fn miawa_update_available(current: &Version, remote_tag: &str) -> Result<bool> {
    let remote =
        Version::parse(remote_tag.trim_start_matches('v')).map_err(|e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Failed to parse Miawa latest version {remote_tag:?}: {e}"
            )))
        })?;

    Ok(remote > *current)
}

/// Resolve a download URL for a Miawa file path via the prepare API.
async fn miawa_prepare_url(file_path: &str) -> Result<Url> {
    let client = miawa_client();
    let prepare: MiawaEnvelope<MiawaPrepare> = client
        .post(format!("{MIAWA_API_BASE}/downloads/prepare"))
        .json(&serde_json::json!({ "file_path": file_path }))
        .send()
        .await
        .map_err(|e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Miawa prepare request failed: {e}"
            )))
        })?
        .json()
        .await
        .map_err(|e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Failed to parse Miawa prepare response: {e}"
            )))
        })?;

    Url::parse(&format!("{MIAWA_HOST}{}", prepare.data.download_url)).map_err(
        |e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Failed to parse Miawa download URL: {e}"
            )))
            .into()
        },
    )
}

// ── Updater plugin helpers ───────────────────────────────────────

fn update_endpoints(source: &str) -> Result<Vec<Url>> {
    let endpoints = match source {
        "github" | "official" => vec![
            "https://github.com/Mystic-Stars/Axolotl/releases/latest/download/latest.json",
        ],
        "cnb" => vec![
            "https://cnb.cool/axlmc/Axolotl/-/git/raw/update/latest.json",
            "https://github.com/Mystic-Stars/Axolotl/releases/latest/download/latest.json",
        ],
        _ => {
            return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
                format!("Unknown update source: {source}"),
            ))
            .into());
        }
    };

    endpoints
        .into_iter()
        .map(|endpoint| {
            Url::parse(endpoint).map_err(|error| {
                theseus::Error::from(theseus::ErrorKind::OtherError(
                    error.to_string(),
                ))
                .into()
            })
        })
        .collect()
}

/// Build the platform-updater with the given endpoints and run a check.
async fn check_with_endpoints<R: Runtime>(
    webview: &Webview<R>,
    endpoints: Vec<Url>,
) -> Result<Option<Update>> {
    let mut updater = webview.updater_builder().endpoints(endpoints)?;

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

/// Plain updater-plugin check against the static endpoints of `source`.
async fn check_with_updater<R: Runtime>(
    webview: &Webview<R>,
    source: &str,
) -> Result<Option<UpdateMetadata>> {
    let endpoints = update_endpoints(source)?;
    let Some(mut update) = check_with_endpoints(webview, endpoints).await?
    else {
        return Ok(None);
    };
    update.timeout = Some(UPDATE_DOWNLOAD_TIMEOUT);

    let metadata = UpdateMetadata {
        rid: webview.resources_table().add(update.clone()),
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        date: None,
        body: update.body.clone(),
        raw_json: update.raw_json,
    };

    Ok(Some(metadata))
}

async fn check_miawa<R: Runtime>(
    webview: &Webview<R>,
) -> Result<Option<UpdateMetadata>> {
    let tag_name = miawa_latest_version().await?;
    let current_version = webview.app_handle().package_info().version.clone();

    if !miawa_update_available(&current_version, &tag_name)? {
        tracing::info!(
            current = %current_version,
            latest = %tag_name,
            "Miawa has no newer version; skipping latest.json check"
        );
        return Ok(None);
    }

    let latest_url =
        miawa_prepare_url(&format!("axolotl/{tag_name}/latest.json")).await?;
    tracing::info!("Miawa latest.json resolved (tag {tag_name}): {latest_url}");

    let Some(mut update) =
        check_with_endpoints(webview, vec![latest_url]).await?
    else {
        return Ok(None);
    };
    update.timeout = Some(UPDATE_DOWNLOAD_TIMEOUT);

    // Redirect the actual download to the Miawa mirror.
    let filename = update
        .download_url
        .path_segments()
        .and_then(|s| s.last().filter(|s| !s.is_empty()))
        .ok_or_else(|| {
            theseus::Error::from(theseus::ErrorKind::OtherError(
                "Could not extract filename from download URL".to_string(),
            ))
        })?
        .to_string();

    let mirror_url =
        miawa_prepare_url(&format!("axolotl/{tag_name}/{filename}")).await?;
    tracing::info!("Miawa mirror download URL (file {filename}): {mirror_url}");
    update.download_url = mirror_url;

    let metadata = UpdateMetadata {
        rid: webview.resources_table().add(update.clone()),
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        date: None,
        body: update.body.clone(),
        raw_json: update.raw_json,
    };

    Ok(Some(metadata))
}

// ── Tauri commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn check_app_update<R: Runtime>(
    webview: Webview<R>,
    source: String,
) -> Result<Option<UpdateMetadata>> {
    match source.as_str() {
        "miawa" => {
            // 1. Try Miawa mirror
            match check_miawa(&webview).await {
                Ok(Some(metadata)) => {
                    tracing::info!(
                        "Update {} available via Miawa mirror",
                        metadata.version
                    );
                    return Ok(Some(metadata));
                }
                Ok(None) => {
                    tracing::info!("No update available via Miawa mirror");
                    return Ok(None);
                }
                Err(e) => {
                    tracing::warn!(
                        "Miawa check failed, falling back to CNB: {e}"
                    );
                }
            }

            // 2. Fallback: CNB
            match check_with_updater(&webview, "cnb").await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        "CNB check failed, falling back to GitHub: {e}"
                    );
                }
            }

            // 3. Fallback: GitHub
            check_with_updater(&webview, "github").await
        }
        "cnb" => {
            // 1. Try CNB
            match check_with_updater(&webview, "cnb").await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        "CNB check failed, falling back to GitHub: {e}"
                    );
                }
            }

            // 2. Fallback: GitHub
            check_with_updater(&webview, "github").await
        }
        "github" | "official" => check_with_updater(&webview, "github").await,
        _ => Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!("Unknown update source: {source}"),
        ))
        .into()),
    }
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
