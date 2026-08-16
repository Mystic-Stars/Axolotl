//! Theseus settings management interface

pub use crate::util::download::DownloadEngine;
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
    let state = State::get().await?;
    let settings = Settings::get(&state.pool).await?;
    Ok(settings)
}

/// Sets entire settings
#[tracing::instrument]
pub async fn set(mut settings: Settings) -> crate::Result<()> {
    let state = State::get().await?;
    let current = Settings::get(&state.pool).await?;
    settings.telemetry = current.telemetry;
    settings.telemetry_consent_version = current.telemetry_consent_version;
    settings.discord_rpc = current.discord_rpc;
    super::terracotta::validate_public_nodes(
        &settings.terracotta_public_nodes,
    )?;
    settings.apply_legacy_download_source_settings();
    settings.update(&state.pool).await?;
    state.update_download_settings(&settings);
    crate::util::download::set_active_engine(settings.download_engine);

    Ok(())
}

#[tracing::instrument]
pub async fn set_download_engine(engine: DownloadEngine) -> crate::Result<()> {
    let state = State::get().await?;
    let mut settings = Settings::get(&state.pool).await?;
    settings.download_engine = engine;
    settings.update(&state.pool).await?;
    crate::util::download::set_active_engine(engine);
    Ok(())
}

#[tracing::instrument]
pub async fn get_privacy() -> crate::Result<PrivacySettings> {
    let state = State::get().await?;
    let settings = Settings::get(&state.pool).await?;
    Ok(PrivacySettings {
        telemetry: settings.telemetry,
        discord_rpc: settings.discord_rpc,
        consent_version: settings.telemetry_consent_version,
    })
}

#[tracing::instrument]
pub async fn set_privacy(
    privacy: PrivacySettings,
) -> crate::Result<PrivacySettings> {
    let state = State::get().await?;
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
		"UPDATE settings SET telemetry = ?, discord_rpc = ?, telemetry_consent_version = ? WHERE id = 0",
	)
	.bind(privacy.telemetry)
	.bind(privacy.discord_rpc)
	.bind(privacy.consent_version)
	.execute(&mut *transaction)
	.await?;
    sqlx::query("DELETE FROM telemetry_outbox")
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;

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
    let mut transaction = state.pool.begin().await?;
    sqlx::query("UPDATE settings SET telemetry = ? WHERE id = 0")
        .bind(enabled)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM telemetry_outbox")
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    if let Err(error) = crate::telemetry::set_enabled(&state, enabled).await {
        tracing::debug!(target: "theseus::telemetry", %error, "Failed to apply telemetry state");
    }
    get_privacy().await
}

#[tracing::instrument]
pub async fn set_discord_rpc(enabled: bool) -> crate::Result<PrivacySettings> {
    let state = State::get().await?;
    sqlx::query("UPDATE settings SET discord_rpc = ? WHERE id = 0")
        .bind(enabled)
        .execute(&state.pool)
        .await?;
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
    let pool = crate::state::db::connect(app_identifier).await?;
    let mut settings = Settings::get(&pool).await?;

    if let Some(prev_custom_dir) = settings.prev_custom_dir {
        settings.prev_custom_dir = None;
        settings.custom_dir = Some(prev_custom_dir);
    }

    settings.update(&pool).await?;

    Ok(())
}
