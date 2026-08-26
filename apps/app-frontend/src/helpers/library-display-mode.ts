export type LibraryDisplayMode = 'standard' | 'cards'

const LIBRARY_DISPLAY_MODE_STORAGE_KEY = 'axolotl-library-display-mode'

export function getLastLibraryDisplayMode(): LibraryDisplayMode {
	const value = globalThis.localStorage?.getItem(LIBRARY_DISPLAY_MODE_STORAGE_KEY)
	return value === 'cards' ? value : 'standard'
}

export function setLastLibraryDisplayMode(mode: LibraryDisplayMode) {
	globalThis.localStorage?.setItem(LIBRARY_DISPLAY_MODE_STORAGE_KEY, mode)
}
