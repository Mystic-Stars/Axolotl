const SHOW_SCROLL_TOP_STORAGE_KEY = 'axolotl-show-scroll-top'
const QUICK_SCROLL_ENABLED_STORAGE_KEY = 'axolotl-quick-scroll-enabled'

export function getShowScrollTop(): boolean {
	const value = localStorage.getItem(SHOW_SCROLL_TOP_STORAGE_KEY)
	return value !== 'false'
}

export function setShowScrollTop(show: boolean) {
	localStorage.setItem(SHOW_SCROLL_TOP_STORAGE_KEY, String(show))
}

export function getQuickScrollEnabled(): boolean {
	const value = localStorage.getItem(QUICK_SCROLL_ENABLED_STORAGE_KEY)
	return value === 'true'
}

export function setQuickScrollEnabled(enabled: boolean) {
	localStorage.setItem(QUICK_SCROLL_ENABLED_STORAGE_KEY, String(enabled))
}
