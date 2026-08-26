import type { ContentSortMode } from './content-sorting'

export interface ContentViewFilters {
	typeFilters: string[]
	statusFilters: string[]
	metadataExcluded: Record<string, string[]>
}

export interface ContentViewState extends ContentViewFilters {
	sortMode: ContentSortMode
	searchQuery: string
	metadataFilterExpanded: boolean
	expandedGroups: string[]
	scrollTop: number
	anchorId?: string
	anchorOffset?: number
}

export interface PinnedContentViewPreferencesV1 extends ContentViewFilters {
	version: 1
	sortMode: ContentSortMode
}

const STORAGE_PREFIX = 'axolotl-content-view-preferences-v1:'

const sortModes = new Set<ContentSortMode>([
	'project-name-asc',
	'project-name-desc',
	'file-name-asc',
	'file-name-desc',
	'date-added-newest',
	'date-added-oldest',
])

function isStringArray(value: unknown): value is string[] {
	return Array.isArray(value) && value.every((entry) => typeof entry === 'string')
}

function isMetadataExcluded(value: unknown): value is Record<string, string[]> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return false
	return Object.values(value).every(isStringArray)
}

function canUseStorage(): boolean {
	return typeof localStorage !== 'undefined'
}

function storageKey(instanceId: string): string {
	return `${STORAGE_PREFIX}${instanceId}`
}

export function cloneContentViewFilters(filters: ContentViewFilters): ContentViewFilters {
	return {
		typeFilters: [...filters.typeFilters],
		statusFilters: [...filters.statusFilters],
		metadataExcluded: Object.fromEntries(
			Object.entries(filters.metadataExcluded).map(([key, values]) => [key, [...values]]),
		),
	}
}

export function getPinnedContentViewPreferences(
	instanceId: string,
): PinnedContentViewPreferencesV1 | null {
	if (!canUseStorage()) return null

	try {
		const raw = localStorage.getItem(storageKey(instanceId))
		if (!raw) return null
		const parsed: unknown = JSON.parse(raw)
		if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return null
		const value = parsed as Partial<PinnedContentViewPreferencesV1>
		if (
			value.version !== 1 ||
			!value.sortMode ||
			!sortModes.has(value.sortMode) ||
			!isStringArray(value.typeFilters) ||
			!isStringArray(value.statusFilters) ||
			!isMetadataExcluded(value.metadataExcluded)
		) {
			return null
		}

		return {
			version: 1,
			sortMode: value.sortMode,
			...cloneContentViewFilters({
				typeFilters: value.typeFilters,
				statusFilters: value.statusFilters,
				metadataExcluded: value.metadataExcluded,
			}),
		}
	} catch {
		return null
	}
}

export function setPinnedContentViewPreferences(
	instanceId: string,
	preferences: Omit<PinnedContentViewPreferencesV1, 'version'>,
): boolean {
	if (!canUseStorage()) return false

	try {
		localStorage.setItem(
			storageKey(instanceId),
			JSON.stringify({
				version: 1,
				sortMode: preferences.sortMode,
				...cloneContentViewFilters(preferences),
			}),
		)
		return true
	} catch {
		return false
	}
}

export function clearPinnedContentViewPreferences(instanceId: string): void {
	if (!canUseStorage()) return

	try {
		localStorage.removeItem(storageKey(instanceId))
	} catch {
		// Storage failures must not block content management.
	}
}
