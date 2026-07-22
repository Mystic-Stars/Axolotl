/*
    tracing is set basd on the environment variable RUST_LOG=xxx, depending on the amount of logs to show
        ERROR > WARN > INFO > DEBUG > TRACE
    eg. RUST_LOG=info will show info, warn, and error logs
        RUST_LOG="theseus=trace" will show *all* messages but from theseus only (and not dependencies using similar crates)
        RUST_LOG="theseus=trace" will show *all* messages but from theseus only (and not dependencies using similar crates)

    Error messages returned to Tauri will display as traced error logs if they return an error.
    This will also include an attached span trace if the error is from a tracing error, and the level is set to info, debug, or trace

    on unix:
        RUST_LOG="theseus=trace" {run command}

    The default is theseus=show, meaning only logs from theseus will be displayed, and at the info or higher level.

*/

#[cfg(debug_assertions)]
const CONSOLE_LOG_MAX_LINES: usize = 5;
#[cfg(debug_assertions)]
const DEFAULT_CONSOLE_COLUMNS: usize = 80;
#[cfg(debug_assertions)]
const CONSOLE_TRUNCATION_MARKER: &str = "... [console output truncated]";

#[cfg(not(debug_assertions))]
const LAUNCHER_LOG_MAX_BYTES: u64 = 10 * 1024 * 1024;
#[cfg(not(debug_assertions))]
const LAUNCHER_WARN_ERROR_MAX_BYTES: u64 = 30 * 1024 * 1024;
#[cfg(not(debug_assertions))]
const LAUNCHER_LOG_MAX_FILES: usize = 5;
#[cfg(not(debug_assertions))]
const LAUNCHER_LOG_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(3 * 24 * 60 * 60);

#[cfg(any(test, not(debug_assertions)))]
#[derive(Clone)]
struct RotatingLogWriter {
    state: std::sync::Arc<std::sync::Mutex<RotatingLogState>>,
    normal_max_bytes: u64,
    warn_error_max_bytes: u64,
}

#[cfg(any(test, not(debug_assertions)))]
struct RotatingLogState {
    logs_dir: std::path::PathBuf,
    session_name: String,
    segment: usize,
    file: std::fs::File,
    bytes_written: u64,
    max_files: usize,
    max_age: std::time::Duration,
}

#[cfg(any(test, not(debug_assertions)))]
impl RotatingLogWriter {
    fn new(
        logs_dir: std::path::PathBuf,
        session_name: String,
        normal_max_bytes: u64,
        warn_error_max_bytes: u64,
        max_files: usize,
        max_age: std::time::Duration,
    ) -> std::io::Result<Self> {
        std::fs::create_dir_all(&logs_dir)?;
        let path = rotating_log_path(&logs_dir, &session_name, 0);
        let file = open_log_file(&path)?;
        let bytes_written = file.metadata()?.len();
        let state = RotatingLogState {
            logs_dir,
            session_name,
            segment: 0,
            file,
            bytes_written,
            max_files: max_files.max(1),
            max_age,
        };
        let writer = Self {
            state: std::sync::Arc::new(std::sync::Mutex::new(state)),
            normal_max_bytes: normal_max_bytes.max(1),
            warn_error_max_bytes: warn_error_max_bytes
                .max(normal_max_bytes)
                .max(1),
        };
        writer.cleanup_old_logs();
        Ok(writer)
    }

    fn write_event(
        &self,
        buffer: &[u8],
        max_file_bytes: u64,
    ) -> std::io::Result<()> {
        let mut state = self.state.lock().map_err(|_| {
            std::io::Error::other("launcher log writer lock poisoned")
        })?;
        state.write_event(buffer, max_file_bytes)
    }

    fn flush(&self) -> std::io::Result<()> {
        let mut state = self.state.lock().map_err(|_| {
            std::io::Error::other("launcher log writer lock poisoned")
        })?;
        std::io::Write::flush(&mut state.file)
    }

    fn cleanup_old_logs(&self) {
        if let Ok(state) = self.state.lock() {
            let active_path = rotating_log_path(
                &state.logs_dir,
                &state.session_name,
                state.segment,
            );
            cleanup_launcher_logs(
                &state.logs_dir,
                state.max_files,
                state.max_age,
                &active_path,
            );
        }
    }
}

#[cfg(any(test, not(debug_assertions)))]
impl RotatingLogState {
    fn write_event(
        &mut self,
        buffer: &[u8],
        max_file_bytes: u64,
    ) -> std::io::Result<()> {
        if self.bytes_written > 0
            && self.bytes_written.saturating_add(buffer.len() as u64)
                > max_file_bytes.max(1)
        {
            self.rotate()?;
        }

        std::io::Write::write_all(&mut self.file, buffer)?;
        self.bytes_written =
            self.bytes_written.saturating_add(buffer.len() as u64);
        Ok(())
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(&mut self.file)?;
        self.segment += 1;
        let path =
            rotating_log_path(&self.logs_dir, &self.session_name, self.segment);
        self.file = open_log_file(&path)?;
        self.bytes_written = self.file.metadata()?.len();
        cleanup_launcher_logs(
            &self.logs_dir,
            self.max_files,
            self.max_age,
            &path,
        );
        Ok(())
    }
}

#[cfg(any(test, not(debug_assertions)))]
fn rotating_log_path(
    logs_dir: &std::path::Path,
    session_name: &str,
    segment: usize,
) -> std::path::PathBuf {
    if segment == 0 {
        logs_dir.join(format!("{session_name}.log"))
    } else {
        logs_dir.join(format!("{session_name}_{segment:03}.log"))
    }
}

#[cfg(any(test, not(debug_assertions)))]
fn open_log_file(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(path)
}

#[cfg(any(test, not(debug_assertions)))]
fn cleanup_launcher_logs(
    logs_dir: &std::path::Path,
    max_files: usize,
    max_age: std::time::Duration,
    active_path: &std::path::Path,
) {
    cleanup_launcher_logs_at(
        logs_dir,
        max_files,
        max_age,
        active_path,
        std::time::SystemTime::now(),
    );
}

#[cfg(any(test, not(debug_assertions)))]
fn cleanup_launcher_logs_at(
    logs_dir: &std::path::Path,
    max_files: usize,
    max_age: std::time::Duration,
    active_path: &std::path::Path,
    now: std::time::SystemTime,
) {
    let Ok(entries) = std::fs::read_dir(logs_dir) else {
        return;
    };
    let mut logs = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if !entry.file_type().ok()?.is_file()
                || !name.starts_with("session_")
                || path.extension()?.to_str()? != "log"
            {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            let created =
                metadata.created().or_else(|_| metadata.modified()).ok();
            Some((created, path))
        })
        .collect::<Vec<_>>();
    logs.retain(|(created, path)| {
        if path != active_path
            && created.is_some_and(|created| {
                launcher_log_is_expired(created, now, max_age)
            })
        {
            return std::fs::remove_file(path).is_err();
        }
        true
    });
    logs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    while logs.len() > max_files.max(1) {
        let Some(index) = logs.iter().position(|(_, path)| path != active_path)
        else {
            break;
        };
        let (_, path) = logs.remove(index);
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(any(test, not(debug_assertions)))]
fn launcher_log_is_expired(
    created: std::time::SystemTime,
    now: std::time::SystemTime,
    max_age: std::time::Duration,
) -> bool {
    now.duration_since(created).is_ok_and(|age| age > max_age)
}

#[cfg(any(test, not(debug_assertions)))]
struct LogEventWriter {
    writer: RotatingLogWriter,
    buffer: Vec<u8>,
    max_file_bytes: u64,
}

#[cfg(any(test, not(debug_assertions)))]
impl LogEventWriter {
    fn commit(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.writer.write_event(&self.buffer, self.max_file_bytes)?;
        self.buffer.clear();
        Ok(())
    }
}

#[cfg(any(test, not(debug_assertions)))]
impl std::io::Write for LogEventWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.commit()?;
        self.writer.flush()
    }
}

#[cfg(any(test, not(debug_assertions)))]
impl Drop for LogEventWriter {
    fn drop(&mut self) {
        let _ = self.commit();
    }
}

#[cfg(any(test, not(debug_assertions)))]
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RotatingLogWriter {
    type Writer = LogEventWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.event_writer(self.normal_max_bytes)
    }

    fn make_writer_for(
        &'a self,
        metadata: &tracing::Metadata<'_>,
    ) -> Self::Writer {
        let max_file_bytes = match *metadata.level() {
            tracing::Level::WARN | tracing::Level::ERROR => {
                self.warn_error_max_bytes
            }
            _ => self.normal_max_bytes,
        };
        self.event_writer(max_file_bytes)
    }
}

#[cfg(any(test, not(debug_assertions)))]
impl RotatingLogWriter {
    fn event_writer(&self, max_file_bytes: u64) -> LogEventWriter {
        LogEventWriter {
            writer: self.clone(),
            buffer: Vec::new(),
            max_file_bytes,
        }
    }
}

#[cfg(debug_assertions)]
fn console_columns() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|columns| (40..=500).contains(columns))
        .unwrap_or(DEFAULT_CONSOLE_COLUMNS)
}

#[cfg(debug_assertions)]
fn next_console_token(input: &str, start: usize) -> (usize, usize, bool) {
    let bytes = input.as_bytes();
    if bytes[start] == 0x1b {
        let mut end = start + 1;
        if bytes.get(end) == Some(&b'[') {
            end += 1;
            while let Some(byte) = bytes.get(end) {
                end += 1;
                if (0x40..=0x7e).contains(byte) {
                    break;
                }
            }
        } else if end < bytes.len() {
            end += 1;
        }
        return (end, 0, false);
    }

    let character = input[start..].chars().next().expect("valid character");
    let end = start + character.len_utf8();
    let width = match character {
        '\n' | '\r' => 0,
        '\t' => 4,
        character if character.is_control() => 0,
        _ => 1,
    };
    (end, width, character == '\n')
}

#[cfg(debug_assertions)]
const MODRINTH_CONSOLE_MESSAGES: &[&str] = &[
    "Attempting Modrinth request",
    "Completed Modrinth request",
    "Modrinth mirror resolved cached file",
    "Modrinth mirror redirected to official CDN; falling back to official source",
    "Modrinth mirror returned an unresolved redirect; falling back to official source",
    "Modrinth request attempt failed; retrying",
    "Modrinth mirror failed; falling back to official source",
    "Modrinth official request failed",
    "Modrinth CDN download progress",
    "Modrinth checksum validation failed; retrying",
    "Modrinth response body failed; retrying",
    "Modrinth mirror response failed; falling back to official source",
    "Modrinth official response failed",
    "Modrinth connection failed; retrying",
    "Modrinth mirror connection failed; falling back to official source",
    "Modrinth official connection failed",
    "Modrinth mirror redirected; treating as cache miss and falling back to official source",
];

#[cfg(debug_assertions)]
fn strip_console_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < input.len() {
        let (end, _, _) = next_console_token(input, index);
        if input.as_bytes()[index] != 0x1b {
            output.push_str(&input[index..end]);
        }
        index = end;
    }

    output
}

#[cfg(debug_assertions)]
fn extract_console_field(input: &str, field: &str) -> Option<String> {
    let marker = format!(" {field}=");
    let value = input.rsplit_once(&marker)?.1;
    if let Some(value) = value.strip_prefix('"') {
        let mut escaped = false;
        for (index, character) in value.char_indices() {
            if character == '"' && !escaped {
                return Some(value[..index].to_string());
            }
            escaped = character == '\\' && !escaped;
            if character != '\\' {
                escaped = false;
            }
        }
        return Some(value.to_string());
    }

    Some(
        value
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string(),
    )
}

#[cfg(debug_assertions)]
fn compact_console_url(value: &str) -> String {
    const MAX_URL_CHARS: usize = 110;

    let value = value
        .split_once('?')
        .map(|(base, _)| format!("{base}?<query omitted>"))
        .unwrap_or_else(|| value.to_string());
    if value.chars().count() <= MAX_URL_CHARS {
        return value;
    }

    let mut output: String = value.chars().take(MAX_URL_CHARS - 3).collect();
    output.push_str("...");
    output
}

#[cfg(debug_assertions)]
fn compact_modrinth_console_event(input: &str) -> Option<String> {
    let input = strip_console_ansi(input);
    let (_, message) = MODRINTH_CONSOLE_MESSAGES
        .iter()
        .filter_map(|message| {
            input.find(message).map(|index| (index, *message))
        })
        .min_by_key(|(index, _)| *index)?;
    let level = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"]
        .into_iter()
        .find(|level| input.contains(&format!(" {level} ")))
        .unwrap_or("INFO");
    let source = extract_console_field(&input, "source");
    let mut output = format!("{level} {message}");

    if source.as_deref() == Some("Mirror") {
        let explicit_status = extract_console_field(&input, "mirror_status");
        let mirror_status = explicit_status.as_deref().unwrap_or_else(|| {
            if message.contains("redirected") {
                "cache_miss"
            } else if message.starts_with("Completed") {
                "completed"
            } else if message.contains("failed") {
                "failed"
            } else {
                "attempting"
            }
        });
        output.push_str(&format!(" mirror_status={mirror_status}"));
    }

    for field in [
        "source",
        "request_kind",
        "method",
        "status",
        "route",
        "attempt",
        "max_attempts",
        "url",
        "mirror_url",
        "redirect_url",
        "final_url",
        "cache_status",
        "elapsed_ms",
        "bytes",
        "downloaded_bytes",
        "expected_bytes",
    ] {
        let value = if field == "source" {
            source.clone()
        } else {
            extract_console_field(&input, field)
        };
        if let Some(mut value) = value {
            if matches!(
                field,
                "url" | "mirror_url" | "redirect_url" | "final_url"
            ) {
                value = compact_console_url(&value);
            }
            if value.chars().any(char::is_whitespace) {
                output.push_str(&format!(" {field}={value:?}"));
            } else {
                output.push_str(&format!(" {field}={value}"));
            }
        }
    }
    output.push('\n');
    Some(output)
}

#[cfg(debug_assertions)]
fn fits_within_console_lines(
    input: &str,
    columns: usize,
    max_lines: usize,
) -> bool {
    let mut index = 0;
    let mut line = 1;
    let mut column = 0;

    while index < input.len() {
        let (end, width, newline) = next_console_token(input, index);
        if newline {
            if end == input.len() {
                return true;
            }
            line += 1;
            column = 0;
        } else if width > 0 {
            if column + width > columns {
                line += 1;
                column = 0;
            }
            column += width;
        }
        if line > max_lines {
            return false;
        }
        index = end;
    }

    true
}

#[cfg(debug_assertions)]
fn truncate_console_event(
    input: &str,
    columns: usize,
    max_lines: usize,
) -> String {
    if columns == 0
        || max_lines == 0
        || fits_within_console_lines(input, columns, max_lines)
    {
        return input.to_string();
    }

    let marker_width = CONSOLE_TRUNCATION_MARKER.len().min(columns);
    let final_line_limit = columns.saturating_sub(marker_width);
    let mut index = 0;
    let mut line = 1;
    let mut column = 0;

    while index < input.len() {
        let (end, width, newline) = next_console_token(input, index);
        if newline {
            if line == max_lines {
                break;
            }
            line += 1;
            column = 0;
            index = end;
            continue;
        }

        if width > 0 && column + width > columns {
            if line == max_lines {
                break;
            }
            line += 1;
            column = 0;
        }
        if line == max_lines && column + width > final_line_limit {
            break;
        }
        column += width;
        index = end;
    }

    let mut output = input[..index].to_string();
    output.push_str("\x1b[0m");
    output.push_str(CONSOLE_TRUNCATION_MARKER);
    output.push('\n');
    output
}

#[cfg(debug_assertions)]
struct TruncatedConsoleWriter {
    stdout: std::io::Stdout,
}

#[cfg(debug_assertions)]
impl std::io::Write for TruncatedConsoleWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let input = String::from_utf8_lossy(buffer);
        let compacted = compact_modrinth_console_event(&input);
        let input = compacted.as_deref().unwrap_or(&input);
        let output = truncate_console_event(
            input,
            console_columns(),
            CONSOLE_LOG_MAX_LINES,
        );
        self.stdout.write_all(output.as_bytes())?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(&mut self.stdout)
    }
}

// Handling for the live development logging
// This will log to the console, and will not log to a file
#[cfg(debug_assertions)]
pub fn start_logger(_app_identifier: &str) -> Option<()> {
    use tracing_subscriber::prelude::*;

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("theseus=info,theseus_gui=info")
        });
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(|| {
            TruncatedConsoleWriter {
                stdout: std::io::stdout(),
            }
        }))
        .with(filter)
        .with(tracing_error::ErrorLayer::default())
        .init();
    Some(())
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn short_console_events_are_unchanged() {
        let input = "short log event\n";
        assert_eq!(truncate_console_event(input, 80, 5), input);
    }

    #[test]
    fn long_console_events_are_limited_to_five_visual_lines() {
        let input = format!("{}\n", "x".repeat(1_000));
        let output = truncate_console_event(&input, 80, 5);

        assert!(output.contains(CONSOLE_TRUNCATION_MARKER));
        assert!(fits_within_console_lines(&output, 80, 5));
        assert!(!output.contains(&"x".repeat(1_000)));
    }

    #[test]
    fn embedded_newlines_count_toward_console_limit() {
        let input = "one\ntwo\nthree\nfour\nfive\nsix\n";
        let output = truncate_console_event(input, 80, 5);

        assert!(output.contains(CONSOLE_TRUNCATION_MARKER));
        assert!(!output.contains("six"));
        assert!(fits_within_console_lines(&output, 80, 5));
    }

    #[test]
    fn ansi_sequences_do_not_count_as_visible_width() {
        let input = format!("\x1b[32m{}\x1b[0m\n", "x".repeat(500));
        let output = truncate_console_event(&input, 80, 5);

        assert!(output.starts_with("\x1b[32m"));
        assert!(output.contains("\x1b[0m"));
        assert!(output.contains(CONSOLE_TRUNCATION_MARKER));
        assert!(fits_within_console_lines(&output, 80, 5));
    }

    #[test]
    fn modrinth_events_drop_long_span_context_and_keep_route_fields() {
        let input = format!(
            "2026-07-20T14:20:45Z INFO generate_pack{{ids=\"{}\"}}:fetch{{method=GET url=\"https://cdn.modrinth.com/data/test/file.mrpack\"}}: Attempting Modrinth request source=Mirror request_kind=\"CDN\" method=GET url=https://mod.mcimirror.top/data/test/file.mrpack route=1 attempt=1 max_attempts=5\n",
            "x".repeat(1_000)
        );
        let output = compact_modrinth_console_event(&input).unwrap();

        assert!(output.starts_with("INFO Attempting Modrinth request"));
        assert!(output.contains("mirror_status=attempting"));
        assert!(output.contains("source=Mirror"));
        assert!(
            output.contains(
                "url=https://mod.mcimirror.top/data/test/file.mrpack"
            )
        );
        assert!(!output.contains(&"x".repeat(1_000)));
        assert!(fits_within_console_lines(&output, 80, 5));
    }

    #[test]
    fn modrinth_redirects_show_cache_miss_and_compact_long_urls() {
        let input = "2026-07-20T14:20:45Z WARN fetch: Modrinth mirror redirected; treating as cache miss and falling back to official source source=Mirror url=https://mod.mcimirror.top/data/test/file.mrpack?ids=very-long-query redirect_url=\"https://cdn.modrinth.com/data/test/file name.mrpack\" status=302 elapsed_ms=100\n";
        let output = compact_modrinth_console_event(input).unwrap();

        assert!(output.contains("mirror_status=cache_miss"));
        assert!(output.contains("status=302"));
        assert!(output.contains("?<query omitted>"));
        assert!(output.contains(
            "redirect_url=\"https://cdn.modrinth.com/data/test/file name.mrpack\""
        ));
        assert!(fits_within_console_lines(&output, 80, 5));
    }

    #[test]
    fn modrinth_cache_events_keep_explicit_status_and_urls() {
        let input = "2026-07-20T14:20:45Z INFO fetch: Modrinth mirror resolved cached file mirror_status=cache_hit source=Mirror mirror_url=https://mod.mcimirror.top/data/test/file.jar final_url=https://cache.mcimirror.top/data/test/file.jar cache_status=HIT status=200 elapsed_ms=100\n";
        let output = compact_modrinth_console_event(input).unwrap();

        assert!(output.contains("mirror_status=cache_hit"));
        assert!(output.contains(
            "mirror_url=https://mod.mcimirror.top/data/test/file.jar"
        ));
        assert!(output.contains(
            "final_url=https://cache.mcimirror.top/data/test/file.jar"
        ));
        assert!(output.contains("cache_status=HIT"));
        assert!(fits_within_console_lines(&output, 80, 5));
    }

    #[test]
    fn non_modrinth_events_are_not_compacted() {
        assert_eq!(
            compact_modrinth_console_event("INFO ordinary event\n"),
            None
        );
    }

    #[test]
    fn launcher_logs_rotate_by_size_and_keep_only_recent_files() {
        let directory = tempfile::tempdir().unwrap();
        let writer = RotatingLogWriter::new(
            directory.path().to_path_buf(),
            "session_20260722_120000".to_string(),
            10,
            30,
            3,
            std::time::Duration::from_secs(3 * 24 * 60 * 60),
        )
        .unwrap();

        for index in 0..6 {
            writer
                .write_event(
                    format!("event{index}\n").as_bytes(),
                    writer.normal_max_bytes,
                )
                .unwrap();
        }
        writer.flush().unwrap();

        let mut logs = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        logs.sort();
        assert_eq!(logs.len(), 3);
        assert!(logs.iter().all(|path| path.metadata().unwrap().len() <= 10));
        assert!(
            logs.iter().any(
                |path| std::fs::read_to_string(path).unwrap() == "event5\n"
            )
        );
    }

    #[test]
    fn warn_and_error_events_use_the_complete_event_limit() {
        let directory = tempfile::tempdir().unwrap();
        let writer = RotatingLogWriter::new(
            directory.path().to_path_buf(),
            "session_20260722_120000".to_string(),
            10,
            30,
            10,
            std::time::Duration::from_secs(3 * 24 * 60 * 60),
        )
        .unwrap();

        writer.write_event(b"12345678", 10).unwrap();
        writer.write_event(b"abcdefgh", 30).unwrap();
        writer.write_event(b"ok", 10).unwrap();
        writer.write_event(&[b'w'; 29], 30).unwrap();
        writer.write_event(&[b'e'; 31], 30).unwrap();
        writer.flush().unwrap();

        let mut logs = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        logs.sort();
        let contents = logs
            .iter()
            .map(std::fs::read)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            contents,
            vec![
                b"12345678abcdefgh".to_vec(),
                b"ok".to_vec(),
                vec![b'w'; 29],
                vec![b'e'; 31],
            ]
        );
    }

    #[test]
    fn launcher_logs_older_than_three_days_are_deleted() {
        let directory = tempfile::tempdir().unwrap();
        let old_path = directory.path().join("session_20260718_120000.log");
        let active_path = directory.path().join("session_20260722_120000.log");
        std::fs::write(&old_path, b"old").unwrap();
        std::fs::write(&active_path, b"active").unwrap();
        let created = old_path
            .metadata()
            .unwrap()
            .created()
            .or_else(|_| old_path.metadata().unwrap().modified())
            .unwrap();

        cleanup_launcher_logs_at(
            directory.path(),
            5,
            std::time::Duration::from_secs(3 * 24 * 60 * 60),
            &active_path,
            created + std::time::Duration::from_secs(4 * 24 * 60 * 60),
        );

        assert!(!old_path.exists());
        assert!(active_path.exists());
    }
}

// Handling for the live production logging
// This will log to a file in the logs directory, and will not show any logs in the console
#[cfg(not(debug_assertions))]
pub fn start_logger(app_identifier: &str) -> Option<()> {
    use crate::prelude::DirectoryInfo;
    use chrono::Local;
    use tracing_subscriber::fmt::time::ChronoLocal;
    use tracing_subscriber::prelude::*;

    // Initialize and get logs directory path
    let logs_dir = if let Some(d) =
        DirectoryInfo::launcher_logs_dir_path(app_identifier)
    {
        d
    } else {
        eprintln!("Could not start logger");
        return None;
    };

    let session_name =
        format!("session_{}", Local::now().format("%Y%m%d_%H%M%S"));
    let writer = match RotatingLogWriter::new(
        logs_dir,
        session_name,
        LAUNCHER_LOG_MAX_BYTES,
        LAUNCHER_WARN_ERROR_MAX_BYTES,
        LAUNCHER_LOG_MAX_FILES,
        LAUNCHER_LOG_MAX_AGE,
    ) {
        Ok(writer) => writer,
        Err(e) => {
            eprintln!("Could not start launcher log writer: {e}");
            return None;
        }
    };

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("theseus=info"));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false) // disable ANSI escape codes
                .with_timer(ChronoLocal::rfc_3339()),
        )
        .with(filter)
        .with(tracing_error::ErrorLayer::default())
        .init();

    Some(())
}
