import type { Ref } from 'vue'
import { computed } from 'vue'

import type { ContentItem } from '../types'
import { useContentSearch } from './content-search'

export interface UseContentGroupingOptions {
	items: Ref<ContentItem[]>
	modpackItems?: Ref<ContentItem[] | undefined>
	sortItems: (items: ContentItem[]) => ContentItem[]
	getItemId: (item: ContentItem) => string
	searchKeys?: string[]
}

export function useContentGrouping(options: UseContentGroupingOptions) {
	const {
		items,
		modpackItems,
		sortItems,
		getItemId,
		searchKeys = ['project.title', 'owner.name', 'file_name'],
	} = options

	const sortedItems = computed(() => sortItems(items.value))

	const modpackItemsNoUpdate = computed(() => {
		const raw = modpackItems?.value ?? []
		return sortItems(
			raw.map((item) => ({
				...item,
				update: null,
			})),
		)
	})

	const modpackChildIdSet = computed(() => {
		return new Set((modpackItems?.value ?? []).map((item) => getItemId(item)))
	})

	const allItemsForSearch = computed(() => {
		return [...sortedItems.value, ...modpackItemsNoUpdate.value]
	})

	const { searchQuery, search } = useContentSearch(allItemsForSearch, searchKeys)

	const searchedAllItems = computed(() => {
		const modpackSearched = search(modpackItemsNoUpdate.value).filter((item) =>
			modpackChildIdSet.value.has(getItemId(item)),
		)
		const regularSearched = search(sortedItems.value).filter(
			(item) => !modpackChildIdSet.value.has(getItemId(item)),
		)
		return [...modpackSearched, ...regularSearched]
	})

	const searchableItemCount = computed(() => {
		const modpackItemsList = modpackItems?.value ?? []
		const regularItems = items.value.filter((item) => !modpackChildIdSet.value.has(getItemId(item)))
		return modpackItemsList.length + regularItems.length
	})

	return {
		searchQuery,
		sortedItems,
		modpackItemsNoUpdate,
		modpackChildIdSet,
		allItemsForSearch,
		searchedAllItems,
		searchableItemCount,
		search,
	}
}
