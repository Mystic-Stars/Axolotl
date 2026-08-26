use crate::event::emit::{emit_instance, emit_process};
use crate::event::{InstancePayloadType, ProcessPayloadType};
#[cfg(feature = "tauri")]
use crate::event::{LogEvent, LogPayload};
use crate::util::io::IOError;
use crate::util::rpc::RpcServer;
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use dashmap::DashMap;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::Deserialize;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::LazyLock;
use std::time::Instant;
#[cfg(feature = "tauri")]
use tauri::Emitter;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use uuid::Uuid;

const LAUNCHER_LOG_PATH: &str = "launcher_log.txt";
const LOG_BUFFER_CAPACITY: usize = 50_000;
const PROCESS_INITIALIZATION_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(15);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct CrashReportSnapshot {
    readable: bool,
    files: BTreeMap<String, (u64, Option<std::time::SystemTime>)>,
}

async fn snapshot_crash_reports(path: &Path) -> CrashReportSnapshot {
    let mut snapshot = CrashReportSnapshot {
        readable: true,
        files: BTreeMap::new(),
    };
    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return snapshot;
        }
        Err(error) => {
            tracing::warn!(
                "Failed to read crash reports directory {}: {error}",
                path.display()
            );
            snapshot.readable = false;
            return snapshot;
        }
    };
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(error) => {
                tracing::warn!(
                    "Failed to scan crash reports directory {}: {error}",
                    path.display()
                );
                snapshot.readable = false;
                break;
            }
        };
        let metadata = match entry.metadata().await {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => continue,
            Err(_) => {
                snapshot.readable = false;
                continue;
            }
        };
        snapshot.files.insert(
            entry.file_name().to_string_lossy().to_string(),
            (metadata.len(), metadata.modified().ok()),
        );
    }
    snapshot
}

fn crash_reports_changed(
    before: &CrashReportSnapshot,
    after: &CrashReportSnapshot,
) -> bool {
    !before.readable || !after.readable || before.files != after.files
}

async fn record_post_upgrade_launch_best_effort(
    instance_id: &str,
    clean: bool,
) {
    let Ok(state) = crate::State::get().await else {
        return;
    };
    if let Err(error) =
        crate::state::instances::commands::record_instance_post_upgrade_launch(
            instance_id,
            clean,
            &state.pool,
        )
        .await
    {
        tracing::warn!(
            "Failed to update post-upgrade launch state for {instance_id}: {error}"
        );
    }
}

struct LogRingBuffer {
    lines: VecDeque<String>,
}

impl LogRingBuffer {
    fn new() -> Self {
        Self {
            lines: VecDeque::new(),
        }
    }

    fn push(&mut self, line: String) {
        if self.lines.len() >= LOG_BUFFER_CAPACITY {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    fn get_all(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }

    fn clear(&mut self) {
        self.lines.clear();
    }
}

static LOG_BUFFERS: LazyLock<DashMap<String, LogRingBuffer>> =
    LazyLock::new(DashMap::new);

pub fn push_log_line(instance_id: &str, line: String) {
    LOG_BUFFERS
        .entry(instance_id.to_string())
        .or_insert_with(LogRingBuffer::new)
        .push(line);
}

pub fn get_log_buffer(instance_id: &str) -> Vec<String> {
    LOG_BUFFERS
        .get(instance_id)
        .map(|buf| buf.get_all())
        .unwrap_or_default()
}

pub fn clear_log_buffer(instance_id: &str) {
    if let Some(mut buf) = LOG_BUFFERS.get_mut(instance_id) {
        buf.clear();
    }
}

pub fn remove_log_buffer(instance_id: &str) {
    LOG_BUFFERS.remove(instance_id);
}

pub struct ProcessManager {
    processes: DashMap<Uuid, Process>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: DashMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_new_process(
        &self,
        instance_id: &str,
        instance_path: &str,
        instance_name: &str,
        mut mc_command: Command,
        post_exit_command: Option<String>,
        logs_folder: PathBuf,
        xml_logging: bool,
        main_class_keep_alive: TempDir,
        rpc_server: RpcServer,
        post_process_init: impl AsyncFnOnce(
            &ProcessMetadata,
            &RpcServer,
        ) -> crate::Result<()>,
    ) -> crate::Result<ProcessMetadata> {
        mc_command.stdout(std::process::Stdio::piped());
        mc_command.stderr(std::process::Stdio::piped());
        mc_command.stdin(std::process::Stdio::piped());

        let mut mc_proc = mc_command.spawn().map_err(IOError::from)?;

        let stdout = mc_proc.stdout.take();
        let stderr = mc_proc.stderr.take();

        let mut process = Process {
            metadata: ProcessMetadata {
                uuid: Uuid::new_v4(),
                start_time: Utc::now(),
                instance_id: instance_id.to_string(),
                instance_path: instance_path.to_string(),
                instance_name: instance_name.to_string(),
            },
            child: mc_proc,
            manually_killed: false,
            output_tasks: Vec::new(),
            rpc_server,
            _main_class_keep_alive: main_class_keep_alive,
        };

        let metadata = process.metadata.clone();

        if !logs_folder.exists() {
            tokio::fs::create_dir_all(&logs_folder)
                .await
                .map_err(|e| IOError::with_path(e, &logs_folder))?;
        }

        let log_path = logs_folder.join(LAUNCHER_LOG_PATH);

        clear_log_buffer(instance_id);

        {
            let mut log_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&log_path)
                .map_err(|e| IOError::with_path(e, &log_path))?;

            // Initialize with timestamp header
            let now = chrono::Local::now();
            writeln!(
                log_file,
                "# Minecraft launcher log started at {}",
                now.format("%Y-%m-%d %H:%M:%S")
            )
            .map_err(|e| IOError::with_path(e, &log_path))?;
            writeln!(log_file, "# Instance: {instance_path} \n")
                .map_err(|e| IOError::with_path(e, &log_path))?;
            writeln!(log_file).map_err(|e| IOError::with_path(e, &log_path))?;
        }

        if let Some(stdout) = stdout {
            let log_path_clone = log_path.clone();

            let instance_id = metadata.instance_id.clone();
            let instance_path = metadata.instance_path.clone();
            let instance_name = metadata.instance_name.clone();
            let process_id = metadata.uuid.to_string();
            process.output_tasks.push(tokio::spawn(async move {
                Process::process_output(
                    &instance_id,
                    &instance_path,
                    &instance_name,
                    &process_id,
                    stdout,
                    log_path_clone,
                    xml_logging,
                )
                .await;
            }));
        }

        if let Some(stderr) = stderr {
            let log_path_clone = log_path.clone();

            let instance_id = metadata.instance_id.clone();
            let instance_path = metadata.instance_path.clone();
            let instance_name = metadata.instance_name.clone();
            let process_id = metadata.uuid.to_string();
            process.output_tasks.push(tokio::spawn(async move {
                Process::process_output(
                    &instance_id,
                    &instance_path,
                    &instance_name,
                    &process_id,
                    stderr,
                    log_path_clone,
                    xml_logging,
                )
                .await;
            }));
        }

        let initialization_result = {
            let initialization_metadata = process.metadata.clone();
            let initialization_rpc = process.rpc_server.clone();
            let initialization = post_process_init(
                &initialization_metadata,
                &initialization_rpc,
            );
            tokio::pin!(initialization);
            tokio::select! {
                result = &mut initialization => result,
                exit_status = process.child.wait() => {
                    let exit_status = exit_status.map_err(IOError::from)?;
                    for output_task in process.output_tasks.drain(..) {
                        if let Err(error) = output_task.await {
                            tracing::warn!(%error, "Minecraft output task failed");
                        }
                    }
                    let _ = Process::append_to_log_file(
                        &log_path,
                        &format!("\n# Process exited with status: {exit_status}\n"),
                    );
                    record_post_upgrade_launch_best_effort(instance_id, false).await;
                    return Err(crate::ErrorKind::LauncherError(format!(
                        "Minecraft exited before launcher initialization completed ({exit_status}). Check the selected Java version, wrapper command, and launcher log.",
                    ))
                    .as_error());
                }
                _ = tokio::time::sleep(PROCESS_INITIALIZATION_TIMEOUT) => {
                    let _ = process.child.kill().await;
                    record_post_upgrade_launch_best_effort(instance_id, false).await;
                    return Err(crate::ErrorKind::LauncherError(
                        "Minecraft launcher initialization did not respond within 15 seconds. Check the selected Java version and wrapper command."
                            .to_string(),
                    )
                    .as_error());
                }
            }
        };
        if let Err(error) = initialization_result {
            tracing::error!("Failed to run post-process init: {error}");
            let _ = process.child.kill().await;
            record_post_upgrade_launch_best_effort(instance_id, false).await;
            return Err(error);
        }

        let crash_reports_before = match crate::State::get().await {
            Ok(state) => Some(
                snapshot_crash_reports(
                    &state.directories.crash_reports_dir(instance_path),
                )
                .await,
            ),
            Err(error) => {
                tracing::warn!(
                    "Failed to snapshot crash reports before launch: {error}"
                );
                None
            }
        };

        self.processes.insert(process.metadata.uuid, process);

        tokio::spawn(Process::sequential_process_manager(
            instance_id.to_string(),
            instance_path.to_string(),
            post_exit_command,
            metadata.uuid,
            crash_reports_before,
        ));

        emit_process(
            instance_id,
            metadata.uuid,
            ProcessPayloadType::Launched,
            "Launched Minecraft",
            None,
        )
        .await?;

        Ok(metadata)
    }

    pub fn get(&self, id: Uuid) -> Option<ProcessMetadata> {
        self.processes.get(&id).map(|x| x.metadata.clone())
    }

    pub fn get_rpc(&self, id: Uuid) -> Option<RpcServer> {
        self.processes.get(&id).map(|x| x.rpc_server.clone())
    }

    pub fn get_all(&self) -> Vec<ProcessMetadata> {
        self.processes
            .iter()
            .map(|x| x.value().metadata.clone())
            .collect()
    }

    pub fn try_wait(
        &self,
        id: Uuid,
    ) -> crate::Result<Option<Option<ExitStatus>>> {
        if let Some(mut process) = self.processes.get_mut(&id) {
            Ok(Some(process.child.try_wait()?))
        } else {
            Ok(None)
        }
    }

    pub async fn wait_for(&self, id: Uuid) -> crate::Result<()> {
        if let Some(mut process) = self.processes.get_mut(&id) {
            process.child.wait().await?;
        }
        Ok(())
    }

    pub async fn kill(&self, id: Uuid) -> crate::Result<()> {
        if let Some(mut process) = self.processes.get_mut(&id) {
            process.manually_killed = true;
            if let Err(error) = process.child.kill().await {
                process.manually_killed = false;
                return Err(error.into());
            }
        }

        Ok(())
    }

    fn remove(&self, id: Uuid) -> Option<Process> {
        self.processes.remove(&id).map(|(_, process)| process)
    }

    fn was_manually_killed(&self, id: Uuid) -> bool {
        self.processes
            .get(&id)
            .is_some_and(|process| process.manually_killed)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProcessMetadata {
    pub uuid: Uuid,
    pub instance_id: String,
    pub instance_path: String,
    pub instance_name: String,
    pub start_time: DateTime<Utc>,
}

#[derive(Debug)]
struct Process {
    metadata: ProcessMetadata,
    child: Child,
    manually_killed: bool,
    output_tasks: Vec<tokio::task::JoinHandle<()>>,
    _main_class_keep_alive: TempDir,
    rpc_server: RpcServer,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct Log4jEvent {
    pub timestamp_millis: Option<i64>,
    pub logger_name: Option<String>,
    pub level: Option<String>,
    pub thread_name: Option<String>,
    pub message: Option<String>,
    pub throwable: Option<String>,
}

impl Process {
    async fn process_output<R>(
        instance_id: &str,
        _instance_path: &str,
        instance_name: &str,
        process_id: &str,
        reader: R,
        log_path: impl AsRef<Path>,
        xml_logging: bool,
    ) where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut buf_reader = BufReader::new(reader);

        if xml_logging {
            let mut reader = Reader::from_reader(buf_reader);
            reader.config_mut().enable_all_checks(false);

            let mut buf = Vec::new();
            let mut current_event = Log4jEvent::default();
            let mut in_event = false;
            let mut in_message = false;
            let mut in_throwable = false;
            let mut current_content = String::new();

            loop {
                match reader.read_event_into_async(&mut buf).await {
                    Err(e) => {
                        tracing::error!(
                            "Error at position {}: {:?}",
                            reader.buffer_position(),
                            e
                        );
                        break;
                    }
                    // exits the loop when reaching end of file
                    Ok(Event::Eof) => break,

                    Ok(Event::Start(e)) => {
                        match e.name().as_ref() {
                            b"log4j:Event" => {
                                // Reset for new event
                                current_event = Log4jEvent::default();
                                in_event = true;

                                // Extract attributes
                                for attr in e.attributes().flatten() {
                                    let key = String::from_utf8_lossy(
                                        attr.key.into_inner(),
                                    )
                                    .to_string();
                                    let value =
                                        String::from_utf8_lossy(&attr.value)
                                            .to_string();

                                    match key.as_str() {
                                        "logger" => {
                                            current_event.logger_name =
                                                Some(value)
                                        }
                                        "level" => {
                                            current_event.level = Some(value)
                                        }
                                        "thread" => {
                                            current_event.thread_name =
                                                Some(value)
                                        }
                                        "timestamp" => {
                                            current_event.timestamp_millis =
                                                value.parse::<i64>().ok()
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            b"log4j:Message" => {
                                in_message = true;
                                current_content = String::new();
                            }
                            b"log4j:Throwable" => {
                                in_throwable = true;
                                current_content = String::new();
                            }
                            _ => {}
                        }
                    }
                    Ok(Event::End(e)) => {
                        match e.name().as_ref() {
                            b"log4j:Message" => {
                                in_message = false;
                                current_event.message =
                                    Some(current_content.clone());
                            }
                            b"log4j:Throwable" => {
                                in_throwable = false;
                                current_event.throwable =
                                    if current_content.is_empty() {
                                        None
                                    } else {
                                        Some(current_content.clone())
                                    };

                                // Write log entry + throwable to file
                                if let Some(formatted_log) =
                                    Self::format_log4j_entry(&current_event)
                                {
                                    if let Err(e) = Process::append_to_log_file(
                                        &log_path,
                                        &formatted_log,
                                    ) {
                                        tracing::error!(
                                            "Failed to write to log file: {}",
                                            e
                                        );
                                    }

                                    if let Some(ref throwable) =
                                        current_event.throwable
                                        && let Err(e) =
                                            Process::append_to_log_file(
                                                &log_path, throwable,
                                            )
                                    {
                                        tracing::error!(
                                            "Failed to write throwable to log file: {}",
                                            e
                                        );
                                    }
                                }

                                Self::emit_log4j_event(
                                    instance_id,
                                    &current_event,
                                );
                            }
                            b"log4j:Event" => {
                                in_event = false;
                                // If no throwable was present, write the log entry at the end of the event
                                if current_event.message.is_some()
                                    && current_event.throwable.is_none()
                                {
                                    if let Some(formatted_log) =
                                        Self::format_log4j_entry(&current_event)
                                        && let Err(e) =
                                            Process::append_to_log_file(
                                                &log_path,
                                                &formatted_log,
                                            )
                                    {
                                        tracing::error!(
                                            "Failed to write to log file: {}",
                                            e
                                        );
                                    }

                                    if let Some(timestamp_millis) =
                                        current_event.timestamp_millis
                                    {
                                        let timestamp =
                                            timestamp_millis.to_string();
                                        let message = current_event
                                            .message
                                            .as_deref()
                                            .unwrap_or("")
                                            .trim();
                                        crate::api::multiplayer::observe_minecraft_log(
                                        instance_id,
                                        instance_name,
                                        process_id,
                                        message,
                                    )
                                    .await;
                                        if let Err(e) = Self::maybe_handle_server_join_logging(
											instance_id,
											&timestamp,
											message,
                                        ).await {
                                            tracing::error!("Failed to handle server join logging: {e}");
                                        }
                                    }

                                    Self::emit_log4j_event(
                                        instance_id,
                                        &current_event,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Event::Text(mut e)) => {
                        if in_message || in_throwable {
                            if let Ok(text) = e.xml_content() {
                                current_content.push_str(&text);
                            }
                        } else if !in_event
                            && !e.inplace_trim_end()
                            && !e.inplace_trim_start()
                            && let Ok(text) = e.xml_content()
                        {
                            if let Err(e) = Process::append_to_log_file(
                                &log_path,
                                &format!("{text}\n"),
                            ) {
                                tracing::error!(
                                    "Failed to write to log file: {}",
                                    e
                                );
                            }
                            Self::emit_legacy_log(instance_id, &text);
                        }
                    }
                    Ok(Event::CData(e)) => {
                        if (in_message || in_throwable)
                            && let Ok(text) = e.xml_content()
                        {
                            current_content.push_str(&text);
                        }
                    }
                    _ => (),
                }

                buf.clear();
            }
        } else {
            let mut line = String::new();

            while let Ok(bytes_read) = buf_reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break; // End of stream
                }

                if !line.is_empty() {
                    if let Err(e) = Self::append_to_log_file(&log_path, &line) {
                        tracing::warn!("Failed to write to log file: {}", e);
                    }
                    Self::emit_legacy_log(instance_id, line.trim_ascii_end());
                    crate::api::multiplayer::observe_minecraft_log(
                        instance_id,
                        instance_name,
                        process_id,
                        line.trim_ascii_end(),
                    )
                    .await;
                    if let Err(e) = Self::maybe_handle_old_server_join_logging(
                        instance_id,
                        line.trim_ascii_end(),
                    )
                    .await
                    {
                        tracing::error!(
                            "Failed to handle old server join logging: {e}"
                        );
                    }
                }

                line.clear();
            }
        }
    }

    fn format_timestamp(timestamp_millis: Option<i64>) -> String {
        if let Some(timestamp_val) = timestamp_millis {
            let datetime_utc = if timestamp_val > i32::MAX as i64 {
                let secs = timestamp_val / 1000;
                let nsecs = ((timestamp_val % 1000) * 1_000_000) as u32;

                chrono::DateTime::<Utc>::from_timestamp(secs, nsecs)
                    .unwrap_or_default()
            } else {
                chrono::DateTime::<Utc>::from_timestamp_secs(timestamp_val)
                    .unwrap_or_default()
            };

            let datetime_local = datetime_utc.with_timezone(&chrono::Local);
            format!("[{}]", datetime_local.format("%H:%M:%S"))
        } else {
            "[??:??:??]".to_string()
        }
    }

    fn format_log4j_entry(event: &Log4jEvent) -> Option<String> {
        let message = event.message.as_ref()?;
        let thread = event.thread_name.as_deref().unwrap_or("");
        let level = event.level.as_deref().unwrap_or("");
        let logger = event.logger_name.as_deref().unwrap_or("");
        let formatted_time = Self::format_timestamp(event.timestamp_millis);

        Some(format!(
            "{} [{}] [{}{}]: {}\n",
            formatted_time,
            thread,
            if !logger.is_empty() {
                format!("{logger}/")
            } else {
                String::new()
            },
            level,
            message.trim()
        ))
    }

    fn emit_log4j_event(instance_id: &str, event: &Log4jEvent) {
        if let Some(formatted) = Self::format_log4j_entry(event) {
            push_log_line(instance_id, formatted.trim_end().to_string());
        }
        if let Some(ref throwable) = event.throwable {
            for line in throwable.lines().filter(|l| !l.is_empty()) {
                push_log_line(instance_id, line.to_string());
            }
        }

        #[cfg(feature = "tauri")]
        {
            if let Ok(event_state) = crate::EventState::get() {
                let _ = event_state.app.emit(
                    "log",
                    LogPayload {
                        instance_id: instance_id.to_string(),
                        event: LogEvent::Log4j(event.clone()),
                    },
                );
            }
        }
        #[cfg(not(feature = "tauri"))]
        {
            let _ = (instance_id, event);
        }
    }

    fn emit_legacy_log(instance_id: &str, message: &str) {
        push_log_line(instance_id, message.to_string());

        #[cfg(feature = "tauri")]
        {
            if let Ok(event_state) = crate::EventState::get() {
                let _ = event_state.app.emit(
                    "log",
                    LogPayload {
                        instance_id: instance_id.to_string(),
                        event: LogEvent::Legacy {
                            message: message.to_string(),
                        },
                    },
                );
            }
        }
        #[cfg(not(feature = "tauri"))]
        {
            let _ = (instance_id, message);
        }
    }

    fn append_to_log_file(
        path: impl AsRef<Path>,
        line: &str,
    ) -> std::io::Result<()> {
        let mut file =
            OpenOptions::new().append(true).create(true).open(path)?;

        file.write_all(line.as_bytes())?;
        Ok(())
    }

    async fn maybe_handle_server_join_logging(
        instance_id: &str,
        timestamp: &str,
        message: &str,
    ) -> crate::Result<()> {
        let timestamp = timestamp
            .parse::<i64>()
            .map(|x| x / 1000)
            .map_err(|x| {
                crate::ErrorKind::OtherError(format!(
                    "Failed to parse timestamp: {x}"
                ))
            })
            .and_then(|x| {
                Utc.timestamp_opt(x, 0).single().ok_or_else(|| {
                    crate::ErrorKind::OtherError(
                        "Failed to convert timestamp to DateTime".to_string(),
                    )
                })
            })?;
        Self::parse_and_insert_server_join(instance_id, message, timestamp)
            .await
    }

    async fn maybe_handle_old_server_join_logging(
        instance_id: &str,
        line: &str,
    ) -> crate::Result<()> {
        if let Some((timestamp, message)) = line.split_once(" [CLIENT] [INFO] ")
        {
            let timestamp =
                NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S")?
                    .and_local_timezone(chrono::Local)
                    .map(|x| x.to_utc())
                    .single()
                    .unwrap_or_else(Utc::now);
            Self::parse_and_insert_server_join(instance_id, message, timestamp)
                .await
        } else {
            Self::parse_and_insert_server_join(instance_id, line, Utc::now())
                .await
        }
    }

    async fn parse_and_insert_server_join(
        instance_id: &str,
        message: &str,
        timestamp: DateTime<Utc>,
    ) -> crate::Result<()> {
        let Some(host_port_string) = message.strip_prefix("Connecting to ")
        else {
            return Ok(());
        };
        let Some((host, port_string)) = host_port_string.rsplit_once(", ")
        else {
            return Ok(());
        };
        let Some(port) = port_string.parse::<u16>().ok() else {
            return Ok(());
        };

        let state = crate::State::get().await?;
        crate::state::server_join_log::JoinLogEntry {
            instance_id: instance_id.to_owned(),
            host: host.to_string(),
            port,
            join_time: timestamp,
        }
        .upsert(&state.pool)
        .await?;
        {
            let instance_id = instance_id.to_owned();
            let host = host.to_owned();
            tokio::spawn(async move {
                let _ = emit_instance(
                    &instance_id,
                    InstancePayloadType::ServerJoined {
                        host,
                        port,
                        timestamp,
                    },
                )
                .await;
            });
        }

        Ok(())
    }

    // Spawns a new child process and inserts it into the hashmap
    // Also, as the process ends, it spawns the follow-up process if it exists
    // By convention, ExitStatus is last command's exit status, and we exit on the first non-zero exit status
    async fn sequential_process_manager(
        instance_id: String,
        instance_path: String,
        post_exit_command: Option<String>,
        uuid: Uuid,
        crash_reports_before: Option<CrashReportSnapshot>,
    ) -> crate::Result<()> {
        async fn update_playtime(
            last_updated_playtime: &mut Instant,
            instance_id: &str,
            force_update: bool,
        ) {
            let elapsed = last_updated_playtime.elapsed().as_secs();
            if elapsed == 0 || (!force_update && elapsed < 60) {
                return;
            }

            let state = match crate::State::get().await {
                Ok(state) => state,
                Err(e) => {
                    tracing::warn!(
                        "Failed to get state for playtime update on instance {}: {}",
                        instance_id,
                        e
                    );
                    return;
                }
            };
            let ended_at = Utc::now();
            let elapsed_duration =
                Duration::seconds(elapsed.min(i64::MAX as u64) as i64);
            let started_at = ended_at - elapsed_duration;
            if let Err(e) =
                crate::state::instances::commands::add_instance_recent_playtime(
                    instance_id,
                    elapsed,
                    &state.pool,
                )
                .await
            {
                tracing::warn!(
                    "Failed to update playtime for instance {}: {}",
                    instance_id,
                    e
                );
            }
            if let Err(e) =
				crate::state::instances::commands::record_instance_daily_playtime(
					instance_id,
					started_at,
					ended_at,
					&state.pool,
				)
				.await
			{
				tracing::warn!(
					"Failed to record daily playtime for instance {}: {}",
					instance_id,
					e
				);
			}
            *last_updated_playtime = Instant::now();
        }

        // Wait on current Minecraft Child
        let mc_exit_status;
        let mut process_missing = false;
        let mut last_updated_playtime = Instant::now();

        let state = crate::State::get().await?;
        if let Err(e) =
            crate::state::instances::commands::record_instance_play_session(
                &instance_id,
                &state.pool,
            )
            .await
        {
            tracing::warn!(
                "Failed to record play session for instance {}: {}",
                instance_id,
                e
            );
        }
        loop {
            if let Some(process) = state.process_manager.try_wait(uuid)? {
                if let Some(t) = process {
                    mc_exit_status = t;
                    break;
                }
            } else {
                mc_exit_status = ExitStatus::default();
                process_missing = true;
                break;
            }

            // sleep for 10ms
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Auto-update playtime every minute
            update_playtime(&mut last_updated_playtime, &instance_id, false)
                .await;
        }

        let manually_killed = state.process_manager.was_manually_killed(uuid);
        if let Some(mut process) = state.process_manager.remove(uuid) {
            for output_task in process.output_tasks.drain(..) {
                if let Err(error) = output_task.await {
                    tracing::warn!(%error, "Minecraft output task failed");
                }
            }
        }
        crate::api::multiplayer::minecraft_process_finished(&instance_id).await;

        // Now fully complete- update playtime one last time
        update_playtime(&mut last_updated_playtime, &instance_id, true).await;

        let crash_reports_after = snapshot_crash_reports(
            &state.directories.crash_reports_dir(&instance_path),
        )
        .await;
        let clean_launch = mc_exit_status.success()
            && !manually_killed
            && !process_missing
            && crash_reports_before.as_ref().is_some_and(|before| {
                !crash_reports_changed(before, &crash_reports_after)
            });
        record_post_upgrade_launch_best_effort(&instance_id, clean_launch)
            .await;

        // Publish play time update
        // Allow failure, it will be stored locally and sent next time
        // Sent in another thread as first call may take a couple seconds and hold up process ending
        let playtime_instance_id = instance_id.clone();
        tokio::spawn(async move {
            if let Err(e) =
                crate::api::instance::try_update_playtime_by_instance_id(
                    &playtime_instance_id,
                )
                .await
            {
                tracing::warn!(
                    "Failed to update playtime for instance {}: {}",
                    playtime_instance_id,
                    e
                );
            }
        });

        let logs_folder = state.directories.instance_logs_dir(&instance_path);
        let log_path = logs_folder.join(LAUNCHER_LOG_PATH);

        if log_path.exists()
            && let Err(e) = Process::append_to_log_file(
                &log_path,
                &format!("\n# Process exited with status: {mc_exit_status}\n"),
            )
        {
            tracing::warn!("Failed to write exit status to log file: {}", e);
        }

        if mc_exit_status.success()
            && !manually_killed
            && let Err(error) =
                crate::api::logs::save_successful_mod_snapshot(&instance_id)
                    .await
        {
            tracing::warn!(
                %error,
                instance = %instance_id,
                "Failed to save successful launch Mod snapshot"
            );
        }

        emit_process(
            &instance_id,
            uuid,
            ProcessPayloadType::Finished,
            "Exited process",
            Some(!mc_exit_status.success() && !manually_killed),
        )
        .await?;

        let _ = state.discord_rpc.clear_to_default(true).await;

        if mc_exit_status.success() {
            // We do not wait on the post exist command to finish running! We let it spawn + run on its own.
            // This behaviour may be changed in the future
            if let Some(hook) = post_exit_command {
                let mut cmd = shlex::split(&hook)
                    .ok_or_else(|| {
                        crate::ErrorKind::LauncherError(format!(
                            "Invalid post-exit command: {hook}",
                        ))
                    })?
                    .into_iter();

                if let Some(command) = cmd.next() {
                    // The post-exit hook runs in the instance's game working
                    // directory, which honours a per-instance override.
                    let game_dir = crate::state::instances::adapters::sqlite::instance_rows::get_instance_path_and_game_dir_override_by_id(
                        &instance_id,
                        &state.pool,
                    )
                    .await?
                    .map(|(path, override_dir)| {
                        state
                            .directories
                            .resolve_game_dir(&path, override_dir.as_deref())
                    })
                    .unwrap_or_else(|| {
                        state.directories.instances_dir().join(&instance_path)
                    });
                    let mut command = Command::new(command);
                    command.args(cmd).current_dir(game_dir);
                    command.spawn().map_err(IOError::from)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod post_upgrade_tests {
    use super::*;

    #[test]
    fn new_or_modified_crash_report_marks_session_changed() {
        let before = CrashReportSnapshot {
            readable: true,
            files: BTreeMap::from([("old.txt".to_string(), (10, None))]),
        };
        let unchanged = before.clone();
        let mut added = before.clone();
        added.files.insert("new.txt".to_string(), (20, None));
        let mut modified = before.clone();
        modified.files.insert("old.txt".to_string(), (11, None));

        assert!(!crash_reports_changed(&before, &unchanged));
        assert!(crash_reports_changed(&before, &added));
        assert!(crash_reports_changed(&before, &modified));
    }
}
