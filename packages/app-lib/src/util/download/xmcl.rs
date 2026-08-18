//! XMCL-compatible download engine.
//!
//! This module ports the core invariants from XMCL's
//! `@xmcl/file-transfer` and `BmclDownloadController`:
//!
//! - all downloads start immediately; concurrency is bounded by
//!   per-authority socket pools instead of a global adaptive controller;
//! - large files are split into a fixed number of byte-range segments,
//!   each independently redirected and resumed;
//! - aggressive TTFB/stall/slow aborts with a committed finishing mode;
//! - optimal-stop source switching with a persistent speed reputation;
//! - a circuit breaker for failing reassignable CDNs.
//!
//! The current implementation is the single-stream resumable core. The
//! fixed-segment path and persisted reputation are added incrementally.

use super::super::fetch;
use super::shared::{SEGMENTED_DOWNLOAD_THRESHOLD, XMCL_RANGE_CONCURRENCY};
use super::slow::{SlowEvent, SlowRule};
use futures::StreamExt;
use reqwest::header;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Semaphore;

const XMCL_TTFB_TIMEOUT: Duration = Duration::from_secs(5);
const XMCL_STALL_TIMEOUT: Duration = Duration::from_secs(5);
const XMCL_SLOW_MIN_FLOW_SECS: Duration = Duration::from_secs(10);
const XMCL_SLOW_WINDOW: Duration = Duration::from_secs(3);
const XMCL_SLOW_CONSECUTIVE: u32 = 2;
const XMCL_STALL_FLOOR: u64 = 16 * 1024;
const XMCL_MIN_GLOBAL_SAMPLES: u64 = 3;
const XMCL_RECONNECT_OVERHEAD_SECS: f64 = 0.6;
const XMCL_MAX_RESUMES: usize = 5;
const XMCL_MAX_NO_PROGRESS: usize = 2;
const XMCL_BMCL_CONCURRENCY: usize = 16;
const XMCL_OTHER_CONCURRENCY: usize = 16;

static AUTHORITY_SEMAPHORES: LazyLock<Mutex<HashMap<String, Arc<Semaphore>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static GLOBAL_SPEED_BPS: AtomicU64 = AtomicU64::new(0);
static XMCL_SINGLE_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(10))
        .tcp_keepalive(Some(Duration::from_secs(10)))
        .tcp_nodelay(true)
        .pool_max_idle_per_host(64)
        .user_agent(crate::launcher_user_agent())
        .build()
        .expect("XMCL single-stream client configuration should be valid")
});
static XMCL_SEGMENT_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(10))
        .tcp_keepalive(Some(Duration::from_secs(10)))
        .tcp_nodelay(true)
        .pool_max_idle_per_host(16)
        .http1_only()
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent(crate::launcher_user_agent())
        .build()
        .expect("XMCL segment client configuration should be valid")
});

const CB_THRESHOLD: u32 = 16;
const CB_COOLDOWN: Duration = Duration::from_secs(30);
const CB_PROBE_EVERY: u32 = 24;
const MIN_MEASURE_BYTES: u64 = 64 * 1024;

#[derive(Default)]
struct Ewma {
    score: f64,
    weight: f64,
    count: u64,
}

#[derive(Default)]
struct Reputation {
    global: Ewma,
    hosts: HashMap<String, Ewma>,
}

#[derive(Default)]
struct BreakerState {
    fails: u32,
    open_until: Option<Instant>,
    probe_tick: u32,
}

static REPUTATION: LazyLock<Mutex<Reputation>> =
    LazyLock::new(|| Mutex::new(Reputation::default()));
static REPUTATION_LOADED: AtomicBool = AtomicBool::new(false);
static BREAKER: LazyLock<Mutex<HashMap<String, BreakerState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ManagedReason {
    Ttfb,
    Stall,
    Slow,
}

#[derive(Debug)]
enum AttemptError {
    Managed { reason: ManagedReason, offset: u64 },
    Http { error: crate::Error, offset: u64 },
    RangeNotSupported,
}

fn authority_of(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
}

fn authority_semaphore(authority: &str) -> Arc<Semaphore> {
    let permits = if authority == "bmclapi2.bangbang93.com" {
        XMCL_BMCL_CONCURRENCY
    } else {
        XMCL_OTHER_CONCURRENCY
    };
    if let Ok(mut semaphores) = AUTHORITY_SEMAPHORES.lock() {
        semaphores
            .entry(authority.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(permits)))
            .clone()
    } else {
        Arc::new(Semaphore::new(permits))
    }
}

fn is_reassignable(authority: &str) -> bool {
    authority == "bmclapi2.bangbang93.com"
}

#[derive(Clone, Copy)]
enum AttemptOutcome {
    Completed,
    Aborted,
    Failed,
}

fn reputation_path() -> Option<std::path::PathBuf> {
    crate::State::get_if_initialized().map(|state| {
        state
            .directories
            .settings_dir
            .join("download-reputation.json")
    })
}

async fn load_reputation_if_needed() {
    if REPUTATION_LOADED.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(path) = reputation_path() else {
        return;
    };
    let Ok(contents) = tokio::fs::read_to_string(&path).await else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return;
    };
    let mut reputation =
        REPUTATION.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(global) = value.get("global") {
        if let (Some(score), Some(weight), Some(count)) = (
            global.get("score").and_then(serde_json::Value::as_f64),
            global.get("weight").and_then(serde_json::Value::as_f64),
            global.get("count").and_then(serde_json::Value::as_u64),
        ) {
            reputation.global = Ewma {
                score,
                weight,
                count,
            };
        }
    }
    if let Some(hosts) =
        value.get("hosts").and_then(serde_json::Value::as_object)
    {
        for (host, host_value) in hosts {
            if let (Some(score), Some(weight), Some(count)) = (
                host_value.get("score").and_then(serde_json::Value::as_f64),
                host_value.get("weight").and_then(serde_json::Value::as_f64),
                host_value.get("count").and_then(serde_json::Value::as_u64),
            ) {
                reputation.hosts.insert(
                    host.clone(),
                    Ewma {
                        score,
                        weight,
                        count,
                    },
                );
            }
        }
    }
}

fn save_reputation() {
    let Some(path) = reputation_path() else {
        return;
    };
    let reputation =
        REPUTATION.lock().unwrap_or_else(|error| error.into_inner());
    let hosts: serde_json::Map<String, serde_json::Value> = reputation
        .hosts
        .iter()
        .map(|(host, ewma)| {
            (
                host.clone(),
                serde_json::json!({
                    "score": ewma.score,
                    "weight": ewma.weight,
                    "count": ewma.count,
                }),
            )
        })
        .collect();
    let value = serde_json::json!({
        "global": {
            "score": reputation.global.score,
            "weight": reputation.global.weight,
            "count": reputation.global.count,
        },
        "hosts": hosts,
    });
    drop(reputation);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ =
        std::fs::write(path, serde_json::to_string(&value).unwrap_or_default());
}

fn should_skip(origin: &str) -> bool {
    if !is_reassignable(origin) {
        return false;
    }
    let mut breaker = BREAKER.lock().unwrap_or_else(|error| error.into_inner());
    let state = breaker.entry(origin.to_string()).or_default();
    let Some(open_until) = state.open_until else {
        return false;
    };
    if Instant::now() >= open_until {
        state.open_until = None;
        state.fails = 0;
        return false;
    }
    state.probe_tick = state.probe_tick.wrapping_add(1);
    state.probe_tick % CB_PROBE_EVERY != 0
}

fn update_ewma(ewma: &mut Ewma, value: f64) {
    ewma.count = ewma.count.saturating_add(1);
    if ewma.weight == 0.0 {
        ewma.score = value;
        ewma.weight = 1.0;
    } else {
        ewma.score = ewma.score * 0.75 + value * 0.25;
        ewma.weight += 1.0;
    }
}

fn report_attempt(
    origin: &str,
    received: u64,
    duration: Duration,
    outcome: AttemptOutcome,
) {
    let now = Instant::now();
    {
        let mut breaker =
            BREAKER.lock().unwrap_or_else(|error| error.into_inner());
        let state = breaker.entry(origin.to_string()).or_default();
        if is_reassignable(origin) {
            if received >= MIN_MEASURE_BYTES {
                state.fails = 0;
                state.open_until = None;
            } else {
                state.fails = state.fails.saturating_add(1);
                if state.fails >= CB_THRESHOLD {
                    state.open_until = Some(now + CB_COOLDOWN);
                    state.fails = 0;
                }
            }
        } else if state.open_until.is_some()
            && !matches!(outcome, AttemptOutcome::Completed)
            && received < MIN_MEASURE_BYTES
        {
            state.open_until = None;
            state.fails = 0;
        }
    }

    if received < MIN_MEASURE_BYTES || !is_reassignable(origin) {
        return;
    }
    let speed = if duration.as_secs_f64() > 0.0 {
        received as f64 / duration.as_secs_f64()
    } else {
        0.0
    };
    let mut reputation =
        REPUTATION.lock().unwrap_or_else(|error| error.into_inner());
    let host = reputation.hosts.entry(origin.to_string()).or_default();
    update_ewma(host, speed);
    if matches!(outcome, AttemptOutcome::Completed) {
        update_ewma(&mut reputation.global, speed);
    }
    drop(reputation);
    save_reputation();
}

async fn report_stall(rule: SlowRule, source: &str, detail: String) {
    let event = SlowEvent {
        rule,
        engine: "xmcl".to_string(),
        source: source.to_string(),
        detail,
    };
    if let Some(state) = crate::State::get_if_initialized() {
        let path = state.directories.settings_dir.join("download.log");
        let _ = super::log::append_stall(&path, &event);
    }
}

fn http_error_for_response(
    status: reqwest::StatusCode,
    url: &str,
) -> crate::Error {
    crate::ErrorKind::HttpError {
        status: status.as_u16(),
        method: "GET".to_string(),
        url: url.to_string(),
    }
    .into()
}

fn content_range_start(response: &reqwest::Response) -> Option<u64> {
    let value = response
        .headers()
        .get(header::CONTENT_RANGE)?
        .to_str()
        .ok()?;
    value
        .strip_prefix("bytes ")
        .and_then(|value| value.split_once('/'))
        .and_then(|(range, _)| range.split_once('-'))
        .and_then(|(start, _)| start.parse::<u64>().ok())
}

fn is_slow_speed(speed: u64, remaining: Option<u64>) -> bool {
    if speed < XMCL_STALL_FLOOR {
        return true;
    }
    let reputation =
        REPUTATION.lock().unwrap_or_else(|error| error.into_inner());
    if reputation.global.count < XMCL_MIN_GLOBAL_SAMPLES
        || reputation.global.score <= 0.0
    {
        return false;
    }
    let e_fresh = reputation.global.score;
    if let Some(remaining) = remaining {
        if remaining == 0 {
            return false;
        }
        let v_min = remaining as f64
            / (XMCL_RECONNECT_OVERHEAD_SECS + remaining as f64 / e_fresh);
        (speed as f64) < v_min * 0.85
    } else {
        (speed as f64) < e_fresh * 0.4
    }
}

async fn download_attempt(
    request: &fetch::DownloadRequest,
    url: &str,
    header_value: Option<(&str, &str)>,
    part_path: &Path,
    resume_offset: u64,
    request_offset: u64,
    range_end: Option<u64>,
    expected_total: Option<u64>,
    require_range: bool,
    no_abort: bool,
    flow_started_at: Instant,
    progress_bytes: Option<Arc<AtomicU64>>,
    progress: &mut Option<&mut fetch::FetchProgressFn<'_>>,
) -> Result<u64, AttemptError> {
    let Some(authority) = authority_of(url) else {
        return Err(AttemptError::Http {
            error: crate::ErrorKind::NetworkError(format!("invalid url {url}"))
                .into(),
            offset: resume_offset,
        });
    };
    let semaphore = authority_semaphore(&authority);
    let _permit =
        semaphore
            .acquire()
            .await
            .map_err(|error| AttemptError::Http {
                error: crate::ErrorKind::AcquireError(error).into(),
                offset: resume_offset,
            })?;
    let _activity = crate::State::get_if_initialized()
        .map(|state| state.begin_download_connection());

    let client = if range_end.is_some() {
        &XMCL_SEGMENT_CLIENT
    } else {
        &XMCL_SINGLE_CLIENT
    };
    let mut http_request = client.get(url);
    if let Some(end) = range_end {
        http_request = http_request
            .header(header::RANGE, format!("bytes={request_offset}-{end}"));
    } else if request_offset > 0 {
        http_request = http_request
            .header(header::RANGE, format!("bytes={request_offset}-"));
    }
    if let Some((name, value)) = header_value {
        http_request = http_request.header(name, value);
    }

    let response = if is_reassignable(&authority) {
        tokio::time::timeout(XMCL_TTFB_TIMEOUT, http_request.send())
            .await
            .map_err(|_| AttemptError::Managed {
                reason: ManagedReason::Ttfb,
                offset: resume_offset,
            })?
            .map_err(|error| AttemptError::Http {
                error: error.into(),
                offset: resume_offset,
            })?
    } else {
        http_request
            .send()
            .await
            .map_err(|error| AttemptError::Http {
                error: error.into(),
                offset: resume_offset,
            })?
    };

    if require_range {
        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(AttemptError::RangeNotSupported);
        }
        if let Some(start) = content_range_start(&response) {
            if start != request_offset {
                return Err(AttemptError::RangeNotSupported);
            }
        }
    } else if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        if let Some(start) = content_range_start(&response) {
            if start != request_offset {
                return Err(AttemptError::RangeNotSupported);
            }
        }
    } else if response.status() != reqwest::StatusCode::OK {
        return Err(AttemptError::Http {
            error: http_error_for_response(response.status(), url),
            offset: resume_offset,
        });
    }

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(part_path)
        .await
        .map_err(|error| AttemptError::Http {
            error: crate::ErrorKind::StdIOError(error).into(),
            offset: resume_offset,
        })?;
    let mut effective_offset = resume_offset;
    if request_offset > 0 && response.status() == reqwest::StatusCode::OK {
        file.set_len(0).await.map_err(|error| AttemptError::Http {
            error: crate::ErrorKind::StdIOError(error).into(),
            offset: resume_offset,
        })?;
        effective_offset = 0;
    }
    if effective_offset > 0 {
        file.seek(std::io::SeekFrom::Start(effective_offset))
            .await
            .map_err(|error| AttemptError::Http {
                error: crate::ErrorKind::StdIOError(error).into(),
                offset: effective_offset,
            })?;
    }

    let mut stream = response.bytes_stream();
    let started = Instant::now();
    let mut offset = effective_offset;
    let mut first_byte_at = None;
    let mut last_byte_at = Instant::now();
    let mut last_progress_update: Option<Instant> = None;
    let mut window_start = Instant::now();
    let mut window_bytes: u64 = 0;
    let mut slow_streak: u32 = 0;

    loop {
        let next = tokio::time::timeout(XMCL_STALL_TIMEOUT, stream.next())
            .await
            .map_err(|_| AttemptError::Managed {
                reason: ManagedReason::Stall,
                offset,
            })?;
        let Some(chunk) = next else {
            break;
        };
        let chunk = chunk.map_err(|error| AttemptError::Http {
            error: error.into(),
            offset,
        })?;
        if chunk.is_empty() {
            continue;
        }
        if first_byte_at.is_none() {
            first_byte_at = Some(Instant::now());
            window_start = Instant::now();
        }
        last_byte_at = Instant::now();
        file.write_all(chunk.as_ref()).await.map_err(|error| {
            AttemptError::Http {
                error: crate::ErrorKind::StdIOError(error).into(),
                offset,
            }
        })?;
        offset = offset.saturating_add(chunk.len() as u64);
        if let Some(state) = crate::State::get_if_initialized() {
            state.record_download_bytes(chunk.len() as u64);
        }
        if let Some(progress_bytes) = &progress_bytes {
            progress_bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        } else if let Some(progress) = progress.as_mut() {
            let total = expected_total.unwrap_or(offset);
            let should_update = last_progress_update.map_or(true, |last| {
                last.elapsed() >= Duration::from_millis(100)
            }) || offset >= total;
            if should_update {
                let _ = progress(offset, total).await;
                fetch::record_install_download_progress(request, offset, total)
                    .await;
                last_progress_update = Some(Instant::now());
            }
        }

        if first_byte_at.is_some() {
            if !no_abort {
                window_bytes = window_bytes.saturating_add(chunk.len() as u64);
                if window_start.elapsed() >= XMCL_SLOW_WINDOW {
                    let window_ms =
                        window_start.elapsed().as_millis().max(1) as u64;
                    let speed = window_bytes.saturating_mul(1000) / window_ms;
                    window_bytes = 0;
                    window_start = Instant::now();
                    let remaining = expected_total
                        .map(|total| total.saturating_sub(offset));
                    if is_slow_speed(speed, remaining) {
                        slow_streak += 1;
                    } else {
                        slow_streak = 0;
                    }
                    if slow_streak >= XMCL_SLOW_CONSECUTIVE
                        && flow_started_at.elapsed() >= XMCL_SLOW_MIN_FLOW_SECS
                    {
                        return Err(AttemptError::Managed {
                            reason: ManagedReason::Slow,
                            offset,
                        });
                    }
                }
            }
        }
    }

    file.flush().await.map_err(|error| AttemptError::Http {
        error: crate::ErrorKind::StdIOError(error).into(),
        offset,
    })?;
    if first_byte_at.is_some() {
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        let speed = (offset.saturating_sub(effective_offset)) as f64 / elapsed;
        GLOBAL_SPEED_BPS.store(speed as u64, Ordering::Relaxed);
    }
    let _ = last_byte_at;
    Ok(offset)
}

async fn run_single_stream(
    request: &fetch::DownloadRequest,
    routes: &[fetch::DownloadRoute],
    part_path: &Path,
    initial_offset: u64,
    expected_total: Option<u64>,
    flow_started_at: Instant,
    progress: &mut Option<&mut fetch::FetchProgressFn<'_>>,
) -> crate::Result<u64> {
    let mut offset = initial_offset;
    let mut resumes = 0;
    let mut no_progress = 0;
    let mut committed = false;
    let mut last_error = None;
    let mut url_index = 0;
    let header_value = request
        .header
        .as_ref()
        .map(|(name, value)| (name.as_str(), value.as_str()));

    while url_index < routes.len() {
        let route = &routes[url_index];
        let url = route.url.as_str();
        let origin = authority_of(url).unwrap_or_default();
        if url_index + 1 < routes.len() && should_skip(&origin) {
            url_index += 1;
            continue;
        }
        let offset_before = offset;
        let attempt_started = Instant::now();
        let result = download_attempt(
            request,
            url,
            header_value,
            part_path,
            offset,
            offset,
            None,
            expected_total,
            false,
            committed,
            flow_started_at,
            None,
            progress,
        )
        .await;
        let received = result
            .as_ref()
            .map(|new_offset| new_offset.saturating_sub(offset_before))
            .unwrap_or(0);
        let outcome = match &result {
            Ok(_) => AttemptOutcome::Completed,
            Err(AttemptError::Managed { .. }) => AttemptOutcome::Aborted,
            Err(_) => AttemptOutcome::Failed,
        };
        report_attempt(&origin, received, attempt_started.elapsed(), outcome);

        match result {
            Ok(new_offset) => {
                if expected_total.map_or(true, |total| new_offset >= total) {
                    return Ok(new_offset);
                }
                offset = new_offset;
                if resumes < XMCL_MAX_RESUMES {
                    resumes += 1;
                    continue;
                }
                committed = false;
                url_index += 1;
            }
            Err(AttemptError::Managed {
                reason,
                offset: error_offset,
            }) => {
                offset = offset.max(error_offset);
                match reason {
                    ManagedReason::Ttfb | ManagedReason::Stall => {
                        report_stall(
                            SlowRule::R1NoProgress,
                            route.source.as_str(),
                            format!("{reason:?} no_progress"),
                        )
                        .await;
                        if no_progress < XMCL_MAX_NO_PROGRESS {
                            no_progress += 1;
                            continue;
                        }
                        no_progress = 0;
                        committed = false;
                        url_index += 1;
                    }
                    ManagedReason::Slow => {
                        report_stall(
                            SlowRule::R2BelowExpectation,
                            route.source.as_str(),
                            "speed_below_floor".to_string(),
                        )
                        .await;
                        if !committed && resumes < XMCL_MAX_RESUMES {
                            resumes += 1;
                            continue;
                        }
                        committed = true;
                    }
                }
            }
            Err(AttemptError::Http {
                error,
                offset: error_offset,
            }) => {
                offset = offset.max(error_offset);
                last_error = Some(error);
                let should_reroll = !is_terminal_http(&last_error);
                if should_reroll && resumes < XMCL_MAX_RESUMES {
                    resumes += 1;
                    continue;
                }
                committed = false;
                url_index += 1;
            }
            Err(AttemptError::RangeNotSupported) => {
                return Err(crate::ErrorKind::NetworkError(
                    "server ignored Range header".to_string(),
                )
                .into());
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        crate::ErrorKind::NetworkError(
            "download failed with no attempts".to_string(),
        )
        .into()
    }))
}

#[derive(Debug)]
enum SegmentError {
    RangeNotSupported,
    Other(crate::Error),
}

fn segment_path(part_path: &Path, index: usize) -> std::path::PathBuf {
    let mut name = part_path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(format!(".seg{index}"));
    part_path.with_file_name(name)
}

async fn run_segment(
    request: &fetch::DownloadRequest,
    routes: &[fetch::DownloadRoute],
    segment_path: &Path,
    start: u64,
    end: u64,
    flow_started_at: Instant,
    progress_bytes: Option<Arc<AtomicU64>>,
) -> Result<u64, SegmentError> {
    let segment_total = end.saturating_sub(start).saturating_add(1);
    let mut relative_offset = 0u64;
    let mut resumes = 0;
    let mut no_progress = 0;
    let mut committed = false;
    let mut last_error = None;
    let mut url_index = 0;
    let header_value = request
        .header
        .as_ref()
        .map(|(name, value)| (name.as_str(), value.as_str()));

    while url_index < routes.len() {
        let route = &routes[url_index];
        let url = route.url.as_str();
        let origin = authority_of(url).unwrap_or_default();
        if url_index + 1 < routes.len() && should_skip(&origin) {
            url_index += 1;
            continue;
        }
        let offset_before = relative_offset;
        let attempt_started = Instant::now();
        let result = download_attempt(
            request,
            url,
            header_value,
            segment_path,
            relative_offset,
            start.saturating_add(relative_offset),
            Some(end),
            Some(segment_total),
            true,
            committed,
            flow_started_at,
            progress_bytes.clone(),
            &mut None,
        )
        .await;
        let received = result
            .as_ref()
            .map(|new_offset| new_offset.saturating_sub(offset_before))
            .unwrap_or(0);
        let outcome = match &result {
            Ok(_) => AttemptOutcome::Completed,
            Err(AttemptError::Managed { .. }) => AttemptOutcome::Aborted,
            Err(_) => AttemptOutcome::Failed,
        };
        report_attempt(&origin, received, attempt_started.elapsed(), outcome);

        match result {
            Ok(new_offset) => {
                if new_offset >= segment_total {
                    return Ok(new_offset);
                }
                relative_offset = new_offset;
                if resumes < XMCL_MAX_RESUMES {
                    resumes += 1;
                    continue;
                }
                committed = false;
                url_index += 1;
            }
            Err(AttemptError::Managed {
                reason,
                offset: error_offset,
            }) => {
                relative_offset = relative_offset.max(error_offset);
                match reason {
                    ManagedReason::Ttfb | ManagedReason::Stall => {
                        report_stall(
                            SlowRule::R1NoProgress,
                            route.source.as_str(),
                            format!("{reason:?} no_progress"),
                        )
                        .await;
                        if no_progress < XMCL_MAX_NO_PROGRESS {
                            no_progress += 1;
                            continue;
                        }
                        no_progress = 0;
                        committed = false;
                        url_index += 1;
                    }
                    ManagedReason::Slow => {
                        report_stall(
                            SlowRule::R2BelowExpectation,
                            route.source.as_str(),
                            "speed_below_floor".to_string(),
                        )
                        .await;
                        if !committed && resumes < XMCL_MAX_RESUMES {
                            resumes += 1;
                            continue;
                        }
                        committed = true;
                    }
                }
            }
            Err(AttemptError::Http {
                error,
                offset: error_offset,
            }) => {
                relative_offset = relative_offset.max(error_offset);
                last_error = Some(error);
                let should_reroll = !is_terminal_http(&last_error);
                if should_reroll && resumes < XMCL_MAX_RESUMES {
                    resumes += 1;
                    continue;
                }
                committed = false;
                url_index += 1;
            }
            Err(AttemptError::RangeNotSupported) => {
                return Err(SegmentError::RangeNotSupported);
            }
        }
    }

    Err(SegmentError::Other(last_error.unwrap_or_else(|| {
        crate::ErrorKind::NetworkError(
            "download failed with no attempts".to_string(),
        )
        .into()
    })))
}

fn is_terminal_http(error: &Option<crate::Error>) -> bool {
    let Some(error) = error else {
        return false;
    };
    match error.raw.as_ref() {
        crate::ErrorKind::HttpError { status, .. } => {
            *status == 404 || *status == 410
        }
        _ => false,
    }
}

async fn download_segments(
    request: &fetch::DownloadRequest,
    routes: &[fetch::DownloadRoute],
    part_path: &Path,
    total: u64,
    flow_started_at: Instant,
    progress: &mut Option<&mut fetch::FetchProgressFn<'_>>,
) -> Result<(), SegmentError> {
    let chunk_size = total.div_ceil(XMCL_RANGE_CONCURRENCY as u64);
    let mut segment_paths = Vec::new();
    for index in 0..XMCL_RANGE_CONCURRENCY {
        let start = (index as u64).saturating_mul(chunk_size);
        if start >= total {
            break;
        }
        segment_paths.push(segment_path(part_path, index));
    }
    let downloaded = Arc::new(AtomicU64::new(0));
    let mut tasks = Vec::new();
    for (index, segment_path) in segment_paths.iter().enumerate() {
        let start = (index as u64).saturating_mul(chunk_size);
        let end = start
            .saturating_add(chunk_size)
            .min(total)
            .saturating_sub(1);
        tasks.push(run_segment(
            request,
            routes,
            segment_path,
            start,
            end,
            flow_started_at,
            Some(Arc::clone(&downloaded)),
        ));
    }
    let segments = async {
        for result in futures::future::join_all(tasks).await {
            result?;
        }
        Ok::<(), SegmentError>(())
    };
    let reporter = async {
        loop {
            let bytes = downloaded.load(Ordering::Relaxed);
            if let Some(progress) = progress.as_mut() {
                let _ = progress(bytes, total).await;
            }
            fetch::record_install_download_progress(request, bytes, total)
                .await;
            if bytes >= total {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    let segment_result =
        match futures::future::select(Box::pin(segments), Box::pin(reporter))
            .await
        {
            futures::future::Either::Left((result, _reporter)) => result,
            futures::future::Either::Right(((), segments)) => segments.await,
        };
    segment_result?;
    concatenate_segments(part_path, total)
        .await
        .map_err(SegmentError::Other)
}

async fn concatenate_segments(
    part_path: &Path,
    total: u64,
) -> crate::Result<()> {
    let mut output = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(part_path)
        .await?;
    let mut buffer = vec![0u8; 64 * 1024];
    for index in 0..XMCL_RANGE_CONCURRENCY {
        let segment_path = segment_path(part_path, index);
        let Ok(mut input) = tokio::fs::File::open(&segment_path).await else {
            continue;
        };
        loop {
            let read = input.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read]).await?;
        }
        tokio::fs::remove_file(&segment_path).await?;
    }
    output.flush().await?;
    let actual = tokio::fs::metadata(part_path).await?.len();
    if actual != total {
        return Err(crate::ErrorKind::NetworkError(format!(
            "segmented download produced {actual} bytes, expected {total}"
        ))
        .into());
    }
    Ok(())
}

async fn cleanup_segment_files(part_path: &Path) {
    for index in 0..XMCL_RANGE_CONCURRENCY {
        let _ = tokio::fs::remove_file(segment_path(part_path, index)).await;
    }
    let _ = tokio::fs::remove_file(part_path).await;
}

async fn download_to_path_inner(
    request: &fetch::DownloadRequest,
    destination: &Path,
    routes: &[fetch::DownloadRoute],
    _semaphore: &fetch::FetchSemaphore,
    part_path: &Path,
    progress: Option<&mut fetch::FetchProgressFn<'_>>,
) -> crate::Result<fetch::DownloadResult> {
    load_reputation_if_needed().await;
    let expected_total = request.integrity.size;
    let initial_offset = tokio::fs::metadata(part_path)
        .await
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let flow_started_at = Instant::now();
    let mut progress = progress;

    if let Some(total) = expected_total {
        if total >= SEGMENTED_DOWNLOAD_THRESHOLD && initial_offset == 0 {
            tracing::debug!(total, "Using segmented XMCL download");
            match download_segments(
                request,
                routes,
                part_path,
                total,
                flow_started_at,
                &mut progress,
            )
            .await
            {
                Ok(()) => {
                    fetch::record_install_download_stage(
                        request,
                        crate::install::DownloadItemStatus::Verifying,
                    )
                    .await;
                    let size =
                        fetch::verify_file(part_path, &request.integrity)
                            .await?;
                    fetch::record_install_download_stage(
                        request,
                        crate::install::DownloadItemStatus::Writing,
                    )
                    .await;
                    fetch::finalize_download(part_path, destination).await?;
                    fetch::record_install_download_finished(request, size)
                        .await;
                    let route = routes.first().cloned().unwrap_or_else(|| {
                        fetch::DownloadRoute {
                            url: request.url.clone(),
                            source: fetch::DownloadRouteSource::Official,
                            is_mirror: false,
                            allow_sensitive_headers: false,
                            supports_range: true,
                            proxy: fetch::ProxyPolicy::System,
                        }
                    });
                    return Ok(fetch::DownloadResult {
                        path: destination.to_path_buf(),
                        url: route.url,
                        source: route.source,
                        size,
                        attempts: 0,
                        fallback_count: 0,
                    });
                }
                Err(SegmentError::RangeNotSupported) => {
                    tracing::debug!(
                        "Range not supported, falling back to single-stream XMCL download"
                    );
                    cleanup_segment_files(part_path).await;
                }
                Err(SegmentError::Other(error)) => return Err(error),
            }
        }
    }

    tracing::debug!(?expected_total, "Using single-stream XMCL download");
    run_single_stream(
        request,
        routes,
        part_path,
        initial_offset,
        expected_total,
        flow_started_at,
        &mut progress,
    )
    .await?;

    fetch::record_install_download_stage(
        request,
        crate::install::DownloadItemStatus::Verifying,
    )
    .await;
    let size = fetch::verify_file(part_path, &request.integrity).await?;
    fetch::record_install_download_stage(
        request,
        crate::install::DownloadItemStatus::Writing,
    )
    .await;
    fetch::finalize_download(part_path, destination).await?;
    fetch::record_install_download_finished(request, size).await;
    let route =
        routes
            .first()
            .cloned()
            .unwrap_or_else(|| fetch::DownloadRoute {
                url: request.url.clone(),
                source: fetch::DownloadRouteSource::Official,
                is_mirror: false,
                allow_sensitive_headers: false,
                supports_range: true,
                proxy: fetch::ProxyPolicy::System,
            });
    Ok(fetch::DownloadResult {
        path: destination.to_path_buf(),
        url: route.url,
        source: route.source,
        size,
        attempts: 0,
        fallback_count: 0,
    })
}

#[inline(never)]
pub(crate) async fn download_to_path(
    request: &fetch::DownloadRequest,
    destination: &Path,
    routes: &[fetch::DownloadRoute],
    semaphore: &fetch::FetchSemaphore,
    part_path: &Path,
    progress: Option<&mut fetch::FetchProgressFn<'_>>,
) -> crate::Result<fetch::DownloadResult> {
    let result = download_to_path_inner(
        request,
        destination,
        routes,
        semaphore,
        part_path,
        progress,
    )
    .await;
    if result.is_err() {
        if let Some(state) = crate::State::get_if_initialized() {
            state.record_download_error();
        }
        cleanup_segment_files(part_path).await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_path_appends_index() {
        let path = segment_path(Path::new("C:\\tmp\\file.part"), 2);
        assert!(path.to_string_lossy().ends_with("file.part.seg2"));
    }

    #[test]
    fn authority_of_extracts_host() {
        assert_eq!(
            authority_of("https://bmclapi2.bangbang93.com/foo").as_deref(),
            Some("bmclapi2.bangbang93.com")
        );
        assert_eq!(authority_of("not a url"), None);
    }

    #[test]
    fn non_reassignable_origin_is_never_skipped() {
        assert!(!should_skip("official.example.com"));
    }

    #[test]
    fn ewma_updates_score() {
        let mut ewma = Ewma::default();
        update_ewma(&mut ewma, 100.0);
        assert_eq!(ewma.count, 1);
        assert_eq!(ewma.score, 100.0);
        update_ewma(&mut ewma, 0.0);
        assert!(ewma.score < 100.0);
        assert_eq!(ewma.count, 2);
    }
}
