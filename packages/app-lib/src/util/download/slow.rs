#![allow(dead_code)]
//! Slow-download detection.
//!
//! The detector is intentionally local and cheap. It only fires while a
//! download is stalled or wasting work, and the resulting events are
//! written to the local compact log.

use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlowRule {
    R1NoProgress,
    R2BelowExpectation,
    R3SegmentWaste,
    R4FrequentSwitches,
}

#[derive(
    Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SlowPreset {
    #[default]
    Sensitive,
    Standard,
    Relaxed,
}

impl SlowPreset {
    pub const fn no_progress_secs(self) -> u64 {
        match self {
            Self::Sensitive => 3,
            Self::Standard => 5,
            Self::Relaxed => 10,
        }
    }

    pub const fn min_speed_bps(self) -> u64 {
        match self {
            Self::Sensitive => 64 * 1024,
            Self::Standard => 32 * 1024,
            Self::Relaxed => 16 * 1024,
        }
    }

    pub const fn min_remaining_bytes(self) -> u64 {
        match self {
            Self::Sensitive => 256 * 1024,
            Self::Standard => 1024 * 1024,
            Self::Relaxed => 4 * 1024 * 1024,
        }
    }

    pub const fn max_segment_waste_ratio(self) -> f64 {
        match self {
            Self::Sensitive => 0.15,
            Self::Standard => 0.25,
            Self::Relaxed => 0.5,
        }
    }

    pub const fn max_switches_per_file(self) -> u32 {
        match self {
            Self::Sensitive => 2,
            Self::Standard => 4,
            Self::Relaxed => 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowEvent {
    pub rule: SlowRule,
    pub engine: String,
    pub source: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct SlowDetector {
    preset: SlowPreset,
    started_at: Option<Instant>,
    last_progress_at: Option<Instant>,
    last_bytes: u64,
    total: Option<u64>,
    segment_waste: u64,
    route_switches: u32,
}

impl Default for SlowDetector {
    fn default() -> Self {
        Self::new(SlowPreset::Standard)
    }
}

impl SlowDetector {
    pub fn new(preset: SlowPreset) -> Self {
        Self {
            preset,
            started_at: None,
            last_progress_at: None,
            last_bytes: 0,
            total: None,
            segment_waste: 0,
            route_switches: 0,
        }
    }

    pub fn start(&mut self, total: Option<u64>) {
        let now = Instant::now();
        self.started_at = Some(now);
        self.last_progress_at = Some(now);
        self.last_bytes = 0;
        self.total = total;
        self.segment_waste = 0;
        self.route_switches = 0;
    }

    pub fn progress(&mut self, bytes: u64, source: &str) -> Option<SlowEvent> {
        let now = Instant::now();
        let Some(started) = self.started_at else {
            return None;
        };
        let elapsed = now.duration_since(started).as_secs();
        let delta = bytes.saturating_sub(self.last_bytes);

        if delta > 0 {
            self.last_bytes = bytes;
            self.last_progress_at = Some(now);
            return None;
        }

        let no_progress_secs = now
            .duration_since(self.last_progress_at.unwrap_or(started))
            .as_secs();
        if no_progress_secs >= self.preset.no_progress_secs() {
            let remaining = self.total.map(|total| total.saturating_sub(bytes));
            if remaining.map_or(true, |value| {
                value >= self.preset.min_remaining_bytes()
            }) {
                return Some(SlowEvent {
                    rule: SlowRule::R1NoProgress,
                    engine: "xmcl".to_string(),
                    source: source.to_string(),
                    detail: format!(
                        "no_progress={no_progress_secs}s elapsed={elapsed}s"
                    ),
                });
            }
        }

        None
    }

    pub fn record_segment_waste(
        &mut self,
        wasted_bytes: u64,
        source: &str,
    ) -> Option<SlowEvent> {
        self.segment_waste = self.segment_waste.saturating_add(wasted_bytes);
        let total = self.total.unwrap_or(0);
        if total == 0 {
            return None;
        }
        let ratio = self.segment_waste as f64 / total as f64;
        if ratio >= self.preset.max_segment_waste_ratio() {
            Some(SlowEvent {
                rule: SlowRule::R3SegmentWaste,
                engine: "xmcl".to_string(),
                source: source.to_string(),
                detail: format!(
                    "wasted={} total={total} ratio={ratio:.2}",
                    self.segment_waste
                ),
            })
        } else {
            None
        }
    }

    pub fn record_route_switch(&mut self, source: &str) -> Option<SlowEvent> {
        self.route_switches += 1;
        if self.route_switches > self.preset.max_switches_per_file() {
            Some(SlowEvent {
                rule: SlowRule::R4FrequentSwitches,
                engine: "xmcl".to_string(),
                source: source.to_string(),
                detail: format!("switches={}", self.route_switches),
            })
        } else {
            None
        }
    }

    pub fn finish(&mut self) {
        self.started_at = None;
        self.last_progress_at = None;
        self.last_bytes = 0;
        self.total = None;
        self.segment_waste = 0;
        self.route_switches = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn no_progress_fires_after_preset_window() {
        let mut detector = SlowDetector::new(SlowPreset::Sensitive);
        detector.start(Some(4 * 1024 * 1024));
        detector.progress(0, "official");
        std::thread::sleep(Duration::from_millis(3100));
        let event = detector.progress(0, "official");
        assert!(matches!(
            event,
            Some(SlowEvent {
                rule: SlowRule::R1NoProgress,
                ..
            })
        ));
    }

    #[test]
    fn route_switch_fires_after_preset_limit() {
        let mut detector = SlowDetector::new(SlowPreset::Sensitive);
        detector.start(Some(1024));
        let mut fired = None;
        for _ in 0..3 {
            fired = detector.record_route_switch("bmclapi");
        }
        assert!(matches!(
            fired,
            Some(SlowEvent {
                rule: SlowRule::R4FrequentSwitches,
                ..
            })
        ));
    }
}
