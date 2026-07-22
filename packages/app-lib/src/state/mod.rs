//! Theseus state management system
use crate::util::fetch::{FetchSemaphore, IoSemaphore};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use tokio::sync::{OnceCell, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::state::instances::watcher::FileWatcher;
use sqlx::SqlitePool;

// Submodules
mod dirs;
pub use self::dirs::*;

mod instance_types;
pub use self::instance_types::*;

pub(crate) mod instances;
pub use self::instances::*;

mod settings;
pub use self::settings::*;

mod process;
pub use self::process::*;

mod java_globals;
pub use self::java_globals::*;

mod discord;
pub use self::discord::*;

mod minecraft_auth;
pub use self::minecraft_auth::*;

pub mod minecraft_skins;

mod cache;
pub use self::cache::*;

mod friends;
pub use self::friends::*;

mod tunnel;
pub use self::tunnel::*;

pub mod db;
pub(crate) mod db_backup;
mod mr_auth;

pub use self::mr_auth::*;

mod legacy_converter;

pub mod attached_world_data;
pub mod server_join_log;

// Global state
// RwLock on state only has concurrent reads, except for config dir change which takes control of the State
static LAUNCHER_STATE: OnceCell<Arc<State>> = OnceCell::const_new();
const MAX_CONCURRENT_INSTALL_JOBS: usize = 1;
pub struct State {
    /// Information on the location of files used in the launcher
    pub directories: DirectoryInfo,

    /// Semaphore used to limit concurrent network requests and avoid errors
    pub fetch_semaphore: FetchSemaphore,
    /// Global capacity for file transfers. Metadata and API requests use their
    /// own semaphores so they cannot delay an active installation.
    pub download_semaphore: FetchSemaphore,
    /// Semaphore used to limit concurrent I/O and avoid errors
    pub io_semaphore: IoSemaphore,
    /// Semaphore to limit concurrent API requests. This is separate from the fetch semaphore
    /// to keep API functionality while the app is performing intensive tasks.
    pub api_semaphore: FetchSemaphore,
    minecraft_metadata_source: AtomicU8,
    minecraft_file_source: AtomicU8,
    modrinth_source: AtomicU8,
    curseforge_source: AtomicU8,
    download_concurrency_target: AtomicUsize,
    download_concurrency_limit: AtomicUsize,
    fetch_concurrency_limit: AtomicUsize,
    api_concurrency_limit: AtomicUsize,
    pub(crate) install_job_semaphore: Semaphore,
    pub(crate) install_db_semaphore: Semaphore,
    pub(crate) install_job_cancellations: DashMap<Uuid, CancellationToken>,

    /// Discord RPC
    pub discord_rpc: DiscordGuard,

    /// Process manager
    pub process_manager: ProcessManager,

    // NOTE: we explicitly must NOT store the app identifier in the state object,
    // because creating the state object is fallible (e.g. database missing),
    // but we rely on the app identifier to create the state (data dir).
    //
    // /// App identifier string (like com.modrinth.AxolotlLauncher)
    // pub app_identifier: String,
    /// Friends socket
    pub friends_socket: FriendsSocket,

    pub restart_after_pending_update: AtomicBool,

    pub(crate) pool: SqlitePool,

    pub(crate) file_watcher: FileWatcher,
}

fn grow_semaphore(
    semaphore: &Semaphore,
    current_limit: &AtomicUsize,
    target: usize,
) {
    let mut current = current_limit.load(Ordering::Acquire);

    while current < target {
        match current_limit.compare_exchange(
            current,
            target,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                semaphore.add_permits(target - current);
                return;
            }
            Err(updated) => current = updated,
        }
    }
}

async fn shrink_semaphore(
    semaphore: &Semaphore,
    current_limit: &AtomicUsize,
    target: &AtomicUsize,
) {
    loop {
        if current_limit.load(Ordering::Acquire)
            <= target.load(Ordering::Acquire)
        {
            return;
        }

        let Ok(permit) = semaphore.acquire().await else {
            return;
        };

        loop {
            let current = current_limit.load(Ordering::Acquire);
            if current <= target.load(Ordering::Acquire) {
                drop(permit);
                return;
            }

            if current_limit
                .compare_exchange(
                    current,
                    current - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                permit.forget();
                break;
            }
        }
    }
}

impl State {
    pub async fn init(app_identifier: String) -> crate::Result<()> {
        let state = LAUNCHER_STATE
            .get_or_try_init(move || Self::initialize_state(app_identifier))
            .await?;

        if let Err(e) =
            crate::install::recovery::recover_interrupted_jobs(state).await
        {
            tracing::error!("Error recovering interrupted install jobs: {e}");
        }

        tokio::task::spawn(async move {
            instances::watcher::watch_instances_init(
                &state.file_watcher,
                &state.directories,
                &state.pool,
            )
            .await;

            let res = tokio::try_join!(
                state.discord_rpc.clear_to_default(true),
                instances::refresh_all_instances(),
                Settings::migrate(&state.pool),
                ModrinthCredentials::refresh_all(),
            );

            if let Err(e) = res {
                tracing::error!("Error running discord RPC: {e}");
            }

            // Axolotl does not connect to Modrinth's private friends socket.
        });

        Ok(())
    }

    /// Get the current launcher state, waiting for initialization
    pub async fn get() -> crate::Result<Arc<Self>> {
        if !LAUNCHER_STATE.initialized() {
            tracing::error!(
                "Attempted to get state before it is initialized - this should never happen!"
            );
            while !LAUNCHER_STATE.initialized() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        Ok(Arc::clone(
            LAUNCHER_STATE.get().expect("State is not initialized!"),
        ))
    }

    pub fn initialized() -> bool {
        LAUNCHER_STATE.initialized()
    }

    pub(crate) fn minecraft_metadata_source(&self) -> DownloadSourceMode {
        DownloadSourceMode::from_u8(
            self.minecraft_metadata_source.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn minecraft_file_source(&self) -> DownloadSourceMode {
        DownloadSourceMode::from_u8(
            self.minecraft_file_source.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn modrinth_source(&self) -> DownloadSourceMode {
        DownloadSourceMode::from_u8(
            self.modrinth_source.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn curseforge_source(&self) -> DownloadSourceMode {
        DownloadSourceMode::from_u8(
            self.curseforge_source.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn download_concurrency(&self) -> usize {
        self.download_concurrency_target.load(Ordering::Acquire)
    }

    pub(crate) fn update_download_settings(
        self: &Arc<Self>,
        settings: &Settings,
    ) {
        self.minecraft_metadata_source
            .store(settings.minecraft_metadata_source as u8, Ordering::Relaxed);
        self.minecraft_file_source
            .store(settings.minecraft_file_source as u8, Ordering::Relaxed);
        self.modrinth_source
            .store(settings.modrinth_source as u8, Ordering::Relaxed);
        self.curseforge_source
            .store(settings.curseforge_source as u8, Ordering::Relaxed);
        self.resize_download_concurrency(
            settings.effective_max_concurrent_downloads(),
        );
    }

    fn resize_download_concurrency(self: &Arc<Self>, target: usize) {
        let target = target.clamp(1, 256);
        self.download_concurrency_target
            .store(target, Ordering::Release);

        grow_semaphore(
            &self.fetch_semaphore.0,
            &self.fetch_concurrency_limit,
            target,
        );
        grow_semaphore(
            &self.download_semaphore.0,
            &self.download_concurrency_limit,
            target,
        );
        grow_semaphore(
            &self.api_semaphore.0,
            &self.api_concurrency_limit,
            target,
        );

        if self.fetch_concurrency_limit.load(Ordering::Acquire) > target {
            let state = Arc::clone(self);
            tokio::spawn(async move {
                shrink_semaphore(
                    &state.fetch_semaphore.0,
                    &state.fetch_concurrency_limit,
                    &state.download_concurrency_target,
                )
                .await;
            });
        }

        if self.download_concurrency_limit.load(Ordering::Acquire) > target {
            let state = Arc::clone(self);
            tokio::spawn(async move {
                shrink_semaphore(
                    &state.download_semaphore.0,
                    &state.download_concurrency_limit,
                    &state.download_concurrency_target,
                )
                .await;
            });
        }

        if self.api_concurrency_limit.load(Ordering::Acquire) > target {
            let state = Arc::clone(self);
            tokio::spawn(async move {
                shrink_semaphore(
                    &state.api_semaphore.0,
                    &state.api_concurrency_limit,
                    &state.download_concurrency_target,
                )
                .await;
            });
        }
    }

    pub fn get_if_initialized() -> Option<Arc<Self>> {
        LAUNCHER_STATE.get().map(Arc::clone)
    }

    #[tracing::instrument]
    async fn initialize_state(
        app_identifier: String,
    ) -> crate::Result<Arc<Self>> {
        tracing::info!("Connecting to app database");
        let pool = db::connect(&app_identifier).await?;

        legacy_converter::migrate_legacy_data(&pool).await?;

        tracing::info!("Fetching app settings");
        let mut settings = Settings::get(&pool).await?;
        let download_concurrency =
            settings.effective_max_concurrent_downloads();
        let fetch_semaphore =
            FetchSemaphore(Semaphore::new(download_concurrency));
        let download_semaphore =
            FetchSemaphore(Semaphore::new(download_concurrency));
        let io_semaphore =
            IoSemaphore(Semaphore::new(settings.max_concurrent_writes));
        let api_semaphore =
            FetchSemaphore(Semaphore::new(download_concurrency));

        tracing::info!("Initializing directories");
        DirectoryInfo::move_launcher_directory(
            &mut settings,
            &pool,
            &io_semaphore,
            &app_identifier,
        )
        .await?;

        let directories =
            DirectoryInfo::init(settings.custom_dir, &app_identifier).await?;

        let discord_rpc = DiscordGuard::init()?;

        tracing::info!("Initializing file watcher");
        let file_watcher = instances::watcher::init_watcher().await?;

        let process_manager = ProcessManager::new();

        let friends_socket = FriendsSocket::new();

        Ok(Arc::new(Self {
            directories,
            fetch_semaphore,
            download_semaphore,
            io_semaphore,
            api_semaphore,
            minecraft_metadata_source: AtomicU8::new(
                settings.minecraft_metadata_source as u8,
            ),
            minecraft_file_source: AtomicU8::new(
                settings.minecraft_file_source as u8,
            ),
            modrinth_source: AtomicU8::new(settings.modrinth_source as u8),
            curseforge_source: AtomicU8::new(settings.curseforge_source as u8),
            download_concurrency_target: AtomicUsize::new(download_concurrency),
            download_concurrency_limit: AtomicUsize::new(download_concurrency),
            fetch_concurrency_limit: AtomicUsize::new(download_concurrency),
            api_concurrency_limit: AtomicUsize::new(download_concurrency),
            install_job_semaphore: Semaphore::new(MAX_CONCURRENT_INSTALL_JOBS),
            install_db_semaphore: Semaphore::new(1),
            install_job_cancellations: DashMap::new(),
            discord_rpc,
            process_manager,
            friends_socket,
            restart_after_pending_update: AtomicBool::new(false),
            pool,
            file_watcher,
            // app_identifier,
        }))
    }
}
