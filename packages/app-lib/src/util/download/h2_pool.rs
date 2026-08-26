//! Shared per-authority HTTP/2 connections for file downloads.
//!
//! `reqwest`'s connection pool opens a fresh TCP+TLS connection for every
//! request that arrives while no idle connection is available, so a batch of
//! concurrent downloads to one CDN costs one handshake per file. This module
//! instead maintains a single long-lived HTTP/2 connection per authority and
//! multiplexes every download as a separate stream over it (`SendRequest` is
//! cheap to clone and each clone opens an independent stream). Handshakes
//! happen once per authority, and large files can also split into range
//! streams over the same connection.

use bytes::Bytes;
use h2::client::SendRequest;
use rustls::ClientConfig;
use rustls_pki_types::ServerName;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Mutex as AsyncMutex;
use tokio_rustls::TlsConnector;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

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
    sender: Mutex<SendRequest<Bytes>>,
    /// Set to true by the driver task when the connection terminates.
    dead: Arc<std::sync::atomic::AtomicBool>,
}

impl SharedH2Connection {
    fn new(sender: SendRequest<Bytes>) -> Self {
        Self {
            sender: Mutex::new(sender),
            dead: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn is_dead(&self) -> bool {
        self.dead.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Sends a request on the shared connection and awaits the response
    /// headers, yielding the response and its receive stream. Each call
    /// opens an independent multiplexed stream.
    pub async fn open(
        &self,
        request: http::Request<()>,
    ) -> Result<http::Response<h2::RecvStream>, h2::Error> {
        let sender = self
            .sender
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let mut sender = sender.ready().await?;
        let (response, _) = sender.send_request(request, true)?;
        response.await
    }
}

type ConnectionSlot = Arc<AsyncMutex<Option<Arc<SharedH2Connection>>>>;

/// Registry of live shared connections, keyed by authority.
static CONNECTIONS: std::sync::LazyLock<
    AsyncMutex<HashMap<String, ConnectionSlot>>,
> = std::sync::LazyLock::new(|| AsyncMutex::new(HashMap::new()));

async fn connection_slot(authority: &str) -> ConnectionSlot {
    let mut connections = CONNECTIONS.lock().await;
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

async fn connect_tcp(host: &str, port: u16) -> std::io::Result<TcpStream> {
    // Prefer the ordered address list from the shared download resolver
    // (IPv4/IPv6 preference and per-IP reliability), falling back to the
    // system resolver when no list is cached yet.
    let addresses =
        crate::util::fetch::DOWNLOAD_DNS_RESOLVER.resolved_addresses(host);
    let mut last_error = None;
    if !addresses.is_empty() {
        for address in addresses {
            match tokio::time::timeout(
                CONNECT_TIMEOUT,
                TcpStream::connect((address, port)),
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
    authority: &str,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    let (host, port) = authority
        .rsplit_once(':')
        .map(|(host, port)| (host, port.parse::<u16>().unwrap_or(443)))
        .unwrap_or((authority, 443));

    // Pre-resolve so `connect_tcp` gets the ordered, reliability-ranked
    // address list shared with the legacy reqwest path.
    crate::util::fetch::DOWNLOAD_DNS_RESOLVER
        .pre_resolve(host)
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

    let (sender, mut connection) = h2::client::handshake(Box::pin(tls))
        .await
        .map_err(|error| {
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
    // `set_target_window_size` returns `()`, while `set_initial_window_size`
    // returns a `Result`, so only the latter needs its error captured.
    connection.set_target_window_size(64 * 1024 * 1024);
    let _ = connection.set_initial_window_size(1024 * 1024);

    let shared = Arc::new(SharedH2Connection::new(sender));

    let dead = Arc::clone(&shared.dead);
    let authority = authority.to_string();
    tokio::spawn(async move {
        let _ = connection.await;
        dead.store(true, std::sync::atomic::Ordering::Release);
        tracing::debug!(authority, "Shared HTTP/2 connection closed");
    });

    Ok(shared)
}

/// Returns the live shared connection for `authority`, establishing one on
/// first use or after a previous connection died.
pub(crate) async fn shared_connection(
    authority: &str,
) -> Result<Arc<SharedH2Connection>, H2ConnectError> {
    let slot = connection_slot(authority).await;
    let mut cached = slot.lock().await;
    if let Some(connection) =
        cached.as_ref().filter(|connection| !connection.is_dead())
    {
        return Ok(Arc::clone(connection));
    }
    let connection = establish(authority).await?;
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
