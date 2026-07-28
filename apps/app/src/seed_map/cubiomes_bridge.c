#include "cubiomes_bridge.h"

#include "biomenoise.h"
#include "finders.h"
#include "generator.h"
#include "noise.h"
#include "rng.h"
#include "util.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

static float axolotl_clamp(float value, float low, float high);

static int axolotl_map_approx_height(
	float *heights,
	const Generator *generator,
	const SurfaceNoise *surface_noise,
	int x,
	int z,
	int width,
	int height,
	int sparse
) {
	if (generator->dim != DIM_OVERWORLD || generator->mc < MC_1_18 ||
		(generator->bn.nptype != -1 && generator->bn.nptype != NP_DEPTH)) {
		return mapApproxHeight(
			heights, NULL, generator, surface_noise, x, z, width, height);
	}

	const uint32_t flags = SAMPLE_NO_BIOME | (sparse ? SAMPLE_NO_SHIFT : 0);
	for (int row = 0; row < height; row++) {
		for (int column = 0; column < width; column++) {
			int64_t noise_parameters[6];
			sampleBiomeNoise(
				&generator->bn, noise_parameters, x + column, 0, z + row, NULL, flags);
			heights[(size_t)row * width + column] =
				(float)noise_parameters[NP_DEPTH] / 76.0f;
		}
	}
	return 0;
}

enum {
	AXOLOTL_FEATURE_VILLAGE = 1 << 0,
	AXOLOTL_FEATURE_OUTPOST = 1 << 1,
	AXOLOTL_FEATURE_SHIPWRECK = 1 << 2,
	AXOLOTL_FEATURE_MONUMENT = 1 << 3,
	AXOLOTL_FEATURE_MANSION = 1 << 4,
	AXOLOTL_FEATURE_ANCIENT_CITY = 1 << 5,
	AXOLOTL_FEATURE_TRAIL_RUINS = 1 << 6,
	AXOLOTL_FEATURE_TRIAL_CHAMBERS = 1 << 7,
	AXOLOTL_FEATURE_RUINED_PORTAL = 1 << 8,
	AXOLOTL_FEATURE_STRONGHOLD = 1 << 9,
	AXOLOTL_FEATURE_SLIME_CHUNK = 1 << 10,
	AXOLOTL_FEATURE_DESERT_PYRAMID = 1 << 11,
	AXOLOTL_FEATURE_JUNGLE_TEMPLE = 1 << 12,
	AXOLOTL_FEATURE_SWAMP_HUT = 1 << 13,
	AXOLOTL_FEATURE_IGLOO = 1 << 14,
	AXOLOTL_FEATURE_OCEAN_RUIN = 1 << 15,
	AXOLOTL_FEATURE_BURIED_TREASURE = 1 << 16,
	AXOLOTL_FEATURE_MINESHAFT = 1 << 17,
	AXOLOTL_FEATURE_DESERT_WELL = 1 << 18,
	AXOLOTL_FEATURE_GEODE = 1 << 19,
	AXOLOTL_FEATURE_FORTRESS = 1 << 20,
	AXOLOTL_FEATURE_BASTION = 1 << 21,
	AXOLOTL_FEATURE_END_CITY = 1 << 22,
	AXOLOTL_FEATURE_END_GATEWAY = 1 << 23,
};

typedef struct AxolotlStructureDefinition {
	int type;
	uint32_t flag;
} AxolotlStructureDefinition;

static const AxolotlStructureDefinition AXOLOTL_STRUCTURES[] = {
	{Village, AXOLOTL_FEATURE_VILLAGE},
	{Outpost, AXOLOTL_FEATURE_OUTPOST},
	{Shipwreck, AXOLOTL_FEATURE_SHIPWRECK},
	{Monument, AXOLOTL_FEATURE_MONUMENT},
	{Mansion, AXOLOTL_FEATURE_MANSION},
	{Ancient_City, AXOLOTL_FEATURE_ANCIENT_CITY},
	{Trail_Ruins, AXOLOTL_FEATURE_TRAIL_RUINS},
	{Trial_Chambers, AXOLOTL_FEATURE_TRIAL_CHAMBERS},
	{Ruined_Portal, AXOLOTL_FEATURE_RUINED_PORTAL},
	{Ruined_Portal_N, AXOLOTL_FEATURE_RUINED_PORTAL},
	{Desert_Pyramid, AXOLOTL_FEATURE_DESERT_PYRAMID},
	{Jungle_Temple, AXOLOTL_FEATURE_JUNGLE_TEMPLE},
	{Swamp_Hut, AXOLOTL_FEATURE_SWAMP_HUT},
	{Igloo, AXOLOTL_FEATURE_IGLOO},
	{Ocean_Ruin, AXOLOTL_FEATURE_OCEAN_RUIN},
	{Treasure, AXOLOTL_FEATURE_BURIED_TREASURE},
	{Mineshaft, AXOLOTL_FEATURE_MINESHAFT},
	{Desert_Well, AXOLOTL_FEATURE_DESERT_WELL},
	{Geode, AXOLOTL_FEATURE_GEODE},
	{Fortress, AXOLOTL_FEATURE_FORTRESS},
	{Bastion, AXOLOTL_FEATURE_BASTION},
	{End_City, AXOLOTL_FEATURE_END_CITY},
	{End_Gateway, AXOLOTL_FEATURE_END_GATEWAY},
};

int32_t axolotl_seed_map_java_version(int32_t version) {
	switch (version) {
		/*
		 * Every release since the 1.21 winter drop shares its Overworld,
		 * Nether, and End generation, so 26.x and the late 1.21.x patches all
		 * map to the newest bundled engine.
		 */
		case 260200:
		case 260102:
		case 260100:
		case 12109:
		case 12106:
		case 12105:
		case 12104: return MC_NEWEST;
		case 12103: return MC_1_21_3;
		case 12101: return MC_1_21_1;
		case 12000: return MC_1_20;
		case 11904: return MC_1_19;
		case 11902: return MC_1_19_2;
		case 11800: return MC_1_18;
		case 11700: return MC_1_17;
		case 11600: return MC_1_16;
		case 11500: return MC_1_15;
		case 11400: return MC_1_14;
		case 11300: return MC_1_13;
		case 11200: return MC_1_12;
		case 11100: return MC_1_11;
		case 11000: return MC_1_10;
		case 10900: return MC_1_9;
		case 10800: return MC_1_8;
		case 10700: return MC_1_7;
		case 10600: return MC_1_6;
		case 10500: return MC_1_5;
		case 10400: return MC_1_4;
		case 10300: return MC_1_3;
		case 10200: return MC_1_2;
		case 10100: return MC_1_1;
		case 10000: return MC_1_0;
		default: return MC_UNDEF;
	}
}

static const unsigned char AXOLOTL_END_VOID[3] = {0x0D, 0x0D, 0x16};

static int axolotl_smooth_height_grid(
	float *grid,
	int width,
	int height,
	int passes
) {
	float *scratch = malloc((size_t)width * (size_t)height * sizeof(float));
	if (!scratch) return -1;
	for (int pass = 0; pass < passes; pass++) {
		for (int row = 0; row < height; row++) {
			const int offset = row * width;
			for (int column = 0; column < width; column++) {
				const float west = grid[offset + (column > 0 ? column - 1 : 0)];
				const float east = grid[offset + (column < width - 1 ? column + 1 : width - 1)];
				scratch[offset + column] =
					(west + 2.0f * grid[offset + column] + east) * 0.25f;
			}
		}
		for (int row = 0; row < height; row++) {
			const int north = (row > 0 ? row - 1 : 0) * width;
			const int offset = row * width;
			const int south = (row < height - 1 ? row + 1 : height - 1) * width;
			for (int column = 0; column < width; column++) {
				grid[offset + column] =
					(scratch[north + column] + 2.0f * scratch[offset + column] +
						scratch[south + column]) *
					0.25f;
			}
		}
	}
	free(scratch);
	return 0;
}

/*
 * The End uses the game's density noise to distinguish real island surface
 * from void. Like the reference map, the height grid is smoothed before slope
 * lighting so surface noise does not make the broad, flat islands look hilly.
 */
static void axolotl_render_end(
	uint8_t *rgb,
	const Generator *generator,
	const SurfaceNoise *surface_noise,
	int32_t x,
	int32_t z,
	int32_t scale,
	int32_t width,
	int32_t height,
	int32_t contours
) {
	if (scale > 4) return;
	const int passes = scale == 1 ? 3 : 1;
	const int padding = passes + 1;
	const int grid_width = width + 2 * padding;
	const int grid_height = height + 2 * padding;
	float *heights = malloc((size_t)grid_width * (size_t)grid_height * sizeof(float));
	if (!heights) return;
	uint8_t *land = malloc((size_t)width * (size_t)height);
	if (!land) {
		free(heights);
		return;
	}
	if (mapEndSurfaceHeight(
			heights, &generator->en, surface_noise,
			x - padding, z - padding, grid_width, grid_height, scale, 0) != 0) {
		free(land);
		free(heights);
		return;
	}
	for (int row = 0; row < height; row++) {
		for (int column = 0; column < width; column++) {
			land[(size_t)row * width + column] =
				heights[(row + padding) * grid_width + column + padding] > 4.0f;
		}
	}
	if (axolotl_smooth_height_grid(heights, grid_width, grid_height, passes) != 0) {
		free(land);
		free(heights);
		return;
	}
	for (int row = 0; row < height; row++) {
		for (int column = 0; column < width; column++) {
			const size_t index = (size_t)row * (size_t)width + (size_t)column;
			uint8_t *pixel = rgb + index * 3;
			if (!land[index]) {
				memcpy(pixel, AXOLOTL_END_VOID, 3);
				continue;
			}
			const int grid_row = row + padding;
			const int grid_column = column + padding;
			const int surface = (int)floorf(heights[grid_row * grid_width + grid_column]);
			const int west = (int)floorf(heights[grid_row * grid_width + grid_column - 1]);
			const int north = (int)floorf(heights[(grid_row - 1) * grid_width + grid_column]);
			const int northwest =
				(int)floorf(heights[(grid_row - 1) * grid_width + grid_column - 1]);
			float factor = 1.0f;
			if (contours && (surface / 16 != west / 16 || surface / 16 != north / 16)) {
				factor = 0.15f;
			} else {
				const int slope = 3 * surface - west - north - northwest;
				if (slope != 0) {
					const float delta = axolotl_clamp((float)slope / 16.0f, -0.3f, 0.3f);
					factor = 1.0f + delta + (delta > 0.0f ? 0.1f : -0.1f);
					if (factor < 0.72f) factor = 0.72f;
				}
			}
			for (int channel = 0; channel < 3; channel++) {
				pixel[channel] =
					(uint8_t)axolotl_clamp((float)pixel[channel] * factor, 0.0f, 255.0f);
			}
		}
	}
	free(land);
	free(heights);
}

static int axolotl_block_height(
	const float *grid,
	int grid_width,
	int grid_height,
	int column,
	int row
) {
	if (column < 0) column = 0;
	if (row < 0) row = 0;
	if (column > grid_width - 1) column = grid_width - 1;
	if (row > grid_height - 1) row = grid_height - 1;
	return (int)floorf(grid[row * grid_width + column]);
}

static float axolotl_clamp(float value, float low, float high) {
	if (value < low) return low;
	if (value > high) return high;
	return value;
}

/*
 * Minecraft maps compare each cell with its northwestern neighbours and use
 * discrete brightness levels for the slope. The reference map applies the
 * same rule to its cubiomes height grid, including 16-block contour bands.
 */
static void axolotl_shade_height_map(
	uint8_t *rgb,
	int32_t width,
	int32_t height,
	int32_t contours,
	const float *grid,
	int grid_width,
	int grid_height,
	int grid_origin_column,
	int grid_origin_row,
	int numerator,
	int denominator
) {
	for (int row = 0; row < height; row++) {
		const int cell_row =
			grid_origin_row + (int)(((int64_t)row * numerator) / denominator);
		for (int column = 0; column < width; column++) {
			const int cell_column =
				grid_origin_column + (int)(((int64_t)column * numerator) / denominator);
			const size_t index = (size_t)row * (size_t)width + (size_t)column;
			const int here =
				axolotl_block_height(grid, grid_width, grid_height, cell_column, cell_row);
			const int west =
				axolotl_block_height(grid, grid_width, grid_height, cell_column - 1, cell_row);
			const int north =
				axolotl_block_height(grid, grid_width, grid_height, cell_column, cell_row - 1);
			const int northwest =
				axolotl_block_height(grid, grid_width, grid_height, cell_column - 1, cell_row - 1);
			float factor = 1.0f;
			if (contours) {
				const int band = here / 16;
				if (band != west / 16 || band != north / 16) factor = 0.15f;
			}
			if (factor == 1.0f) {
				const int slope = 3 * here - west - north - northwest;
				if (slope != 0) {
					const float delta = axolotl_clamp((float)slope / 16.0f, -0.3f, 0.3f);
					factor = 1.0f + delta + (delta > 0.0f ? 0.1f : -0.1f);
					if (factor < 0.72f) factor = 0.72f;
				}
			}
			uint8_t *pixel = rgb + index * 3;
			const int strongest = pixel[0] > pixel[1]
				? (pixel[0] > pixel[2] ? pixel[0] : pixel[2])
				: (pixel[1] > pixel[2] ? pixel[1] : pixel[2]);
			if (factor < 1.0f && strongest > 25 && strongest - strongest * factor < 25)
				factor = (strongest - 25.0f) / strongest;
			for (int channel = 0; channel < 3; channel++) {
				const float shaded = (float)pixel[channel] * factor;
				pixel[channel] = (uint8_t)axolotl_clamp(shaded, 0.0f, 255.0f);
			}
		}
	}
}

/*
 * Far-zoom tiles cover too many blocks for the contiguous 1:4 height map, so
 * relief comes from a sparse grid sampled point by point at each cell
 * center. The grid stays coarse enough to keep far tiles interactive.
 */
static int axolotl_fill_sparse_height_grid(
	const Generator *generator,
	const SurfaceNoise *surface_noise,
	int64_t min_block_x,
	int64_t min_block_z,
	int64_t span_blocks,
	int grid,
	float *out
) {
	for (int row = 0; row < grid; row++) {
		const int64_t block_z =
			min_block_z + span_blocks * (2 * row + 1) / (2 * grid);
		for (int column = 0; column < grid; column++) {
			const int64_t block_x =
				min_block_x + span_blocks * (2 * column + 1) / (2 * grid);
			if (axolotl_map_approx_height(
					out + (size_t)row * grid + column, generator, surface_noise,
					(int)(block_x >> 2), (int)(block_z >> 2), 1, 1, 1) != 0) {
				return -1;
			}
		}
	}
	return 0;
}

int axolotl_seed_map_render(
	uint64_t seed,
	int32_t minecraft_version,
	int32_t generator_flags,
	int32_t dimension,
	int32_t x,
	int32_t z,
	int32_t scale,
	int32_t width,
	int32_t height,
	int32_t elevation,
	int32_t terrain,
	int32_t contours,
	const uint8_t *highlight_mask,
	uint8_t *rgb,
	size_t rgb_len
) {
	if (!rgb || width <= 0 || height <= 0 || scale <= 0) return -1;
	if (rgb_len < (size_t)width * (size_t)height * 3) return -2;

	Generator generator;
	setupGenerator(&generator, minecraft_version, generator_flags);
	applySeed(&generator, dimension, seed);

	Range range = {
		.scale = scale,
		.x = x,
		.z = z,
		.sx = width,
		.sz = height,
		.y = elevation / 4,
		.sy = 1,
	};
	int *biomes = allocCache(&generator, range);
	if (!biomes) return -3;
	int result = genBiomes(&generator, biomes, range);
	if (result != 0) {
		free(biomes);
		return result;
	}

	unsigned char colors[256][3];
	initBiomeColors(colors);
	biomesToImage(rgb, colors, biomes, width, height, 1, 2);

	if (highlight_mask) {
		for (int row = 0; row < height; row++) {
			for (int column = 0; column < width; column++) {
				size_t index = (size_t)row * (size_t)width + (size_t)column;
				const int biome = biomes[index];
				if (biome >= 0 && biome < 256 && highlight_mask[biome]) continue;
				size_t pixel = index * 3;
				if (dimension != DIM_OVERWORLD) {
					rgb[pixel] = (uint8_t)(255 - (((255 - rgb[pixel]) * 51) >> 8));
					rgb[pixel + 1] = (uint8_t)(255 - (((255 - rgb[pixel + 1]) * 51) >> 8));
					rgb[pixel + 2] = (uint8_t)(255 - (((255 - rgb[pixel + 2]) * 51) >> 8));
				} else {
					rgb[pixel] = (uint8_t)((77 * rgb[pixel]) >> 8);
					rgb[pixel + 1] = (uint8_t)((77 * rgb[pixel + 1]) >> 8);
					rgb[pixel + 2] = (uint8_t)((77 * rgb[pixel + 2]) >> 8);
				}
			}
		}
	}

	const int64_t min_block_x = (int64_t)x * scale;
	const int64_t min_block_z = (int64_t)z * scale;
	if (terrain && dimension == DIM_END) {
		SurfaceNoise surface_noise;
		initSurfaceNoise(&surface_noise, DIM_END, seed);
		axolotl_render_end(
			rgb, &generator, &surface_noise,
			x, z, scale, width, height, contours);
	}

	if (terrain && dimension == DIM_OVERWORLD) {
		SurfaceNoise surface_noise;
		initSurfaceNoise(&surface_noise, DIM_OVERWORLD, seed);
		if (scale <= 4) {
			const int grid_x =
				(int)(min_block_x >= 0 ? min_block_x / 4 : (min_block_x - 3) / 4) - 1;
			const int grid_z =
				(int)(min_block_z >= 0 ? min_block_z / 4 : (min_block_z - 3) / 4) - 1;
			const int grid_width = (width * scale) / 4 + 3;
			const int grid_height = (height * scale) / 4 + 3;
			float *heights =
				malloc((size_t)grid_width * (size_t)grid_height * sizeof(float));
			if (heights) {
				if (axolotl_map_approx_height(
						heights, &generator, &surface_noise,
						grid_x, grid_z, grid_width, grid_height, 0) == 0) {
					axolotl_shade_height_map(
						rgb, width, height, contours,
						heights, grid_width, grid_height, 1, 1, scale, 4);
				}
				free(heights);
			}
		} else {
			const int grid = scale == 16 ? 96 : 64;
			float *heights = malloc((size_t)grid * (size_t)grid * sizeof(float));
			if (heights) {
				if (axolotl_fill_sparse_height_grid(
						&generator, &surface_noise, min_block_x, min_block_z,
						(int64_t)width * scale, grid, heights) == 0) {
					axolotl_shade_height_map(
						rgb, width, height, contours,
						heights, grid, grid, 0, 0, grid, width);
				}
				free(heights);
			}
		}
	}
	free(biomes);

	return 0;
}

static int add_feature(
	AxolotlSeedMapFeature *out,
	size_t out_len,
	size_t *count,
	int x,
	int z,
	uint32_t kind,
	uint8_t approximate,
	int8_t end_ship
) {
	if (*count >= out_len) return 0;
	out[*count].x = x;
	out[*count].z = z;
	out[*count].kind = kind;
	out[*count].approximate = approximate;
	out[*count].end_ship = end_ship;
	*count += 1;
	return 1;
}

static int8_t axolotl_end_city_has_ship(uint64_t seed, int block_x, int block_z) {
	Piece pieces[END_CITY_PIECES_MAX];
	const int count = getEndCityPieces(pieces, seed, block_x >> 4, block_z >> 4);
	for (int index = 0; index < count; index++) {
		if (pieces[index].type == END_SHIP) return 1;
	}
	return 0;
}

size_t axolotl_seed_map_find_features(
	uint64_t seed,
	int32_t minecraft_version,
	int32_t generator_flags,
	int32_t dimension,
	int32_t min_x,
	int32_t min_z,
	int32_t max_x,
	int32_t max_z,
	uint32_t feature_mask,
	AxolotlSeedMapFeature *out,
	size_t out_len
) {
	if (!out || out_len == 0) return 0;
	Generator generator;
	setupGenerator(&generator, minecraft_version, generator_flags);
	applySeed(&generator, dimension, seed);
	SurfaceNoise end_surface_noise;
	if (dimension == DIM_END) initSurfaceNoise(&end_surface_noise, DIM_END, seed);

	size_t count = 0;
	for (size_t index = 0; index < sizeof(AXOLOTL_STRUCTURES) / sizeof(AXOLOTL_STRUCTURES[0]); index++) {
		const AxolotlStructureDefinition definition = AXOLOTL_STRUCTURES[index];
		if (!(feature_mask & definition.flag)) continue;

		StructureConfig config;
		if (!getStructureConfig(definition.type, minecraft_version, &config)) continue;
		if (config.dim != dimension || config.regionSize <= 0) continue;

		const int region_span = config.regionSize * 16;
		const int min_region_x = min_x >= 0 ? min_x / region_span : (min_x - region_span + 1) / region_span;
		const int min_region_z = min_z >= 0 ? min_z / region_span : (min_z - region_span + 1) / region_span;
		const int max_region_x = max_x >= 0 ? max_x / region_span : (max_x - region_span + 1) / region_span;
		const int max_region_z = max_z >= 0 ? max_z / region_span : (max_z - region_span + 1) / region_span;

		for (int region_x = min_region_x; region_x <= max_region_x; region_x++) {
			for (int region_z = min_region_z; region_z <= max_region_z; region_z++) {
				Pos position;
				if (!getStructurePos(definition.type, minecraft_version, seed, region_x, region_z, &position)) continue;
				if (position.x < min_x || position.x > max_x || position.z < min_z || position.z > max_z) continue;
				if (!isViableStructurePos(definition.type, &generator, position.x, position.z, 0)) continue;
				if (definition.type == End_City &&
					!isViableEndCityTerrain(&generator, &end_surface_noise, position.x, position.z))
					continue;
				if (dimension == DIM_OVERWORLD && minecraft_version >= MC_1_18 &&
					!isViableStructureTerrain(definition.type, &generator, position.x, position.z))
					continue;
				const int8_t end_ship = definition.type == End_City
					? axolotl_end_city_has_ship(seed, position.x, position.z)
					: -1;
				if (!add_feature(
						out, out_len, &count, position.x, position.z,
						definition.flag, 0, end_ship)) return count;
			}
		}
	}

	if (dimension == DIM_OVERWORLD && feature_mask & AXOLOTL_FEATURE_STRONGHOLD) {
		StrongholdIter stronghold;
		initFirstStronghold(&stronghold, minecraft_version, seed);
		for (int index = 0; index < 32; index++) {
			if (nextStronghold(&stronghold, &generator) <= 0) break;
			if (stronghold.pos.x >= min_x && stronghold.pos.x <= max_x && stronghold.pos.z >= min_z && stronghold.pos.z <= max_z) {
				if (!add_feature(
						out, out_len, &count, stronghold.pos.x, stronghold.pos.z,
						AXOLOTL_FEATURE_STRONGHOLD, 0, -1)) return count;
			}
		}
	}

	if (dimension == DIM_OVERWORLD && feature_mask & AXOLOTL_FEATURE_SLIME_CHUNK) {
		const int min_chunk_x = min_x >= 0 ? min_x / 16 : (min_x - 15) / 16;
		const int min_chunk_z = min_z >= 0 ? min_z / 16 : (min_z - 15) / 16;
		const int max_chunk_x = max_x >= 0 ? max_x / 16 : (max_x - 15) / 16;
		const int max_chunk_z = max_z >= 0 ? max_z / 16 : (max_z - 15) / 16;
		for (int chunk_x = min_chunk_x; chunk_x <= max_chunk_x; chunk_x++) {
			for (int chunk_z = min_chunk_z; chunk_z <= max_chunk_z; chunk_z++) {
				if (!isSlimeChunk(seed, chunk_x, chunk_z)) continue;
				if (!add_feature(
						out, out_len, &count, chunk_x * 16 + 8, chunk_z * 16 + 8,
						AXOLOTL_FEATURE_SLIME_CHUNK, 0, -1)) return count;
			}
		}
	}

	return count;
}

int axolotl_seed_map_get_spawn(
	uint64_t seed,
	int32_t minecraft_version,
	int32_t generator_flags,
	int32_t *x,
	int32_t *z
) {
	if (!x || !z) return -1;
	Generator generator;
	setupGenerator(&generator, minecraft_version, generator_flags);
	applySeed(&generator, DIM_OVERWORLD, seed);
	Pos position = estimateSpawn(&generator, NULL);
	*x = position.x;
	*z = position.z;
	return 0;
}

int32_t axolotl_seed_map_biome_at(
	uint64_t seed,
	int32_t minecraft_version,
	int32_t generator_flags,
	int32_t dimension,
	int32_t x,
	int32_t y,
	int32_t z
) {
	Generator generator;
	setupGenerator(&generator, minecraft_version, generator_flags);
	applySeed(&generator, dimension, seed);
	return getBiomeAt(&generator, 1, x, y, z);
}

typedef struct AxolotlVeinNoise {
	DoublePerlinNoise toggle;
	DoublePerlinNoise vein_a;
	DoublePerlinNoise vein_b;
	DoublePerlinNoise gap;
	PerlinNoise octaves[8];
	uint64_t ore_lo;
	uint64_t ore_hi;
} AxolotlVeinNoise;

/*
 * Seeds the Overworld vein noises the same way the game does: the world seed
 * forks a positional random whose per-noise instances are XORed with the MD5
 * halves of their resource ids.
 */
static void axolotl_init_vein_noise(AxolotlVeinNoise *vein_noise, uint64_t seed) {
	static const double unit_amplitude[] = {1.0};
	Xoroshiro base;
	xSetSeed(&base, seed);
	const uint64_t lo = xNextLong(&base);
	const uint64_t hi = xNextLong(&base);
	PerlinNoise *slot = vein_noise->octaves;
	Xoroshiro xr;
	xr.lo = lo ^ 0x6B86C7820A307171ULL; /* minecraft:ore_veininess */
	xr.hi = hi ^ 0xD87FB0FEFD9C1624ULL;
	slot += xDoublePerlinInit(&vein_noise->toggle, &xr, slot, unit_amplitude, -8, 1, -1);
	xr.lo = lo ^ 0x4CD8D69B9A841649ULL; /* minecraft:ore_vein_a */
	xr.hi = hi ^ 0xCDD63F17BFE8F5EDULL;
	slot += xDoublePerlinInit(&vein_noise->vein_a, &xr, slot, unit_amplitude, -7, 1, -1);
	xr.lo = lo ^ 0x6B26220B31F7C6C9ULL; /* minecraft:ore_vein_b */
	xr.hi = hi ^ 0xAE077EDEBF6AAEC1ULL;
	slot += xDoublePerlinInit(&vein_noise->vein_b, &xr, slot, unit_amplitude, -7, 1, -1);
	xr.lo = lo ^ 0x9C4CC6B2FB0BE4BBULL; /* minecraft:ore_gap */
	xr.hi = hi ^ 0xBD5964705573BB5EULL;
	xDoublePerlinInit(&vein_noise->gap, &xr, slot, unit_amplitude, -5, 1, -1);
	vein_noise->ore_lo = lo ^ 0x9B88124DE600116DULL; /* minecraft:ore */
	vein_noise->ore_hi = hi ^ 0x2AE68055AA4A7761ULL;
}

static uint64_t axolotl_block_position_seed(int32_t x, int32_t y, int32_t z) {
	int64_t l = (int64_t)(int32_t)((uint32_t)x * 3129871u) ^
		((int64_t)z * 116129781LL) ^ (int64_t)y;
	l = l * l * 42317861LL + l * 11LL;
	return (uint64_t)(l >> 16);
}

static Xoroshiro axolotl_vein_block_rng(const AxolotlVeinNoise *vein_noise, int32_t x, int32_t y, int32_t z) {
	Xoroshiro xr;
	xr.lo = axolotl_block_position_seed(x, y, z) ^ vein_noise->ore_lo;
	xr.hi = vein_noise->ore_hi;
	if (xr.lo == 0 && xr.hi == 0) {
		xr.lo = 0x9E3779B97F4A7C15ULL;
		xr.hi = 0x6A09E667F3BCC909ULL;
	}
	return xr;
}

static double axolotl_clamped_map(double value, double from_min, double from_max, double to_min, double to_max) {
	if (value <= from_min) return to_min;
	if (value >= from_max) return to_max;
	return to_min + (value - from_min) / (from_max - from_min) * (to_max - to_min);
}

/*
 * Follows the game's vein rules on a 2-block sampling grid: the low frequency
 * veininess noise selects copper (positive, Y 0..50) or iron (negative,
 * Y -60..-8) with a 20-block edge fade towards a 0.4 threshold. A sampled
 * position counts as ore when the 70% solidness roll passes, both ridge
 * noises stay inside the 0.08 band, the richness roll (10%..30% by
 * veininess) succeeds, and the gap noise is above -0.3 — the same checks and
 * positional-random call order the game uses, evaluated without the noise
 * cell interpolation, so results are close estimates.
 */
size_t axolotl_seed_map_scan_vein(
	uint64_t seed,
	int32_t chunk_x,
	int32_t chunk_z,
	int32_t vein_kind,
	int32_t *out_xyz,
	size_t out_cap
) {
	if (!out_xyz || out_cap == 0) return 0;
	AxolotlVeinNoise vein_noise;
	axolotl_init_vein_noise(&vein_noise, seed);
	const int min_y = vein_kind == 0 ? 0 : -60;
	const int max_y = vein_kind == 0 ? 50 : -8;
	const int block_x = chunk_x * 16;
	const int block_z = chunk_z * 16;
	uint8_t column_taken[64] = {0};
	size_t count = 0;
	for (int y = min_y; y <= max_y && count < out_cap; y += 2) {
		const int edge = (max_y - y) < (y - min_y) ? (max_y - y) : (y - min_y);
		const double edge_fade = edge >= 20 ? 0.0 : -0.2 + 0.01 * (double)edge;
		const double coarse = sampleDoublePerlin(
			&vein_noise.toggle,
			(block_x + 8) * 1.5, y * 1.5, (block_z + 8) * 1.5);
		const double coarse_signed = vein_kind == 0 ? coarse : -coarse;
		if (coarse_signed + edge_fade < 0.34) continue;
		for (int dx = 0; dx < 16 && count < out_cap; dx += 2) {
			for (int dz = 0; dz < 16 && count < out_cap; dz += 2) {
				const int column = (dx >> 1) * 8 + (dz >> 1);
				if (column_taken[column]) continue;
				const int x = block_x + dx;
				const int z = block_z + dz;
				const double toggle =
					sampleDoublePerlin(&vein_noise.toggle, x * 1.5, y * 1.5, z * 1.5);
				const double toggle_signed = vein_kind == 0 ? toggle : -toggle;
				if (toggle_signed + edge_fade < 0.4) continue;
				Xoroshiro block_rng = axolotl_vein_block_rng(&vein_noise, x, y, z);
				if (xNextFloat(&block_rng) > 0.7f) continue;
				if (fabs(sampleDoublePerlin(&vein_noise.vein_a, x * 4.0, y * 4.0, z * 4.0)) >= 0.08)
					continue;
				if (fabs(sampleDoublePerlin(&vein_noise.vein_b, x * 4.0, y * 4.0, z * 4.0)) >= 0.08)
					continue;
				const double richness =
					axolotl_clamped_map(toggle_signed, 0.4, 0.6, 0.1, 0.3);
				if (xNextFloat(&block_rng) >= richness) continue;
				if (sampleDoublePerlin(&vein_noise.gap, x, y, z) <= -0.3) continue;
				column_taken[column] = 1;
				out_xyz[count * 3] = x;
				out_xyz[count * 3 + 1] = y;
				out_xyz[count * 3 + 2] = z;
				count++;
			}
		}
	}
	return count;
}

int axolotl_seed_map_surface_heights(
	uint64_t seed,
	int32_t minecraft_version,
	int32_t generator_flags,
	int32_t x,
	int32_t z,
	int32_t width,
	int32_t height,
	float *out
) {
	if (!out || width <= 0 || height <= 0) return -1;
	Generator generator;
	setupGenerator(&generator, minecraft_version, generator_flags);
	applySeed(&generator, DIM_OVERWORLD, seed);
	SurfaceNoise surface_noise;
	initSurfaceNoise(&surface_noise, DIM_OVERWORLD, seed);
	return mapApproxHeight(out, NULL, &generator, &surface_noise, x, z, width, height);
}
