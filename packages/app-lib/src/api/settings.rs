//! Theseus settings management interface

pub use crate::{
    State,
    state::{
        DownloadSourceMode, Hooks, MemorySettings, PrivacySettings, Settings,
        WindowSize,
    },
};

/// Gets entire settings
#[tracing::instrument]
pub async fn get() -> crate::Result<Settings> {
    State::get().await?;
    Ok(Settings::get().await)
}

/// Sets entire settings
#[tracing::instrument]
pub async fn set(mut settings: Settings) -> crate::Result<()> {
    let state = State::get().await?;
    let current = Settings::get().await;
    settings.telemetry = current.telemetry;
    settings.telemetry_consent_version = current.telemetry_consent_version;
    settings.discord_rpc = current.discord_rpc;
    super::terracotta::validate_public_nodes(
        &settings.terracotta_public_nodes,
    )?;
    crate::util::download_dns::DownloadDnsResolver::with_doh(
        settings.doh_enabled,
    )?;
    settings.apply_legacy_download_source_settings();
    settings.update().await;
    state.apply_runtime_settings(&settings).await?;
    Ok(())
}

#[tracing::instrument]
pub async fn get_privacy() -> crate::Result<PrivacySettings> {
    State::get().await?;
    Ok(Settings::privacy().await)
}

#[tracing::instrument]
pub async fn set_privacy(
    privacy: PrivacySettings,
) -> crate::Result<PrivacySettings> {
    let state = State::get().await?;
    // The queue is dropped before the preference is stored, so a worker that
    // runs in between cannot upload the events this change discards.
    let mut transaction = state.pool.begin().await?;
    sqlx::query("DELETE FROM telemetry_outbox")
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Settings::set_privacy(&privacy).await;

    if let Err(error) =
        crate::telemetry::set_enabled(&state, privacy.telemetry).await
    {
        tracing::debug!(target: "theseus::telemetry", %error, "Failed to apply telemetry state");
    }
    if let Err(error) = state.discord_rpc.clear_to_default(true).await {
        tracing::debug!(target: "theseus::telemetry", %error, "Failed to apply Discord RPC state");
    }
    get_privacy().await
}

#[tracing::instrument]
pub async fn set_telemetry(enabled: bool) -> crate::Result<PrivacySettings> {
    let state = State::get().await?;
    // The queue is dropped before the preference is stored, so a worker that
    // runs in between cannot upload the events this change discards.
    let mut transaction = state.pool.begin().await?;
    sqlx::query("DELETE FROM telemetry_outbox")
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Settings::set_telemetry(enabled).await;
    if let Err(error) = crate::telemetry::set_enabled(&state, enabled).await {
        tracing::debug!(target: "theseus::telemetry", %error, "Failed to apply telemetry state");
    }
    get_privacy().await
}

#[tracing::instrument]
pub async fn set_discord_rpc(enabled: bool) -> crate::Result<PrivacySettings> {
    let state = State::get().await?;
    Settings::set_discord_rpc(enabled).await;
    if let Err(error) = state.discord_rpc.clear_to_default(true).await {
        tracing::debug!(target: "theseus::telemetry", %error, "Failed to apply Discord RPC state");
    }
    get_privacy().await
}

#[tracing::instrument]
pub async fn cancel_directory_change(
    app_identifier: &str,
) -> crate::Result<()> {
    // This is called to handle state initialization errors due to folder migrations
    // failing, so fetching a DB connection pool from `State::get` is not reliable here
    crate::state::db::connect(app_identifier).await?;
    let mut settings = Settings::get().await;

    if let Some(prev_custom_dir) = settings.prev_custom_dir {
        settings.prev_custom_dir = None;
        settings.custom_dir = Some(prev_custom_dir);
    }

    settings.update().await;

    Ok(())
}
