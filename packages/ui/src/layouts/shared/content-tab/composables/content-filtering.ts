import type { MaybeRefOrGetter, Ref } from 'vue'
import { computed, ref, toValue, watch } from 'vue'

import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonProjectTypeCategoryMessages, normalizeProjectType } from '#ui/utils/common-messages'

import type { ClientWarningType, ContentItem } from '../types'

const CLIENT_ONLY_ENVIRONMENTS = new Set(['client_only', 'singleplayer_only'])

export function isClientOnlyEnvironment(env?: string | null): boolean {
	return !!env && CLIENT_ONLY_ENVIRONMENTS.has(env)
}

export function getClientWarningType(item: ContentItem): ClientWarningType | null {
	if (item.pack_client_retained) return 'retained'
	if (item.pack_client_depends) return 'depends'
	if (isClientOnlyEnvironment(item.environment)) return 'environment'
	return null
}

export interface ContentFilterOption {
	id: string
	label: string
}

export interface ContentFilterConfig {
	showTypeFilters?: MaybeRefOrGetter<boolean>
	showUpdateFilter?: MaybeRefOrGetter<boolean>
	showWarningsFilter?: MaybeRefOrGetter<boolean>
	isPackLocked?: Ref<boolean>
	persistKey?: MaybeRefOrGetter<string>
}

const messages = defineMessages({
	updates: {
		id: 'content.filter.updates',
		defaultMessage: '可更新',
	},
	warnings: {
		id: 'content.filter.warnings',
		defaultMessage: 'Warnings',
	},
	enabled: {
		id: 'content.filter.enabled',
		defaultMessage: 'Enabled',
	},
	disabled: {
		id: 'content.filter.disabled',
		defaultMessage: 'Disabled',
	},
})

function readSessionJSON<T>(key: string, fallback: T): T {
	try {
		const raw = sessionStorage.getItem(key)
		return raw !== null ? JSON.parse(raw) : fallback
	} catch {
		return fallback
	}
}

function writeSessionJSON(key: string, value: unknown) {
	try {
		sessionStorage.setItem(key, JSON.stringify(value))
	} catch {
		// 存储空间不足时静默失败
	}
}

export function useContentFilters(items: Ref<ContentItem[]>, config?: ContentFilterConfig) {
	const { formatMessage } = useVIntl()

	const persistKey = computed(() => toValue(config?.persistKey) ?? '')
	const showTypeFilters = computed(() => toValue(config?.showTypeFilters) ?? false)
	const showUpdateFilter = computed(() => toValue(config?.showUpdateFilter) ?? false)
	const showWarningsFilter = computed(() => toValue(config?.showWarningsFilter) ?? false)

	const selectedTypeFilter = ref<string | null>(null)
	const selectedStatusFilters = ref<string[]>([])

	watch(
		persistKey,
		(newKey, oldKey) => {
			if (newKey && newKey !== oldKey) {
				// 从 sessionStorage 读取新 key 的持久化值
				selectedTypeFilter.value = readSessionJSON<string | null>(
					`content-filters-type:${newKey}`,
					null,
				)
				selectedStatusFilters.value = readSessionJSON<string[]>(
					`content-filters-status:${newKey}`,
					[],
				)
			} else if (newKey && oldKey === newKey) {
				// 首次初始化
				selectedTypeFilter.value = readSessionJSON<string | null>(
					`content-filters-type:${newKey}`,
					null,
				)
				selectedStatusFilters.value = readSessionJSON<string[]>(
					`content-filters-status:${newKey}`,
					[],
				)
			}
		},
		{ immediate: true },
	)

	watch(selectedTypeFilter, (val) => {
		if (persistKey.value) {
			writeSessionJSON(`content-filters-type:${persistKey.value}`, val)
		}
	})

	watch(
		selectedStatusFilters,
		(val) => {
			if (persistKey.value) {
				writeSessionJSON(`content-filters-status:${persistKey.value}`, val)
			}
		},
		{ deep: true },
	)

	const typeFilteredItems = computed(() => {
		if (!selectedTypeFilter.value) return items.value
		return items.value.filter(
			(item) => normalizeProjectType(item.project_type) === selectedTypeFilter.value,
		)
	})

	const statusFilteredItems = computed(() => {
		let result = items.value
		if (selectedStatusFilters.value.length > 0) {
			result = result.filter((item) => {
				for (const filter of selectedStatusFilters.value) {
					if (filter === 'updates' && item.update == null) return false
					if (filter === 'enabled' && !item.enabled) return false
					if (filter === 'disabled' && item.enabled) return false
					if (filter === 'warnings' && getClientWarningType(item) === null) return false
				}
				return true
			})
		}
		return result
	})

	const availableStatusFilters = computed<Array<'enabled' | 'disabled'>>(() => {
		const source = typeFilteredItems.value
		const hasEnabledContent = source.some((m) => m.enabled)
		const hasDisabledContent = source.some((m) => !m.enabled)

		return hasEnabledContent && hasDisabledContent ? ['enabled', 'disabled'] : []
	})

	const row1FilterOptions = computed<ContentFilterOption[]>(() => {
		const options: ContentFilterOption[] = []

		if (showTypeFilters.value) {
			const frequency = items.value.reduce((map: Record<string, number>, item) => {
				const normalized = normalizeProjectType(item.project_type)
				map[normalized] = (map[normalized] || 0) + 1
				return map
			}, {})
			const types = Object.keys(frequency).sort((a, b) => frequency[b] - frequency[a])
			for (const type of types) {
				const msg =
					commonProjectTypeCategoryMessages[type as keyof typeof commonProjectTypeCategoryMessages]
				const label = msg ? formatMessage(msg) : type.charAt(0).toUpperCase() + type.slice(1) + 's'
				options.push({ id: type, label })
			}
		}

		return options
	})

	const row2FilterOptions = computed<ContentFilterOption[]>(() => {
		const source = typeFilteredItems.value
		const options: ContentFilterOption[] = []

		if (showUpdateFilter.value && source.some((m) => m.update != null)) {
			options.push({ id: 'updates', label: formatMessage(messages.updates) })
		}

		if (showWarningsFilter.value && source.some((m) => getClientWarningType(m) !== null)) {
			options.push({ id: 'warnings', label: formatMessage(messages.warnings) })
		}

		for (const status of availableStatusFilters.value) {
			options.push({
				id: status,
				label: formatMessage(status === 'enabled' ? messages.enabled : messages.disabled),
			})
		}

		return options
	})

	const allFilterOptions = computed<ContentFilterOption[]>(() => {
		return [...row1FilterOptions.value, ...row2FilterOptions.value]
	})

	const totalCount = computed(() => statusFilteredItems.value.length)

	const filterCounts = computed(() => {
		const counts: Record<string, number> = {}

		const statusSource = statusFilteredItems.value
		for (const item of statusSource) {
			const type = normalizeProjectType(item.project_type)
			counts[type] = (counts[type] || 0) + 1
		}

		const source = typeFilteredItems.value

		counts['updates'] = source.filter((m) => m.update != null).length
		counts['enabled'] = source.filter((m) => m.enabled).length
		counts['disabled'] = source.filter((m) => !m.enabled).length
		counts['warnings'] = source.filter((m) => getClientWarningType(m) !== null).length

		return counts
	})

	watch(
		allFilterOptions,
		() => {
			const validIds = new Set(allFilterOptions.value.map((opt) => opt.id))
			if (selectedTypeFilter.value && !validIds.has(selectedTypeFilter.value)) {
				selectedTypeFilter.value = null
			}
			selectedStatusFilters.value = selectedStatusFilters.value.filter((f) => validIds.has(f))
		},
		{ immediate: true },
	)

	function toggleTypeFilter(filterId: string) {
		if (selectedTypeFilter.value !== filterId) {
			selectedTypeFilter.value = filterId
		}
	}

	function toggleStatusFilter(filterId: string) {
		if (filterId === 'enabled' || filterId === 'disabled') {
			const index = selectedStatusFilters.value.indexOf(filterId)
			const otherStatusFilter = filterId === 'enabled' ? 'disabled' : 'enabled'
			if (index === -1) {
				selectedStatusFilters.value = [
					...selectedStatusFilters.value.filter((filter) => filter !== otherStatusFilter),
					filterId,
				]
			} else {
				selectedStatusFilters.value.splice(index, 1)
			}
			return
		}

		const index = selectedStatusFilters.value.indexOf(filterId)
		if (index === -1) {
			selectedStatusFilters.value.push(filterId)
		} else {
			selectedStatusFilters.value.splice(index, 1)
		}
	}

	function applyFilters(source: ContentItem[]): ContentItem[] {
		let result = source

		if (selectedTypeFilter.value) {
			result = result.filter(
				(item) => normalizeProjectType(item.project_type) === selectedTypeFilter.value,
			)
		}

		if (selectedStatusFilters.value.length > 0) {
			result = result.filter((item) => {
				for (const filter of selectedStatusFilters.value) {
					if (filter === 'updates' && item.update == null) return false
					if (filter === 'enabled' && !item.enabled) return false
					if (filter === 'disabled' && item.enabled) return false
					if (filter === 'warnings' && getClientWarningType(item) === null) return false
				}
				return true
			})
		}

		return result
	}

	return {
		selectedTypeFilter,
		selectedStatusFilters,
		row1FilterOptions,
		row2FilterOptions,
		totalCount,
		filterCounts,
		toggleTypeFilter,
		toggleStatusFilter,
		applyFilters,
	}
}
