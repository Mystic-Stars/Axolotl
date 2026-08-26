export interface ContentFilterSelections {
	typeFilters: string[]
	statusFilters: string[]
}

export interface MetadataFilterOptions {
	key: string
	options: Array<{ value: string }>
}

function cloneMetadataExcluded(excluded: Record<string, string[]>): Record<string, string[]> {
	return Object.fromEntries(Object.entries(excluded).map(([key, values]) => [key, [...values]]))
}

export function pruneContentFilterSelections(
	selections: ContentFilterSelections,
	options: { type: string[]; status: string[] },
	optionsReady: boolean,
): ContentFilterSelections {
	if (!optionsReady) {
		return {
			typeFilters: [...selections.typeFilters],
			statusFilters: [...selections.statusFilters],
		}
	}

	const typeOptions = new Set(options.type)
	const statusOptions = new Set(options.status)
	return {
		typeFilters: selections.typeFilters.filter((filter) => typeOptions.has(filter)),
		statusFilters: selections.statusFilters.filter((filter) => statusOptions.has(filter)),
	}
}

export function pruneMetadataFilterSelections(
	excluded: Record<string, string[]>,
	categories: MetadataFilterOptions[],
	optionsReady: boolean,
): Record<string, string[]> {
	if (!optionsReady) return cloneMetadataExcluded(excluded)

	const categoriesByKey = new Map(categories.map((category) => [category.key, category]))
	const next: Record<string, string[]> = {}
	for (const [key, values] of Object.entries(excluded)) {
		const category = categoriesByKey.get(key)
		if (!category) continue
		const validValues = new Set(category.options.map((option) => option.value))
		const retained = values.filter((value) => validValues.has(value))
		if (retained.length > 0) next[key] = retained
	}
	return next
}
