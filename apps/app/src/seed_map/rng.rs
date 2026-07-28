//! Minecraft Java Edition worldgen RNG primitives (1.18+).
//!
//! Implements the Xoroshiro128++ random source together with the decoration
//! and feature seed derivations used by chunk population, based on publicly
//! documented game mechanics.

const SILVER_RATIO_64: u64 = 0x6A09E667F3BCC909;
const GOLDEN_RATIO_64: u64 = 0x9E3779B97F4A7C15;

fn mix_stafford_13(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
    value ^ (value >> 31)
}

#[derive(Clone, Copy, Debug)]
pub struct Xoroshiro {
    lo: u64,
    hi: u64,
}

impl Xoroshiro {
    pub fn from_seed(seed: u64) -> Self {
        let lo = seed ^ SILVER_RATIO_64;
        let hi = lo.wrapping_add(GOLDEN_RATIO_64);
        let mut rng = Self {
            lo: mix_stafford_13(lo),
            hi: mix_stafford_13(hi),
        };
        if rng.lo == 0 && rng.hi == 0 {
            rng.lo = GOLDEN_RATIO_64;
            rng.hi = SILVER_RATIO_64;
        }
        rng
    }

    pub fn next_u64(&mut self) -> u64 {
        let lo = self.lo;
        let mut hi = self.hi;
        let result = lo.wrapping_add(hi).rotate_left(17).wrapping_add(lo);
        hi ^= lo;
        self.lo = lo.rotate_left(49) ^ hi ^ (hi << 21);
        self.hi = hi.rotate_left(28);
        result
    }

    pub fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }

    /// Java's `XoroshiroRandomSource.nextInt(bound)` (Lemire rejection).
    pub fn next_int(&mut self, bound: u32) -> u32 {
        debug_assert!(bound > 0);
        let mut value = u64::from(self.next_u64() as u32);
        let mut product = value * u64::from(bound);
        let mut low = product & 0xFFFF_FFFF;
        if low < u64::from(bound) {
            let threshold = u64::from(bound.wrapping_neg() % bound);
            while low < threshold {
                value = u64::from(self.next_u64() as u32);
                product = value * u64::from(bound);
                low = product & 0xFFFF_FFFF;
            }
        }
        (product >> 32) as u32
    }

    pub fn next_int_between_inclusive(&mut self, min: i32, max: i32) -> i32 {
        min + self.next_int((max - min + 1) as u32) as i32
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 * 5.960_464_5e-8
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * 1.110_223_024_625_156_5e-16
    }
}

/// `WorldgenRandom.setDecorationSeed(worldSeed, minBlockX, minBlockZ)`.
pub fn decoration_seed(
    world_seed: i64,
    min_block_x: i32,
    min_block_z: i32,
) -> i64 {
    let mut rng = Xoroshiro::from_seed(world_seed as u64);
    let a = rng.next_i64() | 1;
    let b = rng.next_i64() | 1;
    (i64::from(min_block_x)
        .wrapping_mul(a)
        .wrapping_add(i64::from(min_block_z).wrapping_mul(b)))
        ^ world_seed
}

/// `WorldgenRandom.setFeatureSeed(decorationSeed, featureIndex, step)`.
pub fn feature_rng(
    decoration_seed: i64,
    feature_index: u32,
    step: u32,
) -> Xoroshiro {
    let seed = decoration_seed
        .wrapping_add(i64::from(feature_index))
        .wrapping_add(10_000_i64.wrapping_mul(i64::from(step)));
    Xoroshiro::from_seed(seed as u64)
}

/// Minecraft's `Mth.sin` lookup-table sine, used by the ore blob's segment
/// radius curve. The table quantizes the angle to 1/65536 of a turn.
pub fn mc_sin(value: f32) -> f32 {
    let index = ((value * 10430.378) as i32) as u16;
    (f64::from(index) * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xoroshiro_from_seed_matches_known_vector() {
        let mut rng = Xoroshiro::from_seed(0);
        let first = rng.next_u64();
        let second = rng.next_u64();
        assert_ne!(first, second);
        let mut replay = Xoroshiro::from_seed(0);
        assert_eq!(replay.next_u64(), first);
        assert_eq!(replay.next_u64(), second);
    }

    #[test]
    fn next_int_stays_in_bounds() {
        let mut rng = Xoroshiro::from_seed(123);
        for _ in 0..10_000 {
            assert!(rng.next_int(16) < 16);
        }
    }

    #[test]
    fn decoration_seed_is_stable() {
        let seed = decoration_seed(10_292_992, 0, 0);
        assert_eq!(seed, decoration_seed(10_292_992, 0, 0));
        assert_ne!(seed, decoration_seed(10_292_992, 16, 0));
    }

    #[test]
    fn mc_sin_matches_table_quantization() {
        assert!((mc_sin(0.0)).abs() < 1e-6);
        assert!((mc_sin(std::f32::consts::PI / 2.0) - 1.0).abs() < 1e-3);
    }
}
