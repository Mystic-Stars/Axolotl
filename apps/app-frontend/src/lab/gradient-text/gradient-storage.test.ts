import assert from 'node:assert/strict'
import test from 'node:test'

import {
	createDefaultGradientTextState,
	GRADIENT_TEXT_STORAGE_KEY,
	loadGradientTextState,
	parseGradientPresets,
	saveGradientTextState,
	serializeGradientPresets,
} from './gradient-storage.ts'

function createStorage(initial: Record<string, string> = {}) {
	const data = new Map(Object.entries(initial))
	return {
		getItem(key: string) {
			return data.get(key) ?? null
		},
		setItem(key: string, value: string) {
			data.set(key, value)
		},
	}
}

test('recovers from corrupt local storage', () => {
	const state = loadGradientTextState(createStorage({ [GRADIENT_TEXT_STORAGE_KEY]: '{bad json' }))
	assert.deepEqual(state.colors, ['#A855F7', '#22C55E'])
	assert.equal(state.adapterId, 'vanilla')
})

test('persists only valid normalized local state', () => {
	const storage = createStorage()
	const state = createDefaultGradientTextState()
	state.colors = ['#abc']
	state.adapterId = 'minimessage'
	saveGradientTextState(state, storage)
	assert.deepEqual(loadGradientTextState(storage).colors, ['#AABBCC'])
	assert.equal(loadGradientTextState(storage).adapterId, 'minimessage')
})

test('rejects invalid imported presets and round-trips valid presets', () => {
	const presets = parseGradientPresets([
		{ name: 'Sunset', colors: ['#FF0000', '#00FF00'] },
		{ name: 'Solid', colors: ['#112233'] },
		{ name: 'Broken', colors: ['not-a-color'] },
	])
	assert.equal(presets.length, 2)
	assert.deepEqual(JSON.parse(serializeGradientPresets(presets)), [
		{ name: 'Sunset', colors: ['#FF0000', '#00FF00'] },
		{ name: 'Solid', colors: ['#112233'] },
	])
})
