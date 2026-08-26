import assert from 'node:assert/strict'
import test from 'node:test'

import { getBrowseFilterMemory, setBrowseFilterMemory } from './browse-filter-memory.ts'

const storageKey = 'axolotl-browse-filter-memory-v1'
const originalStorageDescriptor = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')

function installMemoryStorage() {
	const values = new Map<string, string>()
	Object.defineProperty(globalThis, 'localStorage', {
		configurable: true,
		value: {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => values.set(key, value),
		},
	})
	return values
}

function restoreStorage() {
	if (originalStorageDescriptor) {
		Object.defineProperty(globalThis, 'localStorage', originalStorageDescriptor)
	} else {
		delete (globalThis as { localStorage?: Storage }).localStorage
	}
}

test('browse filters are remembered independently for each project type', () => {
	installMemoryStorage()

	try {
		setBrowseFilterMemory('mod', {
			filters: [{ type: 'game_version', option: '1.21.1' }],
			toggledGroups: ['all_versions'],
			overriddenProvidedFilterTypes: ['game_version'],
		})
		setBrowseFilterMemory('modpack', {
			filters: [{ type: 'modpack_loader', option: 'neoforge' }],
			toggledGroups: [],
			overriddenProvidedFilterTypes: [],
		})

		assert.deepEqual(getBrowseFilterMemory('mod'), {
			filters: [{ type: 'game_version', option: '1.21.1' }],
			toggledGroups: ['all_versions'],
			overriddenProvidedFilterTypes: ['game_version'],
		})
		assert.deepEqual(getBrowseFilterMemory('modpack'), {
			filters: [{ type: 'modpack_loader', option: 'neoforge' }],
			toggledGroups: [],
			overriddenProvidedFilterTypes: [],
		})
		assert.equal(getBrowseFilterMemory('server'), null)
	} finally {
		restoreStorage()
	}
})

test('invalid browse filter memory is ignored', () => {
	const values = installMemoryStorage()

	try {
		values.set(storageKey, '{invalid json')
		assert.equal(getBrowseFilterMemory('mod'), null)

		values.set(
			storageKey,
			JSON.stringify({
				mod: {
					filters: [{ type: 'game_version', option: 121 }],
					toggledGroups: [],
					overriddenProvidedFilterTypes: [],
				},
			}),
		)
		assert.equal(getBrowseFilterMemory('mod'), null)
	} finally {
		restoreStorage()
	}
})
