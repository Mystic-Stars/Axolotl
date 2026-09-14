use crate::api::Result;
use tauri::{Emitter, Runtime};
use theseus::prelude::*;
use theseus::{ProxyConfig, ProxyTestResult};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("settings")
        .invoke_handler(tauri::generate_handler![
            settings_get,
            settings_set,
            privacy_get,
            privacy_set,
            telemetry_set,
            discord_rpc_set,
            download_engine_set,
            cancel_directory_change,
            proxy_get,
            proxy_set,
            proxy_test
        ])
        .build()
}

pub(crate) fn log_level_for_channel(channel: &str, log_level: &str) -> String {
    if channel == "beta" && !matches!(log_level, "debug" | "trace") {
        "debug".to_string()
    } else {
        log_level.to_string()
    }
}

// Get full settings
// invoke('plugin:settings|settings_get')
#[tauri::command]
pub async fn settings_get() -> Result<Settings> {
    let res = settings::get().await?;
    Ok(res)
}

// Set full settings
// invoke('plugin:settings|settings_set', settings)
#[tauri::command]
pub async fn settings_set(
    app: tauri::AppHandle<impl Runtime>,
    mut settings: Settings,
) -> Result<()> {
    let channel = crate::resolve_update_channel(&app).await?;
    settings.log_level = log_level_for_channel(&channel, &settings.log_level);
    let log_level = settings.log_level.clone();
    settings::set(settings).await?;
    // Apply the log level right away so the new verbosity takes effect without
    // a restart. Invalid values are rejected by the settings table itself.
    if let Err(error) = theseus::set_log_level(&log_level) {
        tracing::warn!("Keeping the previous log level: {error}");
    }
    let _ = app.emit("settings", ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::log_level_for_channel;

    #[test]
    fn beta_only_allows_verbose_log_levels() {
        assert_eq!(log_level_for_channel("beta", "info"), "debug");
        assert_eq!(log_level_for_channel("beta", "debug"), "debug");
        assert_eq!(log_level_for_channel("beta", "trace"), "trace");
    }

    #[test]
    fn release_preserves_the_selected_log_level() {
        assert_eq!(log_level_for_channel("release", "error"), "error");
        assert_eq!(log_level_for_channel("release", "info"), "info");
        assert_eq!(log_level_for_channel("release", "trace"), "trace");
    }
}

#[tauri::command]
pub async fn privacy_get() -> Result<PrivacySettings> {
    Ok(settings::get_privacy().await?)
}

#[tauri::command]
pub async fn privacy_set(privacy: PrivacySettings) -> Result<PrivacySettings> {
    Ok(settings::set_privacy(privacy).await?)
}

#[tauri::command]
pub async fn telemetry_set(enabled: bool) -> Result<PrivacySettings> {
    Ok(settings::set_telemetry(enabled).await?)
}

#[tauri::command]
pub async fn discord_rpc_set(enabled: bool) -> Result<PrivacySettings> {
    Ok(settings::set_discord_rpc(enabled).await?)
}

#[tauri::command]
pub async fn download_engine_set(
    engine: settings::DownloadEngine,
) -> Result<()> {
    settings::set_download_engine(engine).await?;
    Ok(())
}

#[tauri::command]
pub async fn cancel_directory_change<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<()> {
    let identifier = &app.config().identifier;
    settings::cancel_directory_change(identifier).await?;
    Ok(())
}

#[tauri::command]
pub async fn proxy_get() -> Result<ProxyConfig> {
    let state = State::get().await?;
    Ok(state.proxy_config().await?)
}

#[tauri::command]
pub async fn proxy_set(config: ProxyConfig) -> Result<()> {
    let state = State::get().await?;
    config.validate()?;
    state.update_proxy_config(&config).await?;
    Ok(())
}

#[tauri::command]
pub async fn proxy_test(config: ProxyConfig) -> Result<ProxyTestResult> {
    if let Err(e) = config.validate() {
        return Ok(ProxyTestResult {
            success: false,
            latency_ms: None,
            message: e.to_string(),
        });
    }
    let client = theseus::build_proxied_client(&config);
    let started = std::time::Instant::now();
    match client
        .get("http://connect.rom.miui.com/generate_204")
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            let latency_ms = started.elapsed().as_millis() as u64;
            Ok(ProxyTestResult {
                success: true,
                latency_ms: Some(latency_ms),
                message: format!("Connection successful ({latency_ms} ms)"),
            })
        }
        Ok(response) => {
            let latency_ms = started.elapsed().as_millis() as u64;
            Ok(ProxyTestResult {
                success: false,
                latency_ms: Some(latency_ms),
                message: format!("HTTP {}", response.status()),
            })
        }
        Err(e) => Ok(ProxyTestResult {
            success: false,
            latency_ms: None,
            message: format!("{e}"),
        }),
    }
}
