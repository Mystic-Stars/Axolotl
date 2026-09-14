use super::model::{
    ActiveDownloadState, DownloadItemStatus, InstallContinuationState,
    InstallErrorContext, InstallJobEventKind, InstallJobSnapshot,
    InstallJobState, InstallParallelProgress, InstallPauseReason,
    InstallPhaseDetails, InstallPhaseId, InstallProgress, InstallRollbackState,
    InstanceUpgradeResult, MissingModpackContentState,
};
use super::store;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

// Keep per-file progress responsive without emitting every network chunk.
const LIVE_PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(100);
const LIVE_PROGRESS_MIN_BYTES: u64 = 64 * 1024;
const CONTENT_CHECKPOINT_FILE_COUNT: usize = 25;
const CONTENT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(2);

static REPORTER_STATES: LazyLock<
    dashmap::DashMap<Uuid, Weak<Mutex<InstallProgressReporterState>>>,
> = LazyLock::new(dashmap::DashMap::new);

#[derive(Clone, Debug)]
pub struct InstallProgressReporter {
    job_id: Uuid,
    state: Arc<Mutex<InstallProgressReporterState>>,
    /// When set, `update`/`update_with_events` write to the parallel track
    /// (`InstallProgressState.parallel`) instead of the main phase. Used by
    /// the modpack installer to report the concurrent Minecraft core download
    /// while content installation remains the main phase.
    parallel_output: bool,
}

#[derive(Debug)]
struct InstallProgressReporterState {
    job: InstallJobState,
    last_snapshot: Option<InstallJobSnapshot>,
    last_persisted_at: Instant,
    last_persisted_progress: Option<(InstallPhaseId, u64)>,
    settled_files_since_checkpoint: usize,
    checkpoint_scheduled: bool,
    checkpoint_in_flight: bool,
    initialized_from_store: bool,
    postponed_java_versions: HashSet<u32>,
    /// Per-download event throttles. A global throttle makes one busy file
    /// suppress progress events for every other active download.
    last_live_emit_at: HashMap<String, Instant>,
    /// Paths with a pending stalled-download check task, so at most one
    /// delayed check is scheduled per active download at a time.
    pending_stall_checks: HashSet<String>,
}

#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DownloadRequestUpdate {
    Started {
        job_id: Uuid,
        id: String,
        name: String,
        url: String,
        source: String,
        bytes_total: Option<u64>,
        attempt: u32,
        max_attempts: u32,
    },
    Progress {
        job_id: Uuid,
        id: String,
        bytes: u64,
        status: DownloadItemStatus,
        speed_bytes_per_second: Option<u64>,
        eta_seconds: Option<u64>,
    },
    Finished {
        job_id: Uuid,
        id: String,
        bytes: u64,
    },
    Failed {
        job_id: Uuid,
        id: String,
    },
}

impl InstallProgressReporter {
    pub(crate) fn reset_job(job_id: Uuid) {
        REPORTER_STATES.remove(&job_id);
    }

    pub(crate) fn cancellation_token(&self) -> CancellationToken {
        crate::State::get_if_initialized()
            .and_then(|state| {
                state
                    .install_job_cancellations
                    .get(&self.job_id)
                    .map(|entry| entry.value().clone())
            })
            .unwrap_or_else(CancellationToken::new)
    }

    pub(crate) fn job_id(&self) -> Uuid {
        self.job_id
    }

    /// Overlays the in-memory reporter state on a database snapshot. Progress
    /// and active download details are intentionally runtime state and can be
    /// newer than the last checkpoint written to SQLite.
    pub(crate) async fn overlay_snapshot(
        job_id: Uuid,
        mut snapshot: InstallJobSnapshot,
    ) -> Option<InstallJobSnapshot> {
        let state = REPORTER_STATES.get(&job_id)?.upgrade()?;
        let state = state.lock().await;
        snapshot.phase = state.job.progress.phase;
        snapshot.progress = state.job.progress.progress.clone();
        snapshot.details = state.job.progress.details.clone();
        snapshot.parallel = state.job.progress.parallel.clone();
        snapshot.display = state.job.display.clone();
        snapshot.error = state.job.error.clone();
        snapshot.rollback_error = state.job.rollback_error.clone();
        snapshot.pause_reason = state.job.pause_reason.clone();
        snapshot.upgrade_result = state.job.upgrade_result.clone();
        snapshot.summary = state.job.download_summary();
        snapshot.items = state.job.download_items();
        Some(snapshot)
    }

    pub fn new(job_id: Uuid, mut state: InstallJobState) -> Self {
        state.compact_transient_download_events();
        let shared_state = match REPORTER_STATES.entry(job_id) {
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                if let Some(state) = entry.get().upgrade() {
                    state
                } else {
                    let state =
                        Arc::new(Mutex::new(InstallProgressReporterState {
                            job: state,
                            last_snapshot: None,
                            last_persisted_at: Instant::now(),
                            last_persisted_progress: None,
                            settled_files_since_checkpoint: 0,
                            checkpoint_scheduled: false,
                            checkpoint_in_flight: false,
                            initialized_from_store: false,
                            postponed_java_versions: HashSet::new(),
                            last_live_emit_at: HashMap::new(),
                            pending_stall_checks: HashSet::new(),
                        }));
                    entry.insert(Arc::downgrade(&state));
                    state
                }
            }
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                let state =
                    Arc::new(Mutex::new(InstallProgressReporterState {
                        job: state,
                        last_snapshot: None,
                        last_persisted_at: Instant::now(),
                        last_persisted_progress: None,
                        settled_files_since_checkpoint: 0,
                        checkpoint_scheduled: false,
                        checkpoint_in_flight: false,
                        initialized_from_store: false,
                        postponed_java_versions: HashSet::new(),
                        last_live_emit_at: HashMap::new(),
                        pending_stall_checks: HashSet::new(),
                    }));
                entry.insert(Arc::downgrade(&state));
                state
            }
        };
        Self {
            job_id,
            state: shared_state,
            parallel_output: false,
        }
    }

    /// Returns a reporter clone whose phase updates are routed to the
    /// parallel track, leaving the main phase untouched. The underlying job
    /// state is shared, so both tracks still persist to the same job.
    pub fn with_parallel_output(mut self) -> Self {
        self.parallel_output = true;
        self
    }

    pub async fn update(
        &self,
        phase: InstallPhaseId,
        progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
    ) -> crate::Result<()> {
        if self.parallel_output {
            self.update_parallel(phase, progress, details).await
        } else {
            self.update_main(phase, progress, details).await
        }
    }

    async fn update_main(
        &self,
        phase: InstallPhaseId,
        progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
    ) -> crate::Result<()> {
        self.update_with_events(phase, progress, details, Vec::new())
            .await
    }

    /// Updates the parallel track while the main phase keeps its own
    /// progress. The parallel track lives on `InstallProgressState` and is
    /// preserved across main-phase progress updates.
    async fn update_parallel(
        &self,
        phase: InstallPhaseId,
        progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
    ) -> crate::Result<()> {
        let app_state = match crate::State::get().await {
            Ok(app_state) => app_state,
            Err(error) => {
                tracing::warn!(%error, "Failed to access install progress store");
                return Ok(());
            }
        };
        let mut state = self.state.lock().await;
        if let Err(error) = self.sync_latest(&mut state, &app_state).await {
            tracing::warn!(%error, "Failed to load install progress state");
            return Ok(());
        }
        let (current, total) = match &progress {
            Some(progress) => (progress.current, progress.total),
            None => state
                .job
                .progress
                .parallel
                .as_ref()
                .map(|parallel| (parallel.current, parallel.total))
                .unwrap_or((0, 0)),
        };
        state.job.progress.parallel = Some(InstallParallelProgress {
            phase,
            current,
            total,
            details,
        });
        let Some(snapshot) = runtime_snapshot(&state) else {
            return Ok(());
        };
        drop(state);
        emit_install_job(&snapshot).await?;
        Ok(())
    }

    pub async fn set_context(
        &self,
        context: InstallErrorContext,
    ) -> crate::Result<()> {
        self.update_context(Some(context), true).await
    }

    pub async fn set_transient_context(
        &self,
        context: InstallErrorContext,
    ) -> crate::Result<()> {
        self.update_context(Some(context), false).await
    }

    pub async fn clear_context(&self) -> crate::Result<()> {
        self.update_context(None, true).await
    }

    pub async fn is_java_download_postponed(&self, version: u32) -> bool {
        self.state
            .lock()
            .await
            .postponed_java_versions
            .contains(&version)
    }

    pub async fn postpone_java_download(&self, version: u32) {
        self.state
            .lock()
            .await
            .postponed_java_versions
            .insert(version);
    }

    async fn sync_latest(
        &self,
        state: &mut InstallProgressReporterState,
        app_state: &crate::State,
    ) -> crate::Result<()> {
        if !state.initialized_from_store {
            let record = store::get_required(self.job_id, app_state).await?;
            state.job = record.state.clone();
            state.last_snapshot = Some(record.snapshot());
            state.job.compact_transient_download_events();
            state.initialized_from_store = true;
        }
        Ok(())
    }

    async fn update_context(
        &self,
        context: Option<InstallErrorContext>,
        persist: bool,
    ) -> crate::Result<()> {
        let app_state = if persist {
            Some(crate::State::get().await?)
        } else {
            None
        };
        let mut state = self.state.lock().await;
        if let Some(app_state) = &app_state {
            self.sync_latest(&mut state, app_state).await?;
        }
        state.job.set_context(context);

        let Some(app_state) = app_state else {
            return Ok(());
        };

        let record = match store::update_state(
            self.job_id,
            &state.job,
            &app_state,
        )
        .await
        {
            Ok(record) => record,
            Err(error) => {
                tracing::warn!(%error, "Failed to persist install context");
                return Ok(());
            }
        };
        state.mark_persisted();
        if let Err(error) = emit_install_job(&record.snapshot()).await {
            tracing::warn!(%error, "Failed to emit install context");
        }
        Ok(())
    }

    pub async fn persist(&self) -> crate::Result<InstallJobSnapshot> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;

        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        state.last_snapshot = Some(record.snapshot());
        let snapshot = record.snapshot();
        emit_install_job(&snapshot).await?;
        Ok(snapshot)
    }

    pub async fn current_state(&self) -> crate::Result<InstallJobState> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        Ok(state.job.clone())
    }

    pub async fn set_continuation(
        &self,
        continuation: Option<InstallContinuationState>,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        self.sync_latest(&mut state, &app_state).await?;
        state.job.continuation = continuation;
        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        emit_install_job(&record.snapshot()).await
    }

    pub async fn set_missing_content(
        &self,
        missing_content: Option<MissingModpackContentState>,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        state.job.missing_content = missing_content;
        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        emit_install_job(&record.snapshot()).await
    }

    pub async fn set_upgrade_result(
        &self,
        result: InstanceUpgradeResult,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        state.job.upgrade_result = Some(result);
        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        emit_install_job(&record.snapshot()).await
    }

    pub async fn record_events(
        &self,
        events: Vec<InstallJobEventKind>,
    ) -> crate::Result<InstallJobSnapshot> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        let refresh_missing_reason = events.iter().any(|event| {
            matches!(
                event,
                InstallJobEventKind::ContentFileRecovered { .. }
                    | InstallJobEventKind::ContentFileFailed { .. }
            )
        });
        for event in events {
            state.job.record_event(event);
        }
        if refresh_missing_reason {
            refresh_missing_pause_reason(&mut state.job);
        }
        let Some(snapshot) = runtime_snapshot(&state) else {
            return Err(crate::ErrorKind::OtherError(
                "install progress snapshot unavailable".to_string(),
            )
            .into());
        };
        drop(state);
        emit_install_job(&snapshot).await?;
        Ok(snapshot)
    }

    pub(crate) async fn set_rollback(
        &self,
        rollback: Option<InstallRollbackState>,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        state.job.rollback = rollback;
        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        emit_install_job(&record.snapshot()).await
    }

    pub async fn track_rollback_paths(
        &self,
        paths: Vec<String>,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        let Some(snapshot) = state
            .job
            .rollback
            .as_mut()
            .and_then(|rollback| rollback.content.as_mut())
        else {
            return Ok(());
        };
        for path in paths {
            path_util::SafeRelativeUtf8UnixPathBuf::try_from(path.clone())?;
            if !snapshot.replacement_paths.contains(&path) {
                snapshot.replacement_paths.push(path);
            }
        }
        snapshot.replacement_paths.sort();
        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        emit_install_job(&record.snapshot()).await
    }

    pub async fn persist_failure_context(&self, context: InstallErrorContext) {
        if let Err(error) = self.update_context(Some(context), true).await {
            tracing::warn!(
                "Failed to persist install context for failed operation: {error}"
            );
        }
    }

    pub async fn record_download_metrics(
        &self,
        source: impl Into<String>,
        fallback_count: u64,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;

        state
            .job
            .record_event(InstallJobEventKind::DownloadMetrics {
                source: source.into(),
                fallback_count,
            });
        if let Some(snapshot) = runtime_snapshot(&state) {
            drop(state);
            emit_install_job(&snapshot).await?;
        }
        Ok(())
    }

    pub async fn record_download_request(
        &self,
        path: impl Into<String>,
        name: impl Into<String>,
        url: impl Into<String>,
        source: impl Into<String>,
        bytes_total: Option<u64>,
        attempt: u32,
        max_attempts: u32,
    ) -> crate::Result<()> {
        let path = path.into();
        let name = name.into();
        let url = url.into();
        let source = source.into();
        self.record_live_event(
            InstallJobEventKind::DownloadRequestStarted {
                path: path.clone(),
                name: name.clone(),
                url: url.clone(),
                source: source.clone(),
                bytes_total,
                attempt,
                max_attempts,
            },
            DownloadRequestUpdate::Started {
                job_id: self.job_id,
                id: path,
                name,
                url,
                source,
                bytes_total,
                attempt,
                max_attempts,
            },
        )
        .await
    }

    pub async fn record_download_progress(
        &self,
        path: impl Into<String>,
        bytes: u64,
        bytes_total: u64,
    ) -> crate::Result<()> {
        let path = path.into();
        // Progress reporting runs in the socket read path. Never make network
        // workers wait for another file's reporter update; a later sample
        // carries the cumulative byte count and catches the UI up.
        let Ok(mut state) = self.state.try_lock() else {
            return Ok(());
        };
        if !state.initialized_from_store {
            return Ok(());
        }
        let now = Utc::now();
        let emit_too_soon = state
            .last_live_emit_at
            .get(&path)
            .is_some_and(|last| last.elapsed() < LIVE_PROGRESS_EMIT_INTERVAL);
        let Some(active) = state.job.active_downloads.get_mut(&path) else {
            return Ok(());
        };
        let previous_bytes = active.bytes_downloaded;
        if bytes < previous_bytes {
            active.speed_bytes_per_second = None;
            active.speed_sample_started_at = now;
            active.speed_sample_started_bytes = bytes;
        } else if bytes > previous_bytes {
            active.last_progress_at = now;
            let sample_elapsed_ms = now
                .signed_duration_since(active.speed_sample_started_at)
                .num_milliseconds()
                .max(1) as u64;
            if sample_elapsed_ms >= 250 {
                let sample = bytes
                    .saturating_sub(active.speed_sample_started_bytes)
                    .saturating_mul(1_000)
                    .checked_div(sample_elapsed_ms)
                    .unwrap_or(0);
                active.speed_bytes_per_second = Some(sample);
                active.speed_sample_started_at = now;
                active.speed_sample_started_bytes = bytes;
            }
        }
        active.bytes_downloaded = bytes;
        active.bytes_total = Some(bytes_total);
        active.status = DownloadItemStatus::Downloading;

        let threshold = LIVE_PROGRESS_MIN_BYTES.max(bytes_total / 200);
        if bytes.saturating_sub(active.last_reported_bytes) < threshold
            || emit_too_soon
        {
            return Ok(());
        }
        active.last_reported_bytes = bytes;
        state.last_live_emit_at.insert(path.clone(), Instant::now());
        let (speed_bytes_per_second, eta_seconds) =
            live_download_metrics(&state.job);
        let schedule_stall_check =
            state.pending_stall_checks.insert(path.clone());
        drop(state);
        emit_download_request_update(&DownloadRequestUpdate::Progress {
            job_id: self.job_id,
            id: path.clone(),
            bytes,
            status: DownloadItemStatus::Downloading,
            speed_bytes_per_second,
            eta_seconds,
        })
        .await?;

        if schedule_stall_check {
            let reporter = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(3)).await;
                if let Err(error) = reporter
                    .record_download_stalled_if_unchanged(path.clone(), bytes)
                    .await
                {
                    tracing::warn!(%error, "Failed to record stalled download");
                }
                if let Ok(mut state) = reporter.state.try_lock() {
                    state.pending_stall_checks.remove(&path);
                }
            });
        }
        Ok(())
    }

    pub async fn record_download_stage(
        &self,
        path: impl Into<String>,
        status: DownloadItemStatus,
    ) -> crate::Result<()> {
        let path = path.into();
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        let Some(active) = state.job.active_downloads.get_mut(&path) else {
            return Ok(());
        };
        active.status = status;
        let bytes = active.bytes_downloaded;
        let (speed_bytes_per_second, eta_seconds) =
            live_download_metrics(&state.job);
        state.last_live_emit_at.insert(path.clone(), Instant::now());
        drop(state);
        emit_download_request_update(&DownloadRequestUpdate::Progress {
            job_id: self.job_id,
            id: path,
            bytes,
            status,
            speed_bytes_per_second,
            eta_seconds,
        })
        .await
    }

    pub async fn record_download_request_finished(
        &self,
        path: impl Into<String>,
        bytes: u64,
    ) -> crate::Result<()> {
        let path = path.into();
        self.record_live_event(
            InstallJobEventKind::DownloadRequestFinished {
                path: path.clone(),
                bytes,
            },
            DownloadRequestUpdate::Finished {
                job_id: self.job_id,
                id: path,
                bytes,
            },
        )
        .await
    }

    pub async fn record_download_request_failed(
        &self,
        path: impl Into<String>,
    ) -> crate::Result<()> {
        let path = path.into();
        self.record_live_event(
            InstallJobEventKind::DownloadRequestFailed { path: path.clone() },
            DownloadRequestUpdate::Failed {
                job_id: self.job_id,
                id: path,
            },
        )
        .await
    }

    async fn record_download_stalled_if_unchanged(
        &self,
        path: String,
        bytes: u64,
    ) -> crate::Result<()> {
        let mut state = self.state.lock().await;
        let Some(active) = state.job.active_downloads.get_mut(&path) else {
            return Ok(());
        };
        if active.bytes_downloaded != bytes
            || Utc::now()
                .signed_duration_since(active.last_progress_at)
                .num_milliseconds()
                < 3_000
        {
            return Ok(());
        }
        active.speed_bytes_per_second = None;
        let status = active.status;
        let (speed_bytes_per_second, eta_seconds) =
            live_download_metrics(&state.job);
        drop(state);
        emit_download_request_update(&DownloadRequestUpdate::Progress {
            job_id: self.job_id,
            id: path,
            bytes,
            status,
            speed_bytes_per_second,
            eta_seconds,
        })
        .await
    }

    async fn record_live_event(
        &self,
        event: InstallJobEventKind,
        update: DownloadRequestUpdate,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        match &event {
            InstallJobEventKind::DownloadRequestStarted {
                path,
                name,
                url,
                source,
                bytes_total,
                attempt,
                max_attempts,
            } => {
                state.job.active_downloads.insert(
                    path.clone(),
                    ActiveDownloadState {
                        name: name.clone(),
                        url: url.clone(),
                        source: source.clone(),
                        bytes_downloaded: 0,
                        bytes_total: *bytes_total,
                        attempt: *attempt,
                        max_attempts: *max_attempts,
                        status: DownloadItemStatus::Downloading,
                        last_reported_bytes: 0,
                        last_progress_at: Utc::now(),
                        speed_bytes_per_second: None,
                        speed_sample_started_at: Utc::now(),
                        speed_sample_started_bytes: 0,
                    },
                );
            }
            InstallJobEventKind::DownloadRequestFinished { path, .. }
            | InstallJobEventKind::DownloadRequestFailed { path } => {
                state.job.active_downloads.remove(path);
                state.last_live_emit_at.remove(path);
            }
            _ => {}
        }
        drop(state);
        emit_download_request_update(&update).await
    }

    pub async fn preserve_failure_context<T>(
        &self,
        context: InstallErrorContext,
        result: crate::Result<T>,
    ) -> crate::Result<T> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                self.persist_failure_context(context).await;
                Err(error)
            }
        }
    }

    pub async fn update_with_events(
        &self,
        phase: InstallPhaseId,
        progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
        events: Vec<InstallJobEventKind>,
    ) -> crate::Result<()> {
        let app_state = match crate::State::get().await {
            Ok(app_state) => app_state,
            Err(error) => {
                tracing::warn!(%error, "Failed to access install progress store");
                return Ok(());
            }
        };
        let mut state = self.state.lock().await;
        if let Err(error) = self.sync_latest(&mut state, &app_state).await {
            tracing::warn!(%error, "Failed to load install progress state");
            return Ok(());
        }
        let settled_files = events
            .iter()
            .filter(|event| is_content_settlement_event(event))
            .count();
        state.job.set_progress(phase, progress, details);
        for event in events {
            state.job.record_event(event);
        }
        state.settled_files_since_checkpoint = state
            .settled_files_since_checkpoint
            .saturating_add(settled_files);

        let checkpoint_due = !state.checkpoint_in_flight
            && state.settled_files_since_checkpoint > 0
            && (state.settled_files_since_checkpoint
                >= CONTENT_CHECKPOINT_FILE_COUNT
                || state.last_persisted_at.elapsed()
                    >= CONTENT_CHECKPOINT_INTERVAL);
        let checkpoint = checkpoint_due.then(|| {
            state.checkpoint_in_flight = true;
            (state.job.clone(), state.settled_files_since_checkpoint)
        });
        let checkpoint_delay = state.schedule_checkpoint_if_needed();

        let Some(snapshot) = runtime_snapshot(&state) else {
            return Ok(());
        };
        drop(state);
        emit_install_job(&snapshot).await?;
        if let Some(delay) = checkpoint_delay {
            self.schedule_content_checkpoint(delay);
        }
        if let Some((checkpoint_state, settled_files)) = checkpoint {
            self.persist_content_checkpoint(
                &app_state,
                checkpoint_state,
                settled_files,
            )
            .await?;
        }
        Ok(())
    }

    async fn flush_content_checkpoint(&self) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        state.checkpoint_scheduled = false;
        if state.settled_files_since_checkpoint == 0
            || state.checkpoint_in_flight
        {
            return Ok(());
        }
        state.checkpoint_in_flight = true;
        let checkpoint_state = state.job.clone();
        let settled_files = state.settled_files_since_checkpoint;
        drop(state);
        self.persist_content_checkpoint(
            &app_state,
            checkpoint_state,
            settled_files,
        )
        .await
    }

    async fn persist_content_checkpoint(
        &self,
        app_state: &crate::State,
        checkpoint_state: InstallJobState,
        settled_files: usize,
    ) -> crate::Result<()> {
        // Do not hold the reporter mutex while SQLite is busy. Download
        // started/finished events can continue to mutate the runtime state and
        // release their network slots while this snapshot waits to persist.
        let persisted = store::checkpoint_running_state(
            self.job_id,
            &checkpoint_state,
            app_state,
        )
        .await;
        let mut state = self.state.lock().await;
        state.checkpoint_in_flight = false;
        match persisted {
            Ok(Some(record)) => {
                state.last_snapshot = Some(record.snapshot());
                state.mark_checkpoint_persisted(settled_files);
            }
            Ok(None) => {
                // Job finalization or pause won the database race. Never let a
                // delayed runtime checkpoint overwrite that authoritative
                // terminal state.
                state.settled_files_since_checkpoint = 0;
                state.checkpoint_scheduled = false;
            }
            Err(error) => {
                let retry_delay = state.schedule_checkpoint_if_needed();
                drop(state);
                if let Some(delay) = retry_delay {
                    self.schedule_content_checkpoint(delay);
                }
                return Err(error);
            }
        }
        let next_delay = state.schedule_checkpoint_if_needed();
        drop(state);
        if let Some(delay) = next_delay {
            self.schedule_content_checkpoint(delay);
        }
        Ok(())
    }

    fn schedule_content_checkpoint(&self, delay: Duration) {
        let reporter = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            if let Err(error) = reporter.flush_content_checkpoint().await {
                tracing::warn!(
                    job_id = %reporter.job_id,
                    %error,
                    "Failed to flush install content checkpoint"
                );
            }
        });
    }
}

fn runtime_snapshot(
    state: &InstallProgressReporterState,
) -> Option<InstallJobSnapshot> {
    let mut snapshot = state.last_snapshot.clone()?;
    snapshot.phase = state.job.progress.phase;
    snapshot.progress = state.job.progress.progress.clone();
    snapshot.details = state.job.progress.details.clone();
    snapshot.parallel = state.job.progress.parallel.clone();
    snapshot.display = state.job.display.clone();
    snapshot.error = state.job.error.clone();
    snapshot.rollback_error = state.job.rollback_error.clone();
    snapshot.pause_reason = state.job.pause_reason.clone();
    snapshot.upgrade_result = state.job.upgrade_result.clone();
    snapshot.summary = state.job.download_summary();
    snapshot.items = state.job.download_items();
    Some(snapshot)
}

fn refresh_missing_pause_reason(job: &mut InstallJobState) {
    if !matches!(
        job.pause_reason,
        Some(InstallPauseReason::MissingRequiredContent { .. })
    ) {
        return;
    }
    let items = job.download_items();
    let Some(content) = &job.missing_content else {
        let paths = items
            .iter()
            .filter(|item| {
                item.status == DownloadItemStatus::Skipped
                    && item.manual_url.is_some()
                    && item.project_id.is_some()
                    && item.version_id.is_some()
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        job.pause_reason = Some(InstallPauseReason::MissingRequiredContent {
            failed_files: paths.len() as u64,
            paths,
        });
        return;
    };
    let paths = content
        .files
        .iter()
        .filter(|file| {
            items.iter().any(|item| {
                item.id == file.item_id
                    && item.status == DownloadItemStatus::Failed
            })
        })
        .map(|file| file.item_id.clone())
        .collect::<Vec<_>>();
    job.pause_reason = Some(InstallPauseReason::MissingRequiredContent {
        failed_files: paths.len() as u64,
        paths,
    });
}

impl InstallProgressReporterState {
    fn mark_persisted(&mut self) {
        self.last_persisted_at = Instant::now();
        self.settled_files_since_checkpoint = 0;
        self.last_persisted_progress = self
            .job
            .progress
            .progress
            .as_ref()
            .map(|progress| (self.job.progress.phase, progress.current));
    }

    fn mark_checkpoint_persisted(&mut self, settled_files: usize) {
        self.last_persisted_at = Instant::now();
        self.settled_files_since_checkpoint = self
            .settled_files_since_checkpoint
            .saturating_sub(settled_files);
        self.last_persisted_progress = self
            .job
            .progress
            .progress
            .as_ref()
            .map(|progress| (self.job.progress.phase, progress.current));
    }

    fn schedule_checkpoint_if_needed(&mut self) -> Option<Duration> {
        if self.settled_files_since_checkpoint == 0
            || self.checkpoint_scheduled
            || self.checkpoint_in_flight
        {
            return None;
        }
        self.checkpoint_scheduled = true;
        Some(
            CONTENT_CHECKPOINT_INTERVAL
                .saturating_sub(self.last_persisted_at.elapsed()),
        )
    }
}

fn is_content_settlement_event(event: &InstallJobEventKind) -> bool {
    matches!(
        event,
        InstallJobEventKind::ContentFileCompleted { .. }
            | InstallJobEventKind::ContentFileRecovered { .. }
            | InstallJobEventKind::ContentFileSkipped { .. }
            | InstallJobEventKind::ContentFileFailed { .. }
    )
}

fn live_download_metrics(job: &InstallJobState) -> (Option<u64>, Option<u64>) {
    let summary = job.download_summary();
    (summary.speed_bytes_per_second, summary.eta_seconds)
}

#[allow(unused_variables)]
pub async fn emit_install_job(
    snapshot: &InstallJobSnapshot,
) -> crate::Result<()> {
    #[cfg(feature = "tauri")]
    {
        use tauri::Emitter;

        let event_state = crate::EventState::get()?;
        event_state
            .app
            .emit("install_job", snapshot)
            .map_err(crate::event::EventError::from)?;
    }

    Ok(())
}

#[allow(unused_variables)]
async fn emit_download_request_update(
    update: &DownloadRequestUpdate,
) -> crate::Result<()> {
    #[cfg(feature = "tauri")]
    {
        use tauri::Emitter;

        let event_state = crate::EventState::get()?;
        event_state
            .app
            .emit("download_request", update)
            .map_err(crate::event::EventError::from)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::pack::install_from::CreatePackLocation;
    use crate::install::InstallRequest;
    #[cfg(not(feature = "tauri"))]
    use crate::install::model::InstallProgress;
    #[cfg(not(feature = "tauri"))]
    use crate::install::model::InstallProgressSecondary;
    use crate::install::model::{
        InstallJobEventKind, InstallJobExecutionMode, InstallJobKind,
        InstallJobStatus, InstallPauseReason, InstallPhaseDetails,
        InstallPhaseId, MissingModpackContentState,
    };
    use crate::state::{InstanceLink, ModLoader};

    #[cfg(not(feature = "tauri"))]
    fn minecraft_details() -> InstallPhaseDetails {
        InstallPhaseDetails::Minecraft {
            game_version: "1.21.1".to_string(),
            loader: ModLoader::Vanilla,
        }
    }

    #[cfg(not(feature = "tauri"))]
    fn minecraft_progress(current: u64, total: u64) -> InstallProgress {
        InstallProgress {
            current,
            total,
            secondary: None,
        }
    }

    #[cfg(not(feature = "tauri"))]
    async fn stored_minecraft_progress_job(
        current: u64,
        total: u64,
    ) -> (std::sync::Arc<crate::State>, Uuid, InstallProgressReporter) {
        crate::event::EventState::init().await.unwrap();
        let root = tempfile::tempdir().unwrap().keep();
        let app_state =
            crate::State::init_for_test(root.to_string_lossy().to_string())
                .await
                .unwrap();
        let job_id = Uuid::new_v4();
        let mut job = InstallJobState::new(InstallRequest::CreateInstance {
            name: "Test".to_string(),
            game_version: "1.21.1".to_string(),
            loader: ModLoader::Vanilla,
            loader_version: None,
            adjuncts: Vec::new(),
            icon_path: None,
            link: InstanceLink::Unmanaged,
            game_dir_override: None,
        });
        job.set_progress(
            InstallPhaseId::DownloadingMinecraft,
            Some(minecraft_progress(current, total)),
            minecraft_details(),
        );
        store::insert(job_id, &job, InstallJobStatus::Running, &app_state)
            .await
            .unwrap();
        let reporter = InstallProgressReporter::new(job_id, job);
        (app_state, job_id, reporter)
    }

    #[test]
    fn separately_created_reporters_share_job_state() {
        let job_id = Uuid::new_v4();
        let state = InstallJobState::new(InstallRequest::CreateInstance {
            name: "Test".to_string(),
            game_version: "1.21.1".to_string(),
            loader: ModLoader::Vanilla,
            loader_version: None,
            adjuncts: Vec::new(),
            icon_path: None,
            link: InstanceLink::Unmanaged,
            game_dir_override: None,
        });

        let first = InstallProgressReporter::new(job_id, state.clone());
        let second = InstallProgressReporter::new(job_id, state);

        assert!(Arc::ptr_eq(&first.state, &second.state));
    }

    #[tokio::test]
    async fn postponed_java_download_is_shared_by_job_reporters() {
        let job_id = Uuid::new_v4();
        let state = InstallJobState::new(InstallRequest::CreateInstance {
            name: "Test".to_string(),
            game_version: "1.21.1".to_string(),
            loader: ModLoader::Vanilla,
            loader_version: None,
            adjuncts: Vec::new(),
            icon_path: None,
            link: InstanceLink::Unmanaged,
            game_dir_override: None,
        });
        let first = InstallProgressReporter::new(job_id, state.clone());
        let second = InstallProgressReporter::new(job_id, state);

        first.postpone_java_download(21).await;

        assert!(second.is_java_download_postponed(21).await);
        assert!(!second.is_java_download_postponed(17).await);
    }

    #[tokio::test]
    async fn download_progress_never_waits_for_reporter_lock() {
        let job_id = Uuid::new_v4();
        let reporter = InstallProgressReporter::new(
            job_id,
            InstallJobState::new(InstallRequest::CreateInstance {
                name: "Test".to_string(),
                game_version: "1.21.1".to_string(),
                loader: ModLoader::Vanilla,
                loader_version: None,
                adjuncts: Vec::new(),
                icon_path: None,
                link: InstanceLink::Unmanaged,
                game_dir_override: None,
            }),
        );
        let guard = reporter.state.lock().await;
        tokio::time::timeout(
            Duration::from_millis(50),
            reporter.record_download_progress("mods/test.jar", 64, 128),
        )
        .await
        .expect("socket progress callback must not wait for the reporter lock")
        .unwrap();
        drop(guard);
        InstallProgressReporter::reset_job(job_id);
    }

    #[tokio::test]
    async fn download_progress_speed_drops_without_retaining_the_peak() {
        let job_id = Uuid::new_v4();
        let reporter = InstallProgressReporter::new(
            job_id,
            InstallJobState::new(InstallRequest::CreateInstance {
                name: "Speed test".to_string(),
                game_version: "1.21.1".to_string(),
                loader: ModLoader::Vanilla,
                loader_version: None,
                adjuncts: Vec::new(),
                icon_path: None,
                link: InstanceLink::Unmanaged,
                game_dir_override: None,
            }),
        );
        let mib = 1024 * 1024;
        {
            let mut state = reporter.state.lock().await;
            state.initialized_from_store = true;
            state.job.active_downloads.insert(
                "file".to_string(),
                ActiveDownloadState {
                    name: "file".to_string(),
                    url: String::new(),
                    source: String::new(),
                    bytes_downloaded: 40 * mib,
                    bytes_total: Some(100 * mib),
                    attempt: 1,
                    max_attempts: 1,
                    status: DownloadItemStatus::Downloading,
                    last_reported_bytes: u64::MAX,
                    last_progress_at: Utc::now(),
                    speed_bytes_per_second: Some(40 * mib),
                    speed_sample_started_at: Utc::now()
                        - chrono::TimeDelta::seconds(1),
                    speed_sample_started_bytes: 40 * mib,
                },
            );
        }
        reporter
            .record_download_progress("file", 42 * mib, 100 * mib)
            .await
            .unwrap();
        let state = reporter.state.lock().await;
        let speed = state.job.active_downloads["file"]
            .speed_bytes_per_second
            .unwrap();
        assert!(speed > 0 && speed <= 2 * mib);
    }

    #[tokio::test]
    async fn reset_job_starts_resume_with_fresh_typed_state() {
        let job_id = Uuid::new_v4();
        let request = InstallRequest::CreateModpackInstance {
            location: CreatePackLocation::FromFile {
                path: "test.mrpack".into(),
            },
            post_install_edit: None,
        };
        let stale = InstallProgressReporter::new(
            job_id,
            InstallJobState::new(request.clone()),
        );
        let mut resumed = InstallJobState::new(request);
        resumed.missing_content = Some(MissingModpackContentState::default());
        resumed.set_progress(
            InstallPhaseId::DownloadingContent,
            None,
            InstallPhaseDetails::Empty,
        );
        resumed.record_event(InstallJobEventKind::WaitingForUser {
            reason: InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                paths: vec!["mods/missing.jar".to_string()],
            },
        });
        resumed.record_event(InstallJobEventKind::JobQueued {
            kind: InstallJobKind::CreateModpackInstance,
        });

        InstallProgressReporter::reset_job(job_id);
        let fresh = InstallProgressReporter::new(job_id, resumed);

        assert!(!Arc::ptr_eq(&stale.state, &fresh.state));
        assert_eq!(
            fresh
                .state
                .lock()
                .await
                .job
                .execution_mode(InstallJobStatus::Running),
            InstallJobExecutionMode::RecoveryValidation
        );
        InstallProgressReporter::reset_job(job_id);
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn first_concrete_progress_after_phase_only_update_is_persisted() {
        crate::event::EventState::init().await.unwrap();
        let root = tempfile::tempdir().unwrap().keep();
        let app_state =
            crate::State::init_for_test(root.to_string_lossy().to_string())
                .await
                .unwrap();
        let job_id = Uuid::new_v4();
        let mut job = InstallJobState::new(InstallRequest::CreateInstance {
            name: "Test".to_string(),
            game_version: "1.21.1".to_string(),
            loader: ModLoader::Vanilla,
            loader_version: None,
            adjuncts: Vec::new(),
            icon_path: None,
            link: InstanceLink::Unmanaged,
            game_dir_override: None,
        });
        job.record_event(InstallJobEventKind::ContentDownloadStarted {
            files: 1,
            bytes: Some(300),
        });
        job.record_event(InstallJobEventKind::ContentFileCompleted {
            path: "mods/content.jar".to_string(),
            bytes: 268,
        });
        job.set_progress(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: 1,
                total: 1,
                secondary: Some(InstallProgressSecondary {
                    current: 268,
                    total: 300,
                }),
            }),
            InstallPhaseDetails::Empty,
        );
        store::insert(job_id, &job, InstallJobStatus::Running, &app_state)
            .await
            .unwrap();
        let reporter = InstallProgressReporter::new(job_id, job);
        let minecraft_details = minecraft_details();

        reporter
            .update(
                InstallPhaseId::DownloadingMinecraft,
                None,
                minecraft_details.clone(),
            )
            .await
            .unwrap();
        reporter
            .update(
                InstallPhaseId::DownloadingMinecraft,
                Some(minecraft_progress(0, 18)),
                minecraft_details,
            )
            .await
            .unwrap();

        let snapshot = InstallProgressReporter::overlay_snapshot(
            job_id,
            store::get_required(job_id, &app_state)
                .await
                .unwrap()
                .snapshot(),
        )
        .await
        .unwrap();
        assert_eq!(snapshot.phase, InstallPhaseId::DownloadingMinecraft);
        let progress = snapshot.progress.unwrap();
        assert_eq!(progress.current, 0);
        assert_eq!(progress.total, 18);
        assert_eq!(snapshot.summary.bytes_downloaded, 0);
        assert_eq!(snapshot.summary.bytes_total, Some(18));
        InstallProgressReporter::reset_job(job_id);
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn same_progress_counter_remains_throttled() {
        let (app_state, job_id, reporter) =
            stored_minecraft_progress_job(0, 18).await;

        reporter
            .update(
                InstallPhaseId::DownloadingMinecraft,
                Some(minecraft_progress(1, 18)),
                minecraft_details(),
            )
            .await
            .unwrap();

        let snapshot = InstallProgressReporter::overlay_snapshot(
            job_id,
            store::get_required(job_id, &app_state)
                .await
                .unwrap()
                .snapshot(),
        )
        .await
        .unwrap();
        let progress = snapshot.progress.unwrap();
        assert_eq!(progress.current, 1);
        assert_eq!(progress.total, 18);
        InstallProgressReporter::reset_job(job_id);
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn changed_progress_total_is_persisted_immediately() {
        let (app_state, job_id, reporter) =
            stored_minecraft_progress_job(5, 18).await;

        reporter
            .update(
                InstallPhaseId::DownloadingMinecraft,
                Some(minecraft_progress(0, 20)),
                minecraft_details(),
            )
            .await
            .unwrap();

        let snapshot = InstallProgressReporter::overlay_snapshot(
            job_id,
            store::get_required(job_id, &app_state)
                .await
                .unwrap()
                .snapshot(),
        )
        .await
        .unwrap();
        let progress = snapshot.progress.unwrap();
        assert_eq!(progress.current, 0);
        assert_eq!(progress.total, 20);
        InstallProgressReporter::reset_job(job_id);
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn content_settlements_checkpoint_by_count_and_time() {
        let (app_state, job_id, reporter) =
            stored_minecraft_progress_job(0, 100).await;
        reporter.current_state().await.unwrap();
        {
            let mut state = reporter.state.lock().await;
            state.last_persisted_at = Instant::now();
        }
        let completed = |start: usize, count: usize| {
            (start..start + count)
                .map(|index| InstallJobEventKind::ContentFileCompleted {
                    path: format!("mods/{index}.jar"),
                    bytes: 1,
                })
                .collect::<Vec<_>>()
        };

        reporter
            .update_with_events(
                InstallPhaseId::DownloadingContent,
                Some(InstallProgress {
                    current: 24,
                    total: 100,
                    secondary: None,
                }),
                InstallPhaseDetails::Empty,
                completed(0, 24),
            )
            .await
            .unwrap();
        let stored_before_threshold =
            store::get_required(job_id, &app_state).await.unwrap();
        assert_eq!(
            stored_before_threshold
                .state
                .events
                .iter()
                .filter(|event| is_content_settlement_event(&event.kind))
                .count(),
            0
        );

        reporter
            .update_with_events(
                InstallPhaseId::DownloadingContent,
                Some(InstallProgress {
                    current: 25,
                    total: 100,
                    secondary: None,
                }),
                InstallPhaseDetails::Empty,
                completed(24, 1),
            )
            .await
            .unwrap();
        let stored_at_threshold =
            store::get_required(job_id, &app_state).await.unwrap();
        assert_eq!(
            stored_at_threshold
                .state
                .events
                .iter()
                .filter(|event| is_content_settlement_event(&event.kind))
                .count(),
            25
        );

        {
            let mut state = reporter.state.lock().await;
            state.last_persisted_at = Instant::now()
                - CONTENT_CHECKPOINT_INTERVAL
                + Duration::from_millis(50);
            state.checkpoint_scheduled = false;
        }
        reporter
            .update_with_events(
                InstallPhaseId::DownloadingContent,
                Some(InstallProgress {
                    current: 26,
                    total: 100,
                    secondary: None,
                }),
                InstallPhaseDetails::Empty,
                completed(25, 1),
            )
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(250)).await;
        let stored_after_interval =
            store::get_required(job_id, &app_state).await.unwrap();
        assert_eq!(
            stored_after_interval
                .state
                .events
                .iter()
                .filter(|event| is_content_settlement_event(&event.kind))
                .count(),
            26
        );
        InstallProgressReporter::reset_job(job_id);
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn database_checkpoint_does_not_hold_the_reporter_lock() {
        let (app_state, job_id, reporter) =
            stored_minecraft_progress_job(0, 100).await;
        reporter.current_state().await.unwrap();
        let database_permit =
            app_state.install_db_semaphore.acquire().await.unwrap();
        let checkpoint_reporter = reporter.clone();
        let checkpoint = tokio::spawn(async move {
            checkpoint_reporter
                .update_with_events(
                    InstallPhaseId::DownloadingContent,
                    Some(InstallProgress {
                        current: CONTENT_CHECKPOINT_FILE_COUNT as u64,
                        total: 100,
                        secondary: None,
                    }),
                    InstallPhaseDetails::Empty,
                    (0..CONTENT_CHECKPOINT_FILE_COUNT)
                        .map(|index| {
                            InstallJobEventKind::ContentFileCompleted {
                                path: format!("mods/{index}.jar"),
                                bytes: 1,
                            }
                        })
                        .collect(),
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if reporter.state.lock().await.checkpoint_in_flight {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("checkpoint should reach the database wait");

        tokio::time::timeout(
            Duration::from_millis(50),
            reporter.record_download_request(
                "mods/next.jar",
                "next.jar",
                "https://example.invalid/next.jar",
                "test",
                Some(10),
                1,
                1,
            ),
        )
        .await
        .expect("download events must not wait for checkpoint SQLite")
        .unwrap();

        drop(database_permit);
        checkpoint.await.unwrap().unwrap();
        InstallProgressReporter::reset_job(job_id);
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn delayed_checkpoint_cannot_overwrite_terminal_job_state() {
        let (app_state, job_id, reporter) =
            stored_minecraft_progress_job(0, 100).await;
        let stale_state = reporter.current_state().await.unwrap();
        let mut completed_state = stale_state.clone();
        completed_state.record_event(InstallJobEventKind::JobSucceeded {
            instance_id: None,
        });
        store::update_status(
            job_id,
            InstallJobStatus::Succeeded,
            &completed_state,
            &app_state,
        )
        .await
        .unwrap();

        let mut delayed_checkpoint = stale_state;
        delayed_checkpoint.record_event(
            InstallJobEventKind::ContentFileCompleted {
                path: "mods/late.jar".to_string(),
                bytes: 1,
            },
        );
        assert!(
            store::checkpoint_running_state(
                job_id,
                &delayed_checkpoint,
                &app_state,
            )
            .await
            .unwrap()
            .is_none()
        );
        let stored = store::get_required(job_id, &app_state).await.unwrap();
        assert_eq!(stored.status, InstallJobStatus::Succeeded);
        assert!(stored.state.events.iter().any(|event| matches!(
            event.kind,
            InstallJobEventKind::JobSucceeded { .. }
        )));
        assert!(!stored.state.events.iter().any(|event| matches!(
            &event.kind,
            InstallJobEventKind::ContentFileCompleted { path, .. }
                if path == "mods/late.jar"
        )));
        InstallProgressReporter::reset_job(job_id);
    }

    #[test]
    fn download_request_update_matches_frontend_event_contract() {
        let job_id =
            Uuid::parse_str("e7df84c8-b960-4ddb-a75b-bc9012405f1e").unwrap();
        let update = DownloadRequestUpdate::Started {
            job_id,
            id: "mods/example.jar".to_string(),
            name: "example.jar".to_string(),
            url: "https://cdn.modrinth.com/data/example.jar".to_string(),
            source: "official".to_string(),
            bytes_total: Some(4096),
            attempt: 2,
            max_attempts: 4,
        };

        assert_eq!(
            serde_json::to_value(update).unwrap(),
            serde_json::json!({
                "type": "started",
                "job_id": "e7df84c8-b960-4ddb-a75b-bc9012405f1e",
                "id": "mods/example.jar",
                "name": "example.jar",
                "url": "https://cdn.modrinth.com/data/example.jar",
                "source": "official",
                "bytes_total": 4096,
                "attempt": 2,
                "max_attempts": 4,
            })
        );

        let progress = DownloadRequestUpdate::Progress {
            job_id,
            id: "mods/example.jar".to_string(),
            bytes: 2048,
            status: DownloadItemStatus::Verifying,
            speed_bytes_per_second: None,
            eta_seconds: None,
        };
        assert_eq!(
            serde_json::to_value(progress).unwrap(),
            serde_json::json!({
                "type": "progress",
                "job_id": "e7df84c8-b960-4ddb-a75b-bc9012405f1e",
                "id": "mods/example.jar",
                "bytes": 2048,
                "status": "verifying",
                "speed_bytes_per_second": null,
                "eta_seconds": null,
            })
        );
    }

    #[test]
    fn recovered_curseforge_manual_item_clears_pause_count() {
        let path = "manual.jar".to_string();
        let mut job = InstallJobState::new(InstallRequest::CreateInstance {
            name: "CurseForge pack".to_string(),
            game_version: "1.12.2".to_string(),
            loader: ModLoader::Forge,
            loader_version: None,
            adjuncts: Vec::new(),
            icon_path: None,
            link: InstanceLink::CurseForgeModpack {
                project_id: "123".to_string(),
                version_id: "456".to_string(),
            },
            game_dir_override: None,
        });
        job.record_event(InstallJobEventKind::ContentFileSkipped {
            path: path.clone(),
            reason: "CurseForge requires manual download".to_string(),
            project_id: Some("10".to_string()),
            version_id: Some("20".to_string()),
            manual_url: Some(
                "https://www.curseforge.com/minecraft/mc-mods/example/download/20"
                    .to_string(),
            ),
        });
        job.pause_reason = Some(InstallPauseReason::MissingRequiredContent {
            failed_files: 1,
            paths: vec![path.clone()],
        });
        job.record_event(InstallJobEventKind::ContentFileRecovered {
            path,
            bytes: 12,
        });

        refresh_missing_pause_reason(&mut job);

        assert_eq!(
            job.pause_reason,
            Some(InstallPauseReason::MissingRequiredContent {
                failed_files: 0,
                paths: Vec::new(),
            })
        );
    }
}
