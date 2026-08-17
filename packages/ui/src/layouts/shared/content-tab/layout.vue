<script setup lang="ts">
import {
	ChevronUpIcon,
	CodeIcon,
	DownloadIcon,
	DropdownIcon,
	FileIcon,
	LinkIcon,
	ShareIcon,
	TextCursorInputIcon,
	TrashIcon,
} from '@modrinth/assets'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import OverflowMenu, {
	type Option as OverflowMenuOption,
} from '#ui/components/base/OverflowMenu.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages, normalizeProjectType } from '#ui/utils/common-messages'

import ContentCardTable from './components/ContentCardTable.vue'
import ContentEmptyState from './components/ContentEmptyState.vue'
import ContentMetadataFilterBar from './components/ContentMetadataFilterBar.vue'
import ContentSelectionBar from './components/ContentSelectionBar.vue'
import ContentTableHeaderActions from './components/ContentTableHeaderActions.vue'
import ContentToolbar from './components/ContentToolbar.vue'
import ContentTypeFilter from './components/ContentTypeFilter.vue'
import ConfirmBulkUpdateModal from './components/modals/ConfirmBulkUpdateModal.vue'
import ConfirmDeletionModal from './components/modals/ConfirmDeletionModal.vue'
import ConfirmUnlinkModal from './components/modals/ConfirmUnlinkModal.vue'
import ContentDependencyWarningModal from './components/modals/ContentDependencyWarningModal.vue'
import {
	canToggleContentItem,
	getClientWarningType,
	isClientOnlyEnvironment,
	isDisabledContentItem,
	isEnabledContentItem,
	isPresentContentItem,
	useBulkOperation,
	useChangingItems,
	useContentFolderGroups,
	useContentMetadataFilters,
	useContentPipeline,
	useContentSelection,
} from './composables'
import { injectContentManager } from './providers/content-manager'
import type {
	BulkOperationStatus,
	ContentCardTableItem,
	ContentItem,
	ContentWorldGroupMeta,
} from './types'

const { formatMessage, locale } = useVIntl()

const props = withDefaults(
	defineProps<{
		bottomPadding?: boolean
	}>(),
	{
		bottomPadding: true,
	},
)

const emit = defineEmits<{
	visibleItems: [items: ContentItem[]]
}>()

function handleVisibleTableItems(visibleTableItems: ContentCardTableItem[]) {
	const visibleContentItems = visibleTableItems
		.map((item) => findContentItem(item.id))
		.filter((item): item is ContentItem => item !== undefined)

	emit('visibleItems', visibleContentItems)
}

const messages = defineMessages({
	failedToLoad: {
		id: 'content.page-layout.failed-to-load',
		defaultMessage: 'Failed to load content',
	},
	sortAlphabetical: {
		id: 'content.page-layout.sort.alphabetical',
		defaultMessage: 'Alphabetical',
	},
	sortDateAddedNewest: {
		id: 'content.page-layout.sort.date-added-newest',
		defaultMessage: 'Newest first',
	},
	sortDateAddedOldest: {
		id: 'content.page-layout.sort.date-added-oldest',
		defaultMessage: 'Oldest first',
	},
	updateAll: {
		id: 'content.page-layout.update-all',
		defaultMessage: 'Update all',
	},
	noContentFound: {
		id: 'content.page-layout.no-content-found',
		defaultMessage: 'No content found.',
	},
	shareProjectNames: {
		id: 'content.page-layout.share.project-names',
		defaultMessage: 'Project names',
	},
	shareFileNames: {
		id: 'content.page-layout.share.file-names',
		defaultMessage: 'File names',
	},
	shareProjectLinks: {
		id: 'content.page-layout.share.project-links',
		defaultMessage: 'Project links',
	},
	shareMarkdownLinks: {
		id: 'content.page-layout.share.markdown-links',
		defaultMessage: 'Markdown links',
	},
	share: {
		id: 'content.page-layout.share.label',
		defaultMessage: 'Share',
	},
	sortByLabel: {
		id: 'content.page-layout.sort.label',
		defaultMessage: 'Sort by {mode}',
	},
})

const ctx = injectContentManager()
const skipNonEssentialWarnings = computed(() => ctx.skipNonEssentialWarnings?.value ?? false)

// window 级内存（导航切换保留，关软件丢弃）
const memory: Record<string, Map<string, unknown>> = ((
	window as unknown as { __ctMemory?: Record<string, Map<string, unknown>> }
).__ctMemory ??= {})
function getMap<K, V>(namespace: string): Map<K, V> {
	if (!memory[namespace]) memory[namespace] = new Map<string, unknown>()
	return memory[namespace] as Map<K, V>
}

function getItemId(item: ContentItem) {
	return ctx.getItemId?.(item) ?? item.file_path ?? item.file_name ?? item.id
}

function findContentItem(id: string): ContentItem | undefined {
	const item = ctx.items.value.find((i) => getItemId(i) === id)
	if (item) return item
	return ctx.modpackItems?.value?.find((i) => getItemId(i) === id)
}

// 排序方式（导航切换保留，关软件丢弃）
type SortMode = 'alphabetical-asc' | 'alphabetical-desc' | 'date-added-newest' | 'date-added-oldest'

const sortMemory = getMap<string, SortMode>('sort')
const sortMode = ref<SortMode>(
	ctx.instanceId ? (sortMemory.get(ctx.instanceId) ?? 'alphabetical-asc') : 'alphabetical-asc',
)

watch(sortMode, (val) => {
	if (ctx.instanceId) sortMemory.set(ctx.instanceId, val)
})

const sortLabels: Record<SortMode, () => string> = {
	'alphabetical-asc': () => formatMessage(messages.sortAlphabetical),
	'alphabetical-desc': () => formatMessage(messages.sortAlphabetical),
	'date-added-newest': () => formatMessage(messages.sortDateAddedNewest),
	'date-added-oldest': () => formatMessage(messages.sortDateAddedOldest),
}

function cycleSortMode() {
	const modes: SortMode[] = [
		'alphabetical-asc',
		'alphabetical-desc',
		'date-added-newest',
		'date-added-oldest',
	]
	const idx = modes.indexOf(sortMode.value)
	sortMode.value = modes[(idx + 1) % modes.length]
}

function sortItems(items: ContentItem[]): ContentItem[] {
	const arr = [...items]
	switch (sortMode.value) {
		case 'alphabetical-desc':
			return arr.sort((a, b) => {
				const nameA = a.project?.title ?? a.file_name
				const nameB = b.project?.title ?? b.file_name
				return (
					nameB.toLowerCase().localeCompare(nameA.toLowerCase()) ||
					a.file_name.localeCompare(b.file_name)
				)
			})
		case 'date-added-newest':
			return arr.sort((a, b) => {
				const dateA = a.date_added ?? ''
				const dateB = b.date_added ?? ''
				return dateB.localeCompare(dateA) || a.file_name.localeCompare(b.file_name)
			})
		case 'date-added-oldest':
			return arr.sort((a, b) => {
				const dateA = a.date_added ?? ''
				const dateB = b.date_added ?? ''
				return dateA.localeCompare(dateB) || a.file_name.localeCompare(b.file_name)
			})
		default:
			return arr.sort((a, b) => {
				const nameA = a.project?.title ?? a.file_name
				const nameB = b.project?.title ?? b.file_name
				return (
					nameA.toLowerCase().localeCompare(nameB.toLowerCase()) ||
					a.file_name.localeCompare(b.file_name)
				)
			})
	}
}

const {
	searchQuery,
	searchableItemCount,
	modpackItemsNoUpdate,
	modpackChildIdSet,
	selectedTypeFilter,
	row1FilterOptions,
	totalCount,
	filterCounts,
	filteredItems: pipelineFilteredItems,
	filteredModpackItems: pipelineFilteredModpackItems,
	toggleTypeFilter,
} = useContentPipeline({
	items: ctx.items,
	modpackItems: ctx.modpackItems,
	sortItems,
	getItemId,
	showTypeFilters: true,
	showUpdateFilter: ctx.hasUpdateSupport,
	showWarningsFilter: true,
	isPackLocked: ctx.isPackLocked,
	memoryKey: ctx.instanceId,
})

const {
	metadataFilterCategories,
	getSelectedValues: getMetadataSelectedValues,
	setCategorySelection,
	isCategoryFiltering,
	applyMetadataFilters,
} = useContentMetadataFilters(
	computed(() => [...ctx.items.value, ...(ctx.modpackItems?.value ?? [])]),
	ctx.instanceId,
)

const metadataFilterSelectedValues = computed(() =>
	Object.fromEntries(
		metadataFilterCategories.value.map((category) => [
			category.key,
			getMetadataSelectedValues(category.key),
		]),
	),
)
const metadataFilteringKeys = computed(() =>
	metadataFilterCategories.value
		.filter((category) => isCategoryFiltering(category.key))
		.map((category) => category.key),
)
const metadataFilterExpanded = ref(false)

// Metadata filters (作者/环境/状态/更新/类型/加载器/来源/外部文件/开源) apply on
// top of the search pipeline, so the whole table (including modpack groups)
// is filtered consistently.
const filteredItems = computed(() =>
	metadataFilterExpanded.value
		? applyMetadataFilters(pipelineFilteredItems.value)
		: pipelineFilteredItems.value,
)
const filteredModpackItems = computed(() =>
	metadataFilterExpanded.value
		? applyMetadataFilters(pipelineFilteredModpackItems.value)
		: pipelineFilteredModpackItems.value,
)

const { selectedIds, selectedItems, clearSelection, removeFromSelection } = useContentSelection(
	computed(() => {
		return [...modpackItemsNoUpdate.value, ...ctx.items.value]
	}),
	getItemId,
)

const {
	isBulkOperating,
	bulkProgress,
	bulkTotal,
	bulkOperation,
	bulkWaiting,
	runBulk,
	runBulkWithWaiting,
} = useBulkOperation()

// Sync bulk operation state back to the content manager so providers can suppress refreshes
if (ctx.isBulkOperating) {
	watch(isBulkOperating, (val) => {
		ctx.isBulkOperating!.value = val
	})
}

const { isChanging, markChanging, unmarkChanging } = useChangingItems()

const bulkStatusMessage = ref<string | null>(null)
const bulkItemCount = ref(0)

// 整合包分组展开状态（导航切换保留，关软件丢弃）
const expandedGroupsMemory = getMap<string, Set<string>>('expandedGroups')

const refreshing = ref(false)

const expandedGroups = ref<Set<string>>(
	ctx.instanceId ? (expandedGroupsMemory.get(ctx.instanceId) ?? new Set()) : new Set(),
)

function toggleGroupExpand(groupId: string) {
	const newSet = new Set(expandedGroups.value)
	if (newSet.has(groupId)) {
		newSet.delete(groupId)
	} else {
		newSet.add(groupId)
	}
	expandedGroups.value = newSet
	if (ctx.instanceId) expandedGroupsMemory.set(ctx.instanceId, newSet)
}

watch(searchQuery, (query) => {
	if (query.trim()) {
		expandedGroups.value = new Set([...expandedGroups.value, 'modpack'])
		if (ctx.instanceId) expandedGroupsMemory.set(ctx.instanceId, expandedGroups.value)
	}
})

const showScrollToTop = ref(false)
const sidebarVisible = ref(false)
const SCROLL_THRESHOLD = 300

function getScrollContainer(): Element | null {
	return document.querySelector('.app-viewport')
}

function checkSidebarVisibility() {
	const appContents = document.querySelector('.app-contents')
	sidebarVisible.value = appContents?.classList.contains('sidebar-enabled') ?? false
}

function handleScroll() {
	const container = getScrollContainer()
	if (container) {
		showScrollToTop.value = container.scrollTop > SCROLL_THRESHOLD
	}
}

function scrollToTop() {
	const container = getScrollContainer()
	if (container) {
		container.scrollTo({ top: 0, behavior: 'smooth' })
	}
}

onMounted(() => {
	const container = getScrollContainer()
	if (container) {
		container.addEventListener('scroll', handleScroll, { passive: true })
		handleScroll()
		checkSidebarVisibility()
	}
	const observer = new MutationObserver(() => {
		checkSidebarVisibility()
	})
	const appContents = document.querySelector('.app-contents')
	if (appContents) {
		observer.observe(appContents, { attributes: true, attributeFilter: ['class'] })
	}
})

onBeforeUnmount(() => {
	const container = getScrollContainer()
	if (container) {
		container.removeEventListener('scroll', handleScroll)
	}
})

async function handleRefresh() {
	if (refreshing.value) return
	refreshing.value = true
	try {
		await ctx.refresh()
	} finally {
		refreshing.value = false
	}
}

function mapToTableItem(item: ContentItem, group?: string): ContentCardTableItem {
	const base = ctx.mapToTableItem(item)
	const id = getItemId(item)
	return {
		...base,
		id,
		group,
		disabled: isChanging(id) || ctx.isBusy.value || item.installing === true,
		disabledTooltip: ctx.isBusy.value
			? (ctx.busyMessage?.value ?? null)
			: (base.disabledTooltip ?? null),
		toggleDisabled: ctx.isBusy.value || base.toggleDisabled === true,
		toggleDisabledTooltip: ctx.isBusy.value
			? (ctx.busyMessage?.value ?? null)
			: (base.toggleDisabledTooltip ?? null),
		installing: item.installing === true,
		pendingManualDownload: item.pendingManualDownload === true,
		hasUpdate: group ? false : item.update != null,
		rollbackFileName: item.rollback?.file_name,
		isClientOnly:
			isClientOnlyEnvironment(item.environment) ||
			!!item.pack_client_retained ||
			!!item.pack_client_depends,
		clientWarning: getClientWarningType(item),
		hideSwitchVersion: base.hideSwitchVersion ?? !base.versionLink,
		overflowOptions: buildItemOverflowOptions(item),
	}
}

/**
 * Overflow-menu options for a content row. Deletion is always available as a
 * row-level trash button, so the overflow menu only carries non-destructive
 * actions.
 */
function buildItemOverflowOptions(item: ContentItem): OverflowMenuOption[] | undefined {
	const options = ctx.getOverflowOptions?.(item) ?? []
	return options.length > 0 ? options : undefined
}

function datapackTreePath(item: ContentItem): string {
	const path = item.file_path ?? item.file_name
	const segments = path.split(/[\\/]/).filter(Boolean)
	if (
		segments[0]?.toLocaleLowerCase() === 'saves' &&
		segments[2]?.toLocaleLowerCase() === 'datapacks'
	) {
		return segments.slice(1, 2).concat(segments.slice(3)).join('/')
	}
	return path
}

const { folderRows: schematicFolderRows, regularItems: schematicGroupRegularItems } =
	useContentFolderGroups({
		filteredItems,
		modpackChildIdSet,
		searchQuery,
		expandedGroups,
		persistExpandedGroups: (groups) => {
			if (ctx.instanceId) expandedGroupsMemory.set(ctx.instanceId, groups)
		},
		getItemId,
		mapToTableItem,
		isGroupedItem: (item) =>
			['schematic', 'datapack'].includes(normalizeProjectType(item.project_type)),
		treePath: (item) => {
			const path = item.file_path ?? item.file_name
			const segments = path.split(/[\\/]/).filter(Boolean)
			if (segments[0]?.toLocaleLowerCase() === 'schematics') {
				return segments.slice(1).join('/')
			}
			return datapackTreePath(item)
		},
		folderGroupId: (path) => {
			const isWorldDatapackFolder = filteredItems.value.some((item) => {
				if (item.source_kind !== 'world_datapack') return false
				const itemPath = datapackTreePath(item)
				return itemPath === path || itemPath.startsWith(`${path}/`)
			})
			return isWorldDatapackFolder ? `datapack-folder:${path}` : `schematic-folder:${path}`
		},
		folderGroupIdPrefix: ['schematic-folder:', 'datapack-folder:'],
		locale,
	})

function getWorldGroupMeta(row: ContentCardTableItem): ContentWorldGroupMeta | undefined {
	if (!row.isGroupHeader || !row.group?.startsWith('datapack-folder:')) return undefined
	const child = row.groupChildIds
		?.map((id) => findContentItem(id))
		.find((item) => item?.source_kind === 'world_datapack' && item?.groupMeta)
	return child?.groupMeta
}

const folderRows = computed<ContentCardTableItem[]>(() =>
	schematicFolderRows.value.map((row) => {
		const meta = getWorldGroupMeta(row)
		if (!meta) return row
		return {
			...row,
			groupKind: 'world',
			groupMeta: meta,
			project: {
				...row.project,
				title: meta.title ?? row.project.title,
				icon_url: meta.icon_url ?? row.project.icon_url,
			},
		}
	}),
)

const tableItems = computed<ContentCardTableItem[]>(() => {
	const items: ContentCardTableItem[] = []

	const modpackItems = filteredModpackItems.value
	const modpack = ctx.modpack.value

	if (modpack && modpack.project && modpackItems.length > 0) {
		const groupItems: ContentCardTableItem[] = []
		const childIds = modpackItems.map((item) => getItemId(item))
		const presentChildren = modpackItems.filter(isPresentContentItem)
		const allChildrenDisabled =
			presentChildren.length > 0 && presentChildren.every(isDisabledContentItem)
		if (expandedGroups.value.has('modpack')) {
			for (const item of modpackItems) {
				groupItems.push(mapToTableItem(item, 'modpack'))
			}
		}

		items.push({
			id: '__modpack_group__',
			isGroupHeader: true,
			group: 'modpack',
			groupItemCount: modpackItems.length,
			groupChildIds: childIds,
			groupSwitchVersion: ctx.updateModpack,
			project: {
				id: modpack.project.id,
				slug: modpack.project.slug,
				title: modpack.project.title,
				icon_url: modpack.project.icon_url,
			},
			projectLink: modpack.projectLink,
			version: modpack.version
				? {
						id: modpack.version.id,
						version_number: modpack.version.version_number,
						file_name: '',
						date_published: modpack.version.date_published,
					}
				: undefined,
			versionLink: modpack.versionLink,
			owner: modpack.owner,
			enabled: true,
			hasUpdate: modpack.hasUpdate,
			disabled:
				modpack.disabled || ctx.isBusy.value || isBulkOperating.value || allChildrenDisabled,
			disabledTooltip:
				modpack.disabledText ?? (ctx.isBusy.value ? (ctx.busyMessage?.value ?? null) : null),
			downloads: modpack.project.downloads ?? null,
			categories: modpack.categories,
		})

		items.push(...groupItems)
	}

	items.push(...folderRows.value)
	for (const item of schematicGroupRegularItems.value) {
		items.push(mapToTableItem(item))
	}

	return items
})

const hasOutdatedProjects = computed(() => {
	const outdated = ctx.items.value.filter((p) => p.update != null)
	const modpackHasUpdate =
		ctx.bulkUpdateIncludesModpack !== false && (ctx.modpack.value?.hasUpdate ?? false)
	return outdated.length > 0 || modpackHasUpdate
})

//  Deletion
const pendingDeletionItems = ref<ContentItem[]>([])
const confirmDeletionModal = ref<InstanceType<typeof ConfirmDeletionModal>>()
const contentDependencyWarningModal = ref<InstanceType<typeof ContentDependencyWarningModal>>()
const pendingDependencyWarningItems = ref<ContentCardTableItem[]>([])
const pendingDependencyWarningDependents = ref<
	Array<{
		item: ContentCardTableItem
		dependencies: ContentCardTableItem[]
	}>
>([])
const pendingDependencyWarningDisableTargets = ref<ContentItem[]>([])

function mapToDisplayItem(item: ContentItem) {
	return {
		...ctx.mapToTableItem(item),
		id: getItemId(item),
	}
}

async function promptDeleteItems(items: ContentItem[], event?: MouseEvent) {
	if (items.length === 0) return
	pendingDeletionItems.value = items
	pendingDependencyWarningItems.value = []
	pendingDependencyWarningDependents.value = []
	pendingDependencyWarningDisableTargets.value = []
	const deletingIds = new Set(items.map(getItemId))

	const warning = ctx.getDeleteDependencyWarning
		? await Promise.resolve()
				.then(() => ctx.getDeleteDependencyWarning!(items))
				.catch(() => null)
		: null
	if (warning) {
		const remainingDependents = warning.dependents.filter(
			(dependent) => !deletingIds.has(getItemId(dependent.item)),
		)

		if (remainingDependents.length === 0) {
			showDeletionConfirmation(event)
			return
		}

		const relevantDependencyIds = new Set(
			remainingDependents.flatMap((dependent) => dependent.dependencies.map(getItemId)),
		)
		const warningItems = items.filter((item) => relevantDependencyIds.has(getItemId(item)))
		if (warningItems.length === 0) {
			showDeletionConfirmation(event)
			return
		}

		pendingDependencyWarningItems.value = warningItems.map(mapToDisplayItem)
		pendingDependencyWarningDependents.value = remainingDependents.map((dependent) => ({
			item: mapToDisplayItem(dependent.item),
			dependencies: dependent.dependencies
				.filter((dependency) => relevantDependencyIds.has(getItemId(dependency)))
				.map(mapToDisplayItem),
		}))
		pendingDependencyWarningDisableTargets.value = remainingDependents.map(
			(dependent) => dependent.item,
		)
		contentDependencyWarningModal.value?.show()
		return
	}

	showDeletionConfirmation(event)
}

async function showDeletionConfirmation(event?: MouseEvent) {
	if ((event?.shiftKey || skipNonEssentialWarnings.value) && !ctx.isBusy.value) {
		confirmDelete()
	} else {
		await nextTick()
		confirmDeletionModal.value?.show()
	}
}

async function handleDeleteById(id: string, event?: MouseEvent) {
	const item = findContentItem(id)
	if (item) {
		await promptDeleteItems([item], event)
	}
}

async function showBulkDeleteModal(event?: MouseEvent) {
	await promptDeleteItems([...selectedItems.value], event)
}

async function confirmDependencyWarningDelete(disableDependentsAfterDeleting: boolean) {
	if (disableDependentsAfterDeleting) {
		pendingDependencyWarningDisableTargets.value =
			pendingDependencyWarningDisableTargets.value.filter((item) => item.enabled)
	} else {
		pendingDependencyWarningDisableTargets.value = []
	}

	pendingDependencyWarningItems.value = []
	pendingDependencyWarningDependents.value = []
	await confirmDelete()
}

async function disablePendingDependencyWarningDependents() {
	const items = pendingDependencyWarningDisableTargets.value.filter((item) => item.enabled)
	pendingDependencyWarningDisableTargets.value = []
	if (items.length === 0) return

	if (ctx.bulkDisableItems) {
		await ctx.bulkDisableItems(items)
		return
	}

	for (const item of items) {
		const id = getItemId(item)
		markChanging(id)
		try {
			await ctx.toggleEnabled(item)
		} finally {
			unmarkChanging(id)
		}
	}
}

async function confirmDelete() {
	if (ctx.isBusy.value) return
	const itemsToDelete = [...pendingDeletionItems.value]
	pendingDeletionItems.value = []
	if (itemsToDelete.length === 0) return

	if (ctx.bulkDeleteItems && itemsToDelete.length > 1) {
		await runBulkWithWaiting(
			'delete',
			itemsToDelete.length,
			async () => {
				await ctx.bulkDeleteItems!(itemsToDelete)
			},
			() => {
				clearSelection()
			},
		)
		await disablePendingDependencyWarningDependents()
		return
	}

	if (itemsToDelete.length === 1) {
		const item = itemsToDelete[0]
		const id = getItemId(item)
		markChanging(id)
		try {
			await ctx.deleteItem(item)
			removeFromSelection(id)
			await disablePendingDependencyWarningDependents()
		} finally {
			unmarkChanging(id)
		}
		return
	}

	await runBulk(
		'delete',
		itemsToDelete,
		async (item) => {
			await ctx.deleteItem(item)
			removeFromSelection(getItemId(item))
		},
		{ onComplete: clearSelection },
	)
	await disablePendingDependencyWarningDependents()
}

async function handleToggleEnabledById(id: string, _value: boolean) {
	if (ctx.isBusy.value) return
	const item = findContentItem(id)
	if (!item) return
	markChanging(id)
	try {
		await ctx.toggleEnabled(item)
	} finally {
		unmarkChanging(id)
	}
}

async function handleRollbackById(id: string) {
	if (ctx.isBusy.value || !ctx.rollbackItem) return
	const item = findContentItem(id)
	if (!item || !item.rollback) return
	markChanging(id)
	try {
		await ctx.rollbackItem(item)
	} finally {
		unmarkChanging(id)
	}
}

async function bulkEnable() {
	if (ctx.isBusy.value) return
	const items = selectedItems.value.filter(
		(item) => canToggleContentItem(item) && isDisabledContentItem(item),
	)
	if (items.length === 0) return
	if (ctx.bulkEnableItems) {
		await runBulkWithWaiting(
			'enable',
			items.length,
			async () => {
				await ctx.bulkEnableItems!(items)
			},
			clearSelection,
		)
		return
	}
	await runBulk('enable', items, (item) => ctx.toggleEnabled(item), { onComplete: clearSelection })
}

async function bulkDisable() {
	if (ctx.isBusy.value) return
	const items = selectedItems.value.filter(
		(item) => canToggleContentItem(item) && isEnabledContentItem(item),
	)
	if (items.length === 0) return
	if (ctx.bulkDisableItems) {
		await runBulkWithWaiting(
			'disable',
			items.length,
			async () => {
				await ctx.bulkDisableItems!(items)
			},
			clearSelection,
		)
		return
	}
	await runBulk('disable', items, (item) => ctx.toggleEnabled(item), { onComplete: clearSelection })
}

function handleUpdateById(id: string) {
	if (id === '__modpack_group__') {
		ctx.updateModpack?.()
		return
	}
	ctx.updateItem?.(id)
}

function handleSwitchVersionById(id: string) {
	const item = findContentItem(id)
	if (item) {
		ctx.switchVersion?.(item)
	}
}

// Bulk updating
const confirmBulkUpdateModal = ref<InstanceType<typeof ConfirmBulkUpdateModal>>()
const pendingBulkUpdateItems = ref<ContentItem[]>([])
const pendingBulkUpdateAll = ref(false)

const hasBulkUpdateSupport = computed(
	() => !!(ctx.bulkUpdateAll || ctx.bulkUpdateItem || ctx.bulkUpdateItems),
)

function promptUpdateAll(event?: MouseEvent) {
	if (!hasBulkUpdateSupport.value) return
	const items = ctx.items.value.filter((item) => item.update != null)
	const modpackHasUpdate =
		ctx.bulkUpdateIncludesModpack !== false && (ctx.modpack.value?.hasUpdate ?? false)
	if (items.length === 0 && !modpackHasUpdate) return
	pendingBulkUpdateItems.value = items
	pendingBulkUpdateAll.value = true
	if ((event?.shiftKey || skipNonEssentialWarnings.value) && !ctx.isBusy.value) {
		confirmBulkUpdate()
	} else {
		confirmBulkUpdateModal.value?.show()
	}
}

function promptUpdateSelected(event?: MouseEvent) {
	if (!hasBulkUpdateSupport.value) return
	const items = selectedItems.value.filter((item) => item.update != null)
	if (items.length === 0) return
	pendingBulkUpdateItems.value = items
	pendingBulkUpdateAll.value = false
	if ((event?.shiftKey || skipNonEssentialWarnings.value) && !ctx.isBusy.value) {
		confirmBulkUpdate()
	} else {
		confirmBulkUpdateModal.value?.show()
	}
}

async function confirmBulkUpdate() {
	if (ctx.isBusy.value) return
	const items = pendingBulkUpdateItems.value
	const modpackHasUpdate =
		ctx.bulkUpdateIncludesModpack !== false && (ctx.modpack.value?.hasUpdate ?? false)
	if (items.length === 0 && !modpackHasUpdate) return
	if (!hasBulkUpdateSupport.value) return

	const setBulkStatus = (status: BulkOperationStatus) => {
		bulkStatusMessage.value = status.message ?? null
		bulkProgress.value = status.progress ?? bulkProgress.value
		bulkTotal.value = status.total ?? bulkTotal.value
		bulkWaiting.value = status.waiting ?? false
	}

	try {
		if (pendingBulkUpdateAll.value && ctx.bulkUpdateAll) {
			const totalCount = items.length + (modpackHasUpdate ? 1 : 0)
			bulkItemCount.value = totalCount
			await runBulkWithWaiting(
				'update',
				totalCount,
				async () => {
					await ctx.bulkUpdateAll(setBulkStatus)
				},
				() => {
					clearSelection()
					bulkItemCount.value = 0
					bulkStatusMessage.value = null
				},
			)
		} else if (ctx.bulkUpdateItems) {
			bulkItemCount.value = items.length
			await runBulkWithWaiting(
				'update',
				items.length,
				async () => {
					await ctx.bulkUpdateItems(items)
				},
				() => {
					clearSelection()
					bulkItemCount.value = 0
					bulkStatusMessage.value = null
				},
			)
		} else if (ctx.bulkUpdateItem) {
			await runBulk('update', items, ctx.bulkUpdateItem, { onComplete: clearSelection })
		}
	} finally {
		pendingBulkUpdateItems.value = []
		pendingBulkUpdateAll.value = false
	}
}

const confirmUnlinkModal = ref<InstanceType<typeof ConfirmUnlinkModal>>()
</script>

<template>
	<div class="flex flex-col gap-4" :class="{ 'pb-6': props.bottomPadding }">
		<template v-if="!ctx.loading.value">
			<div
				v-if="ctx.error.value"
				class="flex w-full flex-col items-center justify-center gap-4 p-4"
			>
				<div class="universal-card flex flex-col items-center gap-4 p-6">
					<h2 class="m-0 text-xl font-bold">{{ formatMessage(messages.failedToLoad) }}</h2>
					<p class="text-secondary">{{ ctx.error.value.message }}</p>
					<ButtonStyled color="brand">
						<button @click="handleRefresh">{{ formatMessage(commonMessages.retryButton) }}</button>
					</ButtonStyled>
				</div>
			</div>

			<template
				v-else-if="ctx.items.value.length > 0 || (ctx.modpackItems?.value?.length ?? 0) > 0"
			>
				<ContentToolbar
					v-model:search-query="searchQuery"
					:searchable-item-count="searchableItemCount"
					:content-type-label="ctx.contentTypeLabel.value"
					:busy="ctx.isBusy.value"
					:busy-tooltip="ctx.busyMessage?.value"
					:disable-add-content="ctx.disableAddContent?.value"
					:disable-add-content-tooltip="ctx.disableAddContentTooltip"
					:refreshing="refreshing"
					@browse="ctx.browse"
					@refresh="handleRefresh"
				/>

				<ContentTypeFilter
					v-model:selected="selectedTypeFilter"
					:options="row1FilterOptions"
					:total-count="totalCount"
					:filter-counts="filterCounts"
					@toggle="toggleTypeFilter"
				/>

				<ContentCardTable
					v-model:selected-ids="selectedIds"
					:items="tableItems"
					:show-selection="true"
					:expanded-groups="expandedGroups"
					@update:enabled="handleToggleEnabledById"
					@delete="handleDeleteById"
					@update="handleUpdateById"
					@switch-version="handleSwitchVersionById"
					@rollback="handleRollbackById"
					@toggle-expand="toggleGroupExpand"
					@visible-items="handleVisibleTableItems"
				>
					<template #header-project>
						<ContentMetadataFilterBar
							v-model:expanded="metadataFilterExpanded"
							:categories="metadataFilterCategories"
							:model-value="metadataFilterSelectedValues"
							:filtering-keys="metadataFilteringKeys"
							@update:category="setCategorySelection"
						/>
					</template>
					<template #header-actions>
						<ContentTableHeaderActions
							:sort-mode="sortMode"
							:sort-label="formatMessage(messages.sortByLabel, { mode: sortLabels[sortMode]() })"
							:has-bulk-update-support="hasBulkUpdateSupport"
							:has-outdated-projects="hasOutdatedProjects"
							:bulk-update-tooltip="
								ctx.bulkUpdateAllDescription ??
								ctx.bulkUpdateAllLabel ??
								formatMessage(messages.updateAll)
							"
							:is-bulk-operating="isBulkOperating"
							@sort="cycleSortMode"
							@update-all="promptUpdateAll"
						/>
					</template>
					<template #empty>
						<span>{{ formatMessage(messages.noContentFound) }}</span>
					</template>
				</ContentCardTable>
			</template>

			<ContentEmptyState
				v-else
				:content-type-label="ctx.contentTypeLabel.value"
				:busy="ctx.isBusy.value"
				:busy-tooltip="ctx.busyMessage?.value"
				:refreshing="refreshing"
				:disable-add-content="ctx.disableAddContent?.value"
				:disable-add-content-tooltip="ctx.disableAddContentTooltip"
				@browse="ctx.browse"
				@refresh="handleRefresh"
			/>
		</template>

		<ContentSelectionBar
			:selected-items="selectedItems"
			:content-type-label="ctx.contentTypeLabel.value"
			:is-busy="ctx.isBusy.value"
			:busy-tooltip="ctx.busyMessage?.value"
			:is-bulk-operating="isBulkOperating"
			:bulk-operation="bulkOperation"
			:bulk-progress="bulkProgress"
			:bulk-total="bulkTotal"
			:bulk-waiting="bulkWaiting"
			:bulk-status-message="bulkStatusMessage"
			:bulk-item-count="bulkItemCount"
			:aria-label="formatMessage(commonMessages.selectionActionsLabel)"
			:get-item-id="getItemId"
			@clear="clearSelection"
			@enable="bulkEnable"
			@disable="bulkDisable"
		>
			<template #actions>
				<ButtonStyled
					v-if="hasBulkUpdateSupport && selectedItems.some((m) => m.update != null)"
					type="transparent"
					color="green"
					color-fill="text"
					hover-color-fill="background"
				>
					<button
						v-tooltip="formatMessage(commonMessages.updateButton)"
						@click="promptUpdateSelected"
					>
						<DownloadIcon />
						<span class="bar-label">{{ formatMessage(commonMessages.updateButton) }}</span>
					</button>
				</ButtonStyled>

				<ButtonStyled v-if="ctx.shareItems" type="transparent">
					<OverflowMenu
						:options="[
							{
								id: 'share-names',
								action: () => ctx.shareItems!(selectedItems, 'names'),
							},
							{
								id: 'share-file-names',
								action: () => ctx.shareItems!(selectedItems, 'file-names'),
							},
							{
								id: 'share-urls',
								action: () => ctx.shareItems!(selectedItems, 'urls'),
							},
							{
								id: 'share-markdown',
								action: () => ctx.shareItems!(selectedItems, 'markdown'),
							},
						]"
					>
						<ShareIcon />
						<span class="bar-label">{{ formatMessage(messages.share) }}</span>
						<DropdownIcon />
						<template #share-names>
							<TextCursorInputIcon />
							{{ formatMessage(messages.shareProjectNames) }}
						</template>
						<template #share-file-names>
							<FileIcon />
							{{ formatMessage(messages.shareFileNames) }}
						</template>
						<template #share-urls>
							<LinkIcon />
							{{ formatMessage(messages.shareProjectLinks) }}
						</template>
						<template #share-markdown>
							<CodeIcon />
							{{ formatMessage(messages.shareMarkdownLinks) }}
						</template>
					</OverflowMenu>
				</ButtonStyled>
			</template>

			<template #actions-end>
				<div class="mx-1 h-6 w-px bg-surface-5" />

				<ButtonStyled
					type="transparent"
					color="red"
					color-fill="text"
					hover-color-fill="background"
				>
					<button
						v-tooltip="formatMessage(commonMessages.deleteLabel)"
						@click="showBulkDeleteModal"
					>
						<TrashIcon />
						<span class="bar-label">{{ formatMessage(commonMessages.deleteLabel) }}</span>
					</button>
				</ButtonStyled>
			</template>
		</ContentSelectionBar>

		<ConfirmDeletionModal
			ref="confirmDeletionModal"
			:count="pendingDeletionItems.length"
			:item-type="ctx.contentTypeLabel.value"
			:variant="ctx.deletionContext ?? 'instance'"
			:backup-tip="pendingDeletionItems.map((i) => i.project?.title ?? i.file_name).join(', ')"
			:action-disabled="ctx.isBusy.value"
			:action-disabled-tooltip="ctx.busyMessage?.value ?? undefined"
			:symlink-target="ctx.symlinkTarget?.value"
			@delete="confirmDelete"
		/>
		<ContentDependencyWarningModal
			ref="contentDependencyWarningModal"
			:items="pendingDependencyWarningItems"
			:dependents="pendingDependencyWarningDependents"
			:item-type="ctx.contentTypeLabel.value"
			:variant="ctx.deletionContext ?? 'instance'"
			:backup-tip="pendingDeletionItems.map((i) => i.project?.title ?? i.file_name).join(', ')"
			:action-disabled="ctx.isBusy.value"
			:action-disabled-tooltip="ctx.busyMessage?.value ?? undefined"
			:symlink-target="ctx.symlinkTarget?.value"
			@delete="confirmDependencyWarningDelete"
		/>
		<ConfirmBulkUpdateModal
			v-if="hasBulkUpdateSupport"
			ref="confirmBulkUpdateModal"
			:count="
				pendingBulkUpdateItems.length +
				(pendingBulkUpdateAll &&
				ctx.bulkUpdateIncludesModpack !== false &&
				ctx.modpack.value?.hasUpdate
					? 1
					: 0)
			"
			:action-label="ctx.bulkUpdateAllLabel"
			:scope-description="ctx.bulkUpdateAllDescription"
			:server="ctx.deletionContext === 'server'"
			:action-disabled="ctx.isBusy.value"
			:action-disabled-tooltip="ctx.busyMessage?.value ?? undefined"
			:symlink-target="ctx.symlinkTarget?.value"
			@update="confirmBulkUpdate"
		/>
		<ConfirmUnlinkModal
			v-if="ctx.unlinkModpack"
			ref="confirmUnlinkModal"
			:server="ctx.deletionContext === 'server'"
			:backup-tip="ctx.modpack.value?.project.title"
			:action-disabled="ctx.isBusy.value"
			:action-disabled-tooltip="ctx.busyMessage?.value ?? undefined"
			@unlink="ctx.unlinkModpack!()"
		/>

		<slot name="modals" />

		<Transition name="scroll-to-top">
			<button
				v-if="showScrollToTop"
				class="scroll-to-top-btn"
				:class="{ 'sidebar-visible': sidebarVisible }"
				aria-label="Scroll to top"
				@click="scrollToTop"
			>
				<ChevronUpIcon class="size-5" />
			</button>
		</Transition>
	</div>
</template>

<style scoped>
.scroll-to-top-btn {
	@apply fixed bottom-6 z-50 flex items-center justify-center rounded-full bg-brand p-3 text-brand-inverted shadow-lg transition-all duration-200 hover:brightness-110 hover:shadow-xl active:scale-95;
	right: 24px;
}

.scroll-to-top-btn.sidebar-visible {
	right: calc(300px + 24px);
}

.scroll-to-top-enter-active,
.scroll-to-top-leave-active {
	transition:
		opacity 0.2s ease,
		transform 0.2s ease;
}

.scroll-to-top-enter-from,
.scroll-to-top-leave-to {
	opacity: 0;
	transform: translateY(10px);
}
</style>
