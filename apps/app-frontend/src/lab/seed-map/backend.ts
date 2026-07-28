import { invoke } from '@tauri-apps/api/core'

import type { SeedMapDimension } from './biomes.ts'
import type { SeedMapFeature, SeedMapFeatureKind } from './features.ts'
import type { SeedMapOreHit, SeedMapOreKind } from './ores.ts'

export type SeedMapEdition = 'java' | 'java-large-biomes'

export type SeedMapVersionProfile = {
	edition: SeedMapEdition
	version: string
	available: boolean
	dimensions: SeedMapDimension[]
	ores: boolean
	note?: string
}

export type SeedMapTileRequest = {
	epoch: number
	seed: string
	edition: SeedMapEdition
	version: string
	dimension: SeedMapDimension
	x: number
	z: number
	scale: number
	width: number
	height: number
	elevation?: number
	terrain: boolean
	contours: boolean
	highlightBiomes?: number[]
}

export type SeedMapTile = {
	epoch: number
	width: number
	height: number
	bitmap: ImageBitmap
	approximate: boolean
}

export type SeedMapFeatureQuery = {
	seed: string
	edition: SeedMapEdition
	version: string
	dimension: SeedMapDimension
	minX: number
	minZ: number
	maxX: number
	maxZ: number
	featureMask?: number
}

export type SeedMapSpawnPoint = {
	x: number
	z: number
	approximate: boolean
}

export type SeedMapLevelDat = {
	seed: string
	version?: string
}

export type SeedMapOreScanChunk = {
	cx: number
	cz: number
	hits: SeedMapOreHit[]
}

/**
 * Versions the bundled cubiomes engine understands, newest first. Releases
 * since the 1.21 winter drop (26.x and the late 1.21.x patches) share the
 * same world generation and all run on the newest bundled engine.
 */
export const SEED_MAP_FALLBACK_VERSIONS: readonly string[] = [
	'26.2',
	'26.1.2',
	'26.1',
	'1.21.9',
	'1.21.6',
	'1.21.5',
	'1.21.4',
	'1.21.3',
	'1.21.1',
	'1.20',
	'1.19.4',
	'1.19.2',
	'1.18',
	'1.17',
	'1.16',
	'1.15',
	'1.14',
	'1.13',
	'1.12',
	'1.11',
	'1.10',
	'1.9',
	'1.8',
	'1.7',
	'1.6',
	'1.5',
	'1.4',
	'1.3',
	'1.2',
	'1.1',
	'1.0',
]

export const SEED_MAP_DEFAULT_VERSION = SEED_MAP_FALLBACK_VERSIONS[0]

export function fallbackSeedMapProfiles(): SeedMapVersionProfile[] {
	const profiles: SeedMapVersionProfile[] = []
	for (const edition of ['java', 'java-large-biomes'] as const) {
		for (const version of SEED_MAP_FALLBACK_VERSIONS) {
			if (edition === 'java-large-biomes' && compareToMinor(version, 1, 3) < 0) continue
			const dimensions: SeedMapDimension[] = ['overworld']
			if (compareToMinor(version, 1, 16) >= 0) dimensions.push('nether')
			if (compareToMinor(version, 1, 9) >= 0) dimensions.push('end')
			profiles.push({
				edition,
				version,
				available: true,
				dimensions,
				ores: compareToMinor(version, 1, 18) >= 0,
			})
		}
	}
	return profiles
}

function compareToMinor(version: string, major: number, minor: number): number {
	const [versionMajor = 0, versionMinor = 0] = version.split('.').map(Number)
	if (versionMajor !== major) return versionMajor - major
	return versionMinor - minor
}

export async function getSeedMapProfiles(): Promise<SeedMapVersionProfile[]> {
	return await invoke<SeedMapVersionProfile[]>('plugin:seed-map|seed_map_profiles')
}

/**
 * Tiles render natively; this cap only bounds how many blocking renders are
 * in flight at once so pans stay responsive while tiles stream in.
 */
export function seedMapTileConcurrency(
	scale: number,
	hardwareConcurrency = typeof navigator === 'undefined' ? 4 : navigator.hardwareConcurrency || 4,
): number {
	const cores = Math.max(1, Math.floor(hardwareConcurrency))
	const limit = Math.max(2, Math.min(8, cores - 2))
	return scale <= 1 ? Math.min(4, limit) : limit
}

export async function renderSeedMapTile(request: SeedMapTileRequest): Promise<SeedMapTile> {
	const pixels = await invoke<ArrayBuffer>('plugin:seed-map|seed_map_render_tile', {
		request: {
			seed: request.seed,
			edition: request.edition,
			version: request.version,
			dimension: request.dimension,
			x: request.x,
			z: request.z,
			scale: request.scale,
			width: request.width,
			height: request.height,
			elevation: request.elevation ?? 62,
			terrain: request.terrain,
			contours: request.contours,
			highlightBiomes: request.highlightBiomes ?? null,
		},
	})
	const expectedLength = request.width * request.height * 4
	if (pixels.byteLength !== expectedLength) {
		throw new Error('The seed map backend returned an unexpected tile size.')
	}
	const image = new ImageData(new Uint8ClampedArray(pixels), request.width, request.height)
	const bitmap = await createImageBitmap(image, {
		colorSpaceConversion: 'none',
		premultiplyAlpha: 'none',
	})
	return {
		epoch: request.epoch,
		width: request.width,
		height: request.height,
		bitmap,
		approximate: request.terrain || request.contours,
	}
}

export async function findSeedMapFeatures(query: SeedMapFeatureQuery): Promise<SeedMapFeature[]> {
	const features = await invoke<
		{ kind: string; x: number; z: number; approximate: boolean; endShip?: boolean }[]
	>('plugin:seed-map|seed_map_find_features', {
		query: {
			seed: query.seed,
			edition: query.edition,
			version: query.version,
			dimension: query.dimension,
			minX: query.minX,
			minZ: query.minZ,
			maxX: query.maxX,
			maxZ: query.maxZ,
			featureMask: query.featureMask ?? null,
		},
	})
	return features.map((feature) => ({
		kind: feature.kind as SeedMapFeatureKind,
		x: feature.x,
		z: feature.z,
		approximate: feature.approximate,
		endShip: feature.endShip,
	}))
}

export async function getSeedMapSpawn(
	seed: string,
	edition: SeedMapEdition,
	version: string,
): Promise<SeedMapSpawnPoint> {
	return await invoke<SeedMapSpawnPoint>('plugin:seed-map|seed_map_spawn', {
		seed,
		edition,
		version,
	})
}

export async function getSeedMapBiomeAt(query: {
	seed: string
	edition: SeedMapEdition
	version: string
	dimension: SeedMapDimension
	x: number
	y: number
	z: number
}): Promise<number> {
	return await invoke<number>('plugin:seed-map|seed_map_biome_at', query)
}

export async function scanSeedMapOres(request: {
	seed: string
	version: string
	dimension: SeedMapDimension
	ores: SeedMapOreKind[]
	chunks: number[]
}): Promise<SeedMapOreScanChunk[]> {
	return await invoke<SeedMapOreScanChunk[]>('plugin:seed-map|seed_map_scan_ores', {
		request,
	})
}

export async function readSeedMapLevelDat(path: string): Promise<SeedMapLevelDat> {
	return await invoke<SeedMapLevelDat>('plugin:seed-map|seed_map_read_level_dat', { path })
}
