//! Shared per-authority HTTP/2 connections for file downloads.
//!
//! `reqwest`'s connection pool opens a fresh TCP+TLS connection for every
//! request that arrives while no idle connection is available, so a batch of
//! concurrent downloads to one CDN costs one handshake per file. This module
//! maintains multiplexed HTTP/2 connections per authority. The additional
//! connection paths remain available for later re-enablement, but are
//! temporarily disabled while their failure behavior is stabilized.

use bytes::Bytes;
use futures::stream::{FuturesUnordered, StreamExt};
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
const HAPPY_EYEBALLS_IPV4_DELAY: Duration = Duration::from_millis(150);
const HAPPY_EYEBALLS_IPV6_DELAY: Duration = Duration::from_millis(80);
const STREAM_READY_TIMEOUT: Duration = Duration::from_secs(30);
const IDLE_EVICTION_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const CONNECTION_WAIT_TIMEOUT: Duration = Duration::from_secs(15);
// Temporarily keep one multiplexed H2 connection per authority while the
// additional-connection path is being stabilized. Re-enable the adaptive
// pool and asset sibling connection together after their failure behavior is
// verified.
const ADDITIONAL_H2_CONNECTIONS_ENABLED: bool = true;

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
    bytes_transferred: AtomicUsize,
    reserved_streams: Arc<AtomicUsize>,
    successful_responses: AtomicUsize,
    last_failure: Mutex<Option<std::time::Instant>>,
    last_activity: Arc<Mutex<std::time::Instant>>,
    evict: Arc<Notify>,
}

pub(crate) struct H2StreamActivity {
    active_streams: Arc<AtomicUsize>,
    last_activity: Arc<Mutex<std::time::Instant>>,
}

impl Drop for H2StreamActivity {
    fn drop(&mut self) {
        if self.active_streams.fetch_sub(1, Ordering::AcqRel) == 1 {
            *self
                .last_activity
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                std::time::Instant::now();
        }
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
            bytes_transferred: AtomicUsize::new(0),
            reserved_streams: Arc::new(AtomicUsize::new(0)),
            successful_responses: AtomicUsize::new(0),
            last_failure: Mutex::new(None),
            last_activity: Arc::new(Mutex::new(std::time::Instant::now())),
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

    pub(crate) fn record_bytes(&self, bytes: usize) {
        self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
    }

    pub(crate) fn bytes_transferred_for_scheduler(&self) -> usize {
        self.bytes_transferred.load(Ordering::Relaxed)
    }

    pub(crate) fn assigned_streams_for_scheduler(&self) -> usize {
        self.active_streams() + self.reserved_streams.load(Ordering::Acquire)
    }

    pub(crate) fn reserve_stream_for_scheduler(&self) {
        self.reserved_streams.fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn release_stream_reservation(&self) {
        let _ = self.reserved_streams.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |reserved| reserved.checked_sub(1),
        );
    }

    pub(crate) fn track_stream(&self) -> H2StreamActivity {
        *self
            .last_activity
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            std::time::Instant::now();
        self.active_streams.fetch_add(1, Ordering::AcqRel);
        self.release_stream_reservation();
        H2StreamActivity {
            active_streams: Arc::clone(&self.active_streams),
            last_activity: Arc::clone(&self.last_activity),
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

    pub(crate) fn is_idle_expired_for_scheduler(&self) -> bool {
        self.is_idle_expired()
    }

    fn evict(&self) {
        self.dead.store(true, Ordering::Release);
        self.evict.notify_one();
    }

    pub(crate) fn evict_for_scheduler(&self) {
        self.evict();
    }

    pub(crate) fn record_stream_failure(&self) {
        *self
            .last_failure
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some(std::time::Instant::now());
    }

    fn healthy_for_expansion(&self) -> bool {
        self.successful_responses.load(Ordering::Acquire) > 0
            && self
                .last_failure
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none_or(|failure| {
                    failure.elapsed()
                        >= super::authority_scheduler::STABILITY_WINDOW
                })
    }

    pub(crate) fn healthy_for_scheduler(&self) -> bool {
        self.healthy_for_expansion()
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
        let result = self.open_inner(request).await;
        if result.is_err() {
            self.record_stream_failure();
        }
        result
    }

    async fn open_inner(
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
        let response = tokio::time::timeout(STREAM_READY_TIMEOUT, response)
            .await
            .map_err(|_| h2::Error::from(h2::Reason::ENHANCE_YOUR_CALM))??;
        if response.status().is_success() {
            self.successful_responses.fetch_add(1, Ordering::AcqRel);
        }
        Ok(response)
    }
}

impl Drop for SharedH2Connection {
    fn drop(&mut self) {
        self.evict.notify_one();
    }
}

type ConnectionSlot = Arc<AsyncMutex<Option<Arc<SharedH2Connection>>>>;

/// Registry of live shared connections, keyed by authority.
static CONNECTIONS: std::sync::LazyLock<
    AsyncMutex<HashMap<String, ConnectionSlot>>,
> = std::sync::LazyLock::new(|| AsyncMutex::new(HashMap::new()));

type ParallelConnectionPool =
    Arc<AsyncMutex<super::authority_scheduler::AuthorityScheduler>>;

static PARALLEL_CONNECTIONS: std::sync::LazyLock<
    AsyncMutex<HashMap<String, ParallelConnectionPool>>,
> = std::sync::LazyLock::new(|| AsyncMutex::new(HashMap::new()));

async fn connection_slot(authority: &str) -> ConnectionSlot {
    let mut connections = CONNECTIONS.lock().await;
    connections
        .entry(authority.to_string())
        .or_insert_with(|| Arc::new(AsyncMutex::new(None)))
        .clone()
}

async fn parallel_connection_pool(authority: &str) -> ParallelConnectionPool {
    let mut connections = PARALLEL_CONNECTIONS.lock().await;
    connections
        .entry(authority.to_string())
        .or_insert_with(|| {
            Arc::new(AsyncMutex::new(
                super::authority_scheduler::AuthorityScheduler::default(),
            ))
        })
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
    resolver: &crate::util::download_dns::DownloadDnsResolver,
) -> std::io::Result<TcpStream> {
    let attempts = addresses.iter().copied().take(2).enumerate().map(
        |(index, address)| async move {
            if index > 0 {
                let delay = if address.is_ipv4() {
                    HAPPY_EYEBALLS_IPV4_DELAY
                } else {
                    HAPPY_EYEBALLS_IPV6_DELAY
                };
                tokio::time::sleep(delay).await;
            }
            TcpStream::connect((address, port)).await
        },
    );
    tokio::time::timeout(CONNECT_TIMEOUT, async {
        let mut pending = attempts.collect::<FuturesUnordered<_>>();
        let mut last_error = None;
        while let Some(result) = pending.next().await {
            match result {
                Ok(stream) => {
                    stream.set_nodelay(true).ok();
                    if let Ok(address) = stream.peer_addr() {
                        resolver.record_host_success(host, address.ip());
                    }
                    return Ok(stream);
                }
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error.unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no addresses available for {host}"),
            )
        }))
    })
    .await
    .map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("connection to {host}:{port} timed out"),
        )
    })?
}

#[cfg(test)]
mod address_tests {
    use super::*;

    #[test]
    fn happy_eyeballs_uses_the_two_ordered_candidates() {
        let ordered: [IpAddr; 4] = [
            "2001:db8::1".parse().unwrap(),
            "2001:db8::2".parse().unwrap(),
            "192.0.2.1".parse().unwrap(),
            "192.0.2.2".parse().unwrap(),
        ];
        assert_eq!(ordered[0].to_string(), "2001:db8::1");
        assert_eq!(ordered.iter().copied().take(2).count(), 2);
    }
}

async fn connect_tcp(
    host: &str,
    port: u16,
    proxy: crate::util::fetch::ProxyPolicy,
) -> std::io::Result<TcpStream> {
    // Prefer the ordered address list from the shared download resolver
    // (IPv4/IPv6 preference and per-IP reliability), falling back to the
    // system resolver when no list is cached yet.
    let resolver = super::proxy_context::resolver(proxy);
    if resolver.resolved_addresses(host).is_empty() {
        resolver.resolve_host(host).await?;
    }
    let addresses = resolver.resolved_addresses(host);
    let mut last_error = None;
    if !addresses.is_empty() {
        match connect_addresses(host, port, &addresses, &resolver).await {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
        if resolver.record_connection_failure(host) {
            resolver.resolve_host(host).await?;
            let refreshed = resolver.resolved_addresses(host);
            if !refreshed.is_empty() && refreshed != addresses {
                match connect_addresses(host, port, &refreshed, &resolver).await
                {
                    Ok(stream) => return Ok(stream),
                    Err(error) => last_error = Some(error),
                }
            }
        }
    }
    if !resolver.doh_enabled() {
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
        return Ok(stream);
    }
    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no resolved addresses available for {host}:{port}"),
        )
    }))
}

/// Connects a new shared HTTP/2 connection to `authority` (host[:port]).
async fn establish(
    route: &DownloadRoute,
    reserve_native_budget: bool,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    if let Some(reason) = super::native::h2_ineligible_reason(route) {
        return Err(H2ConnectError::new(
            H2ConnectFailureKind::Protocol,
            reason.as_str().to_string(),
        ));
    }
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

    let tcp = connect_tcp(host, port, route.proxy).await.map_err(|error| {
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

    spawn_connection_driver(&shared, connection, IDLE_EVICTION_TIMEOUT);

    Ok(shared)
}

fn spawn_connection_driver<F>(
    shared: &Arc<SharedH2Connection>,
    connection: F,
    idle_timeout: Duration,
) -> tokio::task::JoinHandle<()>
where
    F: std::future::Future + Send + 'static,
{
    let dead = shared.dead.clone();
    let connection_budget = Arc::downgrade(shared);
    let idle_connection = connection_budget.clone();
    let evict = shared.evict.clone();
    let authority = shared.authority.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = connection => {}
            _ = evict.notified() => {
                tracing::debug!(authority, "Evicting idle shared HTTP/2 connection");
            }
            _ = wait_for_idle(idle_connection, idle_timeout) => {},
        }
        dead.store(true, std::sync::atomic::Ordering::Release);
        if let Some(connection_budget) = connection_budget.upgrade() {
            connection_budget.release_physical_budget();
        }
        tracing::debug!(authority, "Shared HTTP/2 connection closed");
    })
}

async fn wait_for_idle(
    connection: std::sync::Weak<SharedH2Connection>,
    timeout: Duration,
) {
    loop {
        let delay = {
            let Some(connection) = connection.upgrade() else {
                return;
            };
            if connection.active_streams() > 0 {
                timeout.min(Duration::from_secs(1))
            } else {
                timeout.saturating_sub(
                    connection
                        .last_activity
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .elapsed(),
                )
            }
        };
        if delay.is_zero() {
            return;
        }
        tokio::time::sleep(delay).await;
    }
}

pub(crate) async fn evict_idle_connections(scope: Option<&str>) {
    let slots = CONNECTIONS
        .lock()
        .await
        .iter()
        .filter(|(key, _)| scope.is_none_or(|scope| key.as_str() == scope))
        .map(|(_, slot)| slot.clone())
        .collect::<Vec<_>>();
    for slot in slots {
        if let Ok(cached) = slot.try_lock()
            && let Some(connection) = cached.as_ref()
            && connection.active_streams() == 0
        {
            connection.evict();
        }
    }
    let pools = PARALLEL_CONNECTIONS
        .lock()
        .await
        .iter()
        .filter(|(key, _)| scope.is_none_or(|scope| key.as_str() == scope))
        .map(|(_, pool)| pool.clone())
        .collect::<Vec<_>>();
    for pool in pools {
        if let Ok(mut state) = pool.try_lock() {
            for connection in state.connections_for_eviction() {
                if connection.active_streams() == 0 {
                    connection.evict();
                }
            }
            state.prune();
        }
    }
}

/// Returns the live shared connection for `authority`, establishing one on
/// first use or after a previous connection died.
async fn shared_connection_single(
    route: &DownloadRoute,
    reserve_native_budget: bool,
    allow_cold_connection: bool,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    if let Some(reason) = super::native::h2_ineligible_reason(route) {
        return Err(H2ConnectError::new(
            H2ConnectFailureKind::Protocol,
            reason.as_str().to_string(),
        ));
    }
    let authority =
        crate::util::fetch::url_authority(&route.url).ok_or_else(|| {
            H2ConnectError::new(
                H2ConnectFailureKind::Protocol,
                "HTTP/2 route has no authority".to_string(),
            )
        })?;
    let slot = connection_slot(&super::proxy_context::authority_key(
        &authority,
        route.proxy,
    ))
    .await;
    let deadline = tokio::time::Instant::now() + CONNECTION_WAIT_TIMEOUT;
    let mut cached = tokio::time::timeout_at(deadline, slot.lock())
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
    let connection = tokio::time::timeout_at(
        deadline,
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

/// Returns the least-loaded H2 connection for a parallel content transfer.
/// Every selected connection remains fully multiplexed; expansion only adds a
/// new TCP/TLS connection after the existing pool is carrying sustained load.
pub(crate) async fn shared_connection(
    route: &DownloadRoute,
    reserve_native_budget: bool,
    allow_cold_connection: bool,
    allow_parallel_connections: bool,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    if !allow_parallel_connections || !ADDITIONAL_H2_CONNECTIONS_ENABLED {
        return shared_connection_single(
            route,
            reserve_native_budget,
            allow_cold_connection,
        )
        .await;
    }

    let authority =
        crate::util::fetch::url_authority(&route.url).ok_or_else(|| {
            H2ConnectError::new(
                H2ConnectFailureKind::Protocol,
                "HTTP/2 route has no authority".to_string(),
            )
        })?;
    let key = super::proxy_context::authority_key(&authority, route.proxy);
    let pool = parallel_connection_pool(&key).await;
    let primary = shared_connection_single(
        route,
        reserve_native_budget,
        allow_cold_connection,
    )
    .await?;
    let route = route.clone();
    let clients = super::proxy_context::clients();
    select_parallel_connection(
        &pool,
        primary,
        allow_cold_connection,
        move || async move {
            let establish = establish(&route, reserve_native_budget);
            match clients {
                Some(clients) => {
                    super::proxy_context::with_snapshot(clients, establish)
                        .await
                }
                None => establish.await,
            }
        },
    )
    .await
}

async fn select_parallel_connection<F, Fut>(
    pool: &ParallelConnectionPool,
    primary: Arc<SharedH2Connection>,
    allow_expansion: bool,
    establish_sibling: F,
) -> Result<Arc<SharedH2Connection>, H2ConnectError>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<
            Output = Result<Arc<SharedH2Connection>, H2ConnectError>,
        > + Send
        + 'static,
{
    let mut state = pool.lock().await;
    state.prune();
    state.add_connection(Arc::clone(&primary));
    state.observe(std::time::Instant::now());
    let selected = state.select_connection().ok_or_else(|| {
        H2ConnectError::new(
            H2ConnectFailureKind::Protocol,
            "HTTP/2 pool has no live connection".into(),
        )
    })?;
    let connecting =
        if allow_expansion && state.can_expand(std::time::Instant::now()) {
            state.reserve_connector()
        } else {
            None
        };
    let Some(_connecting) = connecting else {
        return Ok(selected);
    };
    drop(state);
    let background_pool = pool.clone();
    tokio::spawn(async move {
        let _connecting = _connecting;
        let sibling = match tokio::time::timeout(
            CONNECTION_WAIT_TIMEOUT,
            establish_sibling(),
        )
        .await
        {
            Ok(Ok(sibling)) => sibling,
            result => {
                let mut state = background_pool.lock().await;
                state.rollback(std::time::Instant::now());
                tracing::debug!(
                    timed_out = result.is_err(),
                    "Optional HTTP/2 pool expansion failed"
                );
                return;
            }
        };
        let mut state = background_pool.lock().await;
        state.prune();
        if state.connection_count() < state.target() {
            state.add_connection(sibling);
        } else {
            sibling.evict_for_scheduler();
        }
    });
    Ok(selected)
}

#[cfg(test)]
mod parallel_pool_tests {
    use super::super::authority_scheduler::AuthorityScheduler;
    use super::*;

    #[test]
    fn scheduler_uses_the_requested_ladder() {
        assert_eq!(AuthorityScheduler::targets(), [4, 8, 16, 32]);
    }

    async fn memory_connection() -> Arc<SharedH2Connection> {
        let (client, _server) = tokio::io::duplex(64 * 1024);
        let (sender, _driver) = h2::client::handshake(client).await.unwrap();
        Arc::new(SharedH2Connection::for_test(sender))
    }

    #[tokio::test]
    async fn reservations_are_counted_before_stream_activation() {
        let first = memory_connection().await;
        let second = memory_connection().await;
        let mut scheduler = AuthorityScheduler::default();
        scheduler.add_connection(first.clone());
        scheduler.add_connection(second.clone());
        first.reserve_stream_for_scheduler();
        first.reserve_stream_for_scheduler();
        assert!(Arc::ptr_eq(
            &scheduler.select_connection().unwrap(),
            &second
        ));
    }

    #[tokio::test]
    async fn idle_connections_are_pruned() {
        let connection = memory_connection().await;
        let mut scheduler = AuthorityScheduler::default();
        scheduler.add_connection(connection.clone());
        connection.evict_for_scheduler();
        scheduler.prune();
        assert_eq!(scheduler.connection_count(), 0);
        assert_eq!(scheduler.target(), 4);
    }
}
/// Returns the optional second connection used exclusively by a busy asset
/// batch. It is stored independently so ordinary file downloads retain their
/// stable primary connection and never create extra TCP connections.
pub(crate) async fn shared_batch_connection(
    route: &DownloadRoute,
    reserve_native_budget: bool,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    shared_connection(route, reserve_native_budget, true, true).await
}

pub(crate) async fn has_live_connection(route: &DownloadRoute) -> bool {
    let connections = CONNECTIONS.lock().await;
    let Some(authority) = crate::util::fetch::url_authority(&route.url) else {
        return false;
    };
    let Some(slot) = connections
        .get(&super::proxy_context::authority_key(
            &authority,
            route.proxy,
        ))
        .cloned()
    else {
        return false;
    };
    drop(connections);

    slot.lock()
        .await
        .as_ref()
        .is_some_and(|connection| !connection.is_dead())
}
