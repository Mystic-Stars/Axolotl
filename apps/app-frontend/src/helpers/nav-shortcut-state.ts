const NAV_SHORTCUT_STORAGE_PREFIX = 'axolotl-shortcut-nav-'

/**
 * Whether a navigation shortcut is enabled. Shortcuts are off by default;
 * each one is turned on individually from the Shortcut settings page.
 */
export function getNavShortcutEnabled(id: string): boolean {
	const value = localStorage.getItem(NAV_SHORTCUT_STORAGE_PREFIX + id)
	return value === 'true'
}

export function setNavShortcutEnabled(id: string, enabled: boolean) {
	localStorage.setItem(NAV_SHORTCUT_STORAGE_PREFIX + id, String(enabled))
}
