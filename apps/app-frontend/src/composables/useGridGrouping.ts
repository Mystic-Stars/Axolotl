import { useStorage } from '@vueuse/core'
import dayjs from 'dayjs'
import type { Ref } from 'vue'
import { computed } from 'vue'

export const UNGROUPED_GROUP_KEY = '__ungrouped__'

export type GridGroupingOption = 'Group' | 'Loader' | 'Game version' | 'None'
export type GridSortOption =
	| 'Name'
	| 'Last played'
	| 'Date created'
	| 'Date modified'
	| 'Game version'

export interface GridDisplayState {
	group: GridGroupingOption
	sortBy: GridSortOption
	collapsedGroups: string[]
}

export function useGridGrouping<T extends Record<string, any>>(
	label: string,
	instances: Ref<T[]>,
	options: {
		getSearchText?: (instance: T) => string
		getLoader?: (instance: T) => string
		getGameVersion?: (instance: T) => string
		getGroups?: (instance: T) => string[]
		getLastPlayed?: (instance: T) => string | number | null
		getDateCreated?: (instance: T) => string
		getDateModified?: (instance: T) => string
		formatLoader?: (loader: string) => string
	} = {},
) {
	const state = useStorage<GridDisplayState>(
		`${label}-grid-display-state`,
		{
			group: 'Group',
			sortBy: 'Name',
			collapsedGroups: [],
		},
		localStorage,
		{ mergeDefaults: true },
	)

	const grouping = computed(() => state.value.group)
	const collapsedSectionKeys = computed(() => new Set(state.value.collapsedGroups ?? []))

	const getSectionKey = (sectionName: string) => `${state.value.group}:${sectionName}`

	const isSectionCollapsed = (sectionName: string) => {
		return collapsedSectionKeys.value.has(getSectionKey(sectionName))
	}

	const setSectionCollapsed = (sectionName: string, collapsed: boolean) => {
		const sectionKey = getSectionKey(sectionName)
		const collapsedSections = new Set(state.value.collapsedGroups ?? [])

		if (collapsed) {
			collapsedSections.add(sectionKey)
		} else {
			collapsedSections.delete(sectionKey)
		}

		state.value.collapsedGroups = [...collapsedSections]
	}

	function sortInstances(instances: T[], sortBy: GridSortOption): T[] {
		const sorted = [...instances]
		const getGameVersion = options.getGameVersion ?? ((i: T) => i.game_version ?? '')
		const getLastPlayed = options.getLastPlayed ?? ((i: T) => i.last_played ?? 0)
		const getDateCreated = options.getDateCreated ?? ((i: T) => i.date_created ?? '')
		const getDateModified = options.getDateModified ?? ((i: T) => i.date_modified ?? '')

		switch (sortBy) {
			case 'Name':
				sorted.sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''))
				break
			case 'Game version':
				sorted.sort((a, b) =>
					getGameVersion(a).localeCompare(getGameVersion(b), undefined, { numeric: true }),
				)
				break
			case 'Last played':
				sorted.sort((a, b) => dayjs(getLastPlayed(b)).diff(dayjs(getLastPlayed(a))))
				break
			case 'Date created':
				sorted.sort((a, b) => dayjs(getDateCreated(b)).diff(dayjs(getDateCreated(a))))
				break
			case 'Date modified':
				sorted.sort((a, b) => dayjs(getDateModified(b)).diff(dayjs(getDateModified(a))))
				break
		}

		return sorted
	}

	function groupInstances(instances: T[], group: GridGroupingOption): Map<string, T[]> {
		const instanceMap = new Map<string, T[]>()
		const getLoader = options.getLoader ?? ((i: T) => i.loader ?? '')
		const getGameVersion = options.getGameVersion ?? ((i: T) => i.game_version ?? '')
		const getGroups = options.getGroups ?? ((i: T) => i.groups ?? [])
		const formatLoader = options.formatLoader ?? ((l: string) => l)

		switch (group) {
			case 'Loader':
				instances.forEach((instance) => {
					const loader = formatLoader(getLoader(instance))
					if (!instanceMap.has(loader)) {
						instanceMap.set(loader, [])
					}
					instanceMap.get(loader)!.push(instance)
				})
				break

			case 'Game version':
				instances.forEach((instance) => {
					const gameVersion = getGameVersion(instance)
					if (!instanceMap.has(gameVersion)) {
						instanceMap.set(gameVersion, [])
					}
					instanceMap.get(gameVersion)!.push(instance)
				})
				break

			case 'Group':
				instances.forEach((instance) => {
					const groups = getGroups(instance)
					const category = groups.length > 0 ? groups[0] : UNGROUPED_GROUP_KEY

					if (!instanceMap.has(category)) {
						instanceMap.set(category, [])
					}
					instanceMap.get(category)!.push(instance)
				})
				break

			case 'None':
			default:
				instanceMap.set('None', instances)
				break
		}

		return instanceMap
	}

	function sortSections(
		instanceMap: Map<string, T[]>,
		group: GridGroupingOption,
	): Map<string, T[]> {
		if (group !== 'Group' && group !== 'Game version') {
			return instanceMap
		}

		const sortedEntries = [...instanceMap.entries()].sort((a, b) => {
			if (group === 'Group') {
				if (a[0] === UNGROUPED_GROUP_KEY && b[0] !== UNGROUPED_GROUP_KEY) return -1
				if (a[0] !== UNGROUPED_GROUP_KEY && b[0] === UNGROUPED_GROUP_KEY) return 1
			}
			return a[0].localeCompare(b[0], undefined, { numeric: true })
		})

		instanceMap.clear()
		sortedEntries.forEach((entry) => instanceMap.set(entry[0], entry[1]))
		return instanceMap
	}

	const filteredResults = computed(() => {
		const { group = 'Group', sortBy = 'Name' } = state.value

		const sorted = sortInstances(instances.value, sortBy)
		const grouped = groupInstances(sorted, group)
		return sortSections(grouped, group)
	})

	return {
		state,
		grouping,
		filteredResults,
		isSectionCollapsed,
		setSectionCollapsed,
		UNGROUPED_GROUP_KEY,
	}
}
