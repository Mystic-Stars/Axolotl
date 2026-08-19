const SIDEBAR_EXPANDED_STORAGE_KEY = 'axolotl-right-sidebar-expanded'

export function getSidebarExpanded(): boolean {
	const value = localStorage.getItem(SIDEBAR_EXPANDED_STORAGE_KEY)
	return value !== 'false'
}

export function setSidebarExpanded(expanded: boolean) {
	localStorage.setItem(SIDEBAR_EXPANDED_STORAGE_KEY, String(expanded))
}
