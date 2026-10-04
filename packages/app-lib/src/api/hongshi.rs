use eyre::{Context, bail};
use futures::{FutureExt, StreamExt, future::join_all};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, lookup_host};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use super::multiplayer::{self, MultiplayerProvider};

const API_BASE: &str = "https://hongshi.site";
const NODE_ENDPOINT: &str = "https://hongshi.site/api/server/list";
const CONTROL_PORT: u16 = 8080;
const DATA_PORT: u16 = 8000;
const NODE_PROBE_TIMEOUT: Duration = Duration::from_millis(1500);
const START_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_BINARY_SIZE: u64 = 256 * 1024 * 1024;
const MAX_OUTPUT_LINE: usize = 64 * 1024;
const NODE_CACHE_TTL: Duration = Duration::from_secs(60);
const RATE_LIMIT_RETRY_DELAY: Duration = Duration::from_secs(61);

static HONGSHI_STATE: LazyLock<Mutex<HongshiState>> =
    LazyLock::new(|| Mutex::new(HongshiState::default()));
static HONGSHI_RUNTIME: LazyLock<Mutex<HongshiRuntime>> =
    LazyLock::new(|| Mutex::new(HongshiRuntime::default()));
static BINARY_CHECK: LazyLock<Mutex<Option<BinaryCheck>>> =
    LazyLock::new(|| Mutex::new(None));
static NODE_CACHE: LazyLock<Mutex<NodeCache>> =
    LazyLock::new(|| Mutex::new(NodeCache::default()));
static DETECTED_PORTS: LazyLock<Mutex<HashMap<String, DetectedLanPort>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ANSI_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\))")
        .expect("valid ANSI pattern")
});
static LAN_PORT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)local game hosted on port\s+(\d{1,5})",
        r"(?i)started serving on(?:\s+port)?\s+(\d{1,5})",
        r"(?i)successfully opened port\s+(\d{1,5})",
    ]
    .into_iter()
    .map(|pattern| Regex::new(pattern).expect("valid LAN port pattern"))
    .collect()
});

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum HongshiStatus {
    Unsupported,
    #[default]
    Idle,
    Downloading,
    SelectingNode,
    Starting,
    Open,
    Closed,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HongshiErrorType {
    Unsupported,
    BuildUnavailable,
    NodeList,
    NodeUnavailable,
    InvalidPort,
    Install,
    KernelStart,
    KernelExit,
    KernelOutput,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HongshiNode {
    pub name: String,
    pub address: String,
    pub latency_ms: Option<u64>,
    pub reachable: bool,
    pub cached: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetectedLanPort {
    pub instance_id: String,
    pub instance_name: String,
    pub process_id: String,
    pub port: u16,
    pub detected_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HongshiState {
    pub supported: bool,
    pub status: HongshiStatus,
    pub local_port: Option<u16>,
    pub node: Option<HongshiNode>,
    pub public_address: Option<String>,
    pub created_at: Option<String>,
    pub last_exit_code: Option<i32>,
    pub error_type: Option<HongshiErrorType>,
    pub error_message: Option<String>,
    pub bound_instance_id: Option<String>,
    pub port_changed: bool,
    pub binary_installed: bool,
    pub download_progress: Option<u8>,
}

impl Default for HongshiState {
    fn default() -> Self {
        Self {
            supported: is_supported(),
            status: if is_supported() {
                HongshiStatus::Idle
            } else {
                HongshiStatus::Unsupported
            },
            local_port: None,
            node: None,
            public_address: None,
            created_at: None,
            last_exit_code: None,
            error_type: None,
            error_message: None,
            bound_instance_id: None,
            port_changed: false,
            binary_installed: false,
            download_progress: None,
        }
    }
}

#[derive(Default)]
struct HongshiRuntime {
    generation: u64,
    session: Option<Session>,
}

struct Session {
    id: u64,
    cancellation: CancellationToken,
    done: watch::Receiver<bool>,
}

pub struct PendingStart(oneshot::Receiver<eyre::Result<()>>);

impl PendingStart {
    pub async fn wait(self) -> eyre::Result<()> {
        self.0
            .await
            .wrap_err("RedStone session task stopped unexpectedly")?
    }
}

#[derive(Debug)]
struct Failure {
    kind: HongshiErrorType,
    message: String,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Failure {}

fn failure(kind: HongshiErrorType, message: impl Into<String>) -> eyre::Report {
    eyre::Report::new(Failure {
        kind,
        message: message.into(),
    })
}

fn classify(
    error: &eyre::Report,
    default: HongshiErrorType,
) -> HongshiErrorType {
    error
        .downcast_ref::<Failure>()
        .map_or(default, |failure| failure.kind)
}

#[cfg(target_os = "windows")]
struct JobGuard(windows::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
unsafe impl Send for JobGuard {}

#[cfg(target_os = "windows")]
impl Drop for JobGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(not(target_os = "windows"))]
struct JobGuard;

#[cfg(target_os = "windows")]
fn attach_kill_on_close_job(process_id: u32) -> eyre::Result<JobGuard> {
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };
    unsafe {
        let job = CreateJobObjectW(None, windows::core::PCWSTR::null())
            .wrap_err("failed to create RedStone job object")?;
        let guard = JobGuard(job);
        let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        information.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&information).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .wrap_err("failed to configure RedStone job object")?;
        let process = OpenProcess(
            PROCESS_SET_QUOTA | PROCESS_TERMINATE,
            false,
            process_id,
        )
        .wrap_err("failed to open RedStone process")?;
        let assignment = AssignProcessToJobObject(job, process);
        let _ = windows::Win32::Foundation::CloseHandle(process);
        assignment
            .wrap_err("failed to assign RedStone process to job object")?;
        Ok(guard)
    }
}

#[cfg(not(target_os = "windows"))]
fn attach_kill_on_close_job(_process_id: u32) -> eyre::Result<JobGuard> {
    Ok(JobGuard)
}

fn hidden_command(path: &Path) -> Command {
    let mut command = Command::new(path);
    command.kill_on_drop(true).stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    command.creation_flags(0x0800_0000);
    command
}

pub fn is_supported() -> bool {
    cfg!(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(
            target_os = "macos",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
    ))
}

fn hongshi_root() -> PathBuf {
    let base = crate::state::DirectoryInfo::global_handle_if_ready()
        .map(|directories| directories.config_dir.clone())
        .or_else(|| {
            crate::state::DirectoryInfo::initial_settings_dir_path(
                crate::brand::BUNDLE_IDENTIFIER,
            )
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("hongshi")
}

pub fn logs_dir() -> PathBuf {
    hongshi_root().join("logs")
}
fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "hongshic.exe"
    } else {
        "hongshic"
    }
}
fn binary_path() -> PathBuf {
    hongshi_root().join("v2").join(binary_name())
}
fn node_cache_path() -> PathBuf {
    hongshi_root().join("v2").join("nodes.json")
}

fn valid_binary(bytes: &[u8]) -> bool {
    if cfg!(target_os = "windows") {
        if bytes.len() < 0x40 || !bytes.starts_with(b"MZ") {
            return false;
        }
        let offset =
            u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
        offset.checked_add(4).is_some_and(|end| {
            end <= bytes.len() && bytes[offset..end] == *b"PE\0\0"
        })
    } else if cfg!(target_os = "linux") {
        bytes.starts_with(b"\x7fELF")
    } else if cfg!(target_os = "macos") {
        matches!(
            bytes.get(..4),
            Some([0xfe, 0xed, 0xfa, 0xce])
                | Some([0xce, 0xfa, 0xed, 0xfe])
                | Some([0xfe, 0xed, 0xfa, 0xcf])
                | Some([0xcf, 0xfa, 0xed, 0xfe])
                | Some([0xca, 0xfe, 0xba, 0xbe])
                | Some([0xbe, 0xba, 0xfe, 0xca])
        )
    } else {
        false
    }
}

struct BinaryCheck {
    path: PathBuf,
    modified: Option<SystemTime>,
    length: u64,
    permissions: std::fs::Permissions,
    compatible: bool,
}

async fn compatible_help(
    path: &Path,
    cancellation: &CancellationToken,
) -> eyre::Result<bool> {
    if cancellation.is_cancelled() {
        bail!("RedStone operation cancelled");
    }
    let mut child = hidden_command(path)
        .arg("--print-help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| eyre::eyre!("missing help output"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| eyre::eyre!("missing help errors"))?;
    let read = async {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let mut stdout = stdout.take((MAX_OUTPUT_LINE + 1) as u64);
        let mut stderr = stderr.take((MAX_OUTPUT_LINE + 1) as u64);
        let (status, _, _) = tokio::try_join!(
            child.wait(),
            stdout.read_to_end(&mut output),
            stderr.read_to_end(&mut errors)
        )?;
        let help = String::from_utf8_lossy(&output);
        Ok::<_, eyre::Report>(
            status.success()
                && output.len() <= MAX_OUTPUT_LINE
                && errors.len() <= MAX_OUTPUT_LINE
                && [
                    "--to",
                    "--game-port",
                    "--game-host",
                    "--control-port",
                    "--data-port",
                ]
                .iter()
                .all(|flag| help.contains(flag)),
        )
    };
    let result = tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err(eyre::eyre!("RedStone operation cancelled")),
        result = tokio::time::timeout(Duration::from_secs(5), read) => result.wrap_err("RedStone help check timed out").and_then(|result| result),
    };
    if result.is_err() {
        let _ = child.kill().await;
    }
    result
}

async fn compatible_binary(path: &Path) -> bool {
    compatible_binary_cancellable(path, &CancellationToken::new()).await
}

async fn compatible_binary_cancellable(
    path: &Path,
    cancellation: &CancellationToken,
) -> bool {
    let mut cached = tokio::select! {
        biased;
        _ = cancellation.cancelled() => return false,
        cached = BINARY_CHECK.lock() => cached,
    };
    let Ok(metadata) = tokio::fs::metadata(path).await else {
        *cached = None;
        return false;
    };
    if let Some(check) = cached.as_ref()
        && check.path == path
        && check.modified == metadata.modified().ok()
        && check.length == metadata.len()
        && check.permissions == metadata.permissions()
    {
        return check.compatible;
    }
    let compatible = metadata.len() <= MAX_BINARY_SIZE
        && tokio::fs::read(path)
            .await
            .is_ok_and(|data| valid_binary(&data))
        && compatible_help(path, cancellation).await.unwrap_or(false);
    if cancellation.is_cancelled() {
        return false;
    }
    *cached = Some(BinaryCheck {
        path: path.to_path_buf(),
        modified: metadata.modified().ok(),
        length: metadata.len(),
        permissions: metadata.permissions(),
        compatible,
    });
    compatible
}

fn download_endpoint_for(os: &str, architecture: &str) -> eyre::Result<String> {
    let platform = match os {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        other => bail!("RedStone is not available on {other}"),
    };
    let arch = match architecture {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => bail!("RedStone is not available on {other} architecture"),
    };
    Ok(format!(
        "{API_BASE}/api/download/client?platform={platform}&arch={arch}"
    ))
}

fn retry_after_delay(response: &reqwest::Response) -> Duration {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(RATE_LIMIT_RETRY_DELAY)
}

fn api_error_message(status: reqwest::StatusCode, body: &str) -> String {
    let json = serde_json::from_str::<serde_json::Value>(body).ok();
    let detail = json.as_ref().and_then(|json| json.get("detail"));
    let message = detail
        .and_then(|value| {
            value.as_str().or_else(|| {
                value.get("message").and_then(|value| value.as_str())
            })
        })
        .unwrap_or(status.as_str());
    let expected = detail
        .and_then(|value| value.get("expected_file"))
        .and_then(|value| value.as_str());
    expected.map_or_else(
        || message.to_string(),
        |file| format!("{message} ({file})"),
    )
}

async fn download_client() -> eyre::Result<reqwest::Client> {
    crate::util::fetch::configured_client()
        .await
        .map_err(|error| eyre::eyre!(error.to_string()))
}

async fn cancellable<T>(
    cancellation: &CancellationToken,
    future: impl Future<Output = eyre::Result<T>>,
) -> eyre::Result<T> {
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => bail!("RedStone operation cancelled"),
        result = future => result,
    }
}

async fn update_state(id: u64, update: impl FnOnce(&mut HongshiState)) {
    let runtime = HONGSHI_RUNTIME.lock().await;
    if runtime
        .session
        .as_ref()
        .is_some_and(|session| session.id == id)
    {
        update(&mut *HONGSHI_STATE.lock().await);
    }
}

async fn install_from(
    client: &reqwest::Client,
    endpoint: &str,
    path: &Path,
    id: u64,
    cancellation: &CancellationToken,
) -> eyre::Result<()> {
    let temporary = path.with_file_name(if cfg!(target_os = "windows") {
        "hongshic.download.exe"
    } else {
        "hongshic.download"
    });
    let download = async {
        let mut response = client
            .get(endpoint)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let delay = retry_after_delay(&response);
            warn!(
                retry_after_seconds = delay.as_secs(),
                "RedStone download was rate limited"
            );
            tokio::time::sleep(delay).await;
            response = client
                .get(endpoint)
                .header(reqwest::header::ACCEPT, "application/json")
                .send()
                .await?;
        }
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(failure(
                if status == reqwest::StatusCode::NOT_FOUND {
                    HongshiErrorType::BuildUnavailable
                } else {
                    HongshiErrorType::Install
                },
                api_error_message(status, &body),
            ));
        }
        let total = response.content_length().unwrap_or(0);
        if total > MAX_BINARY_SIZE {
            bail!("RedStone kernel download is too large");
        }
        let mut data = Vec::with_capacity(total.min(MAX_BINARY_SIZE) as usize);
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if data.len() as u64 + chunk.len() as u64 > MAX_BINARY_SIZE {
                bail!("RedStone kernel download is too large");
            }
            data.extend_from_slice(&chunk);
            if total > 0 {
                update_state(id, |state| {
                    state.download_progress =
                        Some(((data.len() as u64 * 100 / total).min(100)) as u8)
                })
                .await;
            }
        }
        if !valid_binary(&data) {
            bail!("downloaded RedStone kernel failed executable validation");
        }
        tokio::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| eyre::eyre!("invalid kernel path"))?,
        )
        .await?;
        tokio::fs::write(&temporary, &data).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(
                &temporary,
                std::fs::Permissions::from_mode(0o755),
            )
            .await?;
        }
        Ok::<_, eyre::Report>(())
    };
    let install = async {
        cancellable(cancellation, download).await?;
        if !compatible_help(&temporary, cancellation).await? {
            bail!(
                "downloaded RedStone kernel does not support the RedStone Online 2 arguments"
            );
        }
        if cancellation.is_cancelled() {
            bail!("RedStone operation cancelled");
        }
        atomic_replace(&temporary, path).await?;
        *BINARY_CHECK.lock().await = None;
        Ok(())
    };
    let result = install.await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&temporary).await;
    }
    result
}

async fn prepare_binary(
    id: u64,
    cancellation: &CancellationToken,
    force: bool,
) -> eyre::Result<PathBuf> {
    prepare_binary_in(&hongshi_root(), id, cancellation, force).await
}

async fn prepare_binary_in(
    root: &Path,
    id: u64,
    cancellation: &CancellationToken,
    force: bool,
) -> eyre::Result<PathBuf> {
    let path = root.join("v2").join(binary_name());
    let compatible = compatible_binary_cancellable(&path, cancellation).await;
    if cancellation.is_cancelled() {
        bail!("RedStone operation cancelled");
    }
    update_state(id, |state| state.binary_installed = compatible).await;
    if compatible && !force {
        return Ok(path);
    }
    update_state(id, |state| {
        state.status = HongshiStatus::Downloading;
        state.download_progress = Some(0);
    })
    .await;
    let client = cancellable(cancellation, download_client()).await?;
    let endpoint =
        download_endpoint_for(std::env::consts::OS, std::env::consts::ARCH)?;
    let result =
        install_from(&client, &endpoint, &path, id, cancellation).await;
    if let Err(error) = result {
        return Err(failure(
            classify(&error, HongshiErrorType::Install),
            format!("{error:#}"),
        ));
    }
    for legacy in [
        root.join(if cfg!(target_os = "windows") {
            "hongshi.exe"
        } else {
            "hongshi"
        }),
        root.join("nodes.json"),
    ] {
        if let Err(error) = tokio::fs::remove_file(&legacy).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(%error, path = %legacy.display(), "failed to remove legacy RedStone file");
        }
    }
    update_state(id, |state| {
        state.binary_installed = true;
        state.download_progress = None;
    })
    .await;
    Ok(path)
}

#[cfg(target_os = "windows")]
async fn atomic_replace(source: &Path, destination: &Path) -> eyre::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MOVE_FILE_FLAGS, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        MoveFileExW,
    };
    use windows::core::PCWSTR;
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVE_FILE_FLAGS(
                MOVEFILE_REPLACE_EXISTING.0 | MOVEFILE_WRITE_THROUGH.0,
            ),
        )
        .wrap_err("failed to atomically replace RedStone file")?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn atomic_replace(source: &Path, destination: &Path) -> eyre::Result<()> {
    tokio::fs::rename(source, destination).await?;
    Ok(())
}

fn unsafe_node_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_loopback() || ip.is_link_local() || ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unicast_link_local()
                || ip.is_unspecified()
        }
    }
}

fn validate_host(host: &str) -> eyre::Result<String> {
    let host = host.trim();
    if host.is_empty()
        || host.len() > 253
        || host.contains(['/', '\\', ':', '?', '#', '@'])
        || host.chars().any(char::is_whitespace)
    {
        bail!("invalid RedStone node address: {host}");
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if unsafe_node_ip(ip) {
            bail!("unsafe RedStone node address: {host}");
        }
    } else if !host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '-'
            })
    }) {
        bail!("invalid RedStone node hostname: {host}");
    }
    Ok(host.to_string())
}

fn parse_node_map(bytes: &[u8]) -> eyre::Result<BTreeMap<String, String>> {
    let raw: BTreeMap<String, String> = serde_json::from_slice(bytes)
        .wrap_err("failed to parse RedStone node list")?;
    raw.into_iter()
        .map(|(name, address)| {
            let name = name.trim();
            if name.is_empty() || name.len() > 64 {
                bail!("invalid RedStone node name");
            }
            Ok((name.to_string(), validate_host(&address)?))
        })
        .collect()
}

#[derive(Default)]
struct NodeCache {
    map: Option<(BTreeMap<String, String>, Instant, bool)>,
    retry_until: Option<Instant>,
}

async fn cached_nodes(
    path: &Path,
) -> eyre::Result<(BTreeMap<String, String>, bool)> {
    Ok((
        parse_node_map(&tokio::fs::read(path).await.wrap_err(
            "failed to fetch RedStone nodes and no cache is available",
        )?)?,
        true,
    ))
}

async fn load_node_map_from(
    client: &reqwest::Client,
    endpoint: &str,
    path: &Path,
    cache: &Mutex<NodeCache>,
    force: bool,
) -> eyre::Result<(BTreeMap<String, String>, bool)> {
    let mut cache = cache.lock().await;
    let cooling_down = cache
        .retry_until
        .is_some_and(|deadline| deadline > Instant::now());
    if let Some((map, fetched, cached)) = &cache.map
        && (cooling_down || (!force && fetched.elapsed() < NODE_CACHE_TTL))
    {
        return Ok((map.clone(), *cached));
    }
    if cooling_down {
        return cached_nodes(path).await;
    }
    match client
        .get(endpoint)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            match response.bytes().await {
                Ok(bytes) => {
                    let map = parse_node_map(&bytes)?;
                    let temporary = path.with_extension("json.new");
                    let write = async {
                        if let Some(parent) = path.parent() {
                            tokio::fs::create_dir_all(parent).await?;
                        }
                        tokio::fs::write(&temporary, serde_json::to_vec(&map)?)
                            .await?;
                        atomic_replace(&temporary, path).await
                    };
                    if let Err(error) = write.await {
                        warn!(%error, "failed to cache RedStone node list");
                        let _ = tokio::fs::remove_file(&temporary).await;
                    }
                    cache.map = Some((map.clone(), Instant::now(), false));
                    cache.retry_until = None;
                    return Ok((map, false));
                }
                Err(error) => {
                    warn!(%error, "failed to read RedStone node list")
                }
            }
        }
        Ok(response) => {
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                cache.retry_until =
                    Instant::now().checked_add(retry_after_delay(&response));
            }
            warn!(status = %response.status(), "RedStone node list request failed");
        }
        Err(error) => warn!(%error, "failed to fetch RedStone nodes"),
    }
    let (map, cached) = if let Some((map, _, _)) = &cache.map {
        (map.clone(), true)
    } else {
        cached_nodes(path).await?
    };
    cache.map = Some((map.clone(), Instant::now(), cached));
    Ok((map, cached))
}

async fn probe_node(
    name: String,
    address: String,
    cached: bool,
) -> HongshiNode {
    let started = Instant::now();
    let reachable = tokio::time::timeout(NODE_PROBE_TIMEOUT, async {
        let addresses = lookup_host((address.as_str(), CONTROL_PORT)).await?;
        for socket in addresses.filter(|socket| !unsafe_node_ip(socket.ip())) {
            if TcpStream::connect(socket).await.is_ok() {
                return Ok::<_, std::io::Error>(true);
            }
        }
        Ok(false)
    })
    .await
    .is_ok_and(|result| result.unwrap_or(false));
    HongshiNode {
        name,
        address,
        latency_ms: reachable.then(|| started.elapsed().as_millis() as u64),
        reachable,
        cached,
    }
}

pub async fn get_nodes(force_refresh: bool) -> eyre::Result<Vec<HongshiNode>> {
    let client = download_client().await?;
    let (nodes, cached) = load_node_map_from(
        &client,
        NODE_ENDPOINT,
        &node_cache_path(),
        &NODE_CACHE,
        force_refresh,
    )
    .await?;
    let mut nodes = join_all(
        nodes
            .into_iter()
            .map(|(name, address)| probe_node(name, address, cached)),
    )
    .await;
    nodes.sort_by_key(|node| {
        (
            !node.reachable,
            node.latency_ms.unwrap_or(u64::MAX),
            node.name.clone(),
        )
    });
    Ok(nodes)
}

pub async fn get_detected_ports() -> Vec<DetectedLanPort> {
    let mut ports = DETECTED_PORTS
        .lock()
        .await
        .values()
        .cloned()
        .collect::<Vec<_>>();
    ports.sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    ports
}

pub async fn get_state() -> HongshiState {
    if HONGSHI_RUNTIME.lock().await.session.is_none() {
        let installed = compatible_binary(&binary_path()).await;
        let runtime = HONGSHI_RUNTIME.lock().await;
        if runtime.session.is_none() {
            HONGSHI_STATE.lock().await.binary_installed = installed;
        }
    }
    HONGSHI_STATE.lock().await.clone()
}

fn parse_endpoint(line: &str) -> eyre::Result<Option<String>> {
    let cleaned = ANSI_PATTERN.replace_all(line, "");
    let Some(endpoint) = cleaned
        .split_whitespace()
        .find_map(|field| field.strip_prefix("endpoint="))
    else {
        return Ok(None);
    };
    let parsed = reqwest::Url::parse(&format!("tcp://{endpoint}"))
        .wrap_err("invalid RedStone endpoint")?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != ""
        || parsed.port().is_none_or(|port| port == 0)
    {
        bail!("invalid RedStone endpoint");
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| eyre::eyre!("missing RedStone endpoint host"))?;
    validate_host(host)?;
    Ok(Some(endpoint.to_string()))
}

enum OutputEvent {
    Endpoint(String),
    Error(String),
}

async fn read_output<R: AsyncRead + Unpin>(
    reader: R,
    stdout: bool,
    log: Arc<Mutex<tokio::fs::File>>,
    sender: mpsc::Sender<OutputEvent>,
) {
    let mut lines = FramedRead::new(
        reader,
        LinesCodec::new_with_max_length(MAX_OUTPUT_LINE),
    );
    while let Some(line) = lines.next().await {
        match line {
            Ok(line) => {
                let cleaned = ANSI_PATTERN.replace_all(&line, "");
                if let Err(error) = log
                    .lock()
                    .await
                    .write_all(
                        format!(
                            "{} {cleaned}\n",
                            if stdout { "stdout" } else { "stderr" }
                        )
                        .as_bytes(),
                    )
                    .await
                {
                    warn!(%error, "failed to write RedStone log");
                }
                if stdout {
                    match parse_endpoint(&line) {
                        Ok(Some(endpoint)) => {
                            if sender
                                .send(OutputEvent::Endpoint(endpoint))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            let _ = sender
                                .send(OutputEvent::Error(error.to_string()))
                                .await;
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                let _ =
                    sender.send(OutputEvent::Error(error.to_string())).await;
                break;
            }
        }
    }
}

struct Kernel {
    child: Child,
    _job: JobGuard,
    readers: Vec<JoinHandle<()>>,
    output: mpsc::Receiver<OutputEvent>,
}

impl Drop for Kernel {
    fn drop(&mut self) {
        for reader in &self.readers {
            reader.abort();
        }
    }
}

async fn spawn_kernel(
    binary: &Path,
    node: &HongshiNode,
    port: u16,
    id: u64,
) -> eyre::Result<Kernel> {
    tokio::fs::create_dir_all(logs_dir()).await?;
    let filename = format!(
        "session-{}-{id}-{}.log",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    let log = Arc::new(Mutex::new(
        tokio::fs::File::create(logs_dir().join(filename)).await?,
    ));
    let mut child = hidden_command(binary)
        .args([
            "-t",
            &node.address,
            "-p",
            &port.to_string(),
            "--game-host",
            "127.0.0.1",
            "--control-port",
            &CONTROL_PORT.to_string(),
            "--data-port",
            &DATA_PORT.to_string(),
        ])
        .env("RUST_LOG", "info")
        .env("NO_COLOR", "1")
        .current_dir(
            binary
                .parent()
                .ok_or_else(|| eyre::eyre!("invalid kernel path"))?,
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .wrap_err("failed to start RedStone kernel")?;
    let process_id = child
        .id()
        .ok_or_else(|| eyre::eyre!("RedStone process has no id"))?;
    let job = attach_kill_on_close_job(process_id)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| eyre::eyre!("missing RedStone stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| eyre::eyre!("missing RedStone stderr"))?;
    let (sender, output) = mpsc::channel(16);
    let readers = vec![
        tokio::spawn(read_output(stdout, true, log.clone(), sender.clone())),
        tokio::spawn(read_output(stderr, false, log, sender)),
    ];
    info!(
        process_id,
        session_id = id,
        "started RedStone Online 2 kernel"
    );
    Ok(Kernel {
        child,
        _job: job,
        readers,
        output,
    })
}

enum Attempt {
    Exited(i32, bool),
    TimedOut,
    Cancelled,
    OutputError(String),
}

async fn supervise_kernel(
    kernel: &mut Kernel,
    id: u64,
    cancellation: &CancellationToken,
    ready: &mut Option<oneshot::Sender<eyre::Result<()>>>,
    timeout: Duration,
) -> eyre::Result<Attempt> {
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    let mut opened = false;
    let mut output_closed = false;
    loop {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Ok(Attempt::Cancelled),
            status = kernel.child.wait() => return Ok(Attempt::Exited(status?.code().unwrap_or(-1), opened)),
            _ = &mut deadline, if !opened => return Ok(Attempt::TimedOut),
            event = kernel.output.recv(), if !output_closed => match event {
                Some(OutputEvent::Endpoint(endpoint)) if !opened => {
                    if let Some(status) = kernel.child.try_wait()? { return Ok(Attempt::Exited(status.code().unwrap_or(-1), false)); }
                    update_state(id, |state| { state.status = HongshiStatus::Open; state.public_address = Some(endpoint); state.created_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()); }).await;
                    opened = true;
                    if let Some(sender) = ready.take() { let _ = sender.send(Ok(())); }
                }
                Some(OutputEvent::Endpoint(_)) => {}
                Some(OutputEvent::Error(error)) => return Ok(Attempt::OutputError(error)),
                None => output_closed = true,
            },
        }
    }
}

async fn finish_kernel(kernel: &mut Kernel) {
    if kernel.child.try_wait().ok().flatten().is_none() {
        let _ = kernel.child.start_kill();
        let _ =
            tokio::time::timeout(Duration::from_secs(3), kernel.child.wait())
                .await;
    }
    for reader in &mut kernel.readers {
        if tokio::time::timeout(Duration::from_secs(1), &mut *reader)
            .await
            .is_err()
        {
            reader.abort();
            let _ = reader.await;
        }
    }
}

async fn host_session(
    id: u64,
    port: u16,
    node_name: Option<String>,
    cancellation: &CancellationToken,
    ready: &mut Option<oneshot::Sender<eyre::Result<()>>>,
) -> eyre::Result<()> {
    let binary = prepare_binary(id, cancellation, false).await?;
    update_state(id, |state| state.status = HongshiStatus::SelectingNode).await;
    let nodes =
        cancellable(cancellation, get_nodes(true))
            .await
            .map_err(|error| {
                failure(HongshiErrorType::NodeList, format!("{error:#}"))
            })?;
    host_kernels(id, port, node_name, &binary, nodes, cancellation, ready).await
}

async fn host_kernels(
    id: u64,
    port: u16,
    node_name: Option<String>,
    binary: &Path,
    mut nodes: Vec<HongshiNode>,
    cancellation: &CancellationToken,
    ready: &mut Option<oneshot::Sender<eyre::Result<()>>>,
) -> eyre::Result<()> {
    if let Some(name) = node_name.as_deref() {
        nodes.retain(|node| node.name == name);
    } else {
        nodes.retain(|node| node.reachable);
    }
    if nodes.is_empty() {
        return Err(failure(
            HongshiErrorType::NodeUnavailable,
            "no selected or reachable RedStone node is available",
        ));
    }
    for node in nodes {
        if cancellation.is_cancelled() {
            bail!("RedStone operation cancelled");
        }
        update_state(id, |state| {
            state.status = HongshiStatus::Starting;
            state.node = Some(node.clone());
            state.last_exit_code = None;
        })
        .await;
        let mut kernel =
            cancellable(cancellation, spawn_kernel(binary, &node, port, id))
                .await
                .map_err(|error| {
                    failure(HongshiErrorType::KernelStart, format!("{error:#}"))
                })?;
        let attempt = supervise_kernel(
            &mut kernel,
            id,
            cancellation,
            ready,
            START_TIMEOUT,
        )
        .await;
        finish_kernel(&mut kernel).await;
        match attempt? {
            Attempt::Cancelled => bail!("RedStone operation cancelled"),
            Attempt::Exited(code, opened) => {
                update_state(id, |state| state.last_exit_code = Some(code))
                    .await;
                if code == 0 {
                    update_state(id, |state| {
                        state.status = HongshiStatus::Closed;
                        state.public_address = None;
                    })
                    .await;
                    if let Some(sender) = ready.take() {
                        let _ = sender.send(Err(failure(
                            HongshiErrorType::KernelStart,
                            "RedStone room closed before returning an address",
                        )));
                    }
                    return Ok(());
                }
                if !opened && code == 1 && node_name.is_none() {
                    continue;
                }
                return Err(failure(
                    if !opened && code == 1 {
                        HongshiErrorType::NodeUnavailable
                    } else {
                        HongshiErrorType::KernelExit
                    },
                    format!("RedStone kernel exited with code {code}"),
                ));
            }
            Attempt::TimedOut if node_name.is_none() => continue,
            Attempt::TimedOut => {
                return Err(failure(
                    HongshiErrorType::KernelStart,
                    "timed out waiting for RedStone tunnel creation",
                ));
            }
            Attempt::OutputError(error) => {
                return Err(failure(HongshiErrorType::KernelOutput, error));
            }
        }
    }
    Err(failure(
        HongshiErrorType::NodeUnavailable,
        "all RedStone nodes failed to create a tunnel",
    ))
}

async fn reserve_session(
    port: Option<u16>,
    instance_id: Option<String>,
) -> eyre::Result<(u64, CancellationToken, watch::Sender<bool>)> {
    let mut runtime = HONGSHI_RUNTIME.lock().await;
    if !is_supported() {
        return Err(failure(
            HongshiErrorType::Unsupported,
            "RedStone is not supported on this platform",
        ));
    }
    if runtime.session.is_some() {
        bail!("RedStone is already running");
    }
    multiplayer::claim_provider(MultiplayerProvider::Hongshi).await?;
    runtime.generation += 1;
    let id = runtime.generation;
    let cancellation = CancellationToken::new();
    let (done, receiver) = watch::channel(false);
    runtime.session = Some(Session {
        id,
        cancellation: cancellation.clone(),
        done: receiver,
    });
    let installed = HONGSHI_STATE.lock().await.binary_installed;
    *HONGSHI_STATE.lock().await = HongshiState {
        local_port: port,
        bound_instance_id: instance_id,
        binary_installed: installed,
        status: if port.is_some() {
            HongshiStatus::Starting
        } else {
            HongshiStatus::Downloading
        },
        ..HongshiState::default()
    };
    Ok((id, cancellation, done))
}

async fn finish_session(
    id: u64,
    cancellation: &CancellationToken,
    result: eyre::Result<()>,
    ready: &mut Option<oneshot::Sender<eyre::Result<()>>>,
    done: watch::Sender<bool>,
) {
    if cancellation.is_cancelled() {
        update_state(id, |state| {
            let installed = state.binary_installed;
            *state = HongshiState {
                binary_installed: installed,
                ..HongshiState::default()
            };
        })
        .await;
    } else if let Err(error) = &result {
        update_state(id, |state| {
            state.status = HongshiStatus::Error;
            state.public_address = None;
            state.download_progress = None;
            state.error_type = Some(classify(error, HongshiErrorType::Unknown));
            state.error_message = Some(format!("{error:#}"));
        })
        .await;
    }
    let mut runtime = HONGSHI_RUNTIME.lock().await;
    if runtime
        .session
        .as_ref()
        .is_some_and(|session| session.id == id)
    {
        runtime.session = None;
        multiplayer::release_provider(MultiplayerProvider::Hongshi).await;
    }
    if let Some(sender) = ready.take() {
        let _ = sender.send(result);
    }
    let _ = done.send(true);
}

pub async fn begin_start(
    local_port: u16,
    node_name: Option<String>,
    instance_id: Option<String>,
) -> eyre::Result<PendingStart> {
    if local_port == 0 {
        return Err(failure(
            HongshiErrorType::InvalidPort,
            "invalid local port",
        ));
    }
    let ports = DETECTED_PORTS.lock().await;
    if let Some(instance_id) = instance_id.as_deref() {
        let port = ports.get(instance_id).ok_or_else(|| {
            eyre::eyre!("selected Minecraft instance is no longer running")
        })?;
        if port.port != local_port {
            bail!("selected Minecraft instance opened a different LAN port");
        }
    }
    let (id, cancellation, done) =
        reserve_session(Some(local_port), instance_id).await?;
    drop(ports);
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let mut ready = Some(sender);
        let result = std::panic::AssertUnwindSafe(host_session(
            id,
            local_port,
            node_name,
            &cancellation,
            &mut ready,
        ))
        .catch_unwind()
        .await
        .unwrap_or_else(|_| {
            Err(failure(
                HongshiErrorType::KernelExit,
                "RedStone session task panicked",
            ))
        });
        finish_session(id, &cancellation, result, &mut ready, done).await;
    });
    Ok(PendingStart(receiver))
}

pub async fn download() -> eyre::Result<()> {
    let (id, cancellation, done) = reserve_session(None, None).await?;
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let mut ready = Some(sender);
        let result = std::panic::AssertUnwindSafe(prepare_binary(
            id,
            &cancellation,
            true,
        ))
        .catch_unwind()
        .await
        .unwrap_or_else(|_| {
            Err(failure(
                HongshiErrorType::Install,
                "RedStone installation task panicked",
            ))
        })
        .map(|_| ());
        if result.is_ok() {
            update_state(id, |state| state.status = HongshiStatus::Idle).await;
        }
        finish_session(id, &cancellation, result, &mut ready, done).await;
    });
    PendingStart(receiver).wait().await
}

pub async fn stop() -> eyre::Result<()> {
    let session = {
        let runtime = HONGSHI_RUNTIME.lock().await;
        runtime.session.as_ref().map(|session| {
            session.cancellation.cancel();
            (session.id, session.done.clone())
        })
    };
    if let Some((id, mut done)) = session {
        while !*done.borrow() {
            done.changed()
                .await
                .wrap_err("RedStone cleanup task stopped unexpectedly")?;
        }
        let runtime = HONGSHI_RUNTIME.lock().await;
        if runtime.generation == id && runtime.session.is_none() {
            let installed = HONGSHI_STATE.lock().await.binary_installed;
            *HONGSHI_STATE.lock().await = HongshiState {
                binary_installed: installed,
                ..HongshiState::default()
            };
        }
    }
    Ok(())
}

pub async fn observe_minecraft_log(
    instance_id: &str,
    instance_name: &str,
    process_id: &str,
    message: &str,
) {
    let port = LAN_PORT_PATTERNS.iter().find_map(|pattern| {
        pattern
            .captures(message)
            .and_then(|captures| captures.get(1))
            .and_then(|value| value.as_str().parse::<u16>().ok())
            .filter(|port| *port > 0)
    });
    let Some(port) = port else {
        return;
    };
    DETECTED_PORTS.lock().await.insert(
        instance_id.to_string(),
        DetectedLanPort {
            instance_id: instance_id.to_string(),
            instance_name: instance_name.to_string(),
            process_id: process_id.to_string(),
            port,
            detected_at: chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        },
    );
    let mut state = HONGSHI_STATE.lock().await;
    if state.bound_instance_id.as_deref() == Some(instance_id)
        && state.local_port.is_some_and(|current| current != port)
    {
        state.port_changed = true;
    }
}

pub async fn minecraft_process_finished(instance_id: &str) {
    DETECTED_PORTS.lock().await.remove(instance_id);
    let should_stop = HONGSHI_STATE.lock().await.bound_instance_id.as_deref()
        == Some(instance_id);
    if should_stop && let Err(error) = stop().await {
        warn!(%error, "failed to stop RedStone after Minecraft exited");
    }
}

#[cfg(test)]
mod tests;
