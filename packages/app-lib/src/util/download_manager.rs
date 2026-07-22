use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const INITIAL_SPEED_FLOOR: u64 = 256 * 1024;
const SAMPLE_INTERVAL_MS: u128 = 100;
const SAMPLE_HISTORY: usize = 30;

struct SpeedState {
    last_sample_at: Instant,
    last_sample_bytes: u64,
    history: VecDeque<u64>,
    current_speed: u64,
    speed_floor: u64,
}

impl Default for SpeedState {
    fn default() -> Self {
        Self {
            last_sample_at: Instant::now(),
            last_sample_bytes: 0,
            history: VecDeque::new(),
            current_speed: 0,
            speed_floor: INITIAL_SPEED_FLOOR,
        }
    }
}

pub struct DownloadManager {
    completed_bytes: AtomicU64,
    speed: Mutex<SpeedState>,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self {
            completed_bytes: AtomicU64::new(0),
            speed: Mutex::new(SpeedState::default()),
        }
    }
}

impl DownloadManager {
    pub fn record_bytes(&self, bytes: u64) {
        self.completed_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn speed_snapshot(&self) -> (u64, u64) {
        let now = Instant::now();
        let total = self.completed_bytes.load(Ordering::Relaxed);
        let mut state = self.speed.lock();
        let elapsed = now.duration_since(state.last_sample_at);
        if elapsed.as_millis() >= SAMPLE_INTERVAL_MS {
            let downloaded = total.saturating_sub(state.last_sample_bytes);
            let instantaneous = (downloaded as f64
                / elapsed.as_secs_f64().max(f64::MIN_POSITIVE))
                as u64;
            state.history.push_front(instantaneous);
            if state.history.len() > SAMPLE_HISTORY {
                state.history.pop_back();
            }
            state.last_sample_at = now;
            state.last_sample_bytes = total;

            let mut weight = state.history.len() as u64;
            let mut weighted_total = 0_u64;
            let mut weight_total = 0_u64;
            for sample in &state.history {
                weighted_total = weighted_total
                    .saturating_add(sample.saturating_mul(weight));
                weight_total += weight;
                weight = weight.saturating_sub(1);
            }
            state.current_speed = weighted_total / weight_total.max(1);

            if state.history.len() >= 10 {
                let recent_average =
                    state.history.iter().take(10).sum::<u64>() / 10;
                let next_floor = recent_average.saturating_mul(85) / 100;
                state.speed_floor = state.speed_floor.max(next_floor);
            }
        }
        (state.current_speed, state.speed_floor)
    }
}
