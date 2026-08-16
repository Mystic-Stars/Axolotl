use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use uuid::Uuid;

use crate::State;
use crate::api::logs::CensoredString;
use crate::prelude::Credentials;
use crate::state::DirectoryInfo;

const ENDPOINT: &str = "https://telemetry.axlmc.org/v1/batch";
const MAX_OUTBOX_EVENTS: i64 = 100;
const MAX_OUTBOX_BYTES: i64 = 2 * 1024 * 1024;
const MAX_EVENT_AGE_SECONDS: i64 = 7 * 24 * 60 * 60;
const MAX_ERROR_CONTEXT_BYTES: usize = 16 * 1024;
const MAX_DISTINCT_ERRORS_PER_DAY: i64 = 20;
const MAX_BATCH_EVENTS: i64 = 10;
const MAX_BATCH_BYTES: usize = 60 * 1024;
const PANIC_MARKER_FILE: &str = "telemetry-panic-marker.json";

static STARTED: AtomicBool = AtomicBool::new(false);
static WAKE_TX: OnceLock<tokio::sync::mpsc::Sender<()>> = OnceLock::new();
static PANIC_MARKER_PATH: OnceLock<PathBuf> = OnceLock::new();
static LOG_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static LOG_RING: LazyLock<Mutex<VecDeque<LogLine>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(256)));
static PENDING_RUST_ERRORS: LazyLock<Mutex<VecDeque<PendingRustError>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(32)));

#[derive(Default, Clone, Copy)]
struct DownloadAggregates {
    files: u64,
    bytes: u64,
    failed: u64,
    stalls: u64,
    wasted: u64,
    switches: u64,
}

static DOWNLOAD_AGGREGATES: LazyLock<Mutex<HashMap<u8, DownloadAggregates>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static BEARER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbearer\s+[a-z0-9._~+/=-]+").expect("valid regex")
});
static SECRET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(authorization|x-api-key|api[_-]?key|access[_-]?token|refresh[_-]?token|client[_-]?secret|token)\b\s*[:=]\s*[^\s,;]+",
	)
	.expect("valid regex")
});
static EMAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9.-]+\.[a-z]{2,}\b")
        .expect("valid regex")
});
static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b[0-9a-f]{8}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{12}\b",
    )
    .expect("valid regex")
});
static NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d+\b").expect("valid regex"));
static WINDOWS_HOME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[a-z]:\\users\\[^\\/\s]+").expect("valid regex")
});
static UNIX_HOME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:/home|/users)/[^/\s]+/").expect("valid regex")
});
static SENSITIVE_QUERY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"(?i)([?&](?:token|access_token|refresh_token|api_key|key|code|secret|session|signature)=)[^&#\s]+",
	)
	.expect("valid regex")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendErrorReport {
    #[serde(default = "default_error_type")]
    pub error_type: String,
    pub message: String,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Clone)]
struct LogLine {
    sequence: u64,
    line: String,
}

#[derive(Clone)]
struct PendingRustError {
    sequence: u64,
    message: String,
    target: String,
}

#[derive(Default)]
struct EventVisitor {
    fields: String,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if !self.fields.is_empty() {
            self.fields.push(' ');
        }
        let _ = write!(self.fields, "{}={value:?}", field.name());
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TelemetryErrorLayer;

impl<S> Layer<S> for TelemetryErrorLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        if metadata.target().contains("telemetry") {
            return;
        }

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        let sequence = LOG_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let line = format!(
            "{} {} {}",
            metadata.level(),
            metadata.target(),
            visitor.fields
        );

        if let Ok(mut ring) = LOG_RING.lock() {
            ring.push_back(LogLine {
                sequence,
                line: line.clone(),
            });
            while ring.len() > 256 {
                ring.pop_front();
            }
        }

        if *metadata.level() == tracing::Level::ERROR
            && let Ok(mut pending) = PENDING_RUST_ERRORS.lock()
        {
            pending.push_back(PendingRustError {
                sequence,
                message: visitor.fields,
                target: metadata.target().to_string(),
            });
            while pending.len() > 32 {
                pending.pop_front();
            }
        }
    }
}

pub(crate) fn error_layer() -> TelemetryErrorLayer {
    TelemetryErrorLayer
}

pub fn install_panic_hook(app_identifier: &str) {
    let Some(settings_dir) =
        DirectoryInfo::initial_settings_dir_path(app_identifier)
    else {
        return;
    };
    let marker_path = settings_dir.join(PANIC_MARKER_FILE);
    if PANIC_MARKER_PATH.set(marker_path).is_err() {
        return;
    }

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        if let Some(path) = PANIC_MARKER_PATH.get() {
            let payload = if let Some(message) =
                panic_info.payload().downcast_ref::<&str>()
            {
                (*message).to_string()
            } else if let Some(message) =
                panic_info.payload().downcast_ref::<String>()
            {
                message.clone()
            } else {
                "Rust panic".to_string()
            };
            let location = panic_info.location().map(|location| {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            });
            let marker = json!({
                "error_type": "rust_panic",
                "message": payload,
                "stack": location,
                "occurred_at": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            });
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, marker.to_string());
        }
        previous(panic_info);
    }));
}

pub(crate) fn start(state: Arc<State>) {
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }

    let (wake_tx, mut wake_rx) = tokio::sync::mpsc::channel(1);
    let _ = WAKE_TX.set(wake_tx);
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(crate::launcher_user_agent())
            .build()
            .unwrap_or_default();

        loop {
            if let Err(error) = run_cycle(&state, &client).await {
                tracing::debug!(target: "theseus::telemetry", %error, "Telemetry cycle failed");
            }

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(60)) => {},
                _ = wake_rx.recv() => {},
            }
        }
    });
}

pub async fn set_enabled(state: &State, enabled: bool) -> crate::Result<()> {
    clear_runtime_buffers();
    sqlx::query("DELETE FROM telemetry_outbox")
        .execute(&state.pool)
        .await?;

    if enabled {
        ensure_identity(&state.pool).await?;
        recover_panic_marker(state).await?;
        enqueue_heartbeat(state).await?;
        wake();
    }
    Ok(())
}

pub async fn submit_frontend_error(
    report: FrontendErrorReport,
) -> crate::Result<()> {
    let state = State::get().await?;
    queue_error(&state, report).await?;
    wake();
    Ok(())
}

pub async fn submit_download_stall(
    engine: &str,
    rule: u8,
    source: &str,
    detail: &str,
    context: &str,
) -> crate::Result<()> {
    let state = State::get().await?;
    let message = format!(
        "download_stall engine={engine} rule={rule} src={source} {detail}"
    );
    queue_error(
        &state,
        FrontendErrorReport {
            error_type: "download_stall".to_string(),
            message,
            stack: None,
            route: Some(source.to_string()),
            command: Some("download".to_string()),
            context: Some(context.to_string()),
        },
    )
    .await?;
    wake();
    Ok(())
}

pub async fn submit_download_error(
    engine: &str,
    category: &str,
    message: &str,
    route: Option<&str>,
    command: Option<&str>,
    context: Option<&str>,
) -> crate::Result<()> {
    let state = State::get().await?;
    let message =
        format!("download_error engine={engine} category={category} {message}");
    queue_error(
        &state,
        FrontendErrorReport {
            error_type: "download_error".to_string(),
            message,
            stack: None,
            route: route.map(str::to_string),
            command: command.map(str::to_string),
            context: context.map(str::to_string),
        },
    )
    .await?;
    wake();
    Ok(())
}

pub fn record_download_stats(
    engine: u8,
    files: u64,
    bytes: u64,
    failed: u64,
    stalls: u64,
    wasted: u64,
    switches: u64,
) {
    if let Ok(mut aggregates) = DOWNLOAD_AGGREGATES.lock() {
        let entry = aggregates.entry(engine).or_default();
        entry.files = entry.files.saturating_add(files);
        entry.bytes = entry.bytes.saturating_add(bytes);
        entry.failed = entry.failed.saturating_add(failed);
        entry.stalls = entry.stalls.saturating_add(stalls);
        entry.wasted = entry.wasted.saturating_add(wasted);
        entry.switches = entry.switches.saturating_add(switches);
    }
}

pub fn notify_online() {
    wake();
}

async fn run_cycle(
    state: &State,
    client: &reqwest::Client,
) -> crate::Result<()> {
    if !is_enabled(state).await? {
        clear_runtime_buffers();
        sqlx::query("DELETE FROM telemetry_outbox")
            .execute(&state.pool)
            .await?;
        return Ok(());
    }

    ensure_identity(&state.pool).await?;
    recover_panic_marker(state).await?;
    drain_rust_errors(state).await;
    enqueue_heartbeat(state).await?;
    cleanup_outbox(state).await?;
    upload_next_batch(state, client).await?;
    Ok(())
}

async fn is_enabled(state: &State) -> crate::Result<bool> {
    let row = sqlx::query(
		"SELECT telemetry, telemetry_consent_version FROM settings WHERE id = 0",
	)
	.fetch_one(&state.pool)
	.await?;
    Ok(row.get::<i64, _>("telemetry") == 1
        && row.get::<i64, _>("telemetry_consent_version") > 0)
}

async fn ensure_identity(pool: &sqlx::SqlitePool) -> crate::Result<String> {
    if let Some(row) = sqlx::query(
        "SELECT installation_id FROM telemetry_identity WHERE id = 0",
    )
    .fetch_optional(pool)
    .await?
    {
        return Ok(row.get("installation_id"));
    }

    let installation_id = Uuid::new_v4().to_string();
    sqlx::query(
		"INSERT OR IGNORE INTO telemetry_identity (id, installation_id) VALUES (0, ?)",
	)
	.bind(&installation_id)
	.execute(pool)
	.await?;
    let row = sqlx::query(
        "SELECT installation_id FROM telemetry_identity WHERE id = 0",
    )
    .fetch_one(pool)
    .await?;
    Ok(row.get("installation_id"))
}

async fn enqueue_heartbeat(state: &State) -> crate::Result<()> {
    let day = Utc::now().format("%Y-%m-%d").to_string();
    let row = sqlx::query(
        "SELECT last_heartbeat_day FROM telemetry_identity WHERE id = 0",
    )
    .fetch_one(&state.pool)
    .await?;
    if row
        .get::<Option<String>, _>("last_heartbeat_day")
        .as_deref()
        == Some(&day)
    {
        return Ok(());
    }

    let event_id = Uuid::new_v4().to_string();
    let download_stats = {
        let mut stats = serde_json::Map::new();
        if let Ok(mut aggregates) = DOWNLOAD_AGGREGATES.lock() {
            for (engine, aggregate) in aggregates.iter() {
                stats.insert(
                    engine.to_string(),
                    json!({
                        "files": aggregate.files,
                        "bytes": aggregate.bytes,
                        "failed": aggregate.failed,
                        "stalls": aggregate.stalls,
                        "wasted": aggregate.wasted,
                        "switches": aggregate.switches,
                    }),
                );
            }
            aggregates.clear();
        }
        Value::Object(stats)
    };
    let payload = json!({
        "type": "heartbeat",
        "event_id": event_id,
        "occurred_at": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        "day": day,
        "download_stats": download_stats,
    });
    insert_outbox_event(
        state,
        &event_id,
        "heartbeat",
        &format!("heartbeat:{day}"),
        &payload,
    )
    .await?;
    sqlx::query(
        "UPDATE telemetry_identity SET last_heartbeat_day = ? WHERE id = 0",
    )
    .bind(day)
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn queue_error(
    state: &State,
    report: FrontendErrorReport,
) -> crate::Result<()> {
    if !is_enabled(state).await? {
        return Ok(());
    }

    let credentials = Credentials::get_all_without_refresh(&state.pool)
        .await?
        .into_iter()
        .map(|entry| entry.1)
        .collect::<Vec<_>>();
    let error_type = truncate_utf8(
        &sanitize_with_credentials(&report.error_type, &credentials),
        128,
    );
    let message = truncate_utf8(
        &sanitize_with_credentials(&report.message, &credentials),
        1024,
    );
    let stack = report
        .stack
        .as_deref()
        .map(|value| sanitize_with_credentials(value, &credentials))
        .map(|value| truncate_utf8(&value, 8192));
    let route = report
        .route
        .as_deref()
        .map(|value| sanitize_with_credentials(value, &credentials))
        .map(|value| truncate_utf8(&value, 256));
    let command = report
        .command
        .as_deref()
        .map(|value| sanitize_with_credentials(value, &credentials))
        .map(|value| truncate_utf8(&value, 256));
    let context = report
        .context
        .as_deref()
        .map(|value| sanitize_with_credentials(value, &credentials))
        .map(|value| truncate_utf8(&value, MAX_ERROR_CONTEXT_BYTES));
    let fingerprint = fingerprint(&error_type, &message, stack.as_deref());
    let day = Utc::now().format("%Y-%m-%d").to_string();

    let known = sqlx::query(
        "SELECT 1 FROM telemetry_error_daily WHERE day = ? AND fingerprint = ?",
    )
    .bind(&day)
    .bind(&fingerprint)
    .fetch_optional(&state.pool)
    .await?
    .is_some();
    if !known {
        let count = sqlx::query(
            "SELECT COUNT(*) AS count FROM telemetry_error_daily WHERE day = ?",
        )
        .bind(&day)
        .fetch_one(&state.pool)
        .await?
        .get::<i64, _>("count");
        if count >= MAX_DISTINCT_ERRORS_PER_DAY {
            return Ok(());
        }
        sqlx::query(
			"INSERT OR IGNORE INTO telemetry_error_daily (day, fingerprint) VALUES (?, ?)",
		)
		.bind(&day)
		.bind(&fingerprint)
		.execute(&state.pool)
		.await?;
    }

    let event_id = Uuid::new_v4().to_string();
    let payload = json!({
        "type": "error",
        "event_id": event_id,
        "occurred_at": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        "fingerprint": fingerprint,
        "occurrence_count": 1,
        "error_type": error_type,
        "message": message,
        "stack": stack,
        "route": route,
        "command": command,
        "context": context,
    });
    let bucket = Utc::now().timestamp() / 600;
    insert_outbox_event(
        state,
        &event_id,
        "error",
        &format!("error:{fingerprint}:{bucket}"),
        &payload,
    )
    .await
}

async fn insert_outbox_event(
    state: &State,
    event_id: &str,
    event_type: &str,
    dedupe_key: &str,
    payload: &Value,
) -> crate::Result<()> {
    let payload = serde_json::to_string(payload)?;
    let now = Utc::now().timestamp();
    let result = sqlx::query(
        r#"
		INSERT INTO telemetry_outbox (
			event_id, event_type, payload, created_at, next_attempt_at,
			size_bytes, dedupe_key
		) VALUES (?, ?, jsonb(?), ?, ?, ?, ?)
		ON CONFLICT(dedupe_key) DO UPDATE SET
			occurrence_count = telemetry_outbox.occurrence_count + 1
		WHERE telemetry_outbox.event_type = 'error'
			AND telemetry_outbox.attempts = 0
		"#,
    )
    .bind(event_id)
    .bind(event_type)
    .bind(&payload)
    .bind(now)
    .bind(now)
    .bind(payload.len() as i64)
    .bind(dedupe_key)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 && event_type == "error" {
        sqlx::query(
            r#"
			INSERT INTO telemetry_outbox (
				event_id, event_type, payload, created_at, next_attempt_at,
				size_bytes, dedupe_key
			) VALUES (?, ?, jsonb(?), ?, ?, ?, ?)
			"#,
        )
        .bind(event_id)
        .bind(event_type)
        .bind(&payload)
        .bind(now)
        .bind(now)
        .bind(payload.len() as i64)
        .bind(format!("{dedupe_key}:{event_id}"))
        .execute(&state.pool)
        .await?;
    }
    cleanup_outbox(state).await
}

async fn cleanup_outbox(state: &State) -> crate::Result<()> {
    let oldest = Utc::now().timestamp() - MAX_EVENT_AGE_SECONDS;
    sqlx::query("DELETE FROM telemetry_outbox WHERE created_at < ?")
        .bind(oldest)
        .execute(&state.pool)
        .await?;
    sqlx::query(
		"DELETE FROM telemetry_outbox WHERE event_id IN (SELECT event_id FROM telemetry_outbox ORDER BY created_at DESC LIMIT -1 OFFSET ?)",
	)
	.bind(MAX_OUTBOX_EVENTS)
	.execute(&state.pool)
	.await?;
    sqlx::query(
		r#"
		DELETE FROM telemetry_outbox
		WHERE event_id IN (
			SELECT event_id FROM (
				SELECT event_id,
					SUM(size_bytes) OVER (ORDER BY created_at DESC, event_id DESC) AS running_bytes
				FROM telemetry_outbox
			)
			WHERE running_bytes > ?
		)
		"#,
	)
	.bind(MAX_OUTBOX_BYTES)
	.execute(&state.pool)
	.await?;
    sqlx::query(
        "DELETE FROM telemetry_error_daily WHERE day < date('now', '-7 days')",
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn upload_next_batch(
    state: &State,
    client: &reqwest::Client,
) -> crate::Result<()> {
    let now = Utc::now().timestamp();
    let rows = sqlx::query(
		"SELECT event_id, json(payload) AS payload, occurrence_count FROM telemetry_outbox WHERE next_attempt_at <= ? ORDER BY created_at LIMIT ?",
	)
	.bind(now)
	.bind(MAX_BATCH_EVENTS)
	.fetch_all(&state.pool)
	.await?;
    if rows.is_empty() {
        return Ok(());
    }

    let mut events = Vec::new();
    let mut event_ids = Vec::new();
    let mut approximate_size = 0;
    for row in rows {
        let payload: String = row.get("payload");
        if approximate_size + payload.len() > MAX_BATCH_BYTES
            && !events.is_empty()
        {
            break;
        }
        let mut event: Value = serde_json::from_str(&payload)?;
        if event.get("type").and_then(Value::as_str) == Some("error") {
            event["occurrence_count"] =
                json!(row.get::<i64, _>("occurrence_count"));
        }
        approximate_size += payload.len();
        events.push(event);
        event_ids.push(row.get::<String, _>("event_id"));
    }

    let installation_id = ensure_identity(&state.pool).await?;
    let batch_id = stable_batch_id(&event_ids);
    let body = json!({
        "schema_version": 1,
        "batch_id": batch_id,
        "installation_id": installation_id,
        "app": {
            "version": env!("CARGO_PKG_VERSION"),
            "environment": if cfg!(debug_assertions) { "development" } else { "production" },
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        },
        "events": events,
    });
    let endpoint = std::env::var("THESEUS_TELEMETRY_ENDPOINT")
        .unwrap_or_else(|_| ENDPOINT.to_string());
    let response = client.post(endpoint).json(&body).send().await;
    match response {
        Ok(response) if response.status().is_success() => {
            delete_events(state, &event_ids).await?;
        }
        Ok(response)
            if response.status().is_client_error()
                && response.status().as_u16() != 429 =>
        {
            delete_events(state, &event_ids).await?;
        }
        _ => schedule_retry(state, &event_ids).await?,
    }
    Ok(())
}

async fn delete_events(
    state: &State,
    event_ids: &[String],
) -> crate::Result<()> {
    for event_id in event_ids {
        sqlx::query("DELETE FROM telemetry_outbox WHERE event_id = ?")
            .bind(event_id)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}

async fn schedule_retry(
    state: &State,
    event_ids: &[String],
) -> crate::Result<()> {
    for event_id in event_ids {
        let row = sqlx::query(
            "SELECT attempts FROM telemetry_outbox WHERE event_id = ?",
        )
        .bind(event_id)
        .fetch_optional(&state.pool)
        .await?;
        let Some(row) = row else { continue };
        let attempts = row.get::<i64, _>("attempts") + 1;
        let delay = match attempts {
            1 => 60,
            2 => 5 * 60,
            3 => 30 * 60,
            _ => 6 * 60 * 60,
        };
        sqlx::query(
			"UPDATE telemetry_outbox SET attempts = ?, next_attempt_at = ? WHERE event_id = ?",
		)
		.bind(attempts)
		.bind(Utc::now().timestamp() + delay)
		.bind(event_id)
		.execute(&state.pool)
		.await?;
    }
    Ok(())
}

async fn recover_panic_marker(state: &State) -> crate::Result<()> {
    let path = state.directories.settings_dir.join(PANIC_MARKER_FILE);
    let Ok(contents) = tokio::fs::read_to_string(&path).await else {
        return Ok(());
    };
    let marker: Value = serde_json::from_str(&contents).unwrap_or_else(|_| {
		json!({ "error_type": "rust_panic", "message": "Previous launcher panic" })
	});
    let report = FrontendErrorReport {
        error_type: marker
            .get("error_type")
            .and_then(Value::as_str)
            .unwrap_or("rust_panic")
            .to_string(),
        message: marker
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Previous launcher panic")
            .to_string(),
        stack: marker
            .get("stack")
            .and_then(Value::as_str)
            .map(str::to_string),
        route: None,
        command: Some("panic_hook".to_string()),
        context: None,
    };
    queue_error(state, report).await?;
    let _ = tokio::fs::remove_file(path).await;
    Ok(())
}

async fn drain_rust_errors(state: &State) {
    let pending = if let Ok(mut errors) = PENDING_RUST_ERRORS.lock() {
        errors.drain(..).collect::<Vec<_>>()
    } else {
        return;
    };
    let ring = LOG_RING
        .lock()
        .map(|ring| ring.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    for error in pending {
        let context = ring
            .iter()
            .filter(|line| {
                line.sequence.saturating_add(40) >= error.sequence
                    && line.sequence <= error.sequence.saturating_add(10)
            })
            .map(|line| line.line.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let _ = queue_error(
            state,
            FrontendErrorReport {
                error_type: "rust_tracing".to_string(),
                message: error.message,
                stack: None,
                route: None,
                command: Some(error.target),
                context: Some(context),
            },
        )
        .await;
    }
}

pub fn sanitize(input: &str) -> String {
    let mut value = input.replace('\0', "");
    value = BEARER_RE
        .replace_all(&value, "Bearer <redacted>")
        .into_owned();
    value = SECRET_RE.replace_all(&value, "$1=<redacted>").into_owned();
    value = SENSITIVE_QUERY_RE
        .replace_all(&value, "$1<redacted>")
        .into_owned();
    value = EMAIL_RE.replace_all(&value, "<email>").into_owned();
    value = WINDOWS_HOME_RE.replace_all(&value, "<home>").into_owned();
    value = UNIX_HOME_RE.replace_all(&value, "<home>/").into_owned();
    value = UUID_RE.replace_all(&value, "<uuid>").into_owned();
    let username = whoami::username();
    if username.len() >= 3 {
        value = value.replace(&username, "<username>");
    }
    value
}

fn sanitize_with_credentials(
    input: &str,
    credentials: &[Credentials],
) -> String {
    let censored = CensoredString::censor(input.to_string(), credentials);
    sanitize(censored.as_str())
}

fn fingerprint(error_type: &str, message: &str, stack: Option<&str>) -> String {
    let normalized = UUID_RE.replace_all(message, "<id>");
    let normalized = NUMBER_RE.replace_all(&normalized, "<n>");
    let stack_head = stack
        .and_then(|stack| stack.lines().next())
        .unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(error_type.as_bytes());
    hasher.update([0]);
    hasher.update(normalized.as_bytes());
    hasher.update([0]);
    hasher.update(stack_head.as_bytes());
    hex_digest(hasher.finalize().as_slice())
}

fn stable_batch_id(event_ids: &[String]) -> String {
    let mut hasher = Sha256::new();
    for event_id in event_ids {
        hasher.update(event_id.as_bytes());
        hasher.update([0]);
    }
    let digest = hex_digest(hasher.finalize().as_slice());
    format!(
        "{}-{}-{}-{}-{}",
        &digest[0..8],
        &digest[8..12],
        &digest[12..16],
        &digest[16..20],
        &digest[20..32]
    )
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn truncate_utf8(input: &str, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input.to_string();
    }
    let mut boundary = max_bytes;
    while !input.is_char_boundary(boundary) {
        boundary -= 1;
    }
    input[..boundary].to_string()
}

fn default_error_type() -> String {
    "frontend".to_string()
}

fn wake() {
    if let Some(sender) = WAKE_TX.get() {
        let _ = sender.try_send(());
    }
}

fn clear_runtime_buffers() {
    if let Ok(mut pending) = PENDING_RUST_ERRORS.lock() {
        pending.clear();
    }
    if let Ok(mut ring) = LOG_RING.lock() {
        ring.clear();
    }
    if let Ok(mut aggregates) = DOWNLOAD_AGGREGATES.lock() {
        aggregates.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_removes_common_identifiers_and_secrets() {
        let input = "Authorization: Bearer abc.def token=secret user@example.com C:\\Users\\Alice\\file 550e8400-e29b-41d4-a716-446655440000 https://example.com/?access_token=secret";
        let output = sanitize(input);
        assert!(!output.contains("abc.def"));
        assert!(!output.contains("secret"));
        assert!(!output.contains("user@example.com"));
        assert!(!output.contains("550e8400"));
        assert!(!output.contains("C:\\Users\\Alice"));
    }

    #[test]
    fn sanitizer_reuses_minecraft_credential_redaction() {
        let mut credentials = Credentials::offline("Player_123").unwrap();
        credentials.access_token =
            "raw-minecraft-access-token-value".to_string();
        let minecraft_uuid = credentials.offline_profile.id.to_string();
        let input = format!(
            "Minecraft 1.20.1 user={} uuid={} token={}",
            credentials.offline_profile.name,
            credentials.offline_profile.id,
            credentials.access_token,
        );
        let output = sanitize_with_credentials(&input, &[credentials]);

        assert!(output.contains("1.20.1"));
        assert!(!output.contains("Player_123"));
        assert!(!output.contains("raw-minecraft-access-token-value"));
        assert!(!output.contains(&minecraft_uuid));
    }

    #[test]
    fn truncation_preserves_utf8_boundaries() {
        assert_eq!(truncate_utf8("ab中文", 5), "ab中");
    }

    #[test]
    fn batch_ids_are_stable() {
        let ids = vec!["one".to_string(), "two".to_string()];
        assert_eq!(stable_batch_id(&ids), stable_batch_id(&ids));
        assert_ne!(stable_batch_id(&ids), stable_batch_id(&ids[..1]));
    }
}
