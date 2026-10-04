use eyre::bail;
use serde::{Deserialize, Serialize};
use std::sync::{
    LazyLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Mutex;

use super::{hongshi, terracotta};

static ACTIVE_PROVIDER: LazyLock<Mutex<Option<MultiplayerProvider>>> =
    LazyLock::new(|| Mutex::new(None));
static MULTIPLAYER_OPERATION: LazyLock<Mutex<()>> =
    LazyLock::new(|| Mutex::new(()));
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) static SESSION_TEST: LazyLock<Mutex<()>> =
    LazyLock::new(|| Mutex::new(()));

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiplayerProvider {
    Terracotta,
    Hongshi,
}

impl std::fmt::Display for MultiplayerProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Terracotta => formatter.write_str("terracotta"),
            Self::Hongshi => formatter.write_str("hongshi"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MultiplayerProviderCapabilities {
    pub provider: MultiplayerProvider,
    pub supported: bool,
    pub can_host: bool,
    pub can_join: bool,
    pub requires_local_port: bool,
    pub unsupported_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MultiplayerState {
    pub active_provider: Option<MultiplayerProvider>,
    pub providers: Vec<MultiplayerProviderCapabilities>,
    pub terracotta: terracotta::TerracottaState,
    pub hongshi: hongshi::HongshiState,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum MultiplayerHostRequest {
    Terracotta {
        player_name: String,
        room_code: Option<String>,
    },
    Hongshi {
        local_port: u16,
        node_name: Option<String>,
        instance_id: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct MultiplayerJoinRequest {
    pub provider: MultiplayerProvider,
    pub player_name: String,
    pub room_code: String,
}

pub async fn claim_provider(provider: MultiplayerProvider) -> eyre::Result<()> {
    let mut active = ACTIVE_PROVIDER.lock().await;
    if SHUTTING_DOWN.load(Ordering::Relaxed) {
        bail!("the launcher is shutting down");
    }

    if let Some(current) = *active
        && current != provider
    {
        bail!("{current} multiplayer is already running");
    }
    *active = Some(provider);
    Ok(())
}

pub async fn release_provider(provider: MultiplayerProvider) {
    let mut active = ACTIVE_PROVIDER.lock().await;
    if *active == Some(provider) {
        *active = None;
    }
}

pub async fn get_state() -> MultiplayerState {
    let active_provider = *ACTIVE_PROVIDER.lock().await;
    let terracotta = terracotta::get_state().await;
    let hongshi = hongshi::get_state().await;
    MultiplayerState {
        active_provider,
        providers: vec![
            MultiplayerProviderCapabilities {
                provider: MultiplayerProvider::Terracotta,
                supported: terracotta::terracotta_platform_key()
                    != "unsupported",
                can_host: true,
                can_join: true,
                requires_local_port: false,
                unsupported_reason: None,
            },
            MultiplayerProviderCapabilities {
                provider: MultiplayerProvider::Hongshi,
                supported: hongshi.supported,
                can_host: true,
                can_join: false,
                requires_local_port: true,
                unsupported_reason: (!hongshi.supported).then(|| {
                    "RedStone is not supported on this platform".to_string()
                }),
            },
        ],
        terracotta,
        hongshi,
    }
}

async fn stop_provider(provider: MultiplayerProvider) -> eyre::Result<()> {
    match provider {
        MultiplayerProvider::Terracotta => terracotta::stop_terracotta().await,
        MultiplayerProvider::Hongshi => hongshi::stop().await,
    }
}

pub async fn switch_provider(
    provider: MultiplayerProvider,
) -> eyre::Result<()> {
    let _operation = MULTIPLAYER_OPERATION.lock().await;
    let active = *ACTIVE_PROVIDER.lock().await;
    if let Some(current) = active
        && current != provider
    {
        stop_provider(current).await?;
    }
    Ok(())
}

pub async fn prepare_terracotta() -> eyre::Result<()> {
    prepare_terracotta_with_options(None, true).await
}

pub async fn prepare_terracotta_with_options(
    binary_path: Option<String>,
    auto_download: bool,
) -> eyre::Result<()> {
    let _operation = MULTIPLAYER_OPERATION.lock().await;
    claim_provider(MultiplayerProvider::Terracotta).await?;
    if terracotta::get_state().await.http_port.is_some() {
        return Ok(());
    }
    if let Err(error) =
        terracotta::start_terracotta(binary_path, auto_download).await
    {
        release_provider(MultiplayerProvider::Terracotta).await;
        return Err(error);
    }
    Ok(())
}

pub async fn stop_terracotta_compat() -> eyre::Result<()> {
    let _operation = MULTIPLAYER_OPERATION.lock().await;
    terracotta::stop_terracotta().await
}

pub async fn reset_terracotta_compat() -> eyre::Result<()> {
    let _operation = MULTIPLAYER_OPERATION.lock().await;
    terracotta::reset_state().await
}

pub async fn host(request: MultiplayerHostRequest) -> eyre::Result<()> {
    match request {
        MultiplayerHostRequest::Terracotta {
            player_name,
            room_code,
        } => {
            let _operation = MULTIPLAYER_OPERATION.lock().await;
            let already_running =
                terracotta::get_state().await.http_port.is_some();
            claim_provider(MultiplayerProvider::Terracotta).await?;
            let result = async {
                if !already_running {
                    terracotta::start_terracotta(None, true).await?;
                }
                terracotta::start_hosting(room_code, player_name).await
            }
            .await;
            if result.is_err() && !already_running {
                let _ = terracotta::stop_terracotta().await;
            }
            result
        }
        MultiplayerHostRequest::Hongshi {
            local_port,
            node_name,
            instance_id,
        } => {
            let pending = {
                let _operation = MULTIPLAYER_OPERATION.lock().await;
                hongshi::begin_start(local_port, node_name, instance_id).await?
            };
            pending.wait().await
        }
    }
}

pub async fn join(request: MultiplayerJoinRequest) -> eyre::Result<()> {
    let _operation = MULTIPLAYER_OPERATION.lock().await;
    if request.provider != MultiplayerProvider::Terracotta {
        bail!(
            "RedStone guests connect directly with the public server address"
        );
    }
    let already_running = terracotta::get_state().await.http_port.is_some();
    claim_provider(MultiplayerProvider::Terracotta).await?;
    let result = async {
        if !already_running {
            terracotta::start_terracotta(None, true).await?;
        }
        terracotta::start_joining(request.room_code, request.player_name).await
    }
    .await;
    if result.is_err() && !already_running {
        let _ = terracotta::stop_terracotta().await;
    }
    result
}

pub async fn stop() -> eyre::Result<()> {
    let _operation = MULTIPLAYER_OPERATION.lock().await;
    let provider = { *ACTIVE_PROVIDER.lock().await };
    if let Some(provider) = provider {
        stop_provider(provider).await?;
    }
    Ok(())
}

pub async fn reset() -> eyre::Result<()> {
    let _operation = MULTIPLAYER_OPERATION.lock().await;
    let provider = { *ACTIVE_PROVIDER.lock().await };
    match provider {
        Some(MultiplayerProvider::Terracotta) => {
            terracotta::reset_state().await
        }
        Some(MultiplayerProvider::Hongshi) => hongshi::stop().await,
        None => Ok(()),
    }
}

pub async fn observe_minecraft_log(
    instance_id: &str,
    instance_name: &str,
    process_id: &str,
    message: &str,
) {
    hongshi::observe_minecraft_log(
        instance_id,
        instance_name,
        process_id,
        message,
    )
    .await;
}

pub async fn minecraft_process_finished(instance_id: &str) {
    hongshi::minecraft_process_finished(instance_id).await;
}

pub async fn shutdown() -> eyre::Result<()> {
    SHUTTING_DOWN.store(true, Ordering::Relaxed);
    stop().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn active_provider_snapshot_does_not_hold_the_mutex() {
        let _serial = SESSION_TEST.lock().await;
        *ACTIVE_PROVIDER.lock().await = Some(MultiplayerProvider::Hongshi);
        let provider = { *ACTIVE_PROVIDER.lock().await };
        let mut active = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            ACTIVE_PROVIDER.lock(),
        )
        .await
        .expect("provider mutex should be released before stopping");
        assert_eq!(provider, Some(MultiplayerProvider::Hongshi));
        *active = None;
    }

    #[tokio::test]
    async fn queued_provider_claim_is_rejected_after_shutdown_begins() {
        let _serial = SESSION_TEST.lock().await;
        let active = ACTIVE_PROVIDER.lock().await;
        let (started, entered) = tokio::sync::oneshot::channel();
        let claim = tokio::spawn(async move {
            let _ = started.send(());
            claim_provider(MultiplayerProvider::Hongshi).await
        });
        entered.await.unwrap();
        assert!(!claim.is_finished());
        SHUTTING_DOWN.store(true, Ordering::Relaxed);
        drop(active);
        let result = claim.await.unwrap();
        SHUTTING_DOWN.store(false, Ordering::Relaxed);
        assert!(result.unwrap_err().to_string().contains("shutting down"));
        assert!(ACTIVE_PROVIDER.lock().await.is_none());
    }
}
