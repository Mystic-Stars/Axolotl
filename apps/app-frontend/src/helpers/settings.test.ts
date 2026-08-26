import assert from 'node:assert/strict'
import test from 'node:test'

import {
	getLastBrowseContentDisplayMode,
	getLastBrowseContentProjectType,
	setLastBrowseContentDisplayMode,
	setLastBrowseContentProjectType,
} from './browse-display-mode.ts'
import { getLastLibraryDisplayMode, setLastLibraryDisplayMode } from './library-display-mode.ts'
import { getSidebarExpanded, setSidebarExpanded } from './sidebar-state.ts'

const storageKey = 'axolotl-browse-content-display-mode'
const projectTypeStorageKey = 'axolotl-browse-content-project-type'
const libraryDisplayModeStorageKey = 'axolotl-library-display-mode'
const sidebarStorageKey = 'axolotl-right-sidebar-expanded'
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

test('browse display mode persists valid values and falls back to the list', () => {
	const values = installMemoryStorage()

	try {
		assert.equal(getLastBrowseContentDisplayMode(), 'list')

		setLastBrowseContentDisplayMode('compact')
		assert.equal(getLastBrowseContentDisplayMode(), 'compact')

		setLastBrowseContentDisplayMode('grid')
		assert.equal(getLastBrowseContentDisplayMode(), 'grid')

		values.set(storageKey, 'invalid')
		assert.equal(getLastBrowseContentDisplayMode(), 'list')
	} finally {
		restoreStorage()
	}
})

test('browse project type persists content types and rejects non-content routes', () => {
	const values = installMemoryStorage()

	try {
		assert.equal(getLastBrowseContentProjectType(), 'modpack')

		setLastBrowseContentProjectType('mod')
		assert.equal(getLastBrowseContentProjectType(), 'mod')

		setLastBrowseContentProjectType('world')
		assert.equal(getLastBrowseContentProjectType(), 'world')

		values.set(projectTypeStorageKey, 'server')
		assert.equal(getLastBrowseContentProjectType(), 'modpack')
	} finally {
		restoreStorage()
	}
})

test('library display mode persists cards and falls back to the standard grid', () => {
	const values = installMemoryStorage()

	try {
		assert.equal(getLastLibraryDisplayMode(), 'standard')

		setLastLibraryDisplayMode('cards')
		assert.equal(getLastLibraryDisplayMode(), 'cards')

		values.set(libraryDisplayModeStorageKey, 'invalid')
		assert.equal(getLastLibraryDisplayMode(), 'standard')
	} finally {
		restoreStorage()
	}
})

test('right sidebar expansion persists and defaults to expanded', () => {
	const values = installMemoryStorage()

	try {
		assert.equal(getSidebarExpanded(), true)

		setSidebarExpanded(false)
		assert.equal(getSidebarExpanded(), false)

		setSidebarExpanded(true)
		assert.equal(getSidebarExpanded(), true)

		values.set(sidebarStorageKey, 'invalid')
		assert.equal(getSidebarExpanded(), true)
	} finally {
		restoreStorage()
	}
})
