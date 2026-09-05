//! Coalesce concurrent downloads for the same destination and integrity.
//!
//! This module is intentionally independent from the downloader engines. It
//! only coordinates the operation supplied by the caller and shares a
//! verified successful result with followers.

use super::fetch::{DownloadResult, Integrity};
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, LazyLock, Weak};
use tokio::sync::watch;

struct InFlight {
    /// Outer `None` means the flight is still pending; `Some(None)` is a
    /// terminal failure or cancellation; `Some(Some(result))` is the shared
    /// successful result. A watch channel retains the latest state, so a
    /// late-arriving follower can read it without waiting for a notification.
    state: watch::Sender<Option<Option<DownloadResult>>>,
}

static FLIGHTS: LazyLock<dashmap::DashMap<String, Weak<InFlight>>> =
    LazyLock::new(dashmap::DashMap::new);

fn key(destination: &Path, integrity: &Integrity) -> String {
    let destination = if cfg!(windows) {
        destination.display().to_string().to_uppercase()
    } else {
        destination.display().to_string()
    };
    format!(
        "{destination}\0size={:?}\0sha1={:?}\0sha512={:?}\0sha256={:?}\0md5={:?}\0content={:?}",
        integrity.size,
        integrity.sha1,
        integrity.sha512,
        integrity.sha256,
        integrity.md5,
        integrity.content,
    )
}

/// Publishes the flight's terminal state when the leader future is dropped,
/// so waiters observe failure or cancellation instead of waiting for a
/// notification that will never fire again.
struct LeaderGuard {
    state: watch::Sender<Option<Option<DownloadResult>>>,
    published: bool,
}

impl LeaderGuard {
    fn new(state: watch::Sender<Option<Option<DownloadResult>>>) -> Self {
        Self {
            state,
            published: false,
        }
    }

    /// Marks the flight as finished. `None` is a terminal failure; `Some`
    /// carries the verified result shared with followers.
    fn publish(mut self, outcome: Option<DownloadResult>) {
        self.published = true;
        let _ = self.state.send(Some(outcome));
    }
}

impl Drop for LeaderGuard {
    fn drop(&mut self) {
        if !self.published {
            // The leader future was dropped before publishing a terminal
            // state (cancellation, panic, or the caller abandoning the
            // flight). Release every waiter with a terminal failure so it can
            // retry its own operation instead of hanging.
            let _ = self.state.send(Some(None));
        }
    }
}

/// Run an operation as a single flight when an integrity contract exists.
/// Followers receive the leader's verified result and never re-hash the file.
/// Failed flights are not cached: followers retry their own operation after
/// the leader publishes the failure, and a canceled leader releases waiters
/// with a terminal failure instead of leaving them pending.
pub(crate) async fn run<F, Fut>(
    destination: &Path,
    integrity: &Integrity,
    operation: F,
) -> crate::Result<DownloadResult>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = crate::Result<DownloadResult>>,
{
    if integrity.is_empty() {
        return operation().await;
    }

    use dashmap::mapref::entry::Entry;
    let flight_key = key(destination, integrity);
    let (flight, leader) = match FLIGHTS.entry(flight_key) {
        Entry::Occupied(mut entry) => match entry.get().upgrade() {
            Some(flight) => (flight, false),
            None => {
                let flight = Arc::new(InFlight {
                    state: watch::channel(None).0,
                });
                entry.insert(Arc::downgrade(&flight));
                (flight, true)
            }
        },
        Entry::Vacant(entry) => {
            let flight = Arc::new(InFlight {
                state: watch::channel(None).0,
            });
            entry.insert(Arc::downgrade(&flight));
            (flight, true)
        }
    };

    if leader {
        let guard = LeaderGuard::new(flight.state.clone());
        match operation().await {
            Ok(downloaded) => {
                guard.publish(Some(downloaded.clone()));
                Ok(downloaded)
            }
            Err(error) => {
                guard.publish(None);
                Err(error)
            }
        }
    } else {
        let mut receiver = flight.state.subscribe();
        // Late arrivals read the retained terminal state directly; only a
        // flight that is still pending is worth waiting on. The watch guard
        // is dropped before any `await` so the waiter future stays `Send`.
        let state = receiver.borrow_and_update().clone();
        match state {
            Some(Some(result)) => return Ok(result),
            Some(None) => return operation().await,
            None => {}
        }
        loop {
            if receiver.changed().await.is_err() {
                // The flight entry was removed without a terminal state; run
                // the operation ourselves rather than hanging.
                return operation().await;
            }
            let state = receiver.borrow_and_update().clone();
            match state {
                Some(Some(result)) => return Ok(result),
                Some(None) => return operation().await,
                None => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn concurrent_callers_share_successful_result() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifact.jar");
        let integrity = Integrity::sha1("deadbeef").with_size(7);
        let calls = Arc::new(AtomicUsize::new(0));
        let first_calls = Arc::clone(&calls);
        let first_path = path.clone();
        let first = run(&path, &integrity, move || async move {
            first_calls.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok(DownloadResult {
                path: first_path,
                url: "https://example.invalid/artifact.jar".into(),
                source: super::super::fetch::DownloadRouteSource::Official,
                size: 7,
                attempts: 1,
                fallback_count: 0,
            })
        });
        let second_path = dir.path().join("artifact.jar");
        let second = run(&second_path, &integrity, || async {
            panic!("follower must not execute operation")
        });
        let (result, shared) = tokio::join!(first, second);
        assert!(result.is_ok());
        assert_eq!(shared.unwrap().size, 7);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn cancelled_leader_publishes_terminal_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifact.jar");
        let integrity = Integrity::sha1("deadbeef").with_size(7);
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let leader_barrier = Arc::clone(&barrier);
        let leader_path = path.clone();
        let leader_integrity = integrity.clone();
        let leader = tokio::spawn(async move {
            let _ = run(&leader_path, &leader_integrity, move || async move {
                leader_barrier.wait().await;
                // Never resolves on its own; aborting the task drops the
                // future so the LeaderGuard must publish a terminal state.
                std::future::pending::<crate::Result<DownloadResult>>().await
            })
            .await;
        });
        barrier.wait().await;
        let retries = Arc::new(AtomicUsize::new(0));
        let follower_retries = Arc::clone(&retries);
        let follower_path = path.clone();
        let follower_integrity = integrity.clone();
        let follower = tokio::spawn(async move {
            run(&follower_path, &follower_integrity, move || async move {
                follower_retries.fetch_add(1, Ordering::Relaxed);
                Err(crate::ErrorKind::OtherError(
                    "follower retried after the leader was canceled".into(),
                )
                .into())
            })
            .await
        });
        // Give the follower time to subscribe while the leader is pending.
        tokio::time::sleep(Duration::from_millis(50)).await;
        leader.abort();
        let result = tokio::time::timeout(Duration::from_secs(2), follower)
            .await
            .expect(
                "follower must observe the canceled leader's terminal state without hanging",
            )
            .expect("follower task must not panic");
        assert!(result.is_err());
        assert_eq!(retries.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn late_follower_retries_failed_flight() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifact.jar");
        let integrity = Integrity::sha1("deadbeef").with_size(7);
        let calls = Arc::new(AtomicUsize::new(0));
        let first_calls = Arc::clone(&calls);
        let first_path = path.clone();
        let first = run(&first_path, &integrity, move || async move {
            first_calls.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(20)).await;
            Err(crate::ErrorKind::OtherError("leader failed".into()).into())
        });
        let second_dest = path.clone();
        let second_result_path = path.clone();
        let second_integrity = integrity.clone();
        let second_calls = Arc::clone(&calls);
        let second = tokio::spawn(async move {
            // The follower subscribes while the failed flight is still
            // retained; it must observe the terminal failure and retry its
            // own operation instead of hanging.
            run(&second_dest, &second_integrity, move || async move {
                second_calls.fetch_add(1, Ordering::Relaxed);
                Ok(DownloadResult {
                    path: second_result_path,
                    url: "https://example.invalid/artifact.jar".into(),
                    source: super::super::fetch::DownloadRouteSource::Official,
                    size: 7,
                    attempts: 1,
                    fallback_count: 0,
                })
            })
            .await
        });
        let (first_result, second_result) = tokio::join!(first, second);
        assert!(first_result.is_err());
        assert_eq!(
            second_result
                .expect("follower task must not panic")
                .expect("follower must retry and succeed")
                .size,
            7
        );
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn different_integrity_contracts_use_different_keys() {
        let path = Path::new("artifact.jar");
        assert_ne!(
            key(path, &Integrity::sha1("a")),
            key(path, &Integrity::sha1("b"))
        );
    }
}
