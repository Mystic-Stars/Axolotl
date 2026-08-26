//! Slow-transfer and optimal-stop policy for the native download engine.

use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(3);
const MIN_FLOW: Duration = Duration::from_secs(10);
const REQUIRED_SLOW_WINDOWS: u8 = 2;
const MIN_REMAINING_BYTES: u64 = 1024 * 1024;
const ABSOLUTE_SPEED_FLOOR: u64 = 16 * 1024;
const COLD_SPEED_FLOOR: u64 = 256 * 1024;
const RECONNECT_OVERHEAD: Duration = Duration::from_millis(600);
const MIN_SAVINGS: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SlowDecision {
    Continue,
    Probe { bytes_per_second: u64 },
    Commit,
}

pub(crate) struct NativeSlowPolicy {
    started_at: Instant,
    window_started_at: Instant,
    window_start_bytes: u64,
    slow_windows: u8,
    committed: bool,
    expected_speed: Option<u64>,
}

impl NativeSlowPolicy {
    pub(crate) fn new(
        starting_bytes: u64,
        expected_speed: Option<u64>,
    ) -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            window_started_at: now,
            window_start_bytes: starting_bytes,
            slow_windows: 0,
            committed: false,
            expected_speed,
        }
    }

    pub(crate) fn observe(
        &mut self,
        downloaded: u64,
        remaining: u64,
    ) -> SlowDecision {
        if self.committed {
            return SlowDecision::Continue;
        }
        let elapsed = self.window_started_at.elapsed();
        if elapsed < WINDOW {
            return SlowDecision::Continue;
        }
        let speed = downloaded
            .saturating_sub(self.window_start_bytes)
            .checked_div(elapsed.as_secs().max(1))
            .unwrap_or(0);
        self.window_started_at = Instant::now();
        self.window_start_bytes = downloaded;

        if remaining < MIN_REMAINING_BYTES
            || estimated_duration(remaining, speed)
                <= RECONNECT_OVERHEAD + MIN_SAVINGS
        {
            self.committed = true;
            return SlowDecision::Commit;
        }
        let expected_floor = self
            .expected_speed
            .map(|expected| expected.saturating_mul(40) / 100)
            .unwrap_or(COLD_SPEED_FLOOR)
            .max(ABSOLUTE_SPEED_FLOOR);
        if speed < expected_floor {
            self.slow_windows = self.slow_windows.saturating_add(1);
        } else {
            self.slow_windows = 0;
        }
        if self.started_at.elapsed() >= MIN_FLOW
            && self.slow_windows >= REQUIRED_SLOW_WINDOWS
        {
            self.slow_windows = 0;
            SlowDecision::Probe {
                bytes_per_second: speed,
            }
        } else {
            SlowDecision::Continue
        }
    }

    pub(crate) fn commit(&mut self) {
        self.committed = true;
    }
}

pub(crate) fn should_switch(
    current_speed: u64,
    candidate_speed: u64,
    remaining_bytes: u64,
    restart_bytes: u64,
) -> bool {
    if current_speed == 0 || candidate_speed == 0 {
        return false;
    }
    if candidate_speed.saturating_mul(100) < current_speed.saturating_mul(125) {
        return false;
    }
    let stay = estimated_duration(remaining_bytes, current_speed);
    let switch =
        RECONNECT_OVERHEAD + estimated_duration(restart_bytes, candidate_speed);
    let required_savings = MIN_SAVINGS.max(stay.mul_f64(0.15));
    switch.saturating_add(required_savings) < stay
}

fn estimated_duration(bytes: u64, speed: u64) -> Duration {
    if speed == 0 {
        return Duration::MAX;
    }
    Duration::from_secs_f64(bytes as f64 / speed as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_requires_repayment_of_restarted_bytes() {
        assert!(!should_switch(1024, 2048, 1024, 1024 * 1024));
    }

    #[test]
    fn faster_candidate_wins_early_in_a_large_download() {
        assert!(should_switch(
            256 * 1024,
            2 * 1024 * 1024,
            64 * 1024 * 1024,
            64 * 1024 * 1024,
        ));
    }
}
