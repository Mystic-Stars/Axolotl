use super::model::{
    InstallErrorContext, InstallJobEventKind, InstallJobSnapshot,
    InstallJobState, InstallPhaseDetails, InstallPhaseId, InstallProgress,
};
use super::store;
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

const PROGRESS_PERSIST_INTERVAL: Duration = Duration::from_millis(750);
const CONTENT_PROGRESS_PERSIST_STEPS: u64 = 25;

static REPORTER_STATES: LazyLock<
    dashmap::DashMap<Uuid, Weak<Mutex<InstallProgressReporterState>>>,
> = LazyLock::new(dashmap::DashMap::new);

#[derive(Clone, Debug)]
pub struct InstallProgressReporter {
    job_id: Uuid,
    state: Arc<Mutex<InstallProgressReporterState>>,
}

#[derive(Debug)]
struct InstallProgressReporterState {
    job: InstallJobState,
    last_persisted_at: Instant,
    last_persisted_progress: Option<(InstallPhaseId, u64)>,
    initialized_from_store: bool,
}

impl InstallProgressReporter {
    pub fn new(job_id: Uuid, state: InstallJobState) -> Self {
        let shared_state = match REPORTER_STATES.entry(job_id) {
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                if let Some(state) = entry.get().upgrade() {
                    state
                } else {
                    let state =
                        Arc::new(Mutex::new(InstallProgressReporterState {
                            job: state,
                            last_persisted_at: Instant::now(),
                            last_persisted_progress: None,
                            initialized_from_store: false,
                        }));
                    entry.insert(Arc::downgrade(&state));
                    state
                }
            }
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                let state =
                    Arc::new(Mutex::new(InstallProgressReporterState {
                        job: state,
                        last_persisted_at: Instant::now(),
                        last_persisted_progress: None,
                        initialized_from_store: false,
                    }));
                entry.insert(Arc::downgrade(&state));
                state
            }
        };
        Self {
            job_id,
            state: shared_state,
        }
    }

    pub async fn update(
        &self,
        phase: InstallPhaseId,
        progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
    ) -> crate::Result<()> {
        self.update_with_events(phase, progress, details, Vec::new())
            .await
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

    async fn sync_latest(
        &self,
        state: &mut InstallProgressReporterState,
        app_state: &crate::State,
    ) -> crate::Result<()> {
        if !state.initialized_from_store {
            state.job =
                store::get_required(self.job_id, app_state).await?.state;
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

        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        emit_install_job(&record.snapshot()).await
    }

    pub async fn persist(&self) -> crate::Result<InstallJobSnapshot> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;

        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        let snapshot = record.snapshot();
        emit_install_job(&snapshot).await?;
        Ok(snapshot)
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

        state
            .job
            .record_event(InstallJobEventKind::DownloadMetrics {
                source: source.into(),
                fallback_count,
            });
        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        emit_install_job(&record.snapshot()).await
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
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        self.sync_latest(&mut state, &app_state).await?;
        let phase_started = state.job.progress.phase != phase
            || matches!(
                &state.job.progress.details,
                InstallPhaseDetails::Empty
            ) && !matches!(&details, InstallPhaseDetails::Empty);
        let force_persist = events.iter().any(|event| {
            matches!(
                event,
                InstallJobEventKind::ContentFileDownloadAttempt { .. }
            )
        });

        state.job.set_progress(phase, progress, details);
        for event in events {
            state.job.record_event(event);
        }

        if !state.should_persist(phase_started || force_persist) {
            return Ok(());
        }

        let record =
            store::update_state(self.job_id, &state.job, &app_state).await?;
        state.mark_persisted();
        emit_install_job(&record.snapshot()).await
    }
}

impl InstallProgressReporterState {
    fn should_persist(&self, phase_started: bool) -> bool {
        if phase_started {
            return true;
        }

        let Some(progress) = &self.job.progress.progress else {
            return true;
        };

        if progress.current >= progress.total {
            return true;
        }

        let progressed_enough =
            if self.job.progress.phase == InstallPhaseId::DownloadingContent {
                self.last_persisted_progress
                    .map(|(phase, current)| {
                        phase != self.job.progress.phase
                            || progress.current.saturating_sub(current)
                                >= CONTENT_PROGRESS_PERSIST_STEPS
                    })
                    .unwrap_or(true)
            } else {
                false
            };

        progressed_enough
            || self.last_persisted_at.elapsed() >= PROGRESS_PERSIST_INTERVAL
    }

    fn mark_persisted(&mut self) {
        self.last_persisted_at = Instant::now();
        self.last_persisted_progress = self
            .job
            .progress
            .progress
            .as_ref()
            .map(|progress| (self.job.progress.phase, progress.current));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::InstallRequest;
    use crate::state::{InstanceLink, ModLoader};

    #[test]
    fn separately_created_reporters_share_job_state() {
        let job_id = Uuid::new_v4();
        let state = InstallJobState::new(InstallRequest::CreateInstance {
            name: "Test".to_string(),
            game_version: "1.21.1".to_string(),
            loader: ModLoader::Vanilla,
            loader_version: None,
            icon_path: None,
            link: InstanceLink::Unmanaged,
        });

        let first = InstallProgressReporter::new(job_id, state.clone());
        let second = InstallProgressReporter::new(job_id, state);

        assert!(Arc::ptr_eq(&first.state, &second.state));
    }
}
