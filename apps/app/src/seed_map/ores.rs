//! Seed-based ore prediction for Java 1.18+.
//!
//! Predicts scattered-ore placement attempts from the world seed by
//! replaying Minecraft's chunk population RNG (decoration seed, feature seed,
//! placement modifiers, and the ore blob's own RNG consumption). Terrain is
//! not simulated, so results are estimates: caves and surface exposure can
//! shift or remove blobs. Attempts are cross-checked against cubiomes'
//! approximate surface height to grade confidence.

use std::io;

use serde::{Deserialize, Serialize};

use super::rng::{Xoroshiro, decoration_seed, feature_rng, mc_sin};
use super::{Dimension, parse_seed, resolve_java_version};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OreKind {
    Diamond,
    Iron,
    IronVein,
    Copper,
    CopperVein,
    Gold,
    Redstone,
    Lapis,
    Coal,
    Netherite,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OreScanRequest {
    pub seed: String,
    pub version: String,
    pub dimension: Dimension,
    pub ores: Vec<OreKind>,
    /// Interleaved chunk coordinates: `[cx0, cz0, cx1, cz1, ...]`.
    pub chunks: Vec<i32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OreHit {
    pub ore: OreKind,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub verified: bool,
    pub y_min: i32,
    pub y_max: i32,
    pub precision: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OreChunkResult {
    pub cx: i32,
    pub cz: i32,
    pub hits: Vec<OreHit>,
}

#[derive(Clone, Copy, Debug)]
enum CountModifier {
    Fixed(u32),
    Uniform { min: u32, max: u32 },
    Rarity(u32),
}

#[derive(Clone, Copy, Debug)]
enum HeightProvider {
    Uniform { min: i32, max: i32 },
    Trapezoid { min: i32, max: i32, plateau: i32 },
}

impl HeightProvider {
    fn sample(self, rng: &mut Xoroshiro) -> i32 {
        match self {
            Self::Uniform { min, max } => {
                rng.next_int_between_inclusive(min, max)
            }
            Self::Trapezoid { min, max, plateau } => {
                let span = max - min;
                if plateau >= span {
                    return rng.next_int_between_inclusive(min, max);
                }
                let lower = (span - plateau) / 2;
                let upper = span - lower;
                min + rng.next_int_between_inclusive(0, upper)
                    + rng.next_int_between_inclusive(0, lower)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PlacedOre {
    feature_index: u32,
    step: u32,
    count: CountModifier,
    height: HeightProvider,
    blob_size: u32,
    discard_on_air: f32,
    precision: u8,
    scattered: bool,
}

const UNDERGROUND_ORES_STEP: u32 = 6;
const UNDERGROUND_DECORATION_STEP: u32 = 7;

/*
 * Global feature indices for the UNDERGROUND_ORES step, computed with
 * Minecraft's FeatureSorter over the vanilla biome sets (validated against
 * official generated data for 1.18.2 through 1.21.3): the base sequence
 * starts at ore_coal_upper = 9 after the nine stone-patch features, and
 * ore_copper sorts AFTER dripstone's ore_copper_large. `ore_diamond_medium`
 * joined the base list in 1.20.5, shifting every later entry by one.
 * Placement counts, height providers, blob sizes, and discard chances come
 * from the vanilla OrePlacements/OreFeatures definitions.
 */
fn overworld_ore_features(kind: OreKind, mc_code: i32) -> Vec<PlacedOre> {
    let has_medium_diamond = mc_code >= 12000;
    let shift = u32::from(has_medium_diamond);
    let ore = |feature_index: u32,
               count: CountModifier,
               height: HeightProvider,
               blob_size: u32,
               discard_on_air: f32,
               precision: u8| PlacedOre {
        feature_index,
        step: UNDERGROUND_ORES_STEP,
        count,
        height,
        blob_size,
        discard_on_air,
        precision,
        scattered: false,
    };
    match kind {
        OreKind::Coal => vec![
            ore(
                9,
                CountModifier::Fixed(30),
                HeightProvider::Uniform { min: 136, max: 319 },
                17,
                0.0,
                88,
            ),
            ore(
                10,
                CountModifier::Fixed(20),
                HeightProvider::Trapezoid {
                    min: 0,
                    max: 192,
                    plateau: 0,
                },
                17,
                0.5,
                84,
            ),
        ],
        OreKind::Iron => vec![
            ore(
                11,
                CountModifier::Fixed(90),
                HeightProvider::Trapezoid {
                    min: 80,
                    max: 384,
                    plateau: 0,
                },
                9,
                0.0,
                88,
            ),
            ore(
                12,
                CountModifier::Fixed(10),
                HeightProvider::Trapezoid {
                    min: -24,
                    max: 56,
                    plateau: 0,
                },
                9,
                0.0,
                90,
            ),
            ore(
                13,
                CountModifier::Fixed(10),
                HeightProvider::Uniform { min: -64, max: 72 },
                4,
                0.0,
                90,
            ),
        ],
        OreKind::Gold => vec![
            ore(
                14,
                CountModifier::Fixed(4),
                HeightProvider::Trapezoid {
                    min: -64,
                    max: 32,
                    plateau: 0,
                },
                9,
                0.5,
                87,
            ),
            ore(
                15,
                CountModifier::Uniform { min: 0, max: 1 },
                HeightProvider::Uniform { min: -64, max: -48 },
                9,
                0.5,
                87,
            ),
        ],
        OreKind::Redstone => vec![
            ore(
                16,
                CountModifier::Fixed(4),
                HeightProvider::Uniform { min: -64, max: 15 },
                8,
                0.0,
                90,
            ),
            ore(
                17,
                CountModifier::Fixed(8),
                HeightProvider::Trapezoid {
                    min: -96,
                    max: -32,
                    plateau: 0,
                },
                8,
                0.0,
                90,
            ),
        ],
        OreKind::Diamond => {
            let mut features = vec![
                ore(
                    18,
                    CountModifier::Fixed(7),
                    HeightProvider::Trapezoid {
                        min: -144,
                        max: 16,
                        plateau: 0,
                    },
                    4,
                    0.5,
                    88,
                ),
                ore(
                    19 + shift,
                    CountModifier::Rarity(9),
                    HeightProvider::Trapezoid {
                        min: -144,
                        max: 16,
                        plateau: 0,
                    },
                    12,
                    0.7,
                    82,
                ),
                ore(
                    20 + shift,
                    CountModifier::Fixed(4),
                    HeightProvider::Trapezoid {
                        min: -144,
                        max: 16,
                        plateau: 0,
                    },
                    8,
                    1.0,
                    92,
                ),
            ];
            if has_medium_diamond {
                features.push(ore(
                    19,
                    CountModifier::Fixed(2),
                    HeightProvider::Uniform { min: -64, max: -4 },
                    8,
                    0.5,
                    88,
                ));
            }
            features
        }
        OreKind::Lapis => vec![
            ore(
                21 + shift,
                CountModifier::Fixed(2),
                HeightProvider::Trapezoid {
                    min: -32,
                    max: 32,
                    plateau: 0,
                },
                7,
                0.0,
                88,
            ),
            ore(
                22 + shift,
                CountModifier::Fixed(4),
                HeightProvider::Uniform { min: -64, max: 64 },
                7,
                1.0,
                92,
            ),
        ],
        OreKind::Copper => vec![ore(
            24 + shift,
            CountModifier::Fixed(16),
            HeightProvider::Trapezoid {
                min: -16,
                max: 112,
                plateau: 0,
            },
            10,
            0.0,
            88,
        )],
        OreKind::IronVein | OreKind::CopperVein | OreKind::Netherite => vec![],
    }
}

/*
 * Large veins come from the Overworld vein noises rather than the population
 * RNG, so they are scanned natively per chunk. Copper veins live at Y 0..50
 * and iron veins at Y -60..-8.
 */
fn scan_vein_hits(
    kind: OreKind,
    world_seed: u64,
    cx: i32,
    cz: i32,
    surface: Option<&SurfaceGrid>,
) -> Vec<OreHit> {
    let vein_code = if kind == OreKind::CopperVein { 0 } else { 1 };
    let mut buffer = [0_i32; 24];
    let count = unsafe {
        super::axolotl_seed_map_scan_vein(
            world_seed,
            cx,
            cz,
            vein_code,
            buffer.as_mut_ptr(),
            buffer.len() / 3,
        )
    };
    buffer[..count * 3]
        .chunks_exact(3)
        .map(|triple| {
            let (x, y, z) = (triple[0], triple[1], triple[2]);
            let verified = surface
                .map(|grid| (y as f32) < grid.height_at(x, z) - 6.0)
                .unwrap_or(false);
            OreHit {
                ore: kind,
                x,
                y,
                z,
                verified,
                y_min: y,
                y_max: y,
                precision: if verified { 93 } else { 78 },
            }
        })
        .collect()
}

/*
 * Ancient debris indices in the UNDERGROUND_DECORATION step are 21 and 22
 * across every supported version (stable under all nether biome orderings).
 * Both are single-attempt scattered-ore features that are always buried.
 */
fn nether_ore_features(kind: OreKind, _mc_code: i32) -> Vec<PlacedOre> {
    if kind != OreKind::Netherite {
        return vec![];
    }
    let ore = |feature_index: u32, height: HeightProvider, blob_size: u32| {
        PlacedOre {
            feature_index,
            step: UNDERGROUND_DECORATION_STEP,
            count: CountModifier::Fixed(1),
            height,
            blob_size,
            discard_on_air: 1.0,
            precision: 95,
            scattered: true,
        }
    };
    vec![
        ore(
            21,
            HeightProvider::Trapezoid {
                min: 8,
                max: 24,
                plateau: 0,
            },
            3,
        ),
        ore(22, HeightProvider::Uniform { min: 8, max: 119 }, 2),
    ]
}

pub fn supports_ores(version_code: i32, dimension: Dimension) -> bool {
    version_code >= 11800 && !matches!(dimension, Dimension::End)
}

pub fn scan_ores(request: OreScanRequest) -> io::Result<Vec<OreChunkResult>> {
    let (mc_version, version_code) = resolve_java_version(&request.version)?;
    if !supports_ores(version_code, request.dimension) {
        return Err(io::Error::other(
            "Ore prediction supports Java 1.18+ Overworld and Nether maps.",
        ));
    }
    if request.chunks.len() % 2 != 0 || request.chunks.len() > 4_096 {
        return Err(io::Error::other("The ore scan chunk list is invalid."));
    }
    let world_seed = parse_seed(&request.seed) as i64;
    let chunk_pairs: Vec<(i32, i32)> = request
        .chunks
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[1]))
        .collect();
    let surface = match request.dimension {
        Dimension::Overworld => {
            SurfaceGrid::sample(world_seed as u64, mc_version, &chunk_pairs)
        }
        _ => None,
    };
    let (world_bottom, world_top) = match request.dimension {
        Dimension::Overworld => (-64, 320),
        _ => (0, 128),
    };

    let requested_kinds = dedupe_kinds(&request.ores);
    let vein_kinds: Vec<OreKind> =
        if matches!(request.dimension, Dimension::Overworld) {
            requested_kinds
                .iter()
                .copied()
                .filter(|kind| {
                    matches!(kind, OreKind::IronVein | OreKind::CopperVein)
                })
                .collect()
        } else {
            Vec::new()
        };
    let mut features: Vec<(OreKind, PlacedOre)> = Vec::new();
    for kind in requested_kinds {
        let kind_features = match request.dimension {
            Dimension::Overworld => overworld_ore_features(kind, version_code),
            Dimension::Nether => nether_ore_features(kind, version_code),
            Dimension::End => vec![],
        };
        features
            .extend(kind_features.into_iter().map(|feature| (kind, feature)));
    }

    let mut results = Vec::with_capacity(chunk_pairs.len());
    for (cx, cz) in chunk_pairs {
        let chunk_decoration_seed =
            decoration_seed(world_seed, cx * 16, cz * 16);
        let mut hits = Vec::new();
        for &kind in &vein_kinds {
            hits.extend(scan_vein_hits(
                kind,
                world_seed as u64,
                cx,
                cz,
                surface.as_ref(),
            ));
        }
        for &(kind, feature) in &features {
            let mut rng = feature_rng(
                chunk_decoration_seed,
                feature.feature_index,
                feature.step,
            );
            let attempts = match feature.count {
                CountModifier::Fixed(count) => count,
                CountModifier::Uniform { min, max } => {
                    min + rng.next_int(max - min + 1)
                }
                CountModifier::Rarity(chance) => {
                    if rng.next_f32() < 1.0 / chance as f32 {
                        1
                    } else {
                        0
                    }
                }
            };
            for _ in 0..attempts {
                let x = cx * 16 + rng.next_int(16) as i32;
                let z = cz * 16 + rng.next_int(16) as i32;
                let y = feature.height.sample(&mut rng);
                let surface_height =
                    surface.as_ref().map(|grid| grid.height_at(x, z));
                let blob = if feature.scattered {
                    Some((y, y))
                } else {
                    simulate_blob(
                        &mut rng,
                        x,
                        y,
                        z,
                        feature.blob_size,
                        feature.discard_on_air,
                        surface_height,
                        world_bottom,
                        world_top,
                    )
                };
                let Some((blob_min_y, blob_max_y)) = blob else {
                    continue;
                };
                let verified = match surface_height {
                    Some(surface_y) => (blob_max_y as f32) < surface_y - 6.0,
                    None => feature.discard_on_air >= 1.0,
                };
                let precision = if verified {
                    feature.precision
                } else {
                    feature.precision.saturating_sub(18).max(50)
                };
                hits.push(OreHit {
                    ore: kind,
                    x,
                    y,
                    z,
                    verified,
                    y_min: blob_min_y,
                    y_max: blob_max_y,
                    precision,
                });
            }
        }
        results.push(OreChunkResult { cx, cz, hits });
    }
    Ok(results)
}

fn dedupe_kinds(kinds: &[OreKind]) -> Vec<OreKind> {
    let mut seen = Vec::new();
    for &kind in kinds {
        if !seen.contains(&kind) {
            seen.push(kind);
        }
    }
    seen
}

/*
 * Replays the RNG consumption of `OreFeature.place`, assuming solid terrain:
 * the blob angle, the two Y jitters, one radius roll per segment, and — for
 * partially air-sensitive ores — one roll per candidate cell. Returns the
 * simulated blob's Y extent, or None when the surface pre-check would skip
 * placement entirely.
 */
#[allow(clippy::too_many_arguments)]
fn simulate_blob(
    rng: &mut Xoroshiro,
    x: i32,
    y: i32,
    z: i32,
    size: u32,
    discard_on_air: f32,
    surface_height: Option<f32>,
    world_bottom: i32,
    world_top: i32,
) -> Option<(i32, i32)> {
    let angle = rng.next_f32() * std::f32::consts::PI;
    let spread = size as f32 / 8.0;
    let margin = (((size as f32 / 16.0) * 2.0 + 1.0) / 2.0).ceil() as i32;
    let x0 = f64::from(x) + f64::from(angle).sin() * f64::from(spread);
    let x1 = f64::from(x) - f64::from(angle).sin() * f64::from(spread);
    let z0 = f64::from(z) + f64::from(angle).cos() * f64::from(spread);
    let z1 = f64::from(z) - f64::from(angle).cos() * f64::from(spread);
    let y0 = f64::from(y + rng.next_int(3) as i32 - 2);
    let y1 = f64::from(y + rng.next_int(3) as i32 - 2);
    let box_min_x = x - (spread.ceil() as i32) - margin;
    let box_min_y = y - 2 - margin;
    let box_min_z = z - (spread.ceil() as i32) - margin;

    if let Some(surface_y) = surface_height {
        if box_min_y as f32 > surface_y {
            return None;
        }
    }

    let segment_count = size as usize;
    let mut segments = vec![[0.0_f64; 4]; segment_count];
    for (index, segment) in segments.iter_mut().enumerate() {
        let progress = index as f32 / segment_count as f32;
        let center_x = lerp(f64::from(progress), x0, x1);
        let center_y = lerp(f64::from(progress), y0, y1);
        let center_z = lerp(f64::from(progress), z0, z1);
        let radius_roll = rng.next_f64() * f64::from(size) / 16.0;
        let radius = ((f64::from(mc_sin(std::f32::consts::PI * progress))
            + 1.0)
            * radius_roll
            + 1.0)
            / 2.0;
        *segment = [center_x, center_y, center_z, radius];
    }
    for first in 0..segment_count.saturating_sub(1) {
        if segments[first][3] <= 0.0 {
            continue;
        }
        for second in first + 1..segment_count {
            if segments[second][3] <= 0.0 {
                continue;
            }
            let dx = segments[first][0] - segments[second][0];
            let dy = segments[first][1] - segments[second][1];
            let dz = segments[first][2] - segments[second][2];
            let dr = segments[first][3] - segments[second][3];
            if dr * dr > dx * dx + dy * dy + dz * dz {
                if dr > 0.0 {
                    segments[second][3] = -1.0;
                } else {
                    segments[first][3] = -1.0;
                }
            }
        }
    }

    let needs_cell_rolls = discard_on_air > 0.0 && discard_on_air < 1.0;
    let mut placed = std::collections::HashSet::new();
    let mut blob_min_y = i32::MAX;
    let mut blob_max_y = i32::MIN;
    for segment in &segments {
        let radius = segment[3];
        if radius < 0.0 {
            continue;
        }
        let min_cell_x = ((segment[0] - radius).floor() as i32).max(box_min_x);
        let min_cell_y = ((segment[1] - radius).floor() as i32).max(box_min_y);
        let min_cell_z = ((segment[2] - radius).floor() as i32).max(box_min_z);
        let max_cell_x = ((segment[0] + radius).floor() as i32).max(min_cell_x);
        let max_cell_y = ((segment[1] + radius).floor() as i32).max(min_cell_y);
        let max_cell_z = ((segment[2] + radius).floor() as i32).max(min_cell_z);
        for cell_x in min_cell_x..=max_cell_x {
            let dx = (f64::from(cell_x) + 0.5 - segment[0]) / radius;
            if dx * dx >= 1.0 {
                continue;
            }
            for cell_y in min_cell_y..=max_cell_y {
                let dy = (f64::from(cell_y) + 0.5 - segment[1]) / radius;
                if dx * dx + dy * dy >= 1.0 {
                    continue;
                }
                for cell_z in min_cell_z..=max_cell_z {
                    let dz = (f64::from(cell_z) + 0.5 - segment[2]) / radius;
                    if dx * dx + dy * dy + dz * dz >= 1.0 {
                        continue;
                    }
                    if cell_y < world_bottom || cell_y >= world_top {
                        continue;
                    }
                    if !placed.insert((cell_x, cell_y, cell_z)) {
                        continue;
                    }
                    if needs_cell_rolls {
                        rng.next_f32();
                    }
                    blob_min_y = blob_min_y.min(cell_y);
                    blob_max_y = blob_max_y.max(cell_y);
                }
            }
        }
    }
    if blob_min_y > blob_max_y {
        return Some((y, y));
    }
    Some((blob_min_y, blob_max_y))
}

fn lerp(progress: f64, from: f64, to: f64) -> f64 {
    from + progress * (to - from)
}

/*
 * A cached approximate surface-height grid at 1:4 scale covering the bounding
 * box of the scanned chunks.
 */
struct SurfaceGrid {
    origin_x: i32,
    origin_z: i32,
    width: i32,
    height: i32,
    values: Vec<f32>,
}

impl SurfaceGrid {
    fn sample(
        seed: u64,
        mc_version: i32,
        chunks: &[(i32, i32)],
    ) -> Option<Self> {
        if chunks.is_empty() {
            return None;
        }
        let min_cx = chunks.iter().map(|chunk| chunk.0).min()?;
        let max_cx = chunks.iter().map(|chunk| chunk.0).max()?;
        let min_cz = chunks.iter().map(|chunk| chunk.1).min()?;
        let max_cz = chunks.iter().map(|chunk| chunk.1).max()?;
        let origin_x = min_cx * 4;
        let origin_z = min_cz * 4;
        let width = (max_cx - min_cx + 1) * 4;
        let height = (max_cz - min_cz + 1) * 4;
        if width <= 0 || height <= 0 || width as i64 * height as i64 > 1 << 20 {
            return None;
        }
        let mut values = vec![0.0_f32; (width * height) as usize];
        let result = unsafe {
            super::axolotl_seed_map_surface_heights(
                seed,
                mc_version,
                0,
                origin_x,
                origin_z,
                width,
                height,
                values.as_mut_ptr(),
            )
        };
        if result != 0 {
            return None;
        }
        Some(Self {
            origin_x,
            origin_z,
            width,
            height,
            values,
        })
    }

    fn height_at(&self, block_x: i32, block_z: i32) -> f32 {
        let column =
            (block_x.div_euclid(4) - self.origin_x).clamp(0, self.width - 1);
        let row =
            (block_z.div_euclid(4) - self.origin_z).clamp(0, self.height - 1);
        self.values[(row * self.width + column) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(
        seed: &str,
        version: &str,
        dimension: Dimension,
        ores: Vec<OreKind>,
    ) -> Vec<OreChunkResult> {
        scan_ores(OreScanRequest {
            seed: seed.to_owned(),
            version: version.to_owned(),
            dimension,
            ores,
            chunks: vec![0, 0, 1, 0, -1, -1],
        })
        .expect("scan should succeed")
    }

    #[test]
    fn ore_scans_are_deterministic() {
        let first = scan(
            "10292992",
            "1.21.3",
            Dimension::Overworld,
            vec![OreKind::Diamond],
        );
        let second = scan(
            "10292992",
            "1.21.3",
            Dimension::Overworld,
            vec![OreKind::Diamond],
        );
        assert_eq!(first.len(), 3);
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.hits.len(), b.hits.len());
            for (left, right) in a.hits.iter().zip(b.hits.iter()) {
                assert_eq!(
                    (left.x, left.y, left.z),
                    (right.x, right.y, right.z)
                );
            }
        }
    }

    #[test]
    fn diamond_attempts_stay_in_distribution_range() {
        for chunk in scan(
            "axolotl",
            "1.21.3",
            Dimension::Overworld,
            vec![OreKind::Diamond],
        ) {
            for hit in chunk.hits {
                assert!(
                    hit.y >= -144 && hit.y <= 16,
                    "y={} out of range",
                    hit.y
                );
                assert!(hit.x >= chunk.cx * 16 && hit.x < chunk.cx * 16 + 16);
                assert!(hit.z >= chunk.cz * 16 && hit.z < chunk.cz * 16 + 16);
            }
        }
    }

    #[test]
    fn netherite_scans_use_the_nether_layout() {
        let results = scan(
            "10292992",
            "1.21.3",
            Dimension::Nether,
            vec![OreKind::Netherite],
        );
        let hits: Vec<_> =
            results.iter().flat_map(|chunk| chunk.hits.iter()).collect();
        assert!(!hits.is_empty());
        for hit in hits {
            assert!(hit.y >= 8 && hit.y <= 119);
            assert!(hit.verified);
        }
    }

    #[test]
    fn vein_hits_stay_inside_their_bands_and_are_deterministic() {
        let chunks: Vec<i32> = (0..8_i32)
            .flat_map(|cx| (0..8_i32).flat_map(move |cz| [cx, cz]))
            .collect();
        let request = OreScanRequest {
            seed: "10292992".to_owned(),
            version: "26.2".to_owned(),
            dimension: Dimension::Overworld,
            ores: vec![OreKind::IronVein, OreKind::CopperVein],
            chunks,
        };
        let first =
            scan_ores(request.clone()).expect("vein scan should succeed");
        let second = scan_ores(request).expect("vein scan should succeed");
        let hits: Vec<_> =
            first.iter().flat_map(|chunk| chunk.hits.iter()).collect();
        for hit in &hits {
            match hit.ore {
                OreKind::IronVein => assert!(hit.y >= -60 && hit.y <= -8),
                OreKind::CopperVein => assert!(hit.y >= 0 && hit.y <= 50),
                other => panic!("unexpected ore kind {other:?}"),
            }
        }
        let second_hits: Vec<_> =
            second.iter().flat_map(|chunk| chunk.hits.iter()).collect();
        assert_eq!(hits.len(), second_hits.len());
    }

    #[test]
    fn old_versions_reject_ore_scans() {
        let error = scan_ores(OreScanRequest {
            seed: "1".to_owned(),
            version: "1.16".to_owned(),
            dimension: Dimension::Overworld,
            ores: vec![OreKind::Iron],
            chunks: vec![0, 0],
        });
        assert!(error.is_err());
    }
}
