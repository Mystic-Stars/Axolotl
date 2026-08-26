use eyre::{Context, bail};
use futures::future::join_all;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::net::{TcpStream, lookup_host};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use super::multiplayer::{self, MultiplayerProvider};

const API_BASE: &str = "https://hongshi.site";
const NODE_ENDPOINT: &str = "https://hongshi.site/newserver.json";
const CONTROL_PORT: u16 = 7000;
const NODE_PROBE_TIMEOUT: Duration = Duration::from_millis(1500);
const START_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_BINARY_SIZE: u64 = 256 * 1024 * 1024;
const DOWNLOAD_URL_CACHE_TTL: Duration = Duration::from_secs(28 * 60);
const DOWNLOAD_RATE_LIMIT_RETRY_DELAY: Duration = Duration::from_secs(61);

static HONGSHI_STATE: LazyLock<Mutex<HongshiState>> =
    LazyLock::new(|| Mutex::new(HongshiState::default()));
static HONGSHI_RUNTIME: LazyLock<Mutex<HongshiRuntime>> =
    LazyLock::new(|| Mutex::new(HongshiRuntime::default()));
static HONGSHI_OPERATION: LazyLock<Mutex<()>> =
    LazyLock::new(|| Mutex::new(()));
static HONGSHI_DOWNLOAD_URL: LazyLock<Mutex<Option<CachedDownloadUrl>>> =
    LazyLock::new(|| Mutex::new(None));
static DETECTED_PORTS: LazyLock<Mutex<HashMap<String, DetectedLanPort>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static LAN_PORT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)local game hosted on port\s+(\d{1,5})",
        r"(?i)started serving on(?:\s+port)?\s+(\d{1,5})",
        r"(?i)successfully opened port\s+(\d{1,5})",
    ]
    .into_iter()
    .map(|pattern| Regex::new(pattern).expect("LAN port regex should compile"))
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
    WaitingForPort,
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
    NodeList,
    NodeUnavailable,
    InvalidPort,
    Install,
    KernelStart,
    KernelExit,
    StatusFile,
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
        let supported = is_supported();
        Self {
            supported,
            status: if supported {
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
            binary_installed: binary_installed(),
            download_progress: None,
        }
    }
}

#[derive(Default)]
struct HongshiRuntime {
    child: Option<Arc<Mutex<Child>>>,
    job: Option<JobGuard>,
    monitor: Option<JoinHandle<()>>,
    status_file: Option<PathBuf>,
    stopping: bool,
}

#[derive(Debug)]
struct TunnelStatusFile {
    status: String,
    server: String,
    port: i32,
    created: Option<String>,
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
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
#[derive(Debug)]
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
        .wrap_err("failed to open RedStone process for job assignment")?;
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
    let base_dir = crate::state::DirectoryInfo::global_handle_if_ready()
        .map(|directories| directories.config_dir.clone())
        .or_else(|| {
            crate::state::DirectoryInfo::initial_settings_dir_path(
                crate::brand::BUNDLE_IDENTIFIER,
            )
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base_dir.join("hongshi")
}

pub fn logs_dir() -> PathBuf {
    hongshi_root().join("logs")
}

fn binary_path() -> PathBuf {
    hongshi_root().join(binary_name())
}

fn binary_installed() -> bool {
    std::fs::read(binary_path()).is_ok_and(|bytes| valid_binary(&bytes))
}

fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "hongshi.exe"
    } else {
        "hongshi"
    }
}

fn status_file_path() -> PathBuf {
    std::env::temp_dir()
        .join("axolotl-hongshi")
        .join(format!("tunnel-{}.ini", std::process::id()))
}

fn node_cache_path() -> PathBuf {
    hongshi_root().join("nodes.json")
}

fn valid_pe(bytes: &[u8]) -> bool {
    if bytes.len() < 0x40 || bytes[0..2] != [b'M', b'Z'] {
        return false;
    }
    let pe_offset =
        u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    pe_offset.checked_add(4).is_some_and(|end| {
        end <= bytes.len() && bytes[pe_offset..end] == *b"PE\0\0"
    })
}

fn valid_binary(bytes: &[u8]) -> bool {
    if cfg!(target_os = "windows") {
        valid_pe(bytes)
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

#[derive(Debug, Deserialize)]
struct HongshiDownloadResponse {
    url: String,
}

#[derive(Debug, Deserialize)]
struct HongshiApiError {
    detail: String,
}

#[derive(Debug)]
struct CachedDownloadUrl {
    url: reqwest::Url,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HongshiNodeResponse {
    Map(BTreeMap<String, String>),
    List(Vec<HongshiNodeEntry>),
}

#[derive(Debug, Deserialize)]
struct HongshiNodeEntry {
    host: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    region: Option<String>,
}

fn download_endpoint_for(os: &str, architecture: &str) -> eyre::Result<String> {
    let platform = match os {
        "windows" => "windows",
        "macos" => "darwin",
        "linux" => "linux",
        other => bail!("RedStone is not available on {other}"),
    };
    let arch = match architecture {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => bail!("RedStone is not available on {other} architecture"),
    };
    Ok(if platform == "windows" {
        format!("{API_BASE}/api/download/windows")
    } else {
        format!("{API_BASE}/api/download/{platform}?arch={arch}")
    })
}

fn download_endpoint() -> eyre::Result<String> {
    download_endpoint_for(std::env::consts::OS, std::env::consts::ARCH)
}

fn is_daily_download_limit(detail: &str) -> bool {
    let detail = detail.to_ascii_lowercase();
    detail.contains("daily")
        || detail.contains("today")
        || detail.contains("每日")
        || detail.contains("今日")
        || detail.contains("当天")
}

fn retry_after_delay(response: &reqwest::Response) -> Duration {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(DOWNLOAD_RATE_LIMIT_RETRY_DELAY)
}

async fn parse_api_error(response: reqwest::Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    serde_json::from_str::<HongshiApiError>(&body)
        .map(|error| error.detail)
        .unwrap_or_else(|_| {
            if body.trim().is_empty() {
                status.to_string()
            } else {
                body
            }
        })
}

async fn download_client() -> eyre::Result<reqwest::Client> {
    crate::util::fetch::configured_client()
        .await
        .map_err(|error| eyre::eyre!(error.to_string()))
}

async fn request_download_url(endpoint: &str) -> eyre::Result<reqwest::Url> {
    {
        let mut cached = HONGSHI_DOWNLOAD_URL.lock().await;
        if let Some(entry) = cached.as_ref()
            && entry.expires_at > Instant::now()
        {
            return Ok(entry.url.clone());
        }
        *cached = None;
    }

    let client = download_client().await?;
    let mut response = client
        .get(endpoint)
        .send()
        .await
        .wrap_err("failed to request RedStone download URL")?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let delay = retry_after_delay(&response);
        let detail = parse_api_error(response).await;
        if is_daily_download_limit(&detail) {
            bail!("RedStone daily download limit reached: {detail}")
        }
        warn!(
            retry_after_seconds = delay.as_secs(),
            "RedStone download URL request was rate limited; retrying once"
        );
        tokio::time::sleep(delay).await;
        response = client
            .get(endpoint)
            .send()
            .await
            .wrap_err("failed to retry RedStone download URL request")?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let detail = parse_api_error(response).await;
            bail!(
                "RedStone download API is still rate limited after waiting; the daily limit may have been reached: {detail}"
            )
        }
    }

    let response = response
        .error_for_status()
        .wrap_err("RedStone download URL request failed")?
        .json::<HongshiDownloadResponse>()
        .await
        .wrap_err("invalid RedStone download response")?;
    let download_url = reqwest::Url::parse(&response.url)
        .wrap_err("invalid RedStone kernel download URL")?;
    if download_url.scheme() != "https" {
        bail!("RedStone kernel download URL must use HTTPS")
    }
    *HONGSHI_DOWNLOAD_URL.lock().await = Some(CachedDownloadUrl {
        url: download_url.clone(),
        expires_at: Instant::now() + DOWNLOAD_URL_CACHE_TTL,
    });
    Ok(download_url)
}

async fn download_inner() -> eyre::Result<()> {
    let _operation = HONGSHI_OPERATION.lock().await;
    if !is_supported() {
        bail!("RedStone is not supported on this platform")
    }
    if HONGSHI_RUNTIME.lock().await.child.is_some() {
        bail!(
            "cannot replace RedStone while the multiplayer service is running"
        )
    }

    let endpoint = download_endpoint()?;
    {
        let mut state = HONGSHI_STATE.lock().await;
        state.status = HongshiStatus::Downloading;
        state.download_progress = Some(0);
        state.error_type = None;
        state.error_message = None;
    }
    let download_url = request_download_url(&endpoint).await?;
    let client = download_client().await?;
    let response = client
        .get(download_url)
        .send()
        .await
        .wrap_err("failed to download RedStone kernel")?
        .error_for_status()
        .wrap_err("RedStone kernel download failed")?;
    let total = response.content_length().unwrap_or(0);
    if total > MAX_BINARY_SIZE {
        bail!("RedStone kernel download is too large")
    }
    let mut downloaded = 0_u64;
    let mut data = Vec::with_capacity(total.min(MAX_BINARY_SIZE) as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
        let chunk = chunk.wrap_err("failed to read RedStone kernel")?;
        downloaded += chunk.len() as u64;
        if downloaded > MAX_BINARY_SIZE {
            bail!("RedStone kernel download is too large")
        }
        data.extend_from_slice(&chunk);
        if total > 0 {
            HONGSHI_STATE.lock().await.download_progress =
                Some(((downloaded * 100 / total).min(100)) as u8);
        }
    }
    if data.len() as u64 > MAX_BINARY_SIZE || !valid_binary(&data) {
        bail!("downloaded RedStone kernel failed executable validation")
    }
    let path = binary_path();
    let parent = path
        .parent()
        .ok_or_else(|| eyre::eyre!("invalid RedStone kernel path"))?;
    tokio::fs::create_dir_all(parent)
        .await
        .wrap_err("failed to create RedStone directory")?;
    let temporary = path.with_extension("download");
    tokio::fs::write(&temporary, &data)
        .await
        .wrap_err("failed to stage RedStone kernel")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&temporary)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&temporary, permissions)?;
    }
    atomic_replace(&temporary, &path)
        .await
        .wrap_err("failed to install RedStone kernel")?;
    let mut state = HONGSHI_STATE.lock().await;
    state.binary_installed = true;
    state.status = HongshiStatus::Idle;
    state.download_progress = None;
    Ok(())
}

pub async fn download() -> eyre::Result<()> {
    let result = download_inner().await;
    if let Err(error) = &result {
        let mut state = HONGSHI_STATE.lock().await;
        state.status = HongshiStatus::Error;
        state.download_progress = None;
        state.error_type = Some(HongshiErrorType::Install);
        state.error_message = Some(format!("{error:#}"));
    }
    result
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

async fn ensure_binary() -> eyre::Result<PathBuf> {
    let path = binary_path();
    if let Ok(existing) = tokio::fs::read(&path).await
        && valid_binary(&existing)
    {
        return Ok(path);
    }
    bail!("RedStone kernel is not installed; download it first")
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

fn parse_node_map(bytes: &[u8]) -> eyre::Result<BTreeMap<String, String>> {
    let raw: HongshiNodeResponse = serde_json::from_slice(bytes)
        .wrap_err("failed to parse RedStone node list")?;
    let raw = match raw {
        HongshiNodeResponse::Map(nodes) => nodes,
        HongshiNodeResponse::List(nodes) => nodes
            .into_iter()
            .enumerate()
            .map(|(index, node)| {
                let name = node
                    .name
                    .or(node.region)
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| format!("Node {}", index + 1));
                (name, node.host)
            })
            .collect(),
    };
    if raw.is_empty() {
        bail!("RedStone node list is empty");
    }

    let mut nodes = BTreeMap::new();
    for (name, address) in raw {
        let name = name.trim();
        if name.is_empty() || name.len() > 64 {
            bail!("invalid RedStone node name");
        }
        nodes.insert(name.to_string(), validate_host(&address)?);
    }
    Ok(nodes)
}

async fn write_node_cache(
    nodes: &BTreeMap<String, String>,
) -> eyre::Result<()> {
    let path = node_cache_path();
    let parent = path
        .parent()
        .ok_or_else(|| eyre::eyre!("invalid RedStone node cache path"))?;
    tokio::fs::create_dir_all(parent).await?;
    let temporary = path.with_extension("json.new");
    tokio::fs::write(&temporary, serde_json::to_vec(nodes)?).await?;
    atomic_replace(&temporary, &path).await?;
    Ok(())
}

async fn load_node_map(
    _force_refresh: bool,
) -> eyre::Result<(BTreeMap<String, String>, bool)> {
    let client = download_client().await?;
    match client.get(NODE_ENDPOINT).send().await {
        Ok(response) => match response.error_for_status() {
            Ok(response) => {
                let bytes = response.bytes().await?;
                let nodes = parse_node_map(&bytes)?;
                if let Err(error) = write_node_cache(&nodes).await {
                    warn!("failed to cache RedStone node list: {error:#}");
                }
                return Ok((nodes, false));
            }
            Err(error) => {
                warn!("RedStone node endpoint returned an error: {error}")
            }
        },
        Err(error) => warn!("failed to fetch RedStone nodes: {error}"),
    }

    let cached = tokio::fs::read(node_cache_path())
        .await
        .wrap_err("failed to fetch RedStone nodes and no cache is available")?;
    Ok((parse_node_map(&cached)?, true))
}

async fn probe_node(
    name: String,
    address: String,
    cached: bool,
) -> HongshiNode {
    let started = Instant::now();
    let socket = lookup_host((address.as_str(), CONTROL_PORT))
        .await
        .ok()
        .and_then(|mut addresses| {
            addresses.find(|socket| !unsafe_node_ip(socket.ip()))
        });
    let reachable = if let Some(socket) = socket {
        tokio::time::timeout(NODE_PROBE_TIMEOUT, TcpStream::connect(socket))
            .await
            .is_ok_and(|result| result.is_ok())
    } else {
        false
    };
    HongshiNode {
        name,
        address,
        latency_ms: reachable.then(|| started.elapsed().as_millis() as u64),
        reachable,
        cached,
    }
}

pub async fn get_nodes(force_refresh: bool) -> eyre::Result<Vec<HongshiNode>> {
    let (nodes, cached) = load_node_map(force_refresh).await?;
    let mut nodes = join_all(
        nodes
            .into_iter()
            .map(|(name, address)| probe_node(name, address, cached)),
    )
    .await;
    sort_nodes(&mut nodes);
    Ok(nodes)
}

fn sort_nodes(nodes: &mut [HongshiNode]) {
    nodes.sort_by_key(|node| {
        (
            !node.reachable,
            node.latency_ms.unwrap_or(u64::MAX),
            node.name.clone(),
        )
    });
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
    let mut state = HONGSHI_STATE.lock().await;
    if !matches!(
        state.status,
        HongshiStatus::Downloading
            | HongshiStatus::SelectingNode
            | HongshiStatus::Starting
            | HongshiStatus::Open
    ) {
        state.binary_installed = binary_installed();
    }
    state.clone()
}

fn parse_tunnel_status(contents: &str) -> eyre::Result<TunnelStatusFile> {
    let mut in_tunnel = false;
    let mut values = HashMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_tunnel = &line[1..line.len() - 1] == "tunnel";
            continue;
        }
        if in_tunnel && let Some((key, value)) = line.split_once('=') {
            values.insert(
                key.trim().to_string(),
                value.split(';').next().unwrap_or("").trim().to_string(),
            );
        }
    }

    let status = values.remove("status").unwrap_or_default();
    if status != "open" && status != "closed" {
        bail!("invalid RedStone tunnel status");
    }
    let server = validate_host(&values.remove("server").unwrap_or_default())?;
    let port = values
        .remove("port")
        .unwrap_or_default()
        .parse::<i32>()
        .wrap_err("invalid RedStone tunnel port")?;
    if (status == "open" && !(1..=65535).contains(&port))
        || (status == "closed" && port != -1)
    {
        bail!("invalid RedStone tunnel port for status {status}");
    }
    Ok(TunnelStatusFile {
        status,
        server,
        port,
        created: values.remove("created"),
    })
}

async fn read_fresh_status(
    path: &Path,
    started_at: SystemTime,
) -> eyre::Result<Option<TunnelStatusFile>> {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH) < started_at {
        return Ok(None);
    }
    let contents = tokio::fs::read_to_string(path).await?;
    parse_tunnel_status(&contents).map(Some)
}

async fn process_exit_code(
    child: &Arc<Mutex<Child>>,
) -> eyre::Result<Option<i32>> {
    let mut child = child.lock().await;
    Ok(child.try_wait()?.map(|status| status.code().unwrap_or(-1)))
}

async fn set_start_error(
    error_type: HongshiErrorType,
    message: String,
    exit_code: Option<i32>,
) {
    let mut state = HONGSHI_STATE.lock().await;
    state.status = HongshiStatus::Error;
    state.public_address = None;
    state.error_type = Some(error_type);
    state.error_message = Some(message);
    state.last_exit_code = exit_code;
}

fn classify_start_error(message: &str) -> HongshiErrorType {
    if message.contains("install RedStone kernel") {
        HongshiErrorType::Install
    } else if message.contains("node")
        || message.contains("fetch RedStone nodes")
    {
        HongshiErrorType::NodeList
    } else if message.contains("status") || message.contains("tunnel creation")
    {
        HongshiErrorType::StatusFile
    } else {
        HongshiErrorType::KernelStart
    }
}

fn error_type_for_exit(exit_code: i32) -> HongshiErrorType {
    match exit_code {
        1 => HongshiErrorType::NodeUnavailable,
        2 => HongshiErrorType::KernelStart,
        _ => HongshiErrorType::KernelExit,
    }
}

fn should_try_next_node(automatic: bool, exit_code: i32) -> bool {
    automatic && exit_code == 1
}

async fn spawn_kernel(
    binary: &Path,
    node: &HongshiNode,
    local_port: u16,
    status_file: &Path,
) -> eyre::Result<(Arc<Mutex<Child>>, JobGuard, SystemTime)> {
    let root = hongshi_root();
    tokio::fs::create_dir_all(&root).await?;
    if let Some(parent) = status_file.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::remove_file(status_file).await;
    let started_at = SystemTime::now();
    let mut child = Command::new(binary)
        .arg("-server")
        .arg(&node.address)
        .arg("-port")
        .arg(local_port.to_string())
        .arg("-status-file")
        .arg(status_file)
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .wrap_err("failed to start RedStone kernel")?;
    let process_id = child
        .id()
        .ok_or_else(|| eyre::eyre!("RedStone process has no process id"))?;
    let job = match attach_kill_on_close_job(process_id) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.start_kill();
            return Err(error);
        }
    };
    info!("started RedStone kernel with pid {process_id}");
    Ok((Arc::new(Mutex::new(child)), job, started_at))
}

async fn wait_until_open(
    child: &Arc<Mutex<Child>>,
    status_file: &Path,
    started_at: SystemTime,
) -> eyre::Result<Result<TunnelStatusFile, i32>> {
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if let Some(exit_code) = process_exit_code(child).await? {
            return Ok(Err(exit_code));
        }
        if let Some(status) = read_fresh_status(status_file, started_at).await?
        {
            if status.status == "open" {
                if let Some(exit_code) = process_exit_code(child).await? {
                    return Ok(Err(exit_code));
                }
                return Ok(Ok(status));
            }
            return Ok(Err(0));
        }
        if Instant::now() >= deadline {
            let mut child = child.lock().await;
            let _ = child.start_kill();
            bail!("timed out waiting for RedStone tunnel creation");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn monitor_kernel(
    child: Arc<Mutex<Child>>,
    status_file: PathBuf,
    started_at: SystemTime,
) {
    loop {
        match process_exit_code(&child).await {
            Ok(Some(exit_code)) => {
                let stopping = HONGSHI_RUNTIME.lock().await.stopping;
                if stopping {
                    let mut state = HONGSHI_STATE.lock().await;
                    *state = HongshiState::default();
                } else if exit_code == 0 {
                    let mut state = HONGSHI_STATE.lock().await;
                    state.status = HongshiStatus::Closed;
                    state.public_address = None;
                    state.last_exit_code = Some(exit_code);
                } else {
                    set_start_error(
                        HongshiErrorType::KernelExit,
                        format!("RedStone kernel exited with code {exit_code}"),
                        Some(exit_code),
                    )
                    .await;
                }
                break;
            }
            Ok(None) => {}
            Err(error) => {
                set_start_error(
                    HongshiErrorType::KernelExit,
                    error.to_string(),
                    None,
                )
                .await;
                break;
            }
        }

        match read_fresh_status(&status_file, started_at).await {
            Ok(Some(status)) if status.status == "closed" => {
                let mut state = HONGSHI_STATE.lock().await;
                state.status = HongshiStatus::Closed;
                state.public_address = None;
                state.created_at = status.created;
            }
            Ok(_) => {}
            Err(error) => {
                warn!("failed to read RedStone status file: {error:#}")
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    {
        let mut runtime = HONGSHI_RUNTIME.lock().await;
        runtime.child = None;
        runtime.job = None;
        runtime.status_file = None;
        runtime.stopping = false;
    }
    multiplayer::release_provider(MultiplayerProvider::Hongshi).await;
}

pub async fn start(
    local_port: u16,
    node_name: Option<String>,
    instance_id: Option<String>,
) -> eyre::Result<()> {
    let _operation = HONGSHI_OPERATION.lock().await;
    if !is_supported() {
        set_start_error(
            HongshiErrorType::Unsupported,
            "RedStone is not supported on this platform".to_string(),
            None,
        )
        .await;
        bail!("RedStone is not supported on this platform");
    }
    if local_port == 0 {
        set_start_error(
            HongshiErrorType::InvalidPort,
            "invalid local port".to_string(),
            None,
        )
        .await;
        bail!("invalid local port");
    }
    if HONGSHI_RUNTIME.lock().await.child.is_some() {
        bail!("RedStone is already running");
    }
    if let Some(instance_id) = instance_id.as_deref() {
        let ports = DETECTED_PORTS.lock().await;
        let detected = ports.get(instance_id).ok_or_else(|| {
            eyre::eyre!("selected Minecraft instance is no longer running")
        })?;
        if detected.port != local_port {
            bail!("selected Minecraft instance opened a different LAN port");
        }
    }
    multiplayer::claim_provider(MultiplayerProvider::Hongshi).await?;

    let result = async {
		{
			let mut state = HONGSHI_STATE.lock().await;
			state.status = HongshiStatus::SelectingNode;
			state.local_port = Some(local_port);
			state.node = None;
			state.public_address = None;
			state.created_at = None;
			state.last_exit_code = None;
			state.error_type = None;
			state.error_message = None;
			state.bound_instance_id = instance_id.clone();
			state.port_changed = false;
		}
        let binary = ensure_binary().await?;

		let mut nodes = get_nodes(false).await?;
		if let Some(name) = node_name.as_deref() {
			nodes.retain(|node| node.name == name);
			if nodes.is_empty() {
				bail!("selected RedStone node no longer exists");
			}
		} else {
			nodes.retain(|node| node.reachable);
		}
		if nodes.is_empty() {
			bail!("no reachable RedStone node is available");
		}

		let status_file = status_file_path();
		let automatic = node_name.is_none();
		let mut last_error = None;
		for node in nodes {
			{
				let mut state = HONGSHI_STATE.lock().await;
				state.status = HongshiStatus::Starting;
				state.node = Some(node.clone());
			}
			let (child, job, started_at) = spawn_kernel(&binary, &node, local_port, &status_file).await?;
			match wait_until_open(&child, &status_file, started_at).await? {
				Ok(tunnel) => {
					if tunnel.server != node.address {
						let mut child = child.lock().await;
						let _ = child.start_kill();
						bail!("RedStone status file returned an unexpected server");
					}
					let public_address = format!("{}:{}", tunnel.server, tunnel.port);
					{
						let mut state = HONGSHI_STATE.lock().await;
						state.status = HongshiStatus::Open;
						state.public_address = Some(public_address);
						state.created_at = tunnel.created;
						state.last_exit_code = None;
					}
					let monitor = tokio::spawn(monitor_kernel(
						child.clone(),
						status_file.clone(),
						started_at,
					));
					let mut runtime = HONGSHI_RUNTIME.lock().await;
					runtime.child = Some(child);
					runtime.job = Some(job);
					runtime.monitor = Some(monitor);
					runtime.status_file = Some(status_file);
					return Ok(());
				}
				Err(exit_code) => {
					last_error = Some(exit_code);
                    if !should_try_next_node(automatic, exit_code) {
						break;
					}
				}
			}
		}

		let exit_code = last_error.unwrap_or(-1);
        let error_type = error_type_for_exit(exit_code);
		let message =
			format!("RedStone failed to create a tunnel (exit code {exit_code}, type {error_type:?})");
		set_start_error(error_type, message.clone(), Some(exit_code)).await;
		bail!(message)
	}
	.await;

    if let Err(error) = result {
        if HONGSHI_STATE.lock().await.status != HongshiStatus::Error {
            let message = error.to_string();
            set_start_error(classify_start_error(&message), message, None)
                .await;
        }
        multiplayer::release_provider(MultiplayerProvider::Hongshi).await;
        return Err(error);
    }
    Ok(())
}

pub async fn stop() -> eyre::Result<()> {
    let _operation = HONGSHI_OPERATION.lock().await;
    let (child, job, monitor, status_file) = {
        let mut runtime = HONGSHI_RUNTIME.lock().await;
        runtime.stopping = true;
        (
            runtime.child.take(),
            runtime.job.take(),
            runtime.monitor.take(),
            runtime.status_file.take(),
        )
    };
    if let Some(child) = child {
        let mut child = child.lock().await;
        let _ = child.start_kill();
        let _ =
            tokio::time::timeout(Duration::from_secs(3), child.wait()).await;
    }
    if let Some(monitor) = monitor {
        monitor.abort();
    }
    drop(job);
    if let Some(path) = status_file {
        let _ = tokio::fs::remove_file(path).await;
    }
    {
        let mut runtime = HONGSHI_RUNTIME.lock().await;
        runtime.stopping = false;
    }
    *HONGSHI_STATE.lock().await = HongshiState::default();
    multiplayer::release_provider(MultiplayerProvider::Hongshi).await;
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
    let should_stop = {
        let state = HONGSHI_STATE.lock().await;
        state.bound_instance_id.as_deref() == Some(instance_id)
            && matches!(
                state.status,
                HongshiStatus::Starting | HongshiStatus::Open
            )
    };
    if should_stop && let Err(error) = stop().await {
        warn!("failed to stop RedStone after Minecraft exited: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_node_map_and_rejects_unsafe_hosts() {
        let nodes =
            parse_node_map(r#"{"南京":"119.45.52.17"}"#.as_bytes()).unwrap();
        assert_eq!(nodes.get("南京").map(String::as_str), Some("119.45.52.17"));
        assert!(parse_node_map(br#"{"local":"127.0.0.1"}"#).is_err());
        assert!(parse_node_map(br#"{"bad":"https://example.com"}"#).is_err());
        let nodes = parse_node_map(
            br#"[{"host":"relay.example.com","region":"cn-east"}]"#,
        )
        .unwrap();
        assert_eq!(
            nodes.get("cn-east").map(String::as_str),
            Some("relay.example.com")
        );
    }

    #[test]
    fn validates_supported_executable_formats() {
        if cfg!(target_os = "windows") {
            let mut binary = vec![0; 0x44];
            binary[0..2].copy_from_slice(b"MZ");
            binary[0x3c..0x40].copy_from_slice(&0x40_u32.to_le_bytes());
            binary[0x40..0x44].copy_from_slice(b"PE\0\0");
            assert!(valid_binary(&binary));
        } else if cfg!(target_os = "linux") {
            assert!(valid_binary(b"\x7fELFtest"));
        } else if cfg!(target_os = "macos") {
            assert!(valid_binary(&[0xcf, 0xfa, 0xed, 0xfe]));
        }
        assert!(!valid_binary(b"not an executable"));
    }

    #[test]
    fn maps_supported_platforms_to_download_endpoints() {
        assert_eq!(
            download_endpoint_for("windows", "x86_64").unwrap(),
            "https://hongshi.site/api/download/windows"
        );
        assert_eq!(
            download_endpoint_for("macos", "aarch64").unwrap(),
            "https://hongshi.site/api/download/darwin?arch=arm64"
        );
        assert_eq!(
            download_endpoint_for("linux", "x86_64").unwrap(),
            "https://hongshi.site/api/download/linux?arch=amd64"
        );
        assert!(download_endpoint_for("linux", "riscv64").is_err());
    }

    #[test]
    fn distinguishes_daily_download_limits_from_short_rate_limits() {
        assert!(is_daily_download_limit("今日下载次数已达上限"));
        assert!(is_daily_download_limit("Daily download limit reached"));
        assert!(!is_daily_download_limit("60 秒内请勿重复请求"));
        assert!(!is_daily_download_limit("Too many requests"));
    }

    #[test]
    fn parses_open_and_closed_status_files() {
        let open = parse_tunnel_status(
			"[tunnel]\nstatus=open\nserver=1.2.3.4\nport=41862\ncreated=2026-08-09 22:42:40\n",
		)
		.unwrap();
        assert_eq!(open.port, 41862);
        assert_eq!(open.server, "1.2.3.4");
        assert!(
            parse_tunnel_status(
                "[tunnel]\nstatus=closed\nserver=1.2.3.4\nport=-1\n"
            )
            .is_ok()
        );
        assert!(
            parse_tunnel_status(
                "[tunnel]\nstatus=open\nserver=1.2.3.4\nport=-1\n"
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn detects_minecraft_lan_ports() {
        observe_minecraft_log(
            "instance-a",
            "Test Instance",
            "process-a",
            "Local game hosted on port 54321",
        )
        .await;
        let ports = get_detected_ports().await;
        assert!(ports.iter().any(|entry| {
            entry.instance_id == "instance-a"
                && entry.instance_name == "Test Instance"
                && entry.process_id == "process-a"
                && entry.port == 54321
        }));
        minecraft_process_finished("instance-a").await;
    }

    #[test]
    fn sorts_reachable_nodes_by_latency() {
        let mut nodes = vec![
            HongshiNode {
                name: "slow".to_string(),
                address: "203.0.113.1".to_string(),
                latency_ms: Some(80),
                reachable: true,
                cached: false,
            },
            HongshiNode {
                name: "offline".to_string(),
                address: "203.0.113.2".to_string(),
                latency_ms: None,
                reachable: false,
                cached: false,
            },
            HongshiNode {
                name: "fast".to_string(),
                address: "203.0.113.3".to_string(),
                latency_ms: Some(20),
                reachable: true,
                cached: false,
            },
        ];
        sort_nodes(&mut nodes);
        assert_eq!(nodes[0].name, "fast");
        assert_eq!(nodes[1].name, "slow");
        assert_eq!(nodes[2].name, "offline");
    }

    #[test]
    fn maps_exit_codes_and_only_fails_over_automatic_nodes() {
        assert_eq!(error_type_for_exit(1), HongshiErrorType::NodeUnavailable);
        assert_eq!(error_type_for_exit(2), HongshiErrorType::KernelStart);
        assert_eq!(error_type_for_exit(3), HongshiErrorType::KernelExit);
        assert!(should_try_next_node(true, 1));
        assert!(!should_try_next_node(false, 1));
        assert!(!should_try_next_node(true, 2));
    }
}
