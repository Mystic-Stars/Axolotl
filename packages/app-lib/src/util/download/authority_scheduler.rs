use super::h2_pool::SharedH2Connection;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) const INITIAL_CONNECTION_TARGET: usize = 4;
pub(crate) const MAX_CONNECTION_TARGET: usize = 32;
pub(crate) const STABILITY_WINDOW: Duration = Duration::from_millis(500);

fn next_target(current: usize) -> usize {
    (current * 2).min(MAX_CONNECTION_TARGET)
}

pub(crate) struct AuthorityScheduler {
    connections: Vec<Arc<SharedH2Connection>>,
    target: usize,
    stable_since: Option<Instant>,
    cooldown_until: Option<Instant>,
    last_byte_sample: usize,
    last_byte_sample_at: Instant,
    connecting: Arc<tokio::sync::Semaphore>,
}

impl Default for AuthorityScheduler {
    fn default() -> Self {
        Self {
            connections: Vec::new(),
            target: INITIAL_CONNECTION_TARGET,
            stable_since: None,
            cooldown_until: None,
            last_byte_sample: 0,
            last_byte_sample_at: Instant::now(),
            connecting: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
}

impl AuthorityScheduler {
    pub(crate) fn prune(&mut self) {
        self.connections.retain(|connection| {
            if connection.is_dead()
                || connection.is_idle_expired_for_scheduler()
            {
                connection.evict_for_scheduler();
                false
            } else {
                true
            }
        });
        if self.connections.is_empty() {
            self.target = INITIAL_CONNECTION_TARGET;
            self.stable_since = None;
        }
    }

    pub(crate) fn add_connection(
        &mut self,
        connection: Arc<SharedH2Connection>,
    ) {
        if !connection.is_dead()
            && !self
                .connections
                .iter()
                .any(|existing| Arc::ptr_eq(existing, &connection))
        {
            self.connections.push(connection);
        }
    }

    pub(crate) fn least_loaded(&self) -> Option<Arc<SharedH2Connection>> {
        self.connections
            .iter()
            .filter(|connection| !connection.is_dead())
            .min_by_key(|connection| {
                connection.assigned_streams_for_scheduler()
            })
            .cloned()
    }

    pub(crate) fn select_connection(
        &mut self,
    ) -> Option<Arc<SharedH2Connection>> {
        let connection = self.least_loaded()?;
        connection.reserve_stream_for_scheduler();
        Some(connection)
    }

    pub(crate) fn observe(&mut self, now: Instant) {
        let total_bytes = self
            .connections
            .iter()
            .map(|connection| connection.bytes_transferred_for_scheduler())
            .sum();
        if now.duration_since(self.last_byte_sample_at) >= STABILITY_WINDOW {
            if total_bytes == self.last_byte_sample {
                self.stable_since = None;
                self.last_byte_sample_at = now;
                return;
            }
            self.last_byte_sample = total_bytes;
            self.last_byte_sample_at = now;
        }
        let Some(since) = self.stable_since else {
            self.stable_since = Some(now);
            return;
        };
        if self.connections.len() >= self.target
            && self.connections.iter().all(|connection| {
                connection.healthy_for_scheduler()
                    && connection.assigned_streams_for_scheduler() > 0
            })
            && now.duration_since(since) >= STABILITY_WINDOW
        {
            self.target = next_target(self.target);
            self.stable_since = None;
        }
    }

    pub(crate) fn target(&self) -> usize {
        self.target
    }

    pub(crate) fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub(crate) fn connections_for_eviction(
        &self,
    ) -> &[Arc<SharedH2Connection>] {
        &self.connections
    }

    pub(crate) fn can_expand(&self, now: Instant) -> bool {
        self.target <= MAX_CONNECTION_TARGET
            && self.connection_count() < self.target
            && self.cooldown_until.is_none_or(|until| now >= until)
    }

    pub(crate) fn reserve_connector(
        &self,
    ) -> Option<tokio::sync::OwnedSemaphorePermit> {
        self.connecting.clone().try_acquire_owned().ok()
    }

    pub(crate) fn rollback(&mut self, now: Instant) {
        self.target = (self.target / 2).max(INITIAL_CONNECTION_TARGET);
        self.stable_since = None;
        self.cooldown_until = Some(now + STABILITY_WINDOW * 4);
    }

    #[cfg(test)]
    pub(crate) fn targets() -> [usize; 4] {
        [4, 8, 16, 32]
    }
}
