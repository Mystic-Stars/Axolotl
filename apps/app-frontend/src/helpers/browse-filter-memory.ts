import type { FilterValue } from '@modrinth/ui'

export interface BrowseFilterMemory {
	filters: FilterValue[]
	toggledGroups: string[]
	overriddenProvidedFilterTypes: string[]
}

const BROWSE_FILTER_MEMORY_STORAGE_KEY = 'axolotl-browse-filter-memory-v1'

function isFilterValue(value: unknown): value is FilterValue {
	if (!value || typeof value !== 'object') return false
	const filter = value as Partial<FilterValue>
	return (
		typeof filter.type === 'string' &&
		typeof filter.option === 'string' &&
		(filter.negative === undefined || typeof filter.negative === 'boolean')
	)
}

function isStringArray(value: unknown): value is string[] {
	return Array.isArray(value) && value.every((item) => typeof item === 'string')
}

function parseFilterMemory(value: unknown): BrowseFilterMemory | null {
	if (!value || typeof value !== 'object') return null
	const memory = value as Partial<BrowseFilterMemory>
	if (!Array.isArray(memory.filters) || !memory.filters.every(isFilterValue)) return null
	if (!isStringArray(memory.toggledGroups)) return null
	if (!isStringArray(memory.overriddenProvidedFilterTypes)) return null
	return {
		filters: memory.filters.map((filter) => ({ ...filter })),
		toggledGroups: [...memory.toggledGroups],
		overriddenProvidedFilterTypes: [...memory.overriddenProvidedFilterTypes],
	}
}

function readFilterMemories(): Record<string, unknown> {
	try {
		const parsed = JSON.parse(localStorage.getItem(BROWSE_FILTER_MEMORY_STORAGE_KEY) ?? '{}')
		return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {}
	} catch {
		return {}
	}
}

export function getBrowseFilterMemory(projectType: string): BrowseFilterMemory | null {
	return parseFilterMemory(readFilterMemories()[projectType])
}

export function setBrowseFilterMemory(projectType: string, memory: BrowseFilterMemory) {
	const memories = readFilterMemories()
	memories[projectType] = {
		filters: memory.filters.map((filter) => ({ ...filter })),
		toggledGroups: [...memory.toggledGroups],
		overriddenProvidedFilterTypes: [...memory.overriddenProvidedFilterTypes],
	}
	localStorage.setItem(BROWSE_FILTER_MEMORY_STORAGE_KEY, JSON.stringify(memories))
}
