import assert from 'node:assert/strict'
import test from 'node:test'

import {
	fallbackSeedMapProfiles,
	SEED_MAP_DEFAULT_VERSION,
	seedMapTileConcurrency,
} from './backend.ts'
import { SEED_MAP_BIOME_NAMES, SEED_MAP_BIOMES, seedMapBiomeGroups } from './biomes.ts'
import { featureMask, SEED_MAP_FEATURES, visibleFeatureDefinitions } from './features.ts'
import {
	applyShareQuery,
	createDefaultSeedMapWorkspace,
	isCurrentSeedMapEpoch,
	sanitizeSeedMapWorkspace,
	SEED_MAP_MIN_ZOOM,
} from './workspace.ts'

test('seed map starts near spawn with a lightweight layer set', () => {
	const workspace = createDefaultSeedMapWorkspace()
	assert.equal(workspace.zoom, 1)
	assert.equal(workspace.terrainEstimation, true)
	assert.equal(workspace.gameVersion, SEED_MAP_DEFAULT_VERSION)
	assert.deepEqual(workspace.visibleFeatures, [
		'village',
		'ruined-portal',
		'stronghold',
		'fortress',
		'bastion',
		'end-city',
	])
	assert.equal(workspace.displayMode, 'structures')
	assert.deepEqual(workspace.selectedOres, ['diamond'])
})

test('seed map workspace recovers from invalid persisted values', () => {
	const state = sanitizeSeedMapWorkspace({
		seed: '  mushroom island  ',
		edition: 'java',
		zoom: 99,
		center: { x: Number.NaN, z: 72 },
		visibleFeatures: ['village', 'invalid'],
	})
	assert.equal(state.seed, 'mushroom island')
	assert.equal(state.zoom, 4)
	assert.deepEqual(state.center, { x: 0, z: 72 })
	assert.deepEqual(state.visibleFeatures, ['village'])
	assert.equal(state.showSpawn, true)
	assert.deepEqual(state.highlightedBiomes, [])
})

test('seed map keeps every offered ore kind and drops unknown ones', () => {
	const state = sanitizeSeedMapWorkspace({
		...createDefaultSeedMapWorkspace(),
		selectedOres: ['diamond', 'iron_vein', 'copper_vein', 'coal', 'bogus'],
	})
	assert.deepEqual(state.selectedOres, ['diamond', 'iron_vein', 'copper_vein', 'coal'])
})

test('seed map supports a closer maximum zoom', () => {
	const close = sanitizeSeedMapWorkspace({ zoom: SEED_MAP_MIN_ZOOM })
	const tooClose = sanitizeSeedMapWorkspace({ zoom: SEED_MAP_MIN_ZOOM - 10 })
	assert.equal(close.zoom, SEED_MAP_MIN_ZOOM)
	assert.equal(tooClose.zoom, SEED_MAP_MIN_ZOOM)
})

test('seed map exposes every categorized cubiomes biome exactly once', () => {
	const groups = seedMapBiomeGroups()
	assert.equal(SEED_MAP_BIOMES.length, 65)
	assert.equal(groups.length, 17)
	assert.equal(groups.flatMap((group) => group.biomes).length, 65)
	assert.equal(new Set(SEED_MAP_BIOMES.map((biome) => biome.id)).size, 65)
	assert.deepEqual(
		groups.filter((group) => group.dimension === 'nether').map((group) => group.category),
		['nether'],
	)
	assert.deepEqual(
		groups.filter((group) => group.dimension === 'end').map((group) => group.category),
		['end'],
	)
	assert.deepEqual(
		SEED_MAP_BIOMES.filter((biome) => !SEED_MAP_BIOME_NAMES[biome.id]),
		[],
	)
})

test('seed map migrates legacy biome highlighting and terrain defaults', () => {
	const state = sanitizeSeedMapWorkspace({
		version: 1,
		highlightBiomeEnabled: true,
		highlightedBiome: 21,
		terrainEstimation: false,
	})
	assert.deepEqual(state.highlightedBiomes, [21])
	assert.equal(state.highlightBiomeEnabled, true)
	assert.equal(state.terrainEstimation, true)
	assert.equal(state.version, 3)
})

test('seed map share queries preserve useful map state', () => {
	const workspace = createDefaultSeedMapWorkspace()
	const shared = applyShareQuery(
		{
			seed: '1234',
			edition: 'java-large-biomes',
			version: '1.21.1',
			dimension: 'nether',
			mode: 'ores',
			ores: 'netherite',
			yMin: '12',
			yMax: '24',
			x: '-500',
			z: '312',
			zoom: '1.25',
		},
		workspace,
	)
	assert.equal(shared.seed, '1234')
	assert.equal(shared.edition, 'java-large-biomes')
	assert.equal(shared.gameVersion, '1.21.1')
	assert.equal(shared.dimension, 'nether')
	assert.equal(shared.displayMode, 'ores')
	assert.deepEqual(shared.selectedOres, ['netherite'])
	assert.equal(shared.oreYMin, 12)
	assert.equal(shared.oreYMax, 24)
	assert.equal(shared.zoom, 1.25)
	assert.deepEqual(shared.center, { x: -500, z: 312 })
})

test('fallback profiles gate dimensions, ores, and large biomes by version', () => {
	const profiles = fallbackSeedMapProfiles()
	const modern = profiles.find(
		(profile) => profile.edition === 'java' && profile.version === '1.21.3',
	)
	assert.deepEqual(modern?.dimensions, ['overworld', 'nether', 'end'])
	assert.equal(modern?.ores, true)
	const legacy = profiles.find((profile) => profile.edition === 'java' && profile.version === '1.9')
	assert.deepEqual(legacy?.dimensions, ['overworld', 'end'])
	assert.equal(legacy?.ores, false)
	const oldest = profiles.find((profile) => profile.edition === 'java' && profile.version === '1.8')
	assert.deepEqual(oldest?.dimensions, ['overworld'])
	assert.ok(
		!profiles.some(
			(profile) => profile.edition === 'java-large-biomes' && profile.version === '1.0',
		),
	)
})

test('feature masks match the selected map layers', () => {
	const mask = featureMask(['village', 'stronghold', 'slime-chunk'])
	const expected = SEED_MAP_FEATURES.filter((feature) =>
		['village', 'stronghold', 'slime-chunk'].includes(feature.kind),
	).reduce((value, feature) => value | feature.mask, 0)
	assert.equal(mask, expected)
})

test('seed map ignores a response from an older tile request epoch', () => {
	assert.equal(isCurrentSeedMapEpoch(8, 8), true)
	assert.equal(isCurrentSeedMapEpoch(7, 8), false)
})

test('seed map tile concurrency reserves capacity for the interface', () => {
	assert.equal(seedMapTileConcurrency(4, 4), 2)
	assert.equal(seedMapTileConcurrency(4, 8), 6)
	assert.equal(seedMapTileConcurrency(4, 16), 8)
	assert.equal(seedMapTileConcurrency(1, 16), 4)
})

test('dense feature layers are omitted until the map is close enough', () => {
	const close = visibleFeatureDefinitions(['slime-chunk', 'village', 'stronghold'], 'overworld', 4)
	const medium = visibleFeatureDefinitions(
		['slime-chunk', 'village', 'stronghold'],
		'overworld',
		64,
	)
	const far = visibleFeatureDefinitions(['slime-chunk', 'village', 'stronghold'], 'overworld', 256)
	assert.deepEqual(
		close.map((feature) => feature.kind),
		['slime-chunk', 'village', 'stronghold'],
	)
	assert.deepEqual(
		medium.map((feature) => feature.kind),
		['stronghold'],
	)
	assert.deepEqual(far, [])
})

test('slime chunks are the first layer in every dimension', () => {
	for (const dimension of ['overworld', 'nether', 'end'] as const) {
		const visible = visibleFeatureDefinitions(
			['village', 'fortress', 'end-city', 'slime-chunk'],
			dimension,
			4,
		)
		assert.equal(visible[0]?.kind, 'slime-chunk')
	}
})
