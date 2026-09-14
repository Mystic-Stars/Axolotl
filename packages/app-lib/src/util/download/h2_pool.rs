//! Shared per-authority HTTP/2 connections for file downloads.
//!
//! `reqwest`'s connection pool opens a fresh TCP+TLS connection for every
//! request that arrives while no idle connection is available, so a batch of
//! concurrent downloads to one CDN costs one handshake per file. This module
//! instead maintains a long-lived HTTP/2 connection per authority and
//! multiplexes every download as a separate stream over it (`SendRequest` is
//! cheap to clone and each clone opens an independent stream). Asset batches
//! may lazily add one sibling under sustained saturation; large files can also
//! split into range streams over their shared connection.

use bytes::Bytes;
use h2::client::SendRequest;
use rustls::ClientConfig;
use rustls_pki_types::ServerName;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::Notify;
use tokio_rustls::TlsConnector;

use crate::util::fetch::DownloadRoute;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const DNS_PREWARM_TIMEOUT: Duration = Duration::from_secs(10);
const STREAM_READY_TIMEOUT: Duration = Duration::from_secs(30);
const IDLE_EVICTION_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const CONNECTION_WAIT_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum H2ConnectFailureKind {
    Tcp,
    Tls,
    Protocol,
}

#[derive(Debug)]
pub(crate) struct H2ConnectError {
    pub(crate) kind: H2ConnectFailureKind,
    detail: String,
}

impl std::fmt::Display for H2ConnectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for H2ConnectError {}

impl H2ConnectError {
    fn new(kind: H2ConnectFailureKind, detail: String) -> Self {
        Self { kind, detail }
    }
}

/// A live shared HTTP/2 connection to one authority.
pub struct SharedH2Connection {
    authority: String,
    sender: Mutex<SendRequest<Bytes>>,
    // One permit represents the shared TCP/TLS connection, not every H2
    // stream opened through it.
    physical_budget: Mutex<Option<super::native_budget::NativeBudgetPermit>>,
    /// Set to true by the driver task when the connection terminates.
    dead: Arc<std::sync::atomic::AtomicBool>,
    /// Number of application streams currently assigned to this connection.
    /// This is deliberately separate from HTTP/2's peer stream accounting: it
    /// lets an asset batch distribute work across sibling TCP connections.
    active_streams: Arc<AtomicUsize>,
    last_activity: Mutex<std::time::Instant>,
    evict: Arc<Notify>,
}

pub(crate) struct H2StreamActivity {
    active_streams: Arc<AtomicUsize>,
}

impl Drop for H2StreamActivity {
    fn drop(&mut self) {
        self.active_streams.fetch_sub(1, Ordering::Release);
    }
}

impl SharedH2Connection {
    fn new(
        authority: String,
        sender: SendRequest<Bytes>,
        physical_budget: Option<super::native_budget::NativeBudgetPermit>,
    ) -> Self {
        Self {
            authority,
            sender: Mutex::new(sender),
            physical_budget: Mutex::new(physical_budget),
            dead: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            active_streams: Arc::new(AtomicUsize::new(0)),
            last_activity: Mutex::new(std::time::Instant::now()),
            evict: Arc::new(Notify::new()),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(sender: SendRequest<Bytes>) -> Self {
        Self::new("test.invalid:443".to_string(), sender, None)
    }

    pub fn is_dead(&self) -> bool {
        self.dead.load(std::sync::atomic::Ordering::Acquire)
    }

    pub(crate) fn active_streams(&self) -> usize {
        self.active_streams.load(Ordering::Acquire)
    }

    pub(crate) fn track_stream(&self) -> H2StreamActivity {
        *self
            .last_activity
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            std::time::Instant::now();
        self.active_streams.fetch_add(1, Ordering::AcqRel);
        H2StreamActivity {
            active_streams: Arc::clone(&self.active_streams),
        }
    }

    fn is_idle_expired(&self) -> bool {
        self.active_streams() == 0
            && self
                .last_activity
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .elapsed()
                >= IDLE_EVICTION_TIMEOUT
    }

    fn evict(&self) {
        self.evict.notify_waiters();
    }

    fn has_physical_budget(&self) -> bool {
        self.physical_budget
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    fn release_physical_budget(&self) {
        self.physical_budget
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
    }

    /// Sends a request on the shared connection and awaits the response
    /// headers, yielding the response and its receive stream. Each call
    /// opens an independent multiplexed stream.
    pub async fn open(
        &self,
        request: http::Request<()>,
    ) -> Result<http::Response<h2::RecvStream>, h2::Error> {
        let ready_started = std::time::Instant::now();
        let sender = self
            .sender
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        *self
            .last_activity
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            std::time::Instant::now();
        let mut sender =
            tokio::time::timeout(STREAM_READY_TIMEOUT, sender.ready())
                .await
                .map_err(|_| {
                    h2::Error::from(h2::Reason::ENHANCE_YOUR_CALM)
                })??;
        let ready_wait = ready_started.elapsed();
        if ready_wait >= Duration::from_millis(25) {
            tracing::debug!(
                authority = %self.authority,
                ready_wait_ms = ready_wait.as_millis(),
                "HTTP/2 stream admission waited for peer or connection capacity"
            );
        }
        let (response, _) = sender.send_request(request, true)?;
        tokio::time::timeout(STREAM_READY_TIMEOUT, response)
            .await
            .map_err(|_| h2::Error::from(h2::Reason::ENHANCE_YOUR_CALM))?
    }
}

type ConnectionSlot = Arc<AsyncMutex<Option<Arc<SharedH2Connection>>>>;

/// Registry of live shared connections, keyed by authority.
static CONNECTIONS: std::sync::LazyLock<
    AsyncMutex<HashMap<String, ConnectionSlot>>,
> = std::sync::LazyLock::new(|| AsyncMutex::new(HashMap::new()));

/// A bounded sibling connection for saturated asset batches. Normal file
/// downloads always use `CONNECTIONS`; a second TCP congestion domain is only
/// created by the asset scheduler after it observes sustained pressure.
static BATCH_CONNECTIONS: std::sync::LazyLock<
    AsyncMutex<HashMap<String, ConnectionSlot>>,
> = std::sync::LazyLock::new(|| AsyncMutex::new(HashMap::new()));

async fn connection_slot(authority: &str) -> ConnectionSlot {
    let mut connections = CONNECTIONS.lock().await;
    connections
        .entry(authority.to_string())
        .or_insert_with(|| Arc::new(AsyncMutex::new(None)))
        .clone()
}

async fn batch_connection_slot(authority: &str) -> ConnectionSlot {
    let mut connections = BATCH_CONNECTIONS.lock().await;
    connections
        .entry(authority.to_string())
        .or_insert_with(|| Arc::new(AsyncMutex::new(None)))
        .clone()
}

fn tls_config() -> Arc<ClientConfig> {
    static INSTALL_PROVIDER: Once = Once::new();
    INSTALL_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    static CONFIG: std::sync::LazyLock<Mutex<Option<Arc<ClientConfig>>>> =
        std::sync::LazyLock::new(|| Mutex::new(None));
    let mut guard = CONFIG
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(config) = guard.as_ref() {
        return Arc::clone(config);
    }
    let mut config = ClientConfig::builder()
        .with_root_certificates(platform_root_certs())
        .with_no_client_auth();
    config.enable_early_data = true;
    config.alpn_protocols = vec![b"h2".to_vec()];
    let config = Arc::new(config);
    *guard = Some(Arc::clone(&config));
    config
}

fn platform_root_certs() -> rustls::RootCertStore {
    let mut store = rustls::RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs();
    for cert in certs.certs {
        let _ = store.add(cert);
    }
    if certs.errors.is_empty() {
        return store;
    }
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    store
}

async fn connect_addresses(
    host: &str,
    port: u16,
    addresses: &[IpAddr],
) -> std::io::Result<TcpStream> {
    let mut last_error = None;
    for address in addresses {
        match tokio::time::timeout(
            CONNECT_TIMEOUT,
            TcpStream::connect((*address, port)),
        )
        .await
        {
            Ok(Ok(stream)) => {
                stream.set_nodelay(true).ok();
                return Ok(stream);
            }
            Ok(Err(error)) => last_error = Some(error),
            Err(_) => {
                last_error = Some(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("connection to {host}:{port} timed out"),
                ));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no addresses available for {host}"),
        )
    }))
}

async fn connect_tcp(host: &str, port: u16) -> std::io::Result<TcpStream> {
    // Prefer the ordered address list from the shared download resolver
    // (IPv4/IPv6 preference and per-IP reliability), falling back to the
    // system resolver when no list is cached yet.
    let resolver = &crate::util::fetch::DOWNLOAD_DNS_RESOLVER;
    let addresses = resolver.resolved_addresses(host);
    let mut last_error = None;
    if !addresses.is_empty() {
        match connect_addresses(host, port, &addresses).await {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
        if resolver.record_connection_failure(host) {
            let _ = tokio::time::timeout(
                CONNECTION_WAIT_TIMEOUT,
                resolver.pre_resolve(host),
            )
            .await;
            let refreshed = resolver.resolved_addresses(host);
            if !refreshed.is_empty() && refreshed != addresses {
                match connect_addresses(host, port, &refreshed).await {
                    Ok(stream) => return Ok(stream),
                    Err(error) => last_error = Some(error),
                }
            }
        }
    }
    let stream = tokio::time::timeout(
        CONNECT_TIMEOUT,
        tokio::net::TcpStream::connect((host, port)),
    )
    .await
    .map_err(|_| {
        last_error.take().unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("connection to {host}:{port} timed out"),
            )
        })
    })?
    .map_err(|error| {
        last_error.take().unwrap_or_else(|| {
            std::io::Error::new(
                error.kind(),
                format!("connection to {host}:{port} failed: {error}"),
            )
        })
    })?;
    stream.set_nodelay(true).ok();
    Ok(stream)
}

/// Connects a new shared HTTP/2 connection to `authority` (host[:port]).
async fn establish(
    route: &DownloadRoute,
    reserve_native_budget: bool,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    let authority =
        crate::util::fetch::url_authority(&route.url).ok_or_else(|| {
            H2ConnectError::new(
                H2ConnectFailureKind::Protocol,
                "HTTP/2 route has no authority".to_string(),
            )
        })?;
    let physical_budget = if reserve_native_budget {
        Some(super::native_budget::acquire(route).await.map_err(|error| {
            H2ConnectError::new(
                H2ConnectFailureKind::Tcp,
                format!("failed to reserve HTTP/2 connection capacity: {error}"),
            )
        })?)
    } else {
        None
    };
    let (host, port) = authority
        .rsplit_once(':')
        .map(|(host, port)| (host, port.parse::<u16>().unwrap_or(443)))
        .unwrap_or((&authority, 443));

    // Pre-resolve so `connect_tcp` gets the ordered, reliability-ranked
    // address list shared with the legacy reqwest path.
    let _ = tokio::time::timeout(
        DNS_PREWARM_TIMEOUT,
        crate::util::fetch::DOWNLOAD_DNS_RESOLVER.pre_resolve(host),
    )
    .await;

    let tcp = connect_tcp(host, port).await.map_err(|error| {
		H2ConnectError::new(
			H2ConnectFailureKind::Tcp,
			format!(
				"failed to establish shared HTTP/2 connection to {authority}: {error}"
			),
		)
	})?;

    let server_name =
        ServerName::try_from(host.to_string()).map_err(|error| {
            H2ConnectError::new(
                H2ConnectFailureKind::Tls,
                format!("invalid server name for {host}: {error}"),
            )
        })?;
    let connector = TlsConnector::from(tls_config());
    let tls = tokio::time::timeout(
        TLS_HANDSHAKE_TIMEOUT,
        connector.connect(server_name, tcp),
    )
    .await
    .map_err(|_| {
        H2ConnectError::new(
            H2ConnectFailureKind::Tls,
            format!("TLS handshake with {authority} timed out"),
        )
    })?
    .map_err(|error| {
        H2ConnectError::new(
            H2ConnectFailureKind::Tls,
            format!("TLS handshake with {authority} failed: {error}"),
        )
    })?;

    let mut builder = h2::client::Builder::new();
    builder
        .initial_window_size(1024 * 1024)
        .initial_connection_window_size(64 * 1024 * 1024);
    let (sender, mut connection) =
        builder.handshake(Box::pin(tls)).await.map_err(|error| {
            H2ConnectError::new(
                H2ConnectFailureKind::Protocol,
                format!("HTTP/2 handshake with {authority} failed: {error}"),
            )
        })?;

    // Tune flow-control windows for high-stream multiplexing (e.g. hundreds
    // of concurrent asset downloads over one connection). With the default
    // 64 KiB connection window, concurrent streams would stall waiting for
    // connection-level window updates; a larger per-stream window also lets
    // the peer send a whole small file without round trips.
    // Keep the adaptive connection target aligned with the initial connection
    // window configured before the handshake.
    connection.set_target_window_size(64 * 1024 * 1024);

    let shared = Arc::new(SharedH2Connection::new(
        authority.clone(),
        sender,
        physical_budget,
    ));

    let dead = Arc::clone(&shared.dead);
    let connection_budget = Arc::clone(&shared);
    let evict = Arc::clone(&shared.evict);
    let authority = authority.to_string();
    tokio::spawn(async move {
        tokio::select! {
            _ = connection => {}
            _ = evict.notified() => {
                tracing::debug!(authority, "Evicting idle shared HTTP/2 connection");
            }
        }
        dead.store(true, std::sync::atomic::Ordering::Release);
        connection_budget.release_physical_budget();
        tracing::debug!(authority, "Shared HTTP/2 connection closed");
    });

    Ok(shared)
}

/// Returns the live shared connection for `authority`, establishing one on
/// first use or after a previous connection died.
pub(crate) async fn shared_connection(
    route: &DownloadRoute,
    reserve_native_budget: bool,
    allow_cold_connection: bool,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    let authority =
        crate::util::fetch::url_authority(&route.url).ok_or_else(|| {
            H2ConnectError::new(
                H2ConnectFailureKind::Protocol,
                "HTTP/2 route has no authority".to_string(),
            )
        })?;
    let slot = connection_slot(&authority).await;
    let mut cached = tokio::time::timeout(CONNECTION_WAIT_TIMEOUT, slot.lock())
        .await
        .map_err(|_| {
            H2ConnectError::new(
                H2ConnectFailureKind::Protocol,
                format!(
                    "timed out waiting for HTTP/2 connection slot {authority}"
                ),
            )
        })?;
    if let Some(connection) = cached.as_ref().filter(|connection| {
        !connection.is_dead() && !connection.is_idle_expired()
    }) {
        if !reserve_native_budget || connection.has_physical_budget() {
            tracing::debug!(authority, "Reusing shared HTTP/2 connection");
            return Ok(Arc::clone(connection));
        }
        return Err(H2ConnectError::new(
            H2ConnectFailureKind::Protocol,
            "shared HTTP/2 connection is not covered by the native connection budget"
                .to_string(),
        ));
    }
    if cached.as_ref().is_some_and(|connection| {
        connection.is_dead() || connection.is_idle_expired()
    }) {
        if let Some(connection) = cached.as_ref() {
            connection.evict();
        }
        *cached = None;
    }
    if !allow_cold_connection {
        return Err(H2ConnectError::new(
            H2ConnectFailureKind::Protocol,
            "HTTP/2 policy requires an existing shared connection".to_string(),
        ));
    }
    tracing::debug!(authority, "Establishing cold shared HTTP/2 connection");
    let connection = tokio::time::timeout(
        CONNECTION_WAIT_TIMEOUT,
        establish(route, reserve_native_budget),
    )
    .await
    .map_err(|_| {
        H2ConnectError::new(
            H2ConnectFailureKind::Tcp,
            format!("timed out establishing HTTP/2 connection to {authority}"),
        )
    })??;
    *cached = Some(Arc::clone(&connection));
    Ok(connection)
}

/// Returns the optional second connection used exclusively by a busy asset
/// batch. It is stored independently so ordinary file downloads retain their
/// stable primary connection and never create extra TCP connections.
pub(crate) async fn shared_batch_connection(
    route: &DownloadRoute,
    reserve_native_budget: bool,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    let authority =
        crate::util::fetch::url_authority(&route.url).ok_or_else(|| {
            H2ConnectError::new(
                H2ConnectFailureKind::Protocol,
                "HTTP/2 route has no authority".to_string(),
            )
        })?;
    let slot = batch_connection_slot(&authority).await;
    let mut cached = tokio::time::timeout(
        CONNECTION_WAIT_TIMEOUT,
        slot.lock(),
    )
    .await
    .map_err(|_| {
        H2ConnectError::new(
            H2ConnectFailureKind::Protocol,
            format!("timed out waiting for asset HTTP/2 connection slot {authority}"),
        )
    })?;
    if let Some(connection) = cached.as_ref().filter(|connection| {
        !connection.is_dead() && !connection.is_idle_expired()
    }) {
        if !reserve_native_budget || connection.has_physical_budget() {
            tracing::debug!(
                authority,
                "Reusing sibling HTTP/2 asset connection"
            );
            return Ok(Arc::clone(connection));
        }
        return Err(H2ConnectError::new(
            H2ConnectFailureKind::Protocol,
            "sibling HTTP/2 connection is not covered by the native connection budget"
                .to_string(),
        ));
    }
    if cached.as_ref().is_some_and(|connection| {
        connection.is_dead() || connection.is_idle_expired()
    }) {
        if let Some(connection) = cached.as_ref() {
            connection.evict();
        }
        *cached = None;
    }
    tracing::debug!(authority, "Establishing sibling HTTP/2 asset connection");
    let connection = tokio::time::timeout(
        CONNECTION_WAIT_TIMEOUT,
        establish(route, reserve_native_budget),
    )
    .await
    .map_err(|_| {
        H2ConnectError::new(
            H2ConnectFailureKind::Tcp,
            format!(
                "timed out establishing asset HTTP/2 connection to {authority}"
            ),
        )
    })??;
    *cached = Some(Arc::clone(&connection));
    Ok(connection)
}

pub(crate) async fn has_live_connection(authority: &str) -> bool {
    let connections = CONNECTIONS.lock().await;
    let Some(slot) = connections.get(authority).cloned() else {
        return false;
    };
    drop(connections);
    let live = slot
        .lock()
        .await
        .as_ref()
        .is_some_and(|connection| !connection.is_dead());
    live
}
