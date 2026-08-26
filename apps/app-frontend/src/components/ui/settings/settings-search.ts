import Fuse from 'fuse.js'

export interface SettingsSearchDocument<T> {
	item: T
	text: string | string[]
}

export const MAX_SETTINGS_SEARCH_RESULTS = 20

export function normalizeSettingsSearchText(value: string): string {
	return value.trim().toLocaleLowerCase().replace(/\s+/g, ' ')
}

export function filterSettingsSearchDocuments<T>(
	query: string,
	documents: SettingsSearchDocument<T>[],
): SettingsSearchDocument<T>[] {
	const normalizedQuery = normalizeSettingsSearchText(query)
	if (!normalizedQuery) return []

	const search = new Fuse(documents, {
		ignoreLocation: true,
		keys: ['text'],
		threshold: 0.35,
	})

	return search
		.search(normalizedQuery, { limit: MAX_SETTINGS_SEARCH_RESULTS })
		.map((result) => result.item)
}
