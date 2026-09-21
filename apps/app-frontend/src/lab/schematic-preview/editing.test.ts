import assert from 'node:assert/strict'
import test from 'node:test'

import {
	filterSchematicAirGeometry,
	isSchematicAir,
	measureSchematicPoints,
	normalizeSchematicAirBlocks,
	schematicBlockPaletteIndex,
	schematicBlockStateKey,
	type SchematicCachedChunk,
	schematicChunkKey,
	selectConnectedSchematicBlocks,
	selectSchematicBlockState,
	selectSchematicCuboid,
	selectSchematicLayer,
	selectSchematicMaterial,
} from './editing.ts'

test('air block detection tolerates noncanonical casing and whitespace', () => {
	assert.equal(isSchematicAir(' Minecraft:Air '), true)
	assert.equal(isSchematicAir('minecraft:CAVE_AIR'), true)
	assert.equal(isSchematicAir('minecraft:void_air'), true)
	assert.equal(isSchematicAir('minecraft:light'), true)
	assert.equal(isSchematicAir('minecraft:barrier'), true)
	assert.equal(isSchematicAir('minecraft:structure_void'), true)
	assert.equal(isSchematicAir('minecraft:stone'), false)
})

test('chunk normalization collapses every air palette entry to index zero', () => {
	const blocks = new Uint32Array([1, 2, 3, 2])
	const normalized = normalizeSchematicAirBlocks(blocks, [
		{ name: 'minecraft:air', properties: {} },
		{ name: 'minecraft:stone', properties: {} },
		{ name: ' Minecraft:CAVE_AIR ', properties: {} },
		{ name: 'minecraft:void_air', properties: {} },
		{ name: 'minecraft:barrier', properties: {} },
	])

	assert.deepEqual([...normalized], [1, 0, 0, 0])
	assert.deepEqual([...blocks], [1, 2, 3, 2])
})

test('mesh validation removes stale geometry whose cached block is air', () => {
	const chunk: SchematicCachedChunk = {
		regionId: 'region-0',
		position: [0, 0, 0],
		blocks: new Uint32Array(4096),
	}
	chunk.blocks[1] = 1
	const mesh = {
		positions: new Float32Array([0, 0, 0, 1, 0, 0]),
		normals: new Float32Array([0, 1, 0, 0, 1, 0]),
		uvs: new Float32Array([0, 0, 1, 0]),
		colors: new Float32Array([1, 1, 1, 1, 1, 1]),
		blockPositions: new Float32Array([0, 0, 0, 1, 0, 0]),
	}
	const filtered = filterSchematicAirGeometry(mesh, chunk, [
		{ name: 'minecraft:air', properties: {} },
		{ name: 'minecraft:stone', properties: {} },
	])

	assert.deepEqual([...filtered.blockPositions], [1, 0, 0])
	assert.deepEqual([...filtered.positions], [1, 0, 0])
})

test('point measurement reports signed offsets, inclusive size, and distance', () => {
	const measurement = measureSchematicPoints([4, -2, 10], [-2, 1, 2])
	assert.deepEqual(measurement.delta, [-6, 3, -8])
	assert.deepEqual(measurement.size, [7, 4, 9])
	assert.equal(measurement.distance, Math.sqrt(109))
})

function fixture() {
	const blocks = new Uint32Array(4096)
	blocks[0] = 1
	blocks[1] = 1
	blocks[16] = 2
	blocks[256] = 2
	const chunk: SchematicCachedChunk = { regionId: 'region-0', position: [-1, 0, 0], blocks }
	return new Map([[schematicChunkKey(chunk.regionId, chunk.position), chunk]])
}

test('palette lookup handles negative chunk coordinates', () => {
	assert.equal(
		schematicBlockPaletteIndex(fixture(), { regionId: 'region-0', position: [-16, 0, 0] }),
		1,
	)
})

test('block state keys preserve properties for states sharing a block name', () => {
	assert.notEqual(
		schematicBlockStateKey({ name: 'minecraft:oak_slab', properties: { type: 'top' } }),
		schematicBlockStateKey({ name: 'minecraft:oak_slab', properties: { type: 'bottom' } }),
	)
	assert.equal(
		schematicBlockStateKey({
			name: 'minecraft:oak_slab',
			properties: { waterlogged: 'false', type: 'top' },
		}),
		'minecraft:oak_slab[type=top,waterlogged=false]',
	)
})

test('cuboid selection skips air and keeps the selected region', () => {
	const selected = selectSchematicCuboid(
		fixture(),
		{ regionId: 'region-0', position: [-16, 0, 0] },
		{ regionId: 'region-0', position: [-15, 1, 1] },
	)
	assert.equal(selected.length, 4)
	assert.ok(selected.every((location) => location.regionId === 'region-0'))
})

test('material, layer, and connected expansion use cached block data', () => {
	const chunks = fixture()
	const palette = [
		{ name: 'minecraft:air', properties: {} },
		{ name: 'minecraft:stone', properties: {} },
		{ name: 'minecraft:dirt', properties: {} },
	]
	assert.equal(selectSchematicMaterial(chunks, palette, 'minecraft:dirt').length, 2)
	const stateChunks = fixture()
	stateChunks.get(schematicChunkKey('region-0', [-1, 0, 0]))!.blocks[2] = 3
	assert.equal(
		selectSchematicBlockState(
			stateChunks,
			[...palette, { name: 'minecraft:dirt', properties: { type: 'top' } }],
			{ name: 'minecraft:dirt', properties: {} },
		).length,
		2,
	)
	assert.equal(selectSchematicLayer(chunks, 0).length, 3)
	assert.equal(
		selectConnectedSchematicBlocks(chunks, {
			regionId: 'region-0',
			position: [-16, 0, 0],
		}).length,
		4,
	)
})
