//! Compact local download-event log.
//!
//! Only stall/error events are written. Each line is intentionally
//! short and numeric so the file stays tiny; `scripts/axolotl/decode-download-log.mjs`
//! can turn it back into readable text.

use super::slow::{SlowEvent, SlowRule};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

pub fn engine_code(engine: &str) -> u8 {
    match engine {
        "xmcl" => 1,
        _ => 0,
    }
}

pub fn source_code(source: &str) -> u8 {
    match source {
        "official" => 0,
        "bmclapi" => 1,
        "mcim" => 2,
        "alternate" => 3,
        "tianpao" => 5,
        _ => 4,
    }
}

pub fn encode_stall(event: &SlowEvent) -> String {
    let rule = match event.rule {
        SlowRule::R1NoProgress => 1,
        SlowRule::R2BelowExpectation => 2,
        SlowRule::R3SegmentWaste => 3,
        SlowRule::R4FrequentSwitches => 4,
    };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!(
        "{timestamp}|{}|{rule}|{}|{}",
        engine_code(&event.engine),
        source_code(&event.source),
        event.detail
    )
}

pub fn append_stall(log_path: &Path, event: &SlowEvent) -> std::io::Result<()> {
    rotate_if_needed(log_path)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{}", encode_stall(event))
}

fn rotate_if_needed(log_path: &Path) -> std::io::Result<()> {
    let Ok(metadata) = std::fs::metadata(log_path) else {
        return Ok(());
    };
    if metadata.len() < MAX_LOG_BYTES {
        return Ok(());
    }
    let rotated = log_path.with_extension("log.1");
    if rotated.exists() {
        std::fs::remove_file(&rotated)?;
    }
    std::fs::rename(log_path, rotated)?;
    Ok(())
}
