import assert from 'node:assert/strict'
import test from 'node:test'

import {
	clearSeedMapHistory,
	loadSeedMapHistory,
	recordSeedMapHistory,
	removeSeedMapHistoryEntry,
	sanitizeSeedMapHistory,
	SEED_MAP_HISTORY_LIMIT,
	seedMapHistoryId,
	updateSeedMapHistoryProgress,
} from './history.ts'

function memoryStorage(): Pick<Storage, 'getItem' | 'setItem'> {
	const values = new Map<string, string>()
	return {
		getItem: (key) => values.get(key) ?? null,
		setItem: (key, value) => void values.set(key, value),
	}
}

test('recording keeps newest entries first and dedupes revisits', () => {
	const storage = memoryStorage()
	recordSeedMapHistory(
		{ seed: '111', edition: 'java', gameVersion: '26.2', source: 'manual' },
		1_000,
		storage,
	)
	recordSeedMapHistory(
		{ seed: '222', edition: 'java', gameVersion: '26.2', source: 'random' },
		2_000,
		storage,
	)
	const entries = recordSeedMapHistory(
		{ seed: '111', edition: 'java', gameVersion: '1.21.3', source: 'manual' },
		3_000,
		storage,
	)
	assert.equal(entries.length, 2)
	assert.equal(entries[0].seed, '111')
	assert.equal(entries[0].gameVersion, '1.21.3')
	assert.equal(entries[0].firstViewedAt, 1_000)
	assert.equal(entries[0].lastViewedAt, 3_000)
	assert.equal(entries[1].seed, '222')
})

test('instance attribution survives a later manual revisit', () => {
	const storage = memoryStorage()
	recordSeedMapHistory(
		{
			seed: '10292992',
			edition: 'java',
			gameVersion: '26.2',
			source: 'instance',
			instanceName: 'Fabric 26.2',
			worldName: '生存世界',
		},
		1_000,
		storage,
	)
	const entries = recordSeedMapHistory(
		{ seed: '10292992', edition: 'java', gameVersion: '26.2', source: 'manual' },
		2_000,
		storage,
	)
	assert.equal(entries[0].source, 'instance')
	assert.equal(entries[0].instanceName, 'Fabric 26.2')
	assert.equal(entries[0].worldName, '生存世界')
	assert.equal(entries[0].lastViewedAt, 2_000)
})

test('the same seed in different editions stays as separate entries', () => {
	const storage = memoryStorage()
	recordSeedMapHistory(
		{ seed: '7', edition: 'java', gameVersion: '26.2', source: 'manual' },
		1_000,
		storage,
	)
	const entries = recordSeedMapHistory(
		{ seed: '7', edition: 'java-large-biomes', gameVersion: '26.2', source: 'manual' },
		2_000,
		storage,
	)
	assert.equal(entries.length, 2)
	assert.notEqual(entries[0].id, entries[1].id)
	assert.equal(seedMapHistoryId('7', 'java'), 'java:7')
})

test('history is capped and blank seeds are ignored', () => {
	const storage = memoryStorage()
	for (let index = 0; index < SEED_MAP_HISTORY_LIMIT + 10; index++) {
		recordSeedMapHistory(
			{ seed: `seed-${index}`, edition: 'java', gameVersion: '26.2', source: 'manual' },
			index,
			storage,
		)
	}
	recordSeedMapHistory(
		{ seed: '   ', edition: 'java', gameVersion: '26.2', source: 'manual' },
		99_999,
		storage,
	)
	const entries = loadSeedMapHistory(storage)
	assert.equal(entries.length, SEED_MAP_HISTORY_LIMIT)
	assert.equal(entries[0].seed, `seed-${SEED_MAP_HISTORY_LIMIT + 9}`)
})

test('remove and clear update the persisted list', () => {
	const storage = memoryStorage()
	recordSeedMapHistory(
		{ seed: 'a', edition: 'java', gameVersion: '26.2', source: 'manual' },
		1_000,
		storage,
	)
	recordSeedMapHistory(
		{ seed: 'b', edition: 'java', gameVersion: '26.2', source: 'manual' },
		2_000,
		storage,
	)
	const afterRemove = removeSeedMapHistoryEntry(seedMapHistoryId('a', 'java'), storage)
	assert.deepEqual(
		afterRemove.map((entry) => entry.seed),
		['b'],
	)
	assert.deepEqual(clearSeedMapHistory(storage), [])
	assert.deepEqual(loadSeedMapHistory(storage), [])
})

test('exploration progress is stored per seed and survives revisits', () => {
	const storage = memoryStorage()
	recordSeedMapHistory(
		{ seed: 'base', edition: 'java', gameVersion: '26.2', source: 'manual' },
		1_000,
		storage,
	)
	const id = seedMapHistoryId('base', 'java')
	updateSeedMapHistoryProgress(id, ['village:0:0'], ['diamond:1:2:3'], storage)
	let entries = loadSeedMapHistory(storage)
	assert.deepEqual(entries[0].completedFeatures, ['village:0:0'])
	assert.deepEqual(entries[0].completedOres, ['diamond:1:2:3'])

	entries = recordSeedMapHistory(
		{ seed: 'base', edition: 'java', gameVersion: '26.2', source: 'manual' },
		2_000,
		storage,
	)
	assert.deepEqual(entries[0].completedFeatures, ['village:0:0'])

	entries = recordSeedMapHistory(
		{
			seed: 'base',
			edition: 'java',
			gameVersion: '26.2',
			source: 'manual',
			completedFeatures: ['village:0:0', 'monument:5:5'],
			completedOres: [],
		},
		3_000,
		storage,
	)
	assert.deepEqual(entries[0].completedFeatures, ['village:0:0', 'monument:5:5'])
	assert.deepEqual(entries[0].completedOres, [])
	assert.deepEqual(
		updateSeedMapHistoryProgress(seedMapHistoryId('missing', 'java'), ['x'], [], storage).map(
			(entry) => entry.seed,
		),
		['base'],
	)
})

test('sanitizing drops malformed entries and duplicate ids', () => {
	const entries = sanitizeSeedMapHistory([
		{ seed: 'ok', edition: 'java', gameVersion: '26.2', source: 'share', lastViewedAt: 10 },
		{ seed: 'ok', edition: 'java', gameVersion: '26.2', source: 'manual', lastViewedAt: 20 },
		{ seed: '', edition: 'java' },
		{ seed: 42 },
		null,
		'nonsense',
		{ seed: 'weird-source', edition: 'java', gameVersion: '26.2', source: 'nope' },
	])
	assert.deepEqual(
		entries.map((entry) => entry.seed),
		['ok', 'weird-source'],
	)
	assert.equal(entries[0].source, 'share')
	assert.equal(entries[1].source, 'manual')
})
