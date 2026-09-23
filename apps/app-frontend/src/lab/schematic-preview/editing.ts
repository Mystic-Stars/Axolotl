import type { SchematicBlockState } from './backend.ts'

export type SchematicBlockLocation = {
	regionId: string
	position: [number, number, number]
}

export type SchematicCachedChunk = {
	regionId: string
	position: [number, number, number]
	blocks: Uint32Array
}

export type SchematicMeshArrays = {
	positions: Float32Array
	normals: Float32Array
	uvs: Float32Array
	colors: Float32Array
	blockPositions: Float32Array
}

export const MAX_SCHEMATIC_SELECTION = 250_000
const SCHEMATIC_AIR_BLOCKS = new Set([
	'minecraft:air',
	'minecraft:cave_air',
	'minecraft:void_air',
	'minecraft:light',
	'minecraft:barrier',
	'minecraft:structure_void',
])

export function isSchematicAir(name: string) {
	return SCHEMATIC_AIR_BLOCKS.has(name.trim().toLowerCase())
}

export function normalizeSchematicAirBlocks(
	blocks: Uint32Array,
	palette: readonly SchematicBlockState[],
) {
	const airIndexes = new Set<number>()
	for (let index = 1; index < palette.length; index += 1) {
		if (isSchematicAir(palette[index]?.name ?? '')) airIndexes.add(index)
	}
	if (airIndexes.size === 0) return blocks
	let normalized: Uint32Array | undefined
	for (let index = 0; index < blocks.length; index += 1) {
		if (!airIndexes.has(blocks[index] ?? 0)) continue
		normalized ??= blocks.slice()
		normalized[index] = 0
	}
	return normalized ?? blocks
}

export function filterSchematicAirGeometry(
	mesh: SchematicMeshArrays,
	chunk: SchematicCachedChunk,
	palette: readonly SchematicBlockState[],
) {
	const keep = new Uint8Array(mesh.blockPositions.length / 3)
	let keptVertices = 0
	const origin = chunk.position.map((value) => value * 16)
	for (let vertex = 0; vertex < keep.length; vertex += 1) {
		const x = Math.round(mesh.blockPositions[vertex * 3] ?? 0) - origin[0]
		const y = Math.round(mesh.blockPositions[vertex * 3 + 1] ?? 0) - origin[1]
		const z = Math.round(mesh.blockPositions[vertex * 3 + 2] ?? 0) - origin[2]
		const blockIndex = y * 256 + z * 16 + x
		const state = palette[chunk.blocks[blockIndex] ?? 0]
		if (x < 0 || x >= 16 || y < 0 || y >= 16 || z < 0 || z >= 16 || !state) continue
		if (isSchematicAir(state.name)) continue
		keep[vertex] = 1
		keptVertices += 1
	}
	if (keptVertices === keep.length) return mesh

	const positions = new Float32Array(keptVertices * 3)
	const normals = new Float32Array(keptVertices * 3)
	const uvs = new Float32Array(keptVertices * 2)
	const colors = new Float32Array(keptVertices * 3)
	const blockPositions = new Float32Array(keptVertices * 3)
	let target = 0
	for (let vertex = 0; vertex < keep.length; vertex += 1) {
		if (!keep[vertex]) continue
		positions.set(mesh.positions.subarray(vertex * 3, vertex * 3 + 3), target * 3)
		normals.set(mesh.normals.subarray(vertex * 3, vertex * 3 + 3), target * 3)
		uvs.set(mesh.uvs.subarray(vertex * 2, vertex * 2 + 2), target * 2)
		colors.set(mesh.colors.subarray(vertex * 3, vertex * 3 + 3), target * 3)
		blockPositions.set(mesh.blockPositions.subarray(vertex * 3, vertex * 3 + 3), target * 3)
		target += 1
	}
	return { positions, normals, uvs, colors, blockPositions }
}

export function measureSchematicPoints(
	from: [number, number, number],
	to: [number, number, number],
) {
	const delta = to.map((value, axis) => value - from[axis]) as [number, number, number]
	const size = delta.map((value) => Math.abs(value) + 1) as [number, number, number]
	return { delta, size, distance: Math.hypot(...delta) }
}

export function schematicChunkKey(regionId: string, position: [number, number, number]) {
	return `${regionId}\u0000${position.join(':')}`
}

export function schematicBlockKey(location: SchematicBlockLocation) {
	return `${location.regionId}\u0000${location.position.join(':')}`
}

export function schematicBlockStateKey(state: SchematicBlockState) {
	const properties = Object.entries(state.properties)
		.sort(([left], [right]) => left.localeCompare(right))
		.map(([key, value]) => `${key}=${value}`)
		.join(',')
	return properties ? `${state.name}[${properties}]` : state.name
}

export function schematicBlockPaletteIndex(
	chunks: ReadonlyMap<string, SchematicCachedChunk>,
	location: SchematicBlockLocation,
) {
	const chunkPosition = location.position.map((value) => Math.floor(value / 16)) as [
		number,
		number,
		number,
	]
	const chunk = chunks.get(schematicChunkKey(location.regionId, chunkPosition))
	if (!chunk) return 0
	const local = location.position.map((value) => ((value % 16) + 16) % 16)
	return chunk.blocks[local[1] * 256 + local[2] * 16 + local[0]] ?? 0
}

function pushLocation(
	result: SchematicBlockLocation[],
	regionId: string,
	position: [number, number, number],
) {
	if (result.length >= MAX_SCHEMATIC_SELECTION) return false
	result.push({ regionId, position })
	return true
}

export function selectSchematicCuboid(
	chunks: ReadonlyMap<string, SchematicCachedChunk>,
	from: SchematicBlockLocation,
	to: SchematicBlockLocation,
) {
	if (from.regionId !== to.regionId) return [to]
	const min = from.position.map((value, axis) => Math.min(value, to.position[axis]))
	const max = from.position.map((value, axis) => Math.max(value, to.position[axis]))
	const result: SchematicBlockLocation[] = []
	for (let y = min[1]; y <= max[1]; y += 1) {
		for (let z = min[2]; z <= max[2]; z += 1) {
			for (let x = min[0]; x <= max[0]; x += 1) {
				const location: SchematicBlockLocation = {
					regionId: from.regionId,
					position: [x, y, z],
				}
				if (
					schematicBlockPaletteIndex(chunks, location) !== 0 &&
					!pushLocation(result, location.regionId, location.position)
				) {
					return result
				}
			}
		}
	}
	return result
}

export function selectSchematicBlocks(
	chunks: ReadonlyMap<string, SchematicCachedChunk>,
	predicate: (
		paletteIndex: number,
		position: [number, number, number],
		regionId: string,
	) => boolean,
) {
	const result: SchematicBlockLocation[] = []
	for (const chunk of chunks.values()) {
		for (let index = 0; index < chunk.blocks.length; index += 1) {
			const paletteIndex = chunk.blocks[index] ?? 0
			if (paletteIndex === 0) continue
			const x = index % 16
			const z = Math.floor(index / 16) % 16
			const y = Math.floor(index / 256)
			const position: [number, number, number] = [
				chunk.position[0] * 16 + x,
				chunk.position[1] * 16 + y,
				chunk.position[2] * 16 + z,
			]
			if (
				predicate(paletteIndex, position, chunk.regionId) &&
				!pushLocation(result, chunk.regionId, position)
			) {
				return result
			}
		}
	}
	return result
}

export function selectSchematicMaterial(
	chunks: ReadonlyMap<string, SchematicCachedChunk>,
	palette: SchematicBlockState[],
	name: string,
) {
	return selectSchematicBlocks(chunks, (paletteIndex) => palette[paletteIndex]?.name === name)
}

export function selectSchematicBlockState(
	chunks: ReadonlyMap<string, SchematicCachedChunk>,
	palette: SchematicBlockState[],
	state: SchematicBlockState,
) {
	const targetKey = schematicBlockStateKey(state)
	return selectSchematicBlocks(
		chunks,
		(paletteIndex) =>
			palette[paletteIndex] !== undefined &&
			schematicBlockStateKey(palette[paletteIndex]) === targetKey,
	)
}

export function selectSchematicLayer(chunks: ReadonlyMap<string, SchematicCachedChunk>, y: number) {
	return selectSchematicBlocks(chunks, (_paletteIndex, position) => position[1] === y)
}

export function selectConnectedSchematicBlocks(
	chunks: ReadonlyMap<string, SchematicCachedChunk>,
	start: SchematicBlockLocation,
) {
	if (schematicBlockPaletteIndex(chunks, start) === 0) return []
	const result: SchematicBlockLocation[] = []
	const queue = [start]
	const visited = new Set([schematicBlockKey(start)])
	const offsets = [
		[-1, 0, 0],
		[1, 0, 0],
		[0, -1, 0],
		[0, 1, 0],
		[0, 0, -1],
		[0, 0, 1],
	] as const
	for (let index = 0; index < queue.length && result.length < MAX_SCHEMATIC_SELECTION; index += 1) {
		const current = queue[index]
		if (!current) break
		result.push(current)
		for (const offset of offsets) {
			const next: SchematicBlockLocation = {
				regionId: start.regionId,
				position: [
					current.position[0] + offset[0],
					current.position[1] + offset[1],
					current.position[2] + offset[2],
				],
			}
			const key = schematicBlockKey(next)
			if (visited.has(key) || schematicBlockPaletteIndex(chunks, next) === 0) continue
			visited.add(key)
			queue.push(next)
		}
	}
	return result
}

export function schematicSelectionBounds(selection: SchematicBlockLocation[]) {
	if (selection.length === 0) return undefined
	const min = [...selection[0].position] as [number, number, number]
	const max = [...selection[0].position] as [number, number, number]
	for (const location of selection.slice(1)) {
		for (let axis = 0; axis < 3; axis += 1) {
			min[axis] = Math.min(min[axis], location.position[axis])
			max[axis] = Math.max(max[axis], location.position[axis])
		}
	}
	return { min, max }
}
