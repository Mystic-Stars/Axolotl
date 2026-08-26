export type LabCategoryFilter = 'all' | 'creation' | 'maintenance' | 'world'
export type LabFavoriteFilter = 'all' | 'favorite' | 'unfavorite'

const LAB_FAVORITE_TOOL_IDS_STORAGE_KEY = 'axolotl-lab-favorite-tool-ids'
const LAB_CATEGORY_FILTER_STORAGE_KEY = 'axolotl-lab-category-filter'
const LAB_FAVORITE_FILTER_STORAGE_KEY = 'axolotl-lab-favorite-filter'

function readStringArray(key: string): string[] {
	try {
		const parsed = JSON.parse(globalThis.localStorage?.getItem(key) ?? '[]')
		return Array.isArray(parsed)
			? parsed.filter((item): item is string => typeof item === 'string')
			: []
	} catch {
		return []
	}
}

export function getLabFavoriteToolIds(): string[] {
	return readStringArray(LAB_FAVORITE_TOOL_IDS_STORAGE_KEY)
}

export function setLabFavoriteToolIds(ids: string[]) {
	globalThis.localStorage?.setItem(LAB_FAVORITE_TOOL_IDS_STORAGE_KEY, JSON.stringify(ids))
}

export function isLabCategoryFilter(value: string): value is LabCategoryFilter {
	return value === 'creation' || value === 'maintenance' || value === 'world'
}

export function getLabCategoryFilter(): LabCategoryFilter {
	const value = globalThis.localStorage?.getItem(LAB_CATEGORY_FILTER_STORAGE_KEY)
	return value && isLabCategoryFilter(value) ? value : 'all'
}

export function setLabCategoryFilter(filter: LabCategoryFilter) {
	if (filter === 'all') globalThis.localStorage?.removeItem(LAB_CATEGORY_FILTER_STORAGE_KEY)
	else globalThis.localStorage?.setItem(LAB_CATEGORY_FILTER_STORAGE_KEY, filter)
}

export function isLabFavoriteFilter(value: string): value is LabFavoriteFilter {
	return value === 'favorite' || value === 'unfavorite'
}

export function getLabFavoriteFilter(): LabFavoriteFilter {
	const value = globalThis.localStorage?.getItem(LAB_FAVORITE_FILTER_STORAGE_KEY)
	return value && isLabFavoriteFilter(value) ? value : 'all'
}

export function setLabFavoriteFilter(filter: LabFavoriteFilter) {
	if (filter === 'all') globalThis.localStorage?.removeItem(LAB_FAVORITE_FILTER_STORAGE_KEY)
	else globalThis.localStorage?.setItem(LAB_FAVORITE_FILTER_STORAGE_KEY, filter)
}
