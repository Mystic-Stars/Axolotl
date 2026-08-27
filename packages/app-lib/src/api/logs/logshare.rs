use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::analyze_crash;
use crate::emit_logshare_ai_event;
use crate::State;

const LOGSHARE_BASE_URL: &str = "http://127.0.0.1:9501";
const LOGSHARE_GZIP_THRESHOLD_BYTES: usize = 64 * 1024;
const LOGSHARE_AI_READ_TIMEOUT_SECS: u64 = 300;
const LOGSHARE_SOURCE_PREFIX: &str = "axolotl";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogShareSettings {
    pub share_provider: String,
    pub ai_source: String,
    pub auto_upload: bool,
    pub multi_file: bool,
    pub no_storage: bool,
    pub show_progress: bool,
}

impl Default for LogShareSettings {
    fn default() -> Self {
        Self {
            share_provider: "logshare".to_string(),
            ai_source: "logshare".to_string(),
            auto_upload: true,
            multi_file: true,
            no_storage: false,
            show_progress: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SharedLog {
    pub id: String,
    pub url: String,
    pub raw: String,
    pub token: String,
    pub provider: String,
    pub instance_id: Option<String>,
    pub instance_name: Option<String>,
    pub truncated: bool,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogShareUploadResponse {
    pub success: bool,
    pub id: String,
    pub url: String,
    pub raw: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogShareDeleteResponse {
    pub success: bool,
    pub deleted: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
struct UploadFile {
    name: String,
    content: String,
}

#[derive(Serialize, Debug, Clone)]
struct UploadMetadata {
    key: String,
    value: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
struct UploadBody {
    content: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    files: Vec<UploadFile>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    metadata: Vec<UploadMetadata>,
    source: String,
}

pub async fn get_log_share_settings() -> crate::Result<LogShareSettings> {
    let state = State::get().await?;
    let row = sqlx::query_as::<_, (String, String, i64, i64, i64, i64)>(
        "SELECT share_provider, ai_source, auto_upload, multi_file, no_storage, show_progress FROM log_share_settings WHERE id = 0",
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(LogShareSettings {
        share_provider: row.0,
        ai_source: row.1,
        auto_upload: row.2 != 0,
        multi_file: row.3 != 0,
        no_storage: row.4 != 0,
        show_progress: row.5 != 0,
    })
}

pub async fn update_log_share_settings(
    settings: LogShareSettings,
) -> crate::Result<()> {
    let settings = normalize_settings(settings);
    let state = State::get().await?;
    sqlx::query(
        "UPDATE log_share_settings SET share_provider = ?, ai_source = ?, auto_upload = ?, multi_file = ?, no_storage = ?, show_progress = ? WHERE id = 0",
    )
    .bind(&settings.share_provider)
    .bind(&settings.ai_source)
    .bind(settings.auto_upload)
    .bind(settings.multi_file)
    .bind(settings.no_storage)
    .bind(settings.show_progress)
    .execute(&state.pool)
    .await?;
    Ok(())
}

fn normalize_settings(mut settings: LogShareSettings) -> LogShareSettings {
    if settings.share_provider != "logshare" {
        settings.share_provider = "mclogs".to_string();
    }
    if settings.ai_source != "logshare" {
        settings.ai_source = "custom".to_string();
    }
    settings
}

pub async fn list_shared_logs() -> crate::Result<Vec<SharedLog>> {
    let state = State::get().await?;
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, Option<String>, i64, i64)>(
        "SELECT id, url, raw, token, provider, instance_id, instance_name, truncated, created_at FROM shared_logs ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| SharedLog {
            id: row.0,
            url: row.1,
            raw: row.2,
            token: row.3,
            provider: row.4,
            instance_id: row.5,
            instance_name: row.6,
            truncated: row.7 != 0,
            created_at: row.8,
        })
        .collect())
}

pub async fn record_shared_log(log: SharedLog) -> crate::Result<()> {
    let state = State::get().await?;
    let created_at = log
        .created_at
        .max(now_seconds());
    sqlx::query(
        "INSERT INTO shared_logs (id, url, raw, token, provider, instance_id, instance_name, truncated, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET url = excluded.url, raw = excluded.raw, token = excluded.token, provider = excluded.provider, instance_id = excluded.instance_id, instance_name = excluded.instance_name, truncated = excluded.truncated",
    )
    .bind(&log.id)
    .bind(&log.url)
    .bind(&log.raw)
    .bind(&log.token)
    .bind(&log.provider)
    .bind(&log.instance_id)
    .bind(&log.instance_name)
    .bind(log.truncated)
    .bind(created_at)
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn remove_shared_log_record(id: &str) -> crate::Result<()> {
    let state = State::get().await?;
    sqlx::query("DELETE FROM shared_logs WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(())
}

pub async fn delete_shared_log(id: String, token: String) -> crate::Result<()> {
    let remote_result = delete_log(&id, &token).await;
    if let Err(error) = remote_result {
        tracing::warn!(
            "Remote LogShare deletion failed for {id}; removing the local record anyway: {error}"
        );
    }
    remove_shared_log_record(&id).await
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn source_identifier() -> String {
    format!("{LOGSHARE_SOURCE_PREFIX}/{}", env!("CARGO_PKG_VERSION"))
}

struct PreparedUpload {
    body: UploadBody,
    gzip: bool,
}

async fn prepare_upload(instance_id: &str) -> crate::Result<PreparedUpload> {
    let settings = get_log_share_settings().await?;
    let analysis = analyze_crash(instance_id).await?;

    let combined = analysis.combined_log.as_str().to_string();
    let files = if settings.multi_file {
        analysis
            .sources
            .iter()
            .map(|source| UploadFile {
                name: source.filename.clone(),
                content: source.content.as_str().to_string(),
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let content = if files.is_empty() {
        combined.clone()
    } else {
        String::new()
    };

    let metadata = vec![
        UploadMetadata {
            key: "launcher".to_string(),
            value: "axolotl".to_string(),
        },
        UploadMetadata {
            key: "matched_mods".to_string(),
            value: analysis
                .mods
                .iter()
                .map(|item| item.file_name.clone())
                .collect::<Vec<_>>()
                .join(", "),
        },
    ];

    let body = UploadBody {
        content,
        files,
        metadata,
        source: source_identifier(),
    };
    let payload = serde_json::to_vec(&body)?;
    let payload_len = payload.len();
    Ok(PreparedUpload {
        body,
        gzip: payload_len >= LOGSHARE_GZIP_THRESHOLD_BYTES,
    })
}

async fn request_client() -> crate::Result<reqwest::Client> {
    let state = State::get().await?;
    let proxy = state.proxy_config().await?;
    let builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .read_timeout(Duration::from_secs(LOGSHARE_AI_READ_TIMEOUT_SECS))
        .user_agent(crate::launcher_user_agent());
    proxy.apply(builder)?.build().map_err(Into::into)
}

pub async fn upload_crash(instance_id: &str) -> crate::Result<LogShareUploadResponse> {
    let prepared = prepare_upload(instance_id).await?;
    let client = request_client().await?;
    let mut request = client
        .post(format!("{LOGSHARE_BASE_URL}/v1/log"))
        .header("Accept", "application/json");

    let body_value = serde_json::to_value(&prepared.body)?;
    if prepared.gzip {
        let bytes = serde_json::to_vec(&body_value)?;
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        );
        use std::io::Write as _;
        encoder
            .write_all(&bytes)
            .map_err(|error| crate::ErrorKind::OtherError(error.to_string()))?;
        let compressed = encoder
            .finish()
            .map_err(|error| crate::ErrorKind::OtherError(error.to_string()))?;
        request = request
            .header("Content-Encoding", "gzip")
            .header("Content-Type", "application/json")
            .body(compressed);
    } else {
        request = request.json(&body_value);
    }

    let response = request.send().await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "LogShare upload failed with HTTP {status}: {}",
            text.chars().take(500).collect::<String>()
        ))
        .into());
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|_| crate::ErrorKind::OtherError("LogShare returned invalid JSON".to_string()))?;
    if value.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(crate::ErrorKind::OtherError(format!(
            "LogShare upload failed: {}",
            value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        ))
        .into());
    }
    Ok(LogShareUploadResponse {
        success: true,
        id: value
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        url: value
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        raw: value
            .get("raw")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        token: value
            .get("token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

pub async fn get_insights(id: &str) -> crate::Result<Value> {
    let client = request_client().await?;
    let response = client
        .get(format!("{LOGSHARE_BASE_URL}/v1/insights/{id}"))
        .header("Accept", "application/json")
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "LogShare analysis failed with HTTP {status}: {}",
            text.chars().take(500).collect::<String>()
        ))
        .into());
    }
    serde_json::from_str(&text).map_err(|_| {
        crate::ErrorKind::OtherError("LogShare returned invalid insights JSON".to_string()).into()
    })
}

pub async fn analyse_crash_direct(instance_id: &str) -> crate::Result<Value> {
    let prepared = prepare_upload(instance_id).await?;
    let client = request_client().await?;
    let response = client
        .post(format!("{LOGSHARE_BASE_URL}/v1/analyse"))
        .header("Accept", "application/json")
        .json(&prepared.body)
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "LogShare analysis failed with HTTP {status}: {}",
            text.chars().take(500).collect::<String>()
        ))
        .into());
    }
    serde_json::from_str(&text).map_err(|_| {
        crate::ErrorKind::OtherError("LogShare returned invalid analysis JSON".to_string()).into()
    })
}

pub async fn delete_log(id: &str, token: &str) -> crate::Result<LogShareDeleteResponse> {
    let client = request_client().await?;
    let response = client
        .delete(format!("{LOGSHARE_BASE_URL}/v1/log/{id}"))
        .bearer_auth(token)
        .header("Accept", "application/json")
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "LogShare deletion failed with HTTP {status}: {}",
            text.chars().take(500).collect::<String>()
        ))
        .into());
    }
    serde_json::from_str(&text).map_err(|_| {
        crate::ErrorKind::OtherError("LogShare returned invalid deletion JSON".to_string()).into()
    })
}

pub async fn ai_analyze_stored(instance_id: &str, id: &str) -> crate::Result<String> {
    let url = format!("{LOGSHARE_BASE_URL}/v1/ai/{id}");
    stream_ai(instance_id, url, None).await
}

pub async fn ai_analyze_direct(instance_id: &str) -> crate::Result<String> {
    let prepared = prepare_upload(instance_id).await?;
    let url = format!("{LOGSHARE_BASE_URL}/v1/ai/analyse");
    let body = json!({
        "content": prepared.body.content,
        "files": prepared.body.files,
        "metadata": prepared.body.metadata,
        "source": prepared.body.source,
    });
    stream_ai(instance_id, url, Some(body)).await
}

async fn stream_ai(
    instance_id: &str,
    url: String,
    body: Option<Value>,
) -> crate::Result<String> {
    let settings = get_log_share_settings().await?;
    let show_progress = settings.show_progress;
    let client = request_client().await?;
    let mut request = client
        .request(
            if body.is_some() {
                reqwest::Method::POST
            } else {
                reqwest::Method::GET
            },
            url.as_str(),
        )
        .header("Accept", "text/event-stream");
    if let Some(payload) = body {
        request = request.json(&payload);
    }

    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await?;
        return Err(crate::ErrorKind::OtherError(format!(
            "LogAgent analysis failed with HTTP {status}: {}",
            text.chars().take(500).collect::<String>()
        ))
        .into());
    }

    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();
    let mut output = String::new();

    loop {
        let maybe = tokio::time::timeout(
            Duration::from_secs(LOGSHARE_AI_READ_TIMEOUT_SECS),
            stream.next(),
        )
        .await
        .map_err(|_| {
            crate::ErrorKind::OtherError("LogAgent analysis timed out".to_string())
        })?;
        let Some(chunk) = maybe else {
            break;
        };
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        for event in parser.feed(&text) {
            if let Some(delta) = event.content {
                output.push_str(&delta);
                if show_progress {
                    emit_logshare_ai_event(
                        instance_id,
                        "delta",
                        json!({ "content": delta }),
                    )
                    .await?;
                }
            }
            if let Some(status_event) = event.status_event {
                if show_progress {
                    emit_logshare_ai_event(
                        instance_id,
                        &status_event.event_type,
                        status_event.data,
                    )
                    .await?;
                }
            }
            if let Some(message) = event.error {
                return Err(crate::ErrorKind::OtherError(format!(
                    "LogAgent analysis failed: {message}"
                ))
                .into());
            }
            if event.done {
                return Ok(output);
            }
        }
    }

    // A stream that ended without `event: done` is acceptable as long as text
    // was produced; otherwise it is a protocol error worth surfacing.
    if output.is_empty() {
        return Err(crate::ErrorKind::OtherError(
            "LogAgent analysis ended without output".to_string(),
        )
        .into());
    }
    Ok(output)
}

struct ParsedEvent {
    content: Option<String>,
    status_event: Option<ParsedStatusEvent>,
    done: bool,
    error: Option<String>,
}

struct ParsedStatusEvent {
    event_type: String,
    data: Value,
}

struct SseParser {
    buffer: String,
}

impl SseParser {
    fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    fn feed(&mut self, chunk: &str) -> Vec<ParsedEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();
        while let Some(pos) = self.buffer.find("\n\n") {
            let block = self.buffer[..pos].to_string();
            self.buffer.drain(..pos + 2);
            if let Some(event) = parse_sse_block(&block) {
                events.push(event);
            }
        }
        events
    }
}

fn parse_sse_block(block: &str) -> Option<ParsedEvent> {
    let mut event_name: Option<String> = None;
    let mut data_lines = Vec::new();

    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("event:") {
            event_name = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            data_lines.push(rest.trim_start().to_string());
        }
    }
    if event_name.is_none() && data_lines.is_empty() {
        return None;
    }

    let data_text = data_lines.join("\n");
    let event_name = event_name.as_deref().unwrap_or("message");

    match event_name {
        "done" => Some(ParsedEvent {
            content: None,
            status_event: None,
            done: true,
            error: None,
        }),
        "error" => Some(ParsedEvent {
            content: None,
            status_event: None,
            done: false,
            error: Some(
                serde_json::from_str::<Value>(&data_text)
                    .ok()
                    .and_then(|value| {
                        value.get("error").and_then(Value::as_str).map(str::to_string)
                    })
                    .unwrap_or_else(|| data_text.clone()),
            ),
        }),
        "status" => {
            let Ok(value) = serde_json::from_str::<Value>(&data_text) else {
                return None;
            };
            let event_type = value
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("status")
                .to_string();
            Some(ParsedEvent {
                content: None,
                status_event: Some(ParsedStatusEvent {
                    event_type,
                    data: value,
                }),
                done: false,
                error: None,
            })
        }
        _ => {
            let Ok(value) = serde_json::from_str::<Value>(&data_text) else {
                return None;
            };
            let content = value
                .pointer("/choices/0/delta/content")
                .and_then(Value::as_str)
                .map(str::to_string);
            Some(ParsedEvent {
                content,
                status_event: None,
                done: false,
                error: None,
            })
        }
    }
}
