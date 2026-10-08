//! Shared receive-side accounting for native HTTP/2 streams.

use bytes::Bytes;
use std::time::{Duration, Instant};

const STREAM_RECV_TIMEOUT: Duration = Duration::from_secs(30);
const MIN_PROGRESS_BYTES: u64 = 256 * 1024;
const MAX_PROGRESS_WAIT: Duration = Duration::from_secs(1);

/// Tracks one active H2 stream in the native automatic-concurrency metrics.
pub(crate) struct H2TransferActivity {
    _activity: Option<crate::state::DownloadConnectionActivity>,
}

impl H2TransferActivity {
    pub(crate) fn begin() -> Self {
        Self {
            _activity: crate::State::get_if_initialized()
                .map(|state| state.begin_download_connection()),
        }
    }

    pub(crate) fn record_bytes(&self, bytes: usize) {
        if let Some(state) = crate::State::get_if_initialized() {
            state.record_download_bytes(bytes as u64);
        }
    }
}

/// Limits install-progress work before it reaches the shared reporter lock.
pub(crate) struct H2ProgressGate {
    last_reported: u64,
    last_reported_at: Instant,
    threshold: u64,
}

impl H2ProgressGate {
    pub(crate) fn new(total_size: u64) -> Self {
        Self {
            last_reported: 0,
            last_reported_at: Instant::now(),
            threshold: MIN_PROGRESS_BYTES.max(total_size / 200),
        }
    }

    pub(crate) fn should_report(
        &mut self,
        downloaded: u64,
        total_size: u64,
    ) -> bool {
        if downloaded < total_size
            && downloaded.saturating_sub(self.last_reported) < self.threshold
            && self.last_reported_at.elapsed() < MAX_PROGRESS_WAIT
        {
            return false;
        }
        if downloaded <= self.last_reported {
            return false;
        }
        self.last_reported = downloaded;
        self.last_reported_at = Instant::now();
        true
    }
}

pub(crate) async fn receive_chunk(
    stream: &mut h2::RecvStream,
    context: &str,
) -> crate::Result<Option<Bytes>> {
    tokio::time::timeout(STREAM_RECV_TIMEOUT, stream.data())
        .await
        .map_err(|_| {
            crate::ErrorKind::NetworkError(format!(
                "HTTP/2 {context} stream receive timed out"
            ))
        })?
        .transpose()
        .map_err(|error| {
            crate::ErrorKind::NetworkError(format!(
                "HTTP/2 {context} stream error: {error}"
            ))
            .into()
        })
}

/// Returns receive-window capacity after the caller has consumed the frame.
pub(crate) fn release_capacity(
    stream: &mut h2::RecvStream,
    bytes: usize,
) -> crate::Result<()> {
    stream
        .flow_control()
        .release_capacity(bytes)
        .map_err(|error| {
            crate::ErrorKind::NetworkError(format!(
                "HTTP/2 flow-control release failed: {error}"
            ))
            .into()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::poll_fn;
    use http::{Request, Response};

    #[test]
    fn progress_gate_throttles_intermediate_updates_and_reports_completion() {
        let total = 2 * 1024 * 1024;
        let mut gate = H2ProgressGate::new(total);

        assert!(!gate.should_report(64 * 1024, total));
        assert!(gate.should_report(256 * 1024, total));
        assert!(!gate.should_report(320 * 1024, total));
        assert!(gate.should_report(total, total));
        assert!(!gate.should_report(total, total));
    }

    #[test]
    fn progress_gate_reports_slow_transfers_on_time() {
        let total = 100 * 1024 * 1024;
        let mut gate = H2ProgressGate::new(total);

        assert!(!gate.should_report(100, total));
        gate.last_reported_at = Instant::now() - MAX_PROGRESS_WAIT;
        assert!(gate.should_report(100, total));
        assert!(!gate.should_report(200, total));
        gate.last_reported_at = Instant::now() - MAX_PROGRESS_WAIT;
        assert!(!gate.should_report(100, total));
        assert!(gate.should_report(200, total));
        assert!(gate.should_report(total, total));
    }

    #[tokio::test]
    async fn receive_capacity_allows_body_larger_than_initial_window() {
        const BODY_SIZE: usize = 2 * 1024 * 1024;
        const FRAME_SIZE: usize = 16 * 1024;

        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(async move {
            let mut connection =
                h2::server::handshake(server_io).await.unwrap();
            let (_, mut respond) = connection.accept().await.unwrap().unwrap();
            // Keep polling the connection while the response body waits for
            // client WINDOW_UPDATE frames.
            tokio::spawn(async move {
                while connection.accept().await.is_some() {}
            });
            let mut stream =
                respond.send_response(Response::new(()), false).unwrap();
            let mut remaining = BODY_SIZE;
            while remaining > 0 {
                stream.reserve_capacity(remaining.min(FRAME_SIZE));
                let capacity = poll_fn(|cx| stream.poll_capacity(cx))
                    .await
                    .unwrap()
                    .unwrap();
                let length = capacity.min(remaining).min(FRAME_SIZE);
                remaining -= length;
                stream
                    .send_data(Bytes::from(vec![0x5a; length]), remaining == 0)
                    .unwrap();
            }
        });

        let mut builder = h2::client::Builder::new();
        builder
            .initial_window_size(1024 * 1024)
            .initial_connection_window_size(64 * 1024 * 1024);
        let (mut sender, mut connection) =
            builder.handshake::<_, Bytes>(client_io).await.unwrap();
        connection.set_target_window_size(64 * 1024 * 1024);
        let client_driver = tokio::spawn(connection);

        let request =
            Request::get("https://h2-flow.test/file").body(()).unwrap();
        sender = sender.ready().await.unwrap();
        let (response, _) = sender.send_request(request, true).unwrap();
        let mut body = response.await.unwrap().into_body();
        let mut received = 0;
        while let Some(chunk) = receive_chunk(&mut body, "test").await.unwrap()
        {
            assert!(chunk.iter().all(|byte| *byte == 0x5a));
            received += chunk.len();
            release_capacity(&mut body, chunk.len()).unwrap();
        }

        assert_eq!(received, BODY_SIZE);
        server.await.unwrap();
        client_driver.abort();
    }
}
