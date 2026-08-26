import assert from 'node:assert/strict'
import test from 'node:test'

import {
	clearPinnedContentViewPreferences,
	getPinnedContentViewPreferences,
	setPinnedContentViewPreferences,
} from './content-view-state.ts'

const originalStorageDescriptor = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')

function installMemoryStorage() {
	const values = new Map<string, string>()
	Object.defineProperty(globalThis, 'localStorage', {
		configurable: true,
		value: {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => values.set(key, value),
			removeItem: (key: string) => values.delete(key),
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

test('keeps pinned content views isolated by instance', () => {
	installMemoryStorage()

	try {
		assert.equal(
			setPinnedContentViewPreferences('instance-a', {
				sortMode: 'file-name-asc',
				typeFilters: ['mod'],
				statusFilters: ['disabled'],
				metadataExcluded: { state: ['enabled'] },
			}),
			true,
		)
		setPinnedContentViewPreferences('instance-b', {
			sortMode: 'date-added-newest',
			typeFilters: ['shader'],
			statusFilters: [],
			metadataExcluded: {},
		})

		assert.deepEqual(getPinnedContentViewPreferences('instance-a'), {
			version: 1,
			sortMode: 'file-name-asc',
			typeFilters: ['mod'],
			statusFilters: ['disabled'],
			metadataExcluded: { state: ['enabled'] },
		})
		assert.equal(getPinnedContentViewPreferences('missing'), null)

		clearPinnedContentViewPreferences('instance-a')
		assert.equal(getPinnedContentViewPreferences('instance-a'), null)
	} finally {
		restoreStorage()
	}
})

test('ignores malformed or unsupported pinned preferences', () => {
	const values = installMemoryStorage()

	try {
		values.set('axolotl-content-view-preferences-v1:broken', '{invalid json')
		assert.equal(getPinnedContentViewPreferences('broken'), null)

		values.set(
			'axolotl-content-view-preferences-v1:unsupported',
			JSON.stringify({
				version: 2,
				sortMode: 'file-name-asc',
				typeFilters: [],
				statusFilters: [],
				metadataExcluded: {},
			}),
		)
		assert.equal(getPinnedContentViewPreferences('unsupported'), null)
	} finally {
		restoreStorage()
	}
})
