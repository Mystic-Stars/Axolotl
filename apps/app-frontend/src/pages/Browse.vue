<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	BookmarkFilledIcon,
	BookmarkIcon,
	CheckIcon,
	ChevronDownIcon,
	ClipboardCopyIcon,
	CurseForgeIcon,
	ExternalIcon,
	FileArchiveIcon,
	GenericListIcon,
	GlobeIcon,
	GridIcon,
	LanguagesIcon,
	ListIcon,
	ModrinthIcon,
	PlusIcon,
	SparklesIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import type {
	BrowseDisplayMode,
	BrowseDisplayModeOption,
	BrowseInstallContentType,
	CardAction,
	ProjectType,
	Tags,
} from '@modrinth/ui'
import {
	BrowsePageLayout,
	BrowseSidebar,
	ButtonStyled,
	commonMessages,
	CreationFlowModal,
	defineMessages,
	EmptyState,
	getLatestMatchingInstallVersion,
	getSelectedInstallPreferences,
	getTargetInstallPreferences,
	injectNotificationManager,
	isVersionTypeMatch,
	PopoutMenu,
	preferencesDiffer,
	provideBrowseManager,
	requestInstall,
	stripServerRuntimeInstallFilters,
	stripServerRuntimeInstallOverrides,
	useBrowseSearch,
	useDebugLogger,
	usesTargetGameVersion,
	useVIntl,
} from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import type { LocationQuery } from 'vue-router'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

import BrowseInstanceSelector from '@/components/browse/BrowseInstanceSelector.vue'
import ContextMenu from '@/components/ui/ContextMenu.vue'
import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import { useAppServerBrowse } from '@/composables/browse/use-app-server-browse'
import { useContentFavorites } from '@/composables/useContentFavorites'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { useTranslationToggle } from '@/composables/useTranslationToggle'
import {
	type BrowseFilterMemory,
	getBrowseFilterMemory,
	setBrowseFilterMemory,
} from '@/helpers/browse-filter-memory'
import { mergeProviderResults } from '@/helpers/browse-merge'
import { createBrowseProjectTabs, getBrowseProjectTabOptions } from '@/helpers/browse-project-tabs'
import {
	completeBrowseReturnNavigation,
	consumeBrowseReturnSnapshot,
	isBrowseReturnSourcePath,
	saveBrowseReturnSnapshot,
} from '@/helpers/browse-return-state.ts'
import {
	cancel_search_request,
	get_project,
	get_project_v3,
	get_project_v3_many,
	get_search_results_v3,
	get_version_many,
} from '@/helpers/cache.js'
import { type FavoriteContentType, isFavoriteContentType } from '@/helpers/content-favorites'
import {
	bilingualTitle,
	type ChineseSearchResolution,
	type ChineseSearchTranslation,
	containsChineseSearchText,
	expandContentSearchQuery,
	resolveChineseContentSearch,
	translateSearchHitTitles,
} from '@/helpers/content-search'
import {
	type CurseForgeCategory,
	getCurseForgeCapability,
	getCurseForgeCategories,
	getCurseForgeFiles,
	getCurseForgeImageUrl,
	searchCurseForgeProjects,
	type UnifiedSearchHit,
} from '@/helpers/curseforge'
import {
	CF_EXTRA_CATEGORY_HEADER,
	curseForgeCategoryValue,
	findUnmappedCurseForgeCategories,
	isCurseForgeOnlyCategoryName,
	localizeCurseForgeCategoryName,
	localizeCurseForgeLabel,
	resolveCurseForgeCategoryIdsFromFilterValues,
} from '@/helpers/curseforge-category-map'
import { instance_listener } from '@/helpers/events.js'
import {
	get as getInstance,
	get_installed_project_ids as getInstalledProjectIds,
} from '@/helpers/instance'
import { getDisplayInstanceIcon } from '@/helpers/instance-icons'
import {
	getMcArchiveGameVersions,
	type McArchiveGameVersion,
	type McArchiveMod,
	searchMcArchiveMods,
} from '@/helpers/mcarchive'
import { get_loader_versions as getLoaderManifest } from '@/helpers/metadata'
import {
	planetMinecraftConnectorAvailable,
	type PlanetMinecraftProject,
	searchPlanetMinecraftProjects,
} from '@/helpers/planet-minecraft'
import {
	curseForgeQueryVariants,
	expandSearchQuery,
	normalizeSearchText,
} from '@/helpers/search-query'
import {
	type BrowseContentSource,
	get as getSettings,
	getLastBrowseContentDisplayMode,
	getLastBrowseContentSource,
	isBrowseContentProjectType,
	set as setSettings,
	setLastBrowseContentDisplayMode,
	setLastBrowseContentProjectType,
	setLastBrowseContentSource,
} from '@/helpers/settings.ts'
import { get_categories, get_game_versions, get_loaders } from '@/helpers/tags'
import { translateSearchDescriptions } from '@/helpers/translation'
import type { GameInstance } from '@/helpers/types'
import { get_instance_worlds } from '@/helpers/worlds'
import i18n from '@/i18n.config'
import { injectContentInstall } from '@/providers/content-install'
import { injectContentSelection, makeContentSelectionKey } from '@/providers/content-selection'
import { injectServerInstall } from '@/providers/server-install'
import {
	createServerInstallContent,
	provideServerInstallContent,
} from '@/providers/setup/server-install-content'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { useTheming } from '@/store/state'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const { installingServerProjects, playServerProject, showAddServerToInstanceModal } =
	injectServerInstall()
const { install: installVersion, installCurseForge } = injectContentInstall()
const contentSelection = injectContentSelection()
const contentFavorites = useContentFavorites()
const queryClient = useQueryClient()
const debugLog = useDebugLogger('Browse')
const router = useRouter()
const route = useRoute()

void contentFavorites.load().catch(handleError)
const projectType = ref<ProjectType>(route.params.projectType as ProjectType)
const WORLD_BROWSE_PROJECT_TYPE = 'world' as ProjectType
const isWorldMapBrowse = computed(() => projectType.value === WORLD_BROWSE_PROJECT_TYPE)
const isWorldMapContext = computed(() => route.query.from === 'world-maps')

function rememberBrowseContentProjectType(type: ProjectType) {
	if (
		!route.query.i &&
		!route.query.sid &&
		!route.query.wid &&
		!route.query.from &&
		isBrowseContentProjectType(type)
	) {
		setLastBrowseContentProjectType(type)
	}
}

watch(projectType, rememberBrowseContentProjectType, { immediate: true })

const curseForgeClassIds: Partial<Record<ProjectType, number>> = {
	mod: 6,
	plugin: 5,
	resourcepack: 12,
	datapack: 6945,
	shader: 6552,
	modpack: 4471,
	[WORLD_BROWSE_PROJECT_TYPE]: 17,
}

const [initialCurseForgeCapability, initialPlanetMinecraftAvailable] = await Promise.all([
	getCurseForgeCapability().catch(() => ({
		status: 'missing_key' as const,
		configured: false,
	})),
	planetMinecraftConnectorAvailable().catch(() => false),
])
const curseForgeCapability = ref(initialCurseForgeCapability)
const planetMinecraftAvailable = ref(initialPlanetMinecraftAvailable)
const rememberedContentSource = getLastBrowseContentSource()

function resolveInitialContentSource(): BrowseContentSource {
	if (route.params.projectType === WORLD_BROWSE_PROJECT_TYPE) {
		return 'curseforge'
	}
	if (route.params.projectType === 'mod' && route.query.source === 'mcarchive') {
		return 'mcarchive'
	}
	if (
		route.params.projectType === 'mod' &&
		route.query.source === 'planet_minecraft' &&
		planetMinecraftAvailable.value
	) {
		return 'planet_minecraft'
	}
	if (curseForgeCapability.value.configured && route.query.source === 'curseforge') {
		return 'curseforge'
	}
	if (route.query.source === 'modrinth') {
		return 'modrinth'
	}
	if (rememberedContentSource === 'curseforge' && curseForgeCapability.value.configured) {
		return 'curseforge'
	}
	if (rememberedContentSource === 'modrinth') {
		return 'modrinth'
	}
	if (rememberedContentSource === 'mcarchive' && route.params.projectType === 'mod') {
		return 'mcarchive'
	}
	if (
		rememberedContentSource === 'planet_minecraft' &&
		route.params.projectType === 'mod' &&
		planetMinecraftAvailable.value
	) {
		return 'planet_minecraft'
	}
	return curseForgeCapability.value.configured ? 'all' : 'modrinth'
}

const contentSource = ref<BrowseContentSource>(resolveInitialContentSource())
const sourceBeforeWorldMapBrowse = ref<BrowseContentSource>(
	isWorldMapBrowse.value ? (getLastBrowseContentSource() ?? 'all') : contentSource.value,
)
if (isWorldMapBrowse.value) {
	contentSource.value = 'curseforge'
}
const curseForgeCategoriesByClass = ref<Record<number, CurseForgeCategory[]>>({})

async function ensureCurseForgeCategories(projectTypeValue: ProjectType) {
	const classId = curseForgeClassIds[projectTypeValue]
	if (!classId || curseForgeCategoriesByClass.value[classId]) return

	const classCategories = await getCurseForgeCategories(classId)
	curseForgeCategoriesByClass.value = {
		...curseForgeCategoriesByClass.value,
		[classId]: classCategories,
	}
}

const initialCurseForgeCategoriesPromise =
	curseForgeCapability.value.configured &&
	(contentSource.value === 'curseforge' || contentSource.value === 'all')
		? ensureCurseForgeCategories(projectType.value).catch(handleError)
		: Promise.resolve()
if (route.query.f || route.query.g) await initialCurseForgeCategoriesPromise

const themeStore = useTheming()
const serverSetupModalRef = ref<InstanceType<typeof CreationFlowModal> | null>(null)
const serverInstallContent = createServerInstallContent({ serverSetupModalRef })
provideServerInstallContent(serverInstallContent)
const {
	serverIdQuery,
	serverFlowFrom,
	isFromWorlds,
	isServerContext,
	isSetupServerContext,
	effectiveServerWorldId,
	serverContextServerData,
	serverContentProjectIds,
	queuedServerInstallProjectIds,
	queuedServerInstallCount,
	selectedServerInstallProjects,
	isInstallingQueuedServerInstalls,
	queuedInstallProgress,
	serverBackUrl,
	serverBackLabel,
	serverBrowseHeading,
	clearQueuedServerInstalls,
	removeQueuedServerInstall,
	flushQueuedServerInstalls,
	discardQueuedServerInstallsAndBack,
	installQueuedServerInstallsAndBack,
	initServerContext,
	watchServerContextChanges,
	searchServerModpacks,
	getServerProjectVersions,
	enforceSetupModpackRoute,
	getQueuedServerInstallPlans,
	setQueuedServerInstallPlans,
	openServerModpackInstallFlow,
	onServerFlowBack,
	handleServerModpackFlowCreate,
	markServerProjectInstalled,
} = serverInstallContent

debugLog('fetching tags (categories, loaders, gameVersions)')
let categories: Ref<Labrinth.Tags.v2.Category[]> = ref([])
let loaders: Ref<Labrinth.Tags.v2.Loader[]> = ref([])
let availableGameVersions: Ref<Labrinth.Tags.v2.GameVersion[]> = ref([])
const mcArchiveGameVersions = ref<McArchiveGameVersion[]>([])
if (!isWorldMapBrowse.value) {
	;[categories, loaders, availableGameVersions] = await Promise.all([
		get_categories()
			.catch(handleError)
			.then(ref<Labrinth.Tags.v2.Category[]>),
		get_loaders()
			.catch(handleError)
			.then(ref<Labrinth.Tags.v2.Loader[]>),
		get_game_versions()
			.catch(handleError)
			.then(ref<Labrinth.Tags.v2.GameVersion[]>),
	])
}

async function refreshMcArchiveGameVersions() {
	if (mcArchiveGameVersions.value.length > 0) return
	mcArchiveGameVersions.value = await getMcArchiveGameVersions().catch((error) => {
		handleError(error)
		return []
	})
}

if (contentSource.value === 'mcarchive') await refreshMcArchiveGameVersions()

const curseForgeCategoryTags = computed(() => {
	const classId = curseForgeClassIds[projectType.value]
	if (!classId) return []

	const classCategories = curseForgeCategoriesByClass.value[classId] ?? []
	const categoriesById = new Map(classCategories.map((category) => [category.id, category]))
	return classCategories
		.filter((category) => !category.isClass)
		.map((category) => {
			const parent = category.parentCategoryId
				? categoriesById.get(category.parentCategoryId)
				: undefined
			const isResolution =
				classId === 12 && category.displayIndex != null && category.displayIndex < 0
			const displayName = localizeCurseForgeCategoryName(category)
			const header = isResolution
				? 'resolutions'
				: parent && parent.id !== classId
					? parent.slug
					: 'categories'
			const headerDisplayName =
				header === 'resolutions' || header === 'categories'
					? undefined
					: localizeCurseForgeLabel(parent?.slug, parent?.name, header)
			return {
				icon: getCurseForgeImageUrl(category.iconUrl, 32) ?? '',
				icon_url: getCurseForgeImageUrl(category.iconUrl, 32),
				name: category.slug,
				display_name: displayName,
				header_display_name: headerDisplayName,
				display_index: category.displayIndex,
				project_type: projectType.value,
				header,
			}
		})
})

const allSourceCategoryTags = computed(() => {
	const classId = curseForgeClassIds[projectType.value]
	const modrinthCategories = categories.value ?? []
	if (!classId) return modrinthCategories

	const classCategories = curseForgeCategoriesByClass.value[classId] ?? []
	if (classCategories.length === 0) return modrinthCategories

	const unmapped = findUnmappedCurseForgeCategories(
		modrinthCategories.map((category) => category.name),
		classCategories,
	)

	const extraTags = unmapped.map((category) => ({
		icon: getCurseForgeImageUrl(category.iconUrl, 32) ?? '',
		icon_url: getCurseForgeImageUrl(category.iconUrl, 32),
		name: curseForgeCategoryValue(category.id),
		display_name: localizeCurseForgeCategoryName(category),
		display_index: category.displayIndex,
		project_type: projectType.value,
		header: CF_EXTRA_CATEGORY_HEADER,
	}))

	return [...modrinthCategories, ...extraTags]
})

const tags: Ref<Tags> = computed(() => ({
	gameVersions:
		contentSource.value === 'mcarchive'
			? mcArchiveGameVersions.value.map((version) => ({
					version: version.name,
					version_type: version.versionType ?? 'release',
					date: '',
					major: false,
				}))
			: (availableGameVersions.value ?? []),
	loaders:
		contentSource.value === 'mcarchive' || contentSource.value === 'planet_minecraft'
			? []
			: (loaders.value ?? []),
	categories:
		contentSource.value === 'mcarchive' || contentSource.value === 'planet_minecraft'
			? []
			: contentSource.value === 'curseforge'
				? curseForgeCategoryTags.value
				: contentSource.value === 'all'
					? allSourceCategoryTags.value
					: (categories.value ?? []),
}))

const instance = ref<GameInstance | null>(null)
const instanceSelector = ref()
const pendingRouteInstanceSwitch = ref<GameInstance | null>(null)
const activeInstance = computed(() => contentSelection.targetInstance.value ?? instance.value)
const mcArchiveAvailable = computed(() => {
	const gameVersion = activeInstance.value?.game_version
	if (!gameVersion) return false
	const metadata = availableGameVersions.value.find((version) => version.version === gameVersion)
	return metadata ? isVersionTypeMatch(metadata.version_type, gameVersion, 'ancient') : false
})
const installedProjectIds = ref<string[] | null>(null)
const installedProjectIdsInstanceId = ref<string | null>(null)
let installedProjectRequestId = 0
const instanceHideInstalled = ref(false)
const newlyInstalled = ref<string[]>([])
const hiddenInstanceProjectIds = ref<Set<string>>(new Set())
const hiddenInstanceProjectIdsInitialized = ref(false)
const isServerInstance = ref(false)

if (isFromWorlds.value && route.params.projectType !== 'server') {
	router.replace({
		path: '/browse/server',
		query: route.query,
	})
}

enforceSetupModpackRoute(route.params.projectType as string | undefined)

const scopedInstalledProjectIds = computed(() =>
	installedProjectIdsInstanceId.value === activeInstance.value?.id
		? (installedProjectIds.value ?? [])
		: [],
)

const allInstalledIds = computed(
	() => new Set([...newlyInstalled.value, ...scopedInstalledProjectIds.value]),
)

function syncHiddenInstanceProjectIds() {
	hiddenInstanceProjectIds.value = new Set([
		...scopedInstalledProjectIds.value,
		...newlyInstalled.value,
	])
	hiddenInstanceProjectIdsInitialized.value = true
}

watch(
	installedProjectIds,
	(ids) => {
		if (!ids) return
		if (!hiddenInstanceProjectIdsInitialized.value) {
			syncHiddenInstanceProjectIds()
		}
	},
	{ immediate: true },
)

watchServerContextChanges()

let initialInstanceFilterPromise: Promise<void> = Promise.resolve()
let initialInstalledProjectsPromise: Promise<void> = Promise.resolve()
await initInstanceContext()

async function refreshInstalledProjectIds() {
	const requestId = ++installedProjectRequestId
	const instanceId = activeInstance.value?.id
	if (!instanceId) {
		installedProjectIds.value = null
		installedProjectIdsInstanceId.value = null
		return
	}
	const requestInstanceId = instanceId
	if (installedProjectIdsInstanceId.value !== requestInstanceId) {
		installedProjectIds.value = null
		installedProjectIdsInstanceId.value = null
		hiddenInstanceProjectIds.value = new Set()
		hiddenInstanceProjectIdsInitialized.value = false
	}

	if (route.query.from === 'worlds') {
		const worlds = await get_instance_worlds(requestInstanceId).catch(handleError)
		if (!worlds) return
		if (requestId !== installedProjectRequestId || activeInstance.value?.id !== requestInstanceId)
			return

		const serverProjectIds = worlds
			.filter((w) => w.type === 'server' && 'project_id' in w && w.project_id)
			.map((w) => (w as { project_id: string }).project_id)
		debugLog('installedServerProjectIds loaded', { count: serverProjectIds.length })
		installedProjectIds.value = serverProjectIds
		installedProjectIdsInstanceId.value = requestInstanceId
		return
	}

	const ids = await getInstalledProjectIds(requestInstanceId).catch(handleError)
	if (
		ids &&
		requestId === installedProjectRequestId &&
		activeInstance.value?.id === requestInstanceId
	) {
		debugLog('installedProjectIds loaded', { count: ids.length })
		installedProjectIds.value = ids
		installedProjectIdsInstanceId.value = requestInstanceId
		await contentSelection.refreshInstalledIdentities()
	}
}

async function initInstanceContext() {
	debugLog('initInstanceContext', {
		queryI: route.query.i,
		queryAi: route.query.ai,
		querySid: route.query.sid,
		queryWid: route.query.wid,
		queryFrom: route.query.from,
	})
	await initServerContext()
	await contentSelection.refreshInstances((route.query.i as string | undefined) ?? undefined)

	if (route.query.i) {
		instance.value = (await getInstance(route.query.i as string).catch(handleError)) ?? null
		if (
			instance.value?.install_stage === 'installed' &&
			(contentSelection.selectedCount.value === 0 ||
				contentSelection.targetInstance.value?.id === instance.value.id)
		) {
			contentSelection.setTarget(instance.value)
		} else if (
			instance.value?.install_stage === 'installed' &&
			contentSelection.selectedCount.value > 0 &&
			contentSelection.targetInstance.value?.id !== instance.value.id
		) {
			pendingRouteInstanceSwitch.value = instance.value
		}
		debugLog('instance loaded', {
			name: instance.value?.name,
			loader: instance.value?.loader,
			gameVersion: instance.value?.game_version,
		})

		if (instance.value?.link?.project_id) {
			debugLog('checking linked project for server status', instance.value.link.project_id)
			initialInstanceFilterPromise = get_project_v3(
				instance.value.link.project_id,
				'must_revalidate',
			)
				.then((projectV3) => {
					if (projectV3?.minecraft_server != null) {
						debugLog('instance is a server instance')
						isServerInstance.value = true
					}
				})
				.catch((error) => handleError(error))
		}
	}
	if (!instance.value && activeInstance.value?.link?.project_id) {
		initialInstanceFilterPromise = get_project_v3(
			activeInstance.value.link.project_id,
			'must_revalidate',
		)
			.then((projectV3) => {
				isServerInstance.value = projectV3?.minecraft_server != null
			})
			.catch((error) => handleError(error))
	}

	if (route.query.ai && !(route.params.projectType === 'modpack')) {
		debugLog('setting instanceHideInstalled from query', route.query.ai)
		instanceHideInstalled.value = route.query.ai === 'true'
	}
	initialInstalledProjectsPromise = refreshInstalledProjectIds()
}

const instanceFilters = computed(() => {
	const filters = []

	if (activeInstance.value) {
		const gameVersion = activeInstance.value.game_version
		if (gameVersion && usesTargetGameVersion(projectType.value)) {
			filters.push({ type: 'game_version', option: gameVersion })
		}

		const platform = activeInstance.value.loader
		const supportedModLoaders = ['fabric', 'forge', 'quilt', 'neoforge']

		if (platform && projectType.value === 'mod' && supportedModLoaders.includes(platform)) {
			filters.push({ type: 'mod_loader', option: platform })
		}

		if (isServerInstance.value) {
			filters.push({ type: 'environment', option: 'client' })
		}

		if (instanceHideInstalled.value && hiddenInstanceProjectIds.value.size > 0) {
			for (const id of hiddenInstanceProjectIds.value) {
				filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
			}
		}
	}

	return filters
})

const serverHideInstalled = ref(false)
const hideSelectedServerInstalls = ref(false)
if (route.query.shi) {
	serverHideInstalled.value = route.query.shi === 'true'
}
const hiddenServerContentProjectIds = ref<Set<string>>(new Set())
const hiddenServerContentProjectIdsInitialized = ref(false)

function syncHiddenServerContentProjectIds() {
	hiddenServerContentProjectIds.value = new Set(serverContentProjectIds.value)
	hiddenServerContentProjectIdsInitialized.value = true
}

watch(
	serverContentProjectIds,
	() => {
		if (!hiddenServerContentProjectIdsInitialized.value) {
			syncHiddenServerContentProjectIds()
		}
	},
	{ immediate: true },
)

const serverContextFilters = computed(() => {
	const filters: { type: string; option: string; negative?: boolean }[] = []
	if (!serverContextServerData.value) return filters
	const pt = projectType.value

	if (pt !== 'modpack') {
		const gameVersion = serverContextServerData.value.mc_version
		if (gameVersion && usesTargetGameVersion(pt)) {
			filters.push({ type: 'game_version', option: gameVersion })
		}

		const platform = serverContextServerData.value.loader?.toLowerCase()
		if (platform && ['fabric', 'forge', 'quilt', 'neoforge'].includes(platform))
			filters.push({ type: 'mod_loader', option: platform })
		if (platform && ['paper', 'purpur'].includes(platform))
			filters.push({ type: 'plugin_loader', option: platform })

		if (pt === 'mod') filters.push({ type: 'environment', option: 'server' })

		if (hideSelectedServerInstalls.value && queuedServerInstallProjectIds.value.size > 0) {
			for (const id of queuedServerInstallProjectIds.value) {
				filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
			}
		}
	}

	if (pt === 'modpack') {
		filters.push(
			{ type: 'environment', option: 'client' },
			{ type: 'environment', option: 'server' },
		)
	}

	if (serverHideInstalled.value && hiddenServerContentProjectIds.value.size > 0) {
		for (const id of hiddenServerContentProjectIds.value) {
			filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
		}
	}

	return filters
})

const combinedProvidedFilters = computed(() => {
	if (contentSource.value === 'mcarchive') return []
	if (isServerContext.value) return serverContextFilters.value
	if (projectType.value === 'modpack' || projectType.value === 'server') return []
	return instanceFilters.value
})

const {
	serverPings,
	contextMenuRef,
	updateServerHits,
	getServerModpackContent,
	getServerCardActions,
	handleRightClick,
	handleOptionsClick,
} = useAppServerBrowse({
	instance,
	isFromWorlds,
	allInstalledIds,
	newlyInstalled,
	installingServerProjects,
	playServerProject,
	showAddServerToInstanceModal,
	handleError,
	router,
})

const { offline } = useNetworkStatus()

const messages = defineMessages({
	add: {
		id: 'app.browse.add',
		defaultMessage: 'Add',
	},
	chooseInstance: {
		id: 'app.browse.choose-instance',
		defaultMessage: 'Choose instance',
	},
	installSelected: {
		id: 'app.browse.install-selected',
		defaultMessage: 'Install {count} content',
	},
	preparingSelected: {
		id: 'app.browse.preparing-selected',
		defaultMessage: 'Preparing {completed}/{total}',
	},
	selected: {
		id: 'app.browse.selected',
		defaultMessage: 'Selected',
	},
	addToFavorites: {
		id: 'app.content-favorites.add',
		defaultMessage: 'Add to favorites',
	},
	removeFromFavorites: {
		id: 'app.content-favorites.remove',
		defaultMessage: 'Remove from favorites',
	},
	noCompatibleVersion: {
		id: 'app.browse.no-compatible-version',
		defaultMessage: 'No compatible version was found for the selected instance.',
	},
	addServersToInstance: {
		id: 'app.browse.add-servers-to-instance',
		defaultMessage: 'Adding server to instance',
	},
	addToAnInstance: {
		id: 'app.browse.add-to-an-instance',
		defaultMessage: 'Add to an instance',
	},
	discoverContent: {
		id: 'app.browse.discover-content',
		defaultMessage: 'Discover content',
	},
	discoverServers: {
		id: 'app.browse.discover-servers',
		defaultMessage: 'Discover servers',
	},
	discoverMaps: {
		id: 'app.browse.discover-maps',
		defaultMessage: 'Discover maps',
	},
	mapsProjectType: {
		id: 'app.browse.project-type.maps',
		defaultMessage: 'Maps',
	},
	mapsUnavailable: {
		id: 'app.browse.maps-unavailable',
		defaultMessage: 'Maps unavailable',
	},
	mapsUnavailableDescription: {
		id: 'app.browse.maps-unavailable-description',
		defaultMessage: 'Configure a CurseForge API key to browse and install maps.',
	},
	mapsNoInstallableFile: {
		id: 'app.browse.maps-no-installable-file',
		defaultMessage: 'The selected CurseForge map does not have an installable file.',
	},
	fuzzySearchPrefix: {
		id: 'app.browse.fuzzy-search.prefix',
		defaultMessage: 'No results found for “{original}”. Showing results for',
	},
	fuzzySearchSuffix: {
		id: 'app.browse.fuzzy-search.suffix',
		defaultMessage: 'instead',
	},
	environmentProvidedByServer: {
		id: 'search.filter.locked.server-environment.title',
		defaultMessage: 'Only client-side mods can be added to the server instance',
	},
	gameVersionProvidedByInstance: {
		id: 'search.filter.locked.instance-game-version.title',
		defaultMessage: 'Game version is provided by the instance',
	},
	gameVersionProvidedByServer: {
		id: 'search.filter.locked.server-game-version.title',
		defaultMessage: 'Game version is provided by the server',
	},
	hideAddedServers: {
		id: 'app.browse.hide-added-servers',
		defaultMessage: 'Hide servers already added',
	},
	installingToServer: {
		id: 'app.browse.server.installing',
		defaultMessage: 'Installing',
	},
	backToInstance: {
		id: 'app.browse.back-to-instance',
		defaultMessage: 'Back to instance',
	},
	serverInstanceContentWarning: {
		id: 'app.browse.server-instance-content-warning',
		defaultMessage:
			'Adding content can break compatibility when joining the server. Any added content will also be lost when you update the server instance content.',
	},
	modLoaderProvidedByInstance: {
		id: 'search.filter.locked.instance-loader.title',
		defaultMessage: 'Loader is provided by the instance',
	},
	modpacksProjectType: {
		id: 'app.browse.project-type.modpacks',
		defaultMessage: 'Modpacks',
	},
	modsProjectType: {
		id: 'app.browse.project-type.mods',
		defaultMessage: 'Mods',
	},
	resourcepacksProjectType: {
		id: 'app.browse.project-type.resourcepacks',
		defaultMessage: 'Resource Packs',
	},
	datapacksProjectType: {
		id: 'app.browse.project-type.datapacks',
		defaultMessage: 'Data Packs',
	},
	shadersProjectType: {
		id: 'app.browse.project-type.shaders',
		defaultMessage: 'Shaders',
	},
	serversProjectType: {
		id: 'app.browse.project-type.servers',
		defaultMessage: 'Servers',
	},
	favoritesProjectType: {
		id: 'app.browse.project-type.favorites',
		defaultMessage: 'Favorites',
	},
	modLoaderProvidedByServer: {
		id: 'search.filter.locked.server-loader.title',
		defaultMessage: 'Loader is provided by the server',
	},
	providedByInstance: {
		id: 'search.filter.locked.instance',
		defaultMessage: 'Provided by the instance',
	},
	providedByServer: {
		id: 'search.filter.locked.server',
		defaultMessage: 'Provided by the server',
	},
	syncFilterButton: {
		id: 'search.filter.locked.instance.sync',
		defaultMessage: 'Sync with instance',
	},
	allSources: {
		id: 'app.browse.source.all',
		defaultMessage: 'All sources',
	},
	compactListView: {
		id: 'app.browse.display-mode.compact-list',
		defaultMessage: 'Compact list',
	},
	gridView: {
		id: 'app.browse.display-mode.grid',
		defaultMessage: 'Grid',
	},
	listView: {
		id: 'app.browse.display-mode.list',
		defaultMessage: 'List',
	},
	modrinthSource: {
		id: 'app.browse.source.modrinth',
		defaultMessage: 'Modrinth',
	},
	switchView: {
		id: 'app.browse.display-mode.switch',
		defaultMessage: 'Switch view',
	},
	curseForgeSource: {
		id: 'app.browse.source.curseforge',
		defaultMessage: 'CurseForge',
	},
	mcArchiveSource: {
		id: 'app.browse.source.mcarchive',
		defaultMessage: 'MCArchive',
	},
	planetMinecraftSource: {
		id: 'app.browse.source.planet-minecraft',
		defaultMessage: 'Planet Minecraft',
	},
	translateProject: {
		id: 'app.project.translation.translate',
		defaultMessage: 'Translate',
	},
	showOriginal: {
		id: 'app.project.translation.show-original',
		defaultMessage: 'Show original',
	},
	translating: {
		id: 'app.project.translation.translating',
		defaultMessage: 'Translating…',
	},
})

const sourceIcon = computed(() => {
	if (!sourceOptions.value.some((option) => option.id === contentSource.value)) {
		return GlobeIcon
	}
	switch (contentSource.value) {
		case 'modrinth':
			return ModrinthIcon
		case 'curseforge':
			return CurseForgeIcon
		case 'mcarchive':
			return FileArchiveIcon
		case 'planet_minecraft':
			return GlobeIcon
		default:
			return GlobeIcon
	}
})

const sourceOptions = computed(() =>
	isWorldMapBrowse.value
		? [{ id: 'curseforge' as const, label: messages.curseForgeSource, icon: CurseForgeIcon }]
		: [
				{ id: 'all' as const, label: messages.allSources, icon: GlobeIcon },
				{ id: 'modrinth' as const, label: messages.modrinthSource, icon: ModrinthIcon },
				...(curseForgeCapability.value.configured
					? [{ id: 'curseforge' as const, label: messages.curseForgeSource, icon: CurseForgeIcon }]
					: []),
				...(projectType.value === 'mod' && mcArchiveAvailable.value
					? [{ id: 'mcarchive' as const, label: messages.mcArchiveSource, icon: FileArchiveIcon }]
					: []),
				...(projectType.value === 'mod' && planetMinecraftAvailable.value
					? [
							{
								id: 'planet_minecraft' as const,
								label: messages.planetMinecraftSource,
								icon: GlobeIcon,
							},
						]
					: []),
			],
)

const currentSourceLabel = computed(() => {
	const current = sourceOptions.value.find((opt) => opt.id === contentSource.value)
	return current?.label ?? messages.allSources
})

const breadcrumbs = useBreadcrumbs()
const browseTitle = computed(() =>
	formatMessage(
		isFromWorlds.value
			? messages.discoverServers
			: isWorldMapBrowse.value
				? messages.discoverMaps
				: messages.discoverContent,
	),
)
breadcrumbs.setName('BrowseTitle', browseTitle.value)
if (instance.value) {
	const instanceLink = `/instance/${encodeURIComponent(instance.value.id)}`
	const instanceIcon = getDisplayInstanceIcon(instance.value.icon_path, instance.value.loader).url
	breadcrumbs.setContext({
		name: instance.value.name,
		link: isFromWorlds.value || isWorldMapContext.value ? `${instanceLink}/worlds` : instanceLink,
		iconUrl: instanceIcon,
	})
} else {
	breadcrumbs.setContext(null)
}

onBeforeRouteLeave((to) => {
	if (isBrowseReturnSourcePath(to.path)) {
		const viewport = document.querySelector<HTMLElement>('.app-viewport')
		saveBrowseReturnSnapshot({
			url: route.fullPath,
			scrollTop: viewport?.scrollTop ?? 0,
			state: {},
		})
	}

	breadcrumbs.setContext({
		name: '?BrowseTitle',
		link: `/browse/${projectType.value}`,
		query: route.query,
	})
})

function resetInstanceContext() {
	if (!instance.value) return

	debugLog('instance context removed, resetting')
	instance.value = null
	installedProjectRequestId += 1
	installedProjectIds.value = null
	installedProjectIdsInstanceId.value = null
	instanceHideInstalled.value = false
	newlyInstalled.value = []
	hiddenInstanceProjectIds.value = new Set()
	hiddenInstanceProjectIdsInitialized.value = false
	isServerInstance.value = false
	breadcrumbs.setName('BrowseTitle', formatMessage(messages.discoverContent))
	breadcrumbs.setContext(null)
}

watch(
	() => route.params.projectType as ProjectType,
	async (newType) => {
		if (isSetupServerContext.value) {
			enforceSetupModpackRoute(newType)
			if (newType !== 'modpack') return
		}

		if (!newType || newType === projectType.value) return

		debugLog('projectType route param changed', { from: projectType.value, to: newType })
		projectType.value = newType
	},
)

watch(
	() => route.query.i,
	(instanceId) => {
		if (!instanceId && route.path.startsWith('/browse')) {
			resetInstanceContext()
		}
	},
)

const selectableProjectTypes = computed(() => {
	const params: LocationQuery = {}

	if (route.query.i) params.i = route.query.i
	if (route.query.ai) params.ai = route.query.ai
	if (route.query.from) params.from = route.query.from
	if (route.query.sid) params.sid = route.query.sid
	if (effectiveServerWorldId.value) params.wid = effectiveServerWorldId.value

	const queryString = new URLSearchParams(params as Record<string, string>).toString()
	const suffix = queryString ? `?${queryString}` : ''

	if (isSetupServerContext.value) {
		return [
			{ label: formatMessage(messages.modpacksProjectType), href: `/browse/modpack${suffix}` },
		]
	}

	if (isFromWorlds.value) {
		return [{ label: formatMessage(messages.serversProjectType), href: `/browse/server${suffix}` }]
	}
	if (isWorldMapContext.value) {
		return [
			{
				label: formatMessage(messages.mapsProjectType),
				href: `/browse/${WORLD_BROWSE_PROJECT_TYPE}${suffix}`,
			},
		]
	}

	return createBrowseProjectTabs(
		{
			modpacks: formatMessage(messages.modpacksProjectType),
			mods: formatMessage(messages.modsProjectType),
			resourcepacks: formatMessage(messages.resourcepacksProjectType),
			datapacks: formatMessage(messages.datapacksProjectType),
			maps: formatMessage(messages.mapsProjectType),
			shaders: formatMessage(messages.shadersProjectType),
			servers: formatMessage(messages.serversProjectType),
			favorites: formatMessage(messages.favoritesProjectType),
		},
		suffix,
		getBrowseProjectTabOptions({
			instance: activeInstance.value,
			hasInstanceContext: !!instance.value,
			isServerInstance: isServerInstance.value,
		}),
	)
})

const installContext = computed(() => {
	if (isServerContext.value && serverContextServerData.value) {
		return {
			name: serverContextServerData.value.name,
			loader: serverContextServerData.value.loader ?? '',
			gameVersion: serverContextServerData.value.mc_version ?? '',
			serverId: serverIdQuery.value,
			upstream: serverContextServerData.value.upstream,
			iconSrc: null as string | null,
			isMedal: serverContextServerData.value.is_medal,
			backUrl: serverBackUrl.value,
			backLabel: serverBackLabel.value,
			heading: serverBrowseHeading.value,
			queuedCount: queuedServerInstallCount.value,
			selectedProjects: selectedServerInstallProjects.value,
			isInstallingSelected: isInstallingQueuedServerInstalls.value,
			skipNonEssentialWarnings: themeStore.getFeatureFlag('skip_non_essential_warnings'),
			installProgress: queuedInstallProgress.value,
			clearQueued: clearQueuedServerInstalls,
			clearSelected: clearQueuedServerInstalls,
			onBack: flushQueuedServerInstalls,
			discardSelectedAndBack: discardQueuedServerInstallsAndBack,
			installSelected: installQueuedServerInstallsAndBack,
		}
	}
	if (activeInstance.value) {
		const target = activeInstance.value
		const displayIcon = getDisplayInstanceIcon(target.icon_path, target.loader)
		const processing = ['validating', 'reviewing', 'queueing'].includes(
			contentSelection.state.value,
		)
		return {
			showInstallHeader: !!instance.value,
			name: target.name,
			loader: target.loader,
			gameVersion: target.game_version,
			iconSrc: displayIcon.url,
			iconFrameless: displayIcon.frameless,
			backUrl: instance.value
				? `/instance/${encodeURIComponent(instance.value.id)}${isFromWorlds.value || isWorldMapContext.value ? '/worlds' : ''}`
				: route.fullPath,
			backLabel: formatMessage(messages.backToInstance),
			heading: formatMessage(
				isFromWorlds.value ? messages.addServersToInstance : commonMessages.installingContentLabel,
			),
			warning:
				isServerInstance.value && !isFromWorlds.value
					? formatMessage(messages.serverInstanceContentWarning)
					: undefined,
			selectedProjects: contentSelection.selectedProjects.value,
			isInstallingSelected: processing,
			installProgress: contentSelection.progress.value,
			installButtonLabel: formatMessage(messages.installSelected, {
				count: contentSelection.selectedCount.value,
			}),
			processingLabel: formatMessage(messages.preparingSelected, {
				completed: contentSelection.progress.value.completed,
				total: contentSelection.progress.value.total,
			}),
			clearSelected: contentSelection.clear,
			installSelected: contentSelection.installSelected,
		}
	}
	return null
})

const installingProjectIds = ref<Set<string>>(new Set())
const CART_CONTENT_TYPES = new Set(['mod', 'resourcepack', 'datapack', 'shader', 'world'])

function projectInstallingKey(projectId: string, instanceId?: string | null) {
	return `${instanceId ?? activeInstance.value?.id ?? ''}\0${projectId}`
}

function setProjectInstalling(
	projectId: string,
	installing: boolean,
	instanceId?: string | null,
) {
	const key = projectInstallingKey(projectId, instanceId)
	const next = new Set(installingProjectIds.value)
	if (installing) {
		next.add(key)
	} else {
		next.delete(key)
	}
	installingProjectIds.value = next
}

async function selectTargetInstance(target: GameInstance) {
	contentSelection.setTarget(target)
	newlyInstalled.value = []
	hiddenInstanceProjectIds.value = new Set()
	hiddenInstanceProjectIdsInitialized.value = false
	if (route.query.i) {
		instance.value = target
		await router.replace({ query: { ...route.query, i: target.id } })
		breadcrumbs.setContext({
			name: target.name,
			link: `/instance/${encodeURIComponent(target.id)}`,
			iconUrl: getDisplayInstanceIcon(target.icon_path, target.loader).url,
		})
	}
	await refreshInstalledProjectIds()
	await searchState.refreshSearch()
}

async function cancelTargetInstanceSwitch() {
	const target = contentSelection.targetInstance.value
	if (!target || !route.query.i || route.query.i === target.id) return
	instance.value = target
	await router.replace({ query: { ...route.query, i: target.id } })
	breadcrumbs.setContext({
		name: target.name,
		link: `/instance/${encodeURIComponent(target.id)}`,
		iconUrl: getDisplayInstanceIcon(target.icon_path, target.loader).url,
	})
}

async function toggleContentSelection(
	project: (Labrinth.Search.v2.ResultSearchProject & Labrinth.Search.v3.ResultSearchProject) & {
		provider: BrowseContentProvider
		provider_project_id?: string
		latest_version?: string | null
	},
	contentType: string,
) {
	const target = activeInstance.value
	if (!target) {
		instanceSelector.value?.show()
		return
	}
	const providerProjectId =
		project.provider === 'curseforge'
			? (project.provider_project_id ?? project.project_id.replace(/^curseforge:/, ''))
			: project.project_id
	const key = makeContentSelectionKey(project.provider, providerProjectId)
	if (contentSelection.isSelected(key)) {
		contentSelection.remove(key)
		return
	}

	setProjectInstalling(project.project_id, true, target.id)
	try {
		const preferences = getInstanceInstallTargetPreferences(contentType)
		let versionId = project.latest_version || null
		if (project.provider === 'modrinth') {
			versionId =
				getLatestMatchingInstallVersion(
					await getInstallProjectVersions(project.project_id),
					preferences,
				)?.id ?? null
		} else {
			const files = await getCurseForgeFiles(Number(providerProjectId), {
				gameVersion: usesTargetGameVersion(contentType) ? target.game_version : undefined,
				modLoaderType: contentType === 'mod' ? curseForgeLoaderTypes[target.loader] : undefined,
			})
			versionId = files.files.find((file) => file.isAvailable)?.id.toString() ?? null
		}
		if (!versionId) {
			throw new Error(
				contentType === WORLD_BROWSE_PROJECT_TYPE
					? formatMessage(messages.mapsNoInstallableFile)
					: formatMessage(messages.noCompatibleVersion),
			)
		}
		await contentSelection.add({
			key,
			provider: project.provider,
			projectId: project.provider === 'modrinth' ? project.project_id : providerProjectId,
			providerProjectId,
			versionId,
			contentType: contentType as 'mod' | 'resourcepack' | 'datapack' | 'shader' | 'world',
			title: project.title,
			iconUrl: project.icon_url,
			slug: project.slug,
			preferences,
		})
	} finally {
		setProjectInstalling(project.project_id, false, target.id)
	}
}

const serverInstallQueue = {
	get: getQueuedServerInstallPlans,
	set: setQueuedServerInstallPlans,
}

function getCurrentSelectedInstallPreferences(projectTypeValue: string) {
	return getSelectedInstallPreferences({
		contentType: projectTypeValue,
		selectedFilters: searchState.currentFilters.value,
		providedFilters: combinedProvidedFilters.value,
		overriddenProvidedFilterTypes: searchState.overriddenProvidedFilterTypes.value,
	})
}

function getServerInstallTargetPreferences(contentType: BrowseInstallContentType) {
	return getTargetInstallPreferences(
		{
			gameVersion: serverContextServerData.value?.mc_version,
			loader: serverContextServerData.value?.loader,
		},
		contentType,
	)
}

function getInstanceInstallTargetPreferences(projectTypeValue: string) {
	return getTargetInstallPreferences(
		{
			gameVersion: activeInstance.value?.game_version,
			loader: activeInstance.value?.loader,
		},
		projectTypeValue,
	)
}

async function getInstallProjectVersions(projectId: string) {
	const project = await get_project(projectId, 'must_revalidate')
	return (await get_version_many(
		project.versions,
		'must_revalidate',
	)) as Labrinth.Versions.v2.Version[]
}

async function chooseInstanceInstallVersion(
	project: Labrinth.Search.v2.ResultSearchProject & Labrinth.Search.v3.ResultSearchProject,
	projectTypeValue: string,
) {
	const targetInstance = activeInstance.value
	if (!targetInstance) {
		return { versionId: null as string | null }
	}

	const selectedPreferences = getCurrentSelectedInstallPreferences(projectTypeValue)
	const targetPreferences = getInstanceInstallTargetPreferences(projectTypeValue)
	if (!preferencesDiffer(selectedPreferences, targetPreferences)) {
		return { versionId: null as string | null }
	}

	const selectedVersion = getLatestMatchingInstallVersion(
		await getInstallProjectVersions(project.project_id),
		selectedPreferences,
	)

	if (!selectedVersion) {
		return { versionId: null as string | null }
	}

	return { versionId: selectedVersion.id }
}

type BrowseContentProvider = 'modrinth' | 'curseforge' | 'mcarchive' | 'planet_minecraft'

function isBrowseContentProvider(provider: unknown): provider is BrowseContentProvider {
	return (
		provider === 'modrinth' ||
		provider === 'curseforge' ||
		provider === 'mcarchive' ||
		provider === 'planet_minecraft'
	)
}

function getCardActions(
	result: Labrinth.Search.v2.ResultSearchProject | Labrinth.Search.v3.ResultSearchProject,
	currentProjectType: string,
): CardAction[] {
	if (currentProjectType === 'server') {
		return getServerCardActions(result as Labrinth.Search.v3.ResultSearchProject)
	}

	// Non-server project actions
	const projectResult = result as (Labrinth.Search.v2.ResultSearchProject &
		Labrinth.Search.v3.ResultSearchProject) & {
		installed?: boolean
		installing?: boolean
		provider?: unknown
		provider_project_id?: string
	}
	if (!isBrowseContentProvider(projectResult.provider)) return []
	if (projectResult.provider === 'mcarchive' || projectResult.provider === 'planet_minecraft')
		return []
	const providerProjectId =
		projectResult.provider === 'curseforge'
			? (projectResult.provider_project_id ?? projectResult.project_id.replace(/^curseforge:/, ''))
			: projectResult.project_id
	const selectionKey = makeContentSelectionKey(projectResult.provider, providerProjectId)
	const isInstalling =
		installingProjectIds.value.has(projectInstallingKey(projectResult.project_id)) ||
		contentSelection.isInstalling(selectionKey)
	const isSelected = contentSelection.isSelected(selectionKey)
	const isInstalled =
		(isServerContext.value
			? serverContentProjectIds.value.has(projectResult.project_id || '')
			: !!activeInstance.value && allInstalledIds.value.has(projectResult.project_id || '')) ||
		(!isServerContext.value &&
			!!activeInstance.value &&
			contentSelection.isInstalledIdentity(
				projectResult.provider,
				providerProjectId,
				projectResult.slug,
			)) ||
		(isServerContext.value &&
			serverContextServerData.value?.upstream?.project_id === projectResult.project_id)
	const favoriteAction =
		!isServerContext.value && isFavoriteContentType(currentProjectType)
			? createFavoriteCardAction(projectResult, providerProjectId, currentProjectType)
			: null

	if (
		isServerContext.value &&
		projectResult.provider === 'modrinth' &&
		['modpack', 'mod', 'plugin', 'datapack'].includes(currentProjectType)
	) {
		const isQueued = queuedServerInstallProjectIds.value.has(projectResult.project_id)
		const isInstallingSelection = isInstallingQueuedServerInstalls.value
		const validatingInstall =
			isInstalling && currentProjectType !== 'modpack' && !isInstallingSelection
		const installLabel = isInstalled
			? commonMessages.installedLabel
			: isQueued
				? isInstalling || isInstallingSelection
					? validatingInstall
						? commonMessages.validatingLabel
						: messages.installingToServer
					: commonMessages.selectedLabel
				: isInstalling || isInstallingSelection
					? validatingInstall
						? commonMessages.validatingLabel
						: messages.installingToServer
					: commonMessages.installButton
		return [
			{
				key: 'install',
				label: formatMessage(installLabel),
				icon:
					isInstalling || isInstallingSelection
						? SpinnerIcon
						: isQueued || isInstalled
							? CheckIcon
							: PlusIcon,
				iconClass: isInstalling || isInstallingSelection ? 'animate-spin' : undefined,
				disabled: isInstalled || isInstalling || isInstallingSelection,
				color: isQueued && !isInstalling && !isInstallingSelection ? 'green' : 'brand',
				type: 'outlined',
				onClick: async () => {
					const installInstanceId = activeInstance.value?.id ?? null
					if (isQueued) {
						removeQueuedServerInstall(projectResult.project_id)
						return
					}

					const contentType = currentProjectType as BrowseInstallContentType
					const isModpack = contentType === 'modpack'
					const shouldShowInstalling = isModpack || !isQueued
					if (shouldShowInstalling) {
						setProjectInstalling(projectResult.project_id, true, installInstanceId)
					}
					try {
						await requestInstall({
							project: projectResult,
							contentType,
							mode: isModpack ? 'immediate' : 'queue',
							selectedFilters: isModpack
								? []
								: stripServerRuntimeInstallFilters(searchState.currentFilters.value),
							providedFilters: isModpack ? [] : combinedProvidedFilters.value,
							overriddenProvidedFilterTypes: isModpack
								? []
								: stripServerRuntimeInstallOverrides(
										searchState.overriddenProvidedFilterTypes.value,
									),
							targetPreferences: getServerInstallTargetPreferences(contentType),
							getProjectVersions: getInstallProjectVersions,
							queue: serverInstallQueue,
							install: (plan) =>
								openServerModpackInstallFlow({
									projectId: plan.projectId,
									versionId: plan.versionId,
									name: plan.project.name,
									iconUrl: plan.project.icon_url ?? undefined,
								}),
						})
					} catch (err) {
						handleError(err as Error)
					} finally {
						if (shouldShowInstalling) {
							setProjectInstalling(projectResult.project_id, false, installInstanceId)
						}
					}
				},
			},
			...(favoriteAction ? [favoriteAction] : []),
		]
	}

	const isModpack =
		projectResult.project_types?.includes('modpack') || projectResult.project_type === 'modpack'
	if (CART_CONTENT_TYPES.has(currentProjectType)) {
		if (
			currentProjectType === WORLD_BROWSE_PROJECT_TYPE &&
			projectResult.provider !== 'curseforge'
		) {
			return []
		}
		return [
			{
				key: 'install',
				label: formatMessage(
					isInstalling
						? commonMessages.validatingLabel
						: isSelected
							? messages.selected
							: activeInstance.value
								? commonMessages.installButton
								: messages.chooseInstance,
				),
				compactLabel:
					!isInstalling && !isSelected && !activeInstance.value
						? formatMessage(messages.add)
						: undefined,
				icon: isInstalling ? SpinnerIcon : isSelected ? CheckIcon : PlusIcon,
				iconClass: isInstalling ? 'animate-spin' : undefined,
				disabled: isInstalled || isInstalling,
				color: isSelected ? 'green' : 'brand',
				type: 'outlined',
				onClick: async () => {
					try {
						await toggleContentSelection(projectResult, currentProjectType)
					} catch (error) {
						handleError(error)
					}
				},
			},
			...(favoriteAction ? [favoriteAction] : []),
		]
	}
	const shouldUseInstallIcon = !!instance.value || isModpack
	const installActionLabel = isInstalling
		? messages.installingToServer
		: isInstalled
			? commonMessages.installedLabel
			: shouldUseInstallIcon
				? commonMessages.installButton
				: messages.addToAnInstance
	const compactInstallLabel =
		!isInstalling && !isInstalled && !shouldUseInstallIcon ? formatMessage(messages.add) : undefined

	return [
		{
			key: 'install',
			label: formatMessage(installActionLabel),
			compactLabel: compactInstallLabel,
			icon: isInstalling ? SpinnerIcon : isInstalled ? CheckIcon : PlusIcon,
			iconClass: isInstalling ? 'animate-spin' : undefined,
			disabled: isInstalled || isInstalling,
			color: 'brand',
			type: 'outlined',
			onClick: async () => {
				const installInstanceId = instance.value?.id ?? null
				setProjectInstalling(projectResult.project_id, true, installInstanceId)
				try {
					const selectedInstall =
						instance.value && projectResult.provider === 'modrinth'
							? await chooseInstanceInstallVersion(projectResult, currentProjectType)
							: { versionId: null as string | null }
					if (selectedInstall === null) {
						setProjectInstalling(projectResult.project_id, false, installInstanceId)
						return
					}
					const selectedPreferences = getCurrentSelectedInstallPreferences(currentProjectType)
					const installContent =
						projectResult.provider === 'curseforge'
							? installCurseForge
							: projectResult.provider === 'modrinth'
								? installVersion
								: null
					if (!installContent) return
					await installContent(
						projectResult.provider_project_id ?? projectResult.project_id,
						selectedInstall.versionId,
						instance.value ? instance.value.id : null,
						'SearchCard',
						(versionId, installedProjectIds) => {
							setProjectInstalling(projectResult.project_id, false, installInstanceId)
							if (versionId && activeInstance.value?.id === installInstanceId) {
								onSearchResultsInstalled(installedProjectIds ?? [projectResult.project_id])
							}
						},
						(profile) => {
							router.push(isModpack ? '/downloads' : `/instance/${profile}`)
						},
						{
							preferredLoader: instance.value?.loader ?? selectedPreferences.loaders?.[0],
							preferredGameVersion:
								instance.value?.game_version ?? selectedPreferences.gameVersions?.[0],
						},
					)
				} catch (err) {
					setProjectInstalling(projectResult.project_id, false, installInstanceId)
					handleError(err)
				}
			},
		},
	]
}

function createFavoriteCardAction(
	project: Labrinth.Search.v2.ResultSearchProject & {
		provider: BrowseContentProvider
	},
	providerProjectId: string,
	contentType: FavoriteContentType,
): CardAction {
	const favorite = contentFavorites.isFavorite(project.provider, providerProjectId)
	const pending = contentFavorites.isPending(project.provider, providerProjectId)
	return {
		key: 'favorite',
		label: formatMessage(favorite ? messages.removeFromFavorites : messages.addToFavorites),
		icon: pending ? SpinnerIcon : favorite ? BookmarkFilledIcon : BookmarkIcon,
		iconClass: pending ? 'animate-spin' : undefined,
		disabled: pending,
		color: favorite ? 'brand' : undefined,
		type: 'transparent',
		circular: true,
		tooltip: formatMessage(favorite ? messages.removeFromFavorites : messages.addToFavorites),
		onClick: async () => {
			try {
				await contentFavorites.toggle({
					provider: project.provider,
					project_id: providerProjectId,
					content_type: contentType,
				})
			} catch (error) {
				handleError(error)
			}
		},
	}
}

function onSearchResultInstalled(id: string) {
	if (isServerContext.value) {
		markServerProjectInstalled(id)
		return
	}
	if (!newlyInstalled.value.includes(id)) {
		newlyInstalled.value = [...newlyInstalled.value, id]
	}
}

function onSearchResultsInstalled(ids: string[]) {
	if (isServerContext.value) {
		for (const id of ids) {
			markServerProjectInstalled(id)
		}
		return
	}
	newlyInstalled.value = Array.from(new Set([...newlyInstalled.value, ...ids]))
}

const curseForgeLoaderTypes: Record<string, number> = {
	forge: 1,
	fabric: 4,
	quilt: 5,
	neoforge: 6,
}

function extractQuotedFilterValues(source: string): string[] {
	return [...source.matchAll(/[`"]([^`"]+)[`"]/g)].map((match) => match[1])
}

function getFirstSearchFilter(filters: string, field: string) {
	// Modrinth search filter values are backtick-quoted (`value`); keep double-quote
	// support for any legacy/manual strings.
	return new RegExp(`${field}\\s*(?:=|IN\\s*\\[)\\s*[\`"]([^\`"]+)`).exec(filters)?.[1]
}

function getSearchFilterValues(filters: string, field: string) {
	const values: string[] = []
	const pattern = new RegExp(`${field}\\s*(?:=\\s*[\`"]([^\`"]+)[\`"]|IN\\s*\\[([^\\]]+)\\])`, 'g')
	for (const match of filters.matchAll(pattern)) {
		if (match[1]) {
			values.push(match[1])
		} else if (match[2]) {
			values.push(...extractQuotedFilterValues(match[2]))
		}
	}
	return values
}

function stripCurseForgeOnlyCategoryFilters(requestParams: string) {
	const params = new URLSearchParams(
		requestParams.startsWith('?') ? requestParams.slice(1) : requestParams,
	)
	const filters = params.get('new_filters')
	if (!filters) return requestParams

	const parts = filters
		.split(' AND ')
		.map((part) => part.trim())
		.filter(Boolean)
		.flatMap((part) => {
			if (!part.includes('categories') || !part.includes('cf:')) return [part]

			const equalMatch = /^categories\s*=\s*[`"]([^`"]+)[`"]$/.exec(part)
			if (equalMatch) {
				return equalMatch[1].startsWith('cf:') ? [] : [part]
			}

			const inMatch = /^categories\s+IN\s+\[([^\]]+)\]$/.exec(part)
			if (inMatch) {
				const kept = extractQuotedFilterValues(inMatch[1]).filter(
					(value) => !value.startsWith('cf:'),
				)
				if (kept.length === 0) return []
				if (kept.length === 1) return [`categories = \`${kept[0]}\``]
				return [`categories IN [${kept.map((value) => `\`${value}\``).join(', ')}]`]
			}

			// Unknown shape containing cf: — drop rather than send invalid MR facets.
			return []
		})

	if (parts.length === 0) {
		params.delete('new_filters')
	} else {
		params.set('new_filters', parts.join(' AND '))
	}

	const query = params.toString()
	return query ? `?${query}` : ''
}

function getCurseForgeCategoryIds(filters: string) {
	const classId = curseForgeClassIds[projectType.value]
	if (!classId || contentSource.value === 'modrinth') return []

	const classCategories = curseForgeCategoriesByClass.value[classId] ?? []
	if (classCategories.length === 0) return []

	const loaderSlugs = new Set(Object.keys(curseForgeLoaderTypes))
	return resolveCurseForgeCategoryIdsFromFilterValues(
		getSearchFilterValues(filters, 'categories'),
		classCategories,
		loaderSlugs,
	)
}

function getCurseForgeSortField(sort: string | null) {
	switch (sort) {
		case 'downloads':
			return 6
		case 'newest':
			return 11
		case 'updated':
			return 3
		case 'follows':
			return 12
		default:
			return undefined
	}
}

const CURSEFORGE_LOADER_SLUGS = new Set([
	'forge',
	'fabric',
	'quilt',
	'neoforge',
	'liteloader',
	'bukkit',
	'spigot',
	'paper',
	'bungeecord',
	'velocity',
	'sponge',
	'waterfall',
	'folia',
	'purpur',
	'iris',
	'optifine',
	'canvas',
	'geyser',
])

function mapCurseForgeHit(hit: UnifiedSearchHit) {
	const categories = hit.categories.map((cat) => {
		const normalized = cat.toLowerCase().replace(/[_\s]+/g, '-')
		if (CURSEFORGE_LOADER_SLUGS.has(normalized)) {
			return normalized
		}
		return localizeCurseForgeLabel(cat)
	})

	return {
		project_id: `curseforge:${hit.project_id}`,
		provider_project_id: hit.project_id,
		provider: 'curseforge' as const,
		project_type: hit.project_type,
		slug: hit.slug,
		author: hit.author,
		author_url: hit.author_url,
		title: hit.title,
		description: hit.description,
		categories,
		display_categories: categories,
		versions: hit.versions,
		downloads: hit.downloads,
		follows: 0,
		icon_url: getCurseForgeImageUrl(hit.icon_url),
		date_created: hit.date_created,
		date_modified: hit.date_modified,
		latest_version: hit.latest_version ?? '',
		license: '',
		client_side: 'unknown',
		server_side: 'unknown',
		gallery: hit.gallery,
		featured_gallery: hit.gallery[0] ?? null,
		color: null,
		website_url: hit.website_url,
		source_url: hit.source_url,
		allow_mod_distribution: hit.allow_mod_distribution,
	}
}

type ChineseSearchHit = Labrinth.Search.v2.ResultSearchProject & {
	provider: 'modrinth' | 'curseforge'
	provider_project_id?: string
	installed?: boolean
	chinese_search_score?: number
}

type McArchiveSearchHit = Labrinth.Search.v2.ResultSearchProject & {
	provider: 'mcarchive'
	provider_project_id: string
}

function mapMcArchiveHit(project: McArchiveMod): McArchiveSearchHit {
	const versions = Array.from(
		new Set(
			project.modVersions.flatMap((version) =>
				version.gameVersions.map((gameVersion) => gameVersion.name),
			),
		),
	)
	return {
		project_id: `mcarchive:${project.uuid}`,
		provider_project_id: project.uuid,
		provider: 'mcarchive',
		project_type: 'mod',
		project_types: ['mod'],
		slug: project.slug,
		author: '',
		title: project.name,
		description: project.summary ?? project.description ?? '',
		categories: [],
		display_categories: [],
		versions,
		downloads: 0,
		follows: 0,
		icon_url: '',
		date_created: '',
		date_modified: '',
		latest_version: '',
		license: '',
		client_side: 'unknown',
		server_side: 'unknown',
		gallery: [],
		featured_gallery: null,
		color: null,
	} as McArchiveSearchHit
}

function mapPlanetMinecraftHit(project: PlanetMinecraftProject) {
	return {
		project_id: `planet_minecraft:${project.id}`,
		provider_project_id: project.id,
		provider: 'planet_minecraft' as const,
		project_type: 'mod',
		slug: project.id,
		author: '',
		title: project.title,
		description: project.summary ?? '',
		categories: [],
		display_categories: [],
		versions: Array.from(new Set(project.versions.flatMap((version) => version.gameVersions))),
		downloads: 0,
		follows: 0,
		icon_url: '',
		date_created: '',
		date_modified: '',
		latest_version: '',
		license: '',
		client_side: 'unknown',
		server_side: 'unknown',
		gallery: [],
		featured_gallery: null,
		color: null,
	} as Labrinth.Search.v2.ResultSearchProject & {
		provider: 'planet_minecraft'
		provider_project_id: string
	}
}

interface DirectModrinthProject {
	id: string
	slug?: string
	project_types?: string[]
	name: string
	summary: string
	published?: string
	updated?: string
	downloads?: number
	followers?: number
	categories?: string[]
	additional_categories?: string[]
	loaders?: string[]
	game_versions?: string[]
	icon_url?: string
	color?: number
	gallery?: Array<{ url?: string; raw_url?: string; featured?: boolean }>
}

function replaceSearchQuery(requestParams: string, query: string) {
	const params = new URLSearchParams(
		requestParams.startsWith('?') ? requestParams.slice(1) : requestParams,
	)
	params.set('query', query)
	const result = params.toString()
	return result ? `?${result}` : ''
}

/**
 * Upper bound on sequential fallback search attempts when the primary query
 * yields no hits. Each attempt queries the providers in parallel.
 */
const MAX_SEARCH_VARIANT_ATTEMPTS = 3

/**
 * A dictionary-split replacement result must be clearly better than the
 * primary compact-query match before it may replace the primary hits.
 */
const MIN_VARIANT_REPLACEMENT_HITS = 5

interface FuzzySearchNotice {
	/** The query exactly as the user typed it. */
	original: string
	/** The rewritten query that actually produced the shown results. */
	used: string
}

let pendingFuzzyNotice: FuzzySearchNotice | null = null
const searchNotice = ref<FuzzySearchNotice | null>(null)

function findChineseTranslation(
	resolution: ChineseSearchResolution | null,
	provider: 'modrinth' | 'curseforge',
	slug?: string | null,
): ChineseSearchTranslation | undefined {
	if (!resolution || !slug) return undefined
	const normalizedSlug = slug.toLocaleLowerCase()
	return resolution.translations.find((translation) => {
		const candidate =
			provider === 'modrinth' ? translation.modrinthSlug : translation.curseforgeSlug
		return candidate?.toLocaleLowerCase() === normalizedSlug
	})
}

function applyChineseTranslation(
	hit: ChineseSearchHit,
	resolution: ChineseSearchResolution | null,
): ChineseSearchHit {
	if (hit.provider !== 'modrinth' && hit.provider !== 'curseforge') return hit
	const provider = hit.provider
	const translation = findChineseTranslation(resolution, provider, hit.slug)
	if (!translation) return hit
	return {
		...hit,
		title: bilingualTitle(translation.chineseName, hit.title),
		chinese_search_score: (translation.exact ? 10 : 0) + translation.matchScore,
	}
}

function matchesDirectModrinthFilters(
	project: DirectModrinthProject,
	gameVersion: string | undefined,
	loader: string | undefined,
	categoryValues: string[],
) {
	if (!project.project_types?.includes(projectType.value)) return false
	if (gameVersion && !project.game_versions?.includes(gameVersion)) return false
	if (loader && !project.loaders?.includes(loader)) return false
	const modrinthCategories = categoryValues.filter(
		(value) => !value.startsWith('cf:') && curseForgeLoaderTypes[value] === undefined,
	)
	if (
		modrinthCategories.length > 0 &&
		!modrinthCategories.some(
			(category) =>
				project.categories?.includes(category) || project.additional_categories?.includes(category),
		)
	) {
		return false
	}
	return true
}

function mapDirectModrinthProject(project: DirectModrinthProject): ChineseSearchHit {
	const gallery = project.gallery?.flatMap((item) => (item.url ? [item.url] : [])) ?? []
	return {
		project_id: project.id,
		project_type: project.project_types?.[0] ?? projectType.value,
		slug: project.slug,
		author: '',
		title: project.name,
		description: project.summary,
		categories: [...(project.categories ?? []), ...(project.additional_categories ?? [])],
		display_categories: project.categories ?? [],
		versions: project.game_versions ?? [],
		downloads: project.downloads ?? 0,
		follows: project.followers ?? 0,
		icon_url: project.icon_url,
		date_created: project.published ?? '',
		date_modified: project.updated ?? '',
		latest_version: '',
		license: '',
		client_side: 'unknown',
		server_side: 'unknown',
		gallery,
		featured_gallery: project.gallery?.find((item) => item.featured)?.url ?? gallery[0] ?? null,
		color: project.color ?? null,
		provider: 'modrinth',
	} as ChineseSearchHit
}

function dedupeProviderHits(hits: ChineseSearchHit[]) {
	const seen = new Set<string>()
	return hits.filter((hit) => {
		const key = `${hit.provider}:${hit.project_id}`
		if (seen.has(key)) return false
		seen.add(key)
		return true
	})
}

function rankChineseProviderHits(hits: ChineseSearchHit[], sort: string | null) {
	const metric = (hit: ChineseSearchHit) => {
		switch (sort) {
			case 'downloads':
				return hit.downloads ?? 0
			case 'follows':
				return hit.follows ?? 0
			case 'newest':
				return Date.parse(hit.date_created ?? '') || 0
			case 'updated':
				return Date.parse(hit.date_modified ?? '') || 0
			default:
				return null
		}
	}
	if (sort && sort !== 'relevance') {
		return [...hits].sort((left, right) => (metric(right) ?? 0) - (metric(left) ?? 0))
	}
	if (!hits.some((hit) => hit.chinese_search_score)) return hits
	return hits
		.map((hit, index) => ({ hit, index }))
		.sort((left, right) => {
			const score = (right.hit.chinese_search_score ?? 0) - (left.hit.chinese_search_score ?? 0)
			if (score !== 0) return score
			const downloads = (right.hit.downloads ?? 0) - (left.hit.downloads ?? 0)
			if (downloads !== 0) return downloads
			return left.index - right.index
		})
		.map(({ hit }) => hit)
}

async function search(requestParams: string, signal: AbortSignal) {
	debugLog('searching v3', requestParams)
	pendingFuzzyNotice = null
	const isServer = projectType.value === 'server'
	if (isWorldMapBrowse.value && !curseForgeCapability.value.configured) {
		return {
			projectHits: [],
			serverHits: [],
			total_hits: 0,
			per_page: 20,
		}
	}
	const params = new URLSearchParams(requestParams)
	const limit = Math.min(Number(params.get('limit') ?? 20), 50)
	const offset = Number(params.get('offset') ?? 0)
	const rawQuery = params.get('query') ?? ''
	const filters = params.get('new_filters') ?? ''
	const gameVersion = getFirstSearchFilter(filters, 'game_versions')
	const isMcArchiveBrowse =
		mcArchiveAvailable.value &&
		(contentSource.value === 'mcarchive' || route.query.source === 'mcarchive')
	if (isMcArchiveBrowse) {
		if (projectType.value !== 'mod') {
			return {
				projectHits: [],
				serverHits: [],
				total_hits: 0,
				per_page: limit,
			}
		}
		const projects = await searchMcArchiveMods(normalizeSearchText(rawQuery), gameVersion).catch(
			(error) => {
				debugLog('mcarchive search failed', error)
				throw error
			},
		)
		signal.throwIfAborted()
		// The MCArchive list endpoint deliberately returns project summaries. It does
		// not include modVersions, so applying the selected Minecraft version here
		// would discard every result. Version compatibility is resolved after opening
		// the project detail, where the full release list is available.
		const hits = projects.map(mapMcArchiveHit)
		const safeOffset = offset < hits.length ? offset : 0
		return {
			projectHits: hits.slice(safeOffset, safeOffset + limit),
			serverHits: [],
			total_hits: hits.length,
			per_page: limit,
		}
	}
	if (contentSource.value === 'planet_minecraft') {
		if (projectType.value !== 'mod' || !planetMinecraftAvailable.value) {
			return { projectHits: [], serverHits: [], total_hits: 0, per_page: limit }
		}
		const projects = await searchPlanetMinecraftProjects(
			normalizeSearchText(rawQuery),
			gameVersion,
		).catch((error) => {
			debugLog('planet minecraft search failed', error)
			throw error
		})
		signal.throwIfAborted()
		const hits = projects.map(mapPlanetMinecraftHit)
		return {
			projectHits: hits.slice(offset, offset + limit),
			serverHits: [],
			total_hits: hits.length,
			per_page: limit,
		}
	}
	let chineseResolution: ChineseSearchResolution | null = null
	if (!isWorldMapBrowse.value && containsChineseSearchText(rawQuery)) {
		chineseResolution = await resolveChineseContentSearch(rawQuery).catch((error) => {
			debugLog('chinese search resolution failed, using original query', error)
			return null
		})
	}
	const categoryValues = getSearchFilterValues(filters, 'categories')
	const hasOnlyCurseForgeExclusiveCategories =
		categoryValues.length > 0 &&
		categoryValues.every(
			(value) => isCurseForgeOnlyCategoryName(value) || curseForgeLoaderTypes[value] !== undefined,
		) &&
		categoryValues.some((value) => isCurseForgeOnlyCategoryName(value))

	const includeModrinth =
		!isWorldMapBrowse.value &&
		(contentSource.value !== 'curseforge' || isServer) &&
		!(contentSource.value === 'all' && hasOnlyCurseForgeExclusiveCategories)
	let includeCurseForge =
		!isServer &&
		contentSource.value !== 'modrinth' &&
		curseForgeCapability.value.configured &&
		curseForgeClassIds[projectType.value] !== undefined

	if (includeCurseForge) {
		await ensureCurseForgeCategories(projectType.value).catch(handleError)
	}

	const loader = categoryValues.find((value) => curseForgeLoaderTypes[value] !== undefined)
	const nonLoaderCategoryValues = categoryValues.filter(
		(value) => curseForgeLoaderTypes[value] === undefined,
	)
	const curseForgeCategoryIds = includeCurseForge ? getCurseForgeCategoryIds(filters) : []

	// In unified browse, never mix unfiltered CurseForge hits with filtered Modrinth hits.
	// If the user picked categories that cannot be mapped to CF, only query Modrinth.
	if (
		contentSource.value === 'all' &&
		includeCurseForge &&
		nonLoaderCategoryValues.length > 0 &&
		curseForgeCategoryIds.length === 0 &&
		!hasOnlyCurseForgeExclusiveCategories
	) {
		includeCurseForge = false
		debugLog('skipping unfiltered curseforge results for unmapped categories', {
			categoryValues: nonLoaderCategoryValues,
		})
	}
	signal.throwIfAborted()

	let modrinthRequestParams =
		includeModrinth && (includeCurseForge || hasOnlyCurseForgeExclusiveCategories)
			? stripCurseForgeOnlyCategoryFilters(requestParams)
			: requestParams
	if (chineseResolution?.modrinthQuery) {
		modrinthRequestParams = replaceSearchQuery(
			modrinthRequestParams,
			chineseResolution.modrinthQuery,
		)
	}
	const providerRequestGroupId = crypto.randomUUID()
	const modrinthRequestId = `${providerRequestGroupId}:modrinth`
	const curseForgeRequestId = `${providerRequestGroupId}:curseforge`
	const activeProviderRequestIds = new Set<string>()
	function trackProviderRequest<T>(requestId: string, request: Promise<T>) {
		activeProviderRequestIds.add(requestId)
		return request.finally(() => activeProviderRequestIds.delete(requestId))
	}
	let rejectProviderSearch: (reason?: unknown) => void = () => {}
	const providerSearchCancelled = new Promise<never>((_, reject) => {
		rejectProviderSearch = reject
	})
	const cancelProviderRequests = () => {
		for (const requestId of activeProviderRequestIds) {
			cancel_search_request(requestId).catch((error) => {
				debugLog('failed to cancel provider search', { requestId, error })
			})
		}
		rejectProviderSearch(signal.reason)
	}
	signal.addEventListener('abort', cancelProviderRequests, { once: true })

	interface ProviderAttemptStep {
		modrinthQuery: string | null
		curseforgeFilter: string | null
	}

	const queryExpansion = expandSearchQuery(rawQuery)
	const isChineseQuery = chineseResolution?.isChinese ?? false
	const curseForgeQueryBase = chineseResolution?.curseforgeQuery ?? rawQuery
	const curseForgeQueryVariantsList = queryExpansion
		? curseForgeQueryVariants(curseForgeQueryBase)
		: []
	// An absent query must keep CurseForge browsable unfiltered: `null` means
	// "skip CurseForge in this attempt" (used by fallback steps), so an empty
	// query maps to an empty filter, which the request layer turns into no
	// searchFilter.
	const primaryCurseForgeFilter = queryExpansion ? (curseForgeQueryVariantsList[0] ?? null) : ''

	const runProviderAttempt = async (
		attemptParams: {
			modrinthParams: string | null
			curseforgeFilter: string | null
		},
		includeDirectModrinth: boolean,
	) => {
		const modrinthRequest =
			includeModrinth && attemptParams.modrinthParams !== null
				? queryClient.fetchQuery({
						queryKey: ['search', 'v3', attemptParams.modrinthParams],
						queryFn: () =>
							trackProviderRequest(
								modrinthRequestId,
								get_search_results_v3(
									attemptParams.modrinthParams,
									'must_revalidate',
									modrinthRequestId,
								) as Promise<{
									result: Labrinth.Search.v3.SearchResults & {
										hits: (Labrinth.Search.v3.ResultSearchProject & {
											installed?: boolean
										})[]
									}
								} | null>,
							),
						staleTime: 30_000,
					})
				: Promise.resolve(null)
		const curseForgeRequest =
			includeCurseForge && attemptParams.curseforgeFilter !== null
				? trackProviderRequest(
						curseForgeRequestId,
						searchCurseForgeProjects(
							{
								classId: curseForgeClassIds[projectType.value]!,
								categoryIds: curseForgeCategoryIds,
								searchFilter: attemptParams.curseforgeFilter || undefined,
								gameVersion: gameVersion || undefined,
								modLoaderType: loader ? curseForgeLoaderTypes[loader] : undefined,
								sortField: getCurseForgeSortField(params.get('index')),
								sortOrder: 'desc',
								index: offset,
								pageSize: limit,
							},
							curseForgeRequestId,
						),
					)
				: Promise.resolve(null)
		const directModrinthRequest =
			includeModrinth &&
			includeDirectModrinth &&
			!isServer &&
			offset === 0 &&
			(chineseResolution?.modrinthSlugs.length ?? 0) > 0
				? get_project_v3_many(chineseResolution!.modrinthSlugs, 'must_revalidate')
				: Promise.resolve([])
		const [modrinthResult, curseForgeResult, directModrinthResult] = await Promise.allSettled([
			modrinthRequest,
			curseForgeRequest,
			directModrinthRequest,
		])
		return {
			rawResults: modrinthResult.status === 'fulfilled' ? modrinthResult.value : null,
			rawCurseForge: curseForgeResult.status === 'fulfilled' ? curseForgeResult.value : null,
			rawDirectModrinth:
				directModrinthResult.status === 'fulfilled'
					? (directModrinthResult.value as DirectModrinthProject[])
					: [],
			modrinthError: modrinthResult.status === 'rejected' ? modrinthResult.reason : null,
			curseForgeError: curseForgeResult.status === 'rejected' ? curseForgeResult.reason : null,
			directModrinthError:
				directModrinthResult.status === 'rejected' ? directModrinthResult.reason : null,
		}
	}
	type ProviderAttemptResults = Awaited<ReturnType<typeof runProviderAttempt>>

	const attemptHasHits = (results: ProviderAttemptResults) =>
		(results.rawResults?.result.total_hits ?? 0) > 0 ||
		(results.rawCurseForge?.total_hits ?? 0) > 0 ||
		results.rawDirectModrinth.length > 0

	let cachedCompactSplit: string | null | undefined
	const resolveCompactSplit = async () => {
		if (cachedCompactSplit === undefined) {
			cachedCompactSplit = await expandContentSearchQuery(rawQuery)
				.then((expansion) => expansion.suggestedSplit ?? null)
				.catch((error) => {
					debugLog('search query expansion failed', error)
					return null
				})
		}
		return cachedCompactSplit
	}

	const buildFallbackSteps = async (): Promise<ProviderAttemptStep[]> => {
		const steps: ProviderAttemptStep[] = []
		if (!queryExpansion) return steps
		const mrVariants = isChineseQuery ? [] : queryExpansion.modrinthVariants.slice(1)
		const cfVariants = queryExpansion.curseforgeVariants.slice(1)
		if (queryExpansion.compact && !isChineseQuery && includeModrinth) {
			const dictSplit = await resolveCompactSplit()
			if (dictSplit && !queryExpansion.modrinthVariants.includes(dictSplit)) {
				steps.push({
					modrinthQuery: dictSplit,
					curseforgeFilter: curseForgeQueryVariants(dictSplit)[0] ?? null,
				})
			}
		}
		const maxVariants = Math.max(mrVariants.length, cfVariants.length)
		for (
			let index = 0;
			index < maxVariants && steps.length < MAX_SEARCH_VARIANT_ATTEMPTS;
			index += 1
		) {
			const modrinthQuery = mrVariants[index] ?? null
			const curseforgeFilter = cfVariants[index] ?? null
			if (modrinthQuery !== null || curseforgeFilter !== null) {
				steps.push({ modrinthQuery, curseforgeFilter })
			}
		}
		return steps
	}

	const shouldImproveCompactMatch = (results: ProviderAttemptResults) => {
		if (!queryExpansion || !queryExpansion.compact || isChineseQuery) return false
		if (!includeModrinth || (results.rawResults?.result.hits.length ?? 0) === 0) return false
		const total = results.rawResults?.result.total_hits ?? 0
		return total >= 1 && total <= 2
	}

	const performSearch = async (): Promise<ProviderAttemptResults> => {
		if (includeCurseForge) {
			debugLog('curseforge filters', {
				filters,
				categoryValues,
				categoryIds: curseForgeCategoryIds,
				gameVersion,
				loader,
				searchFilter: primaryCurseForgeFilter,
			})
		}
		const attemptParams = (step: ProviderAttemptStep) => ({
			modrinthParams:
				step.modrinthQuery === null
					? null
					: replaceSearchQuery(modrinthRequestParams, step.modrinthQuery),
			curseforgeFilter: step.curseforgeFilter,
		})
		const primaryResults = await runProviderAttempt(
			{
				modrinthParams: includeModrinth ? modrinthRequestParams : null,
				curseforgeFilter: primaryCurseForgeFilter,
			},
			true,
		)
		let results = primaryResults
		if (!attemptHasHits(primaryResults)) {
			const fallbackAttempts = await buildFallbackSteps()
			for (const step of fallbackAttempts) {
				signal.throwIfAborted()
				results = await runProviderAttempt(attemptParams(step), false)
				if (attemptHasHits(results)) {
					debugLog('search fallback matched query variant', step)
					pendingFuzzyNotice = {
						original: rawQuery,
						used:
							step.modrinthQuery ??
							(step.curseforgeFilter ? step.curseforgeFilter.replaceAll('-', ' ') : ''),
					}
					break
				}
			}
		} else if (shouldImproveCompactMatch(primaryResults)) {
			// Server fuzzy matching can return a single poor hit for compact
			// queries (e.g. `irisshaders`); when the dictionary splits the query
			// into a clearly better result set, prefer the split form.
			const swapQuery = await resolveCompactSplit()
			if (swapQuery && !queryExpansion!.modrinthVariants.includes(swapQuery)) {
				const swapped = await runProviderAttempt(
					attemptParams({ modrinthQuery: swapQuery, curseforgeFilter: null }),
					false,
				)
				if ((swapped.rawResults?.result.total_hits ?? 0) >= MIN_VARIANT_REPLACEMENT_HITS) {
					debugLog('search compact query replaced with split form', swapQuery)
					pendingFuzzyNotice = { original: rawQuery, used: swapQuery }
					results = {
						...swapped,
						rawCurseForge: primaryResults.rawCurseForge,
						rawDirectModrinth: primaryResults.rawDirectModrinth,
					}
				}
			}
		}
		return results
	}

	const searched = await Promise.race([performSearch(), providerSearchCancelled]).finally(() =>
		signal.removeEventListener('abort', cancelProviderRequests),
	)
	const rawResults = searched.rawResults
	const rawCurseForge = searched.rawCurseForge
	const rawDirectModrinth = searched.rawDirectModrinth

	if (searched.modrinthError) {
		debugLog('modrinth search failed', searched.modrinthError)
	}
	if (searched.curseForgeError) {
		debugLog('curseforge search failed', searched.curseForgeError)
	}
	if (searched.directModrinthError) {
		debugLog('direct modrinth chinese candidates failed', searched.directModrinthError)
	}

	if (!rawResults && !rawCurseForge && rawDirectModrinth.length === 0) {
		const error =
			searched.modrinthError ??
			searched.curseForgeError ??
			new Error('No content providers are available')
		throw error
	}

	if (isServer) {
		if (!rawResults) throw new Error('The server project provider is unavailable')
		const hits = rawResults.result.hits ?? []
		updateServerHits(hits)
		return {
			projectHits: [],
			serverHits: hits,
			total_hits: rawResults.result.total_hits ?? 0,
			per_page: rawResults.result.hits_per_page,
		}
	}

	const hits = (rawResults?.result.hits ?? []).map((hit) => {
		const mapped = {
			...hit,
			title: hit.name,
			description: hit.summary,
			provider: 'modrinth' as const,
		} as unknown as Labrinth.Search.v2.ResultSearchProject & {
			installed?: boolean
			provider: 'modrinth' | 'curseforge' | 'mcarchive' | 'planet_minecraft'
		}

		if (activeInstance.value || isServerContext.value) {
			const installedIds = activeInstance.value
				? allInstalledIds.value
				: serverContentProjectIds.value
			mapped.installed = installedIds.has(hit.project_id)
		}

		return applyChineseTranslation(mapped, chineseResolution)
	})

	const directModrinthHits = rawDirectModrinth
		.filter((project) => matchesDirectModrinthFilters(project, gameVersion, loader, categoryValues))
		.slice(0, limit)
		.map(mapDirectModrinthProject)
		.map((hit) => applyChineseTranslation(hit, chineseResolution))
		.map((hit) => {
			if (activeInstance.value || isServerContext.value) {
				const installedIds = activeInstance.value
					? allInstalledIds.value
					: serverContentProjectIds.value
				hit.installed = installedIds.has(hit.project_id)
			}
			return hit
		})
	const modrinthHits = rankChineseProviderHits(
		dedupeProviderHits([...directModrinthHits, ...hits]),
		params.get('index'),
	)
	const directModrinthHitIds = new Set(directModrinthHits.map((hit) => hit.project_id))
	const searchedModrinthHitIds = new Set(hits.map((hit) => hit.project_id))
	const injectedModrinthCount = [...directModrinthHitIds].filter(
		(id) => !searchedModrinthHitIds.has(id),
	).length
	const curseForgeHits = (rawCurseForge?.hits ?? [])
		.map(mapCurseForgeHit)
		.map((hit) => {
			if (activeInstance.value) hit.installed = allInstalledIds.value.has(hit.project_id)
			return hit
		})
		.map((hit) => applyChineseTranslation(hit as ChineseSearchHit, chineseResolution))
	const locale = i18n.global.locale.value
	const [localizedModrinthHits, localizedCurseForgeHits] = await Promise.all([
		translateSearchHitTitles(modrinthHits, locale),
		translateSearchHitTitles(curseForgeHits, locale),
	])
	return {
		projectHits:
			contentSource.value === 'all'
				? mergeProviderResults({
						modrinthHits: localizedModrinthHits,
						curseForgeHits: localizedCurseForgeHits,
						sort: params.get('index'),
						query: params.get('query'),
						limit,
					})
				: contentSource.value === 'curseforge'
					? localizedCurseForgeHits
					: localizedModrinthHits.slice(0, limit),
		serverHits: [],
		total_hits:
			contentSource.value === 'all'
				? Math.max(
						(rawResults?.result.total_hits ?? 0) + injectedModrinthCount,
						rawCurseForge?.total_hits ?? 0,
					)
				: contentSource.value === 'curseforge'
					? (rawCurseForge?.total_hits ?? 0)
					: (rawResults?.result.total_hits ?? 0) + injectedModrinthCount,
		per_page: limit,
	}
}

const isServerFilterContext = computed(() => isServerContext.value || isServerInstance.value)

const lockedFilterMessages = computed(() => ({
	gameVersion: formatMessage(
		isServerFilterContext.value
			? messages.gameVersionProvidedByServer
			: messages.gameVersionProvidedByInstance,
	),
	modLoader: formatMessage(
		isServerFilterContext.value
			? messages.modLoaderProvidedByServer
			: messages.modLoaderProvidedByInstance,
	),
	environment: formatMessage(messages.environmentProvidedByServer),
	syncButton: formatMessage(messages.syncFilterButton),
	providedBy: formatMessage(
		isServerFilterContext.value ? messages.providedByServer : messages.providedByInstance,
	),
}))

const browseReturnSnapshot = consumeBrowseReturnSnapshot(route.fullPath)

const displayMode = ref<BrowseDisplayMode>(getLastBrowseContentDisplayMode())

const displayModeOptions = computed<BrowseDisplayModeOption[]>(() => [
	{ id: 'list', label: formatMessage(messages.listView), icon: ListIcon },
	{ id: 'compact', label: formatMessage(messages.compactListView), icon: GenericListIcon },
	{ id: 'grid', label: formatMessage(messages.gridView), icon: GridIcon },
])

const displayModeTooltip = computed(() => formatMessage(messages.switchView))

function setDisplayMode(mode: BrowseDisplayMode) {
	if (mode === 'gallery') return
	displayMode.value = mode
	setLastBrowseContentDisplayMode(mode)
}

const searchState = useBrowseSearch({
	projectType,
	tags,
	providedFilters: combinedProvidedFilters,
	installContextLoader: computed(() => installContext.value?.loader),
	search: async (requestParams: string, signal: AbortSignal) => {
		const response = await search(requestParams, signal)
		searchNotice.value = pendingFuzzyNotice
		return response
	},
	persistentQueryParams: ['i', 'ai', 'shi', 'sid', 'wid', 'from', 'source'],
	getExtraQueryParams: () => ({
		sid: serverIdQuery.value || undefined,
		wid: effectiveServerWorldId.value || undefined,
		ai: instanceHideInstalled.value ? 'true' : undefined,
		shi: serverHideInstalled.value ? 'true' : undefined,
		source:
			isWorldMapBrowse.value || contentSource.value === 'all' ? undefined : contentSource.value,
	}),
	displayMode,
})

const NON_FILTER_BROWSE_QUERY_PARAMS = new Set([
	'i',
	'ai',
	'shi',
	'sid',
	'wid',
	'from',
	'source',
	'q',
	's',
	'ss',
	'm',
	'o',
	'page',
	'b',
])

function hasExplicitFilterQuery() {
	return Object.keys(route.query).some((key) => !NON_FILTER_BROWSE_QUERY_PARAMS.has(key))
}

function availableRememberedFilters(type: ProjectType, filters: BrowseFilterMemory['filters']) {
	const filterTypes =
		type === 'server' ? searchState.serverFilterTypes.value : searchState.filters.value
	return filters.filter((filter) => {
		const filterType = filterTypes.find((candidate) => candidate.id === filter.type)
		return (
			filterType &&
			(filterType.allows_custom_options ||
				filterType.options.some((option) => option.id === filter.option))
		)
	})
}

function applyRememberedFilters(type: ProjectType, resetWhenMissing: boolean) {
	const memory = getBrowseFilterMemory(type)
	if (!memory && !resetWhenMissing) return

	if (type === 'server') {
		searchState.serverCurrentFilters.value = memory
			? availableRememberedFilters(type, memory.filters)
			: [{ type: 'server_status', option: 'online' }]
		searchState.serverToggledGroups.value = memory ? [...memory.toggledGroups] : []
		return
	}

	const filterTypeIds = new Set(searchState.filters.value.map((filter) => filter.id))
	searchState.currentFilters.value = memory ? availableRememberedFilters(type, memory.filters) : []
	searchState.toggledGroups.value = memory ? [...memory.toggledGroups] : []
	searchState.overriddenProvidedFilterTypes.value = memory
		? memory.overriddenProvidedFilterTypes.filter((filterType) => filterTypeIds.has(filterType))
		: []
}

if (!hasExplicitFilterQuery()) {
	applyRememberedFilters(projectType.value, false)
}

watch(projectType, (type) => applyRememberedFilters(type, true))

watch(
	[
		projectType,
		searchState.currentFilters,
		searchState.toggledGroups,
		searchState.overriddenProvidedFilterTypes,
		searchState.serverCurrentFilters,
		searchState.serverToggledGroups,
	],
	() => {
		const isServer = projectType.value === 'server'
		setBrowseFilterMemory(projectType.value, {
			filters: isServer ? searchState.serverCurrentFilters.value : searchState.currentFilters.value,
			toggledGroups: isServer
				? searchState.serverToggledGroups.value
				: searchState.toggledGroups.value,
			overriddenProvidedFilterTypes: isServer
				? []
				: searchState.overriddenProvidedFilterTypes.value,
		})
	},
	{ deep: true },
)

/** Translation state for search result titles and descriptions. */
const originalProjectHits = shallowRef<typeof searchState.projectHits.value>([])
const originalServerHits = shallowRef<typeof searchState.serverHits.value>([])
let isUpdatingProjectHitsFromTranslation = false
const {
	translationActive,
	translationLoading,
	start: startTranslation,
	isStale,
	done: doneTranslation,
	toggle,
	cancel: cancelTranslation,
} = useTranslationToggle()

// Keep a pristine copy when genuine search results arrive (project hits).
watch(
	() => searchState.projectHits.value,
	(hits) => {
		if (isUpdatingProjectHitsFromTranslation) return
		if (hits && hits.length > 0) {
			originalProjectHits.value = hits
			const version = cancelTranslation()
			void autoTranslateNewSearchResults(version, false)
		}
	},
	{ flush: 'sync' },
)
// Keep a pristine copy when genuine search results arrive (server hits).
watch(
	() => searchState.serverHits.value,
	(hits) => {
		if (isUpdatingProjectHitsFromTranslation) return
		if (hits && hits.length > 0) {
			originalServerHits.value = hits
			const version = cancelTranslation()
			// Always restart auto-translate since this bumps the version and
			// cancels the one started by the projectHits watcher.
			const useServer = originalServerHits.value.length > 0
			void autoTranslateNewSearchResults(version, useServer)
		}
	},
	{ flush: 'sync' },
)

async function autoTranslateNewSearchResults(version: number, useServer: boolean) {
	try {
		const hits = useServer ? originalServerHits.value : originalProjectHits.value
		if (!hits?.length) return

		const translated = await translateSearchDescriptions(
			hits,
			i18n.global.locale.value,
			false,
			useServer,
		)

		if (isStale(version)) return

		if (translated !== hits) {
			isUpdatingProjectHitsFromTranslation = true
			if (useServer) searchState.serverHits.value = translated
			else searchState.projectHits.value = translated
			translationActive.value = true
			isUpdatingProjectHitsFromTranslation = false
		}
	} catch (error) {
		debugLog('Automatic search result translation failed', error)
	}
}

async function translateCurrentHits() {
	const version = startTranslation()
	try {
		const serverHits = originalServerHits.value
		const projectHits = originalProjectHits.value
		const useServer = serverHits.length > 0
		const hits = useServer ? serverHits : projectHits
		if (!hits || hits.length === 0) return

		const translated = await translateSearchDescriptions(
			hits,
			i18n.global.locale.value,
			true,
			useServer,
		)
		if (isStale(version)) return // superseded

		if (translated !== hits) {
			isUpdatingProjectHitsFromTranslation = true
			if (useServer) searchState.serverHits.value = translated
			else searchState.projectHits.value = translated
			translationActive.value = true
		}
		isUpdatingProjectHitsFromTranslation = false
	} finally {
		doneTranslation(version)
	}
}

function toggleTranslation() {
	toggle(
		() => {
			isUpdatingProjectHitsFromTranslation = true
			searchState.projectHits.value = originalProjectHits.value
			searchState.serverHits.value = originalServerHits.value
			isUpdatingProjectHitsFromTranslation = false
		},
		() => void translateCurrentHits(),
	)
}

watch(contentSource, async (source) => {
	searchState.projectHits.value = []
	searchState.totalHits.value = 0
	searchState.currentPage.value = 1
	searchState.loading.value = true
	searchState.currentFilters.value = searchState.currentFilters.value.filter(
		(filter) => !filter.type.startsWith('category_'),
	)
	if (source === 'curseforge' || source === 'all') {
		await ensureCurseForgeCategories(projectType.value).catch(handleError)
	}
	if (source === 'mcarchive') await refreshMcArchiveGameVersions()
	await searchState.refreshSearch()
})

watch(projectType, async (type, previousType) => {
	if (type === WORLD_BROWSE_PROJECT_TYPE) {
		sourceBeforeWorldMapBrowse.value = contentSource.value
		contentSource.value = 'curseforge'
		return
	}
	if (previousType === WORLD_BROWSE_PROJECT_TYPE) {
		contentSource.value = sourceBeforeWorldMapBrowse.value
	}
	if (
		type !== 'mod' &&
		(contentSource.value === 'mcarchive' || contentSource.value === 'planet_minecraft')
	) {
		contentSource.value = 'all'
		setLastBrowseContentSource('all')
	}
	if (contentSource.value === 'curseforge' || contentSource.value === 'all') {
		await ensureCurseForgeCategories(type).catch(handleError)
	}
})

watch(
	mcArchiveAvailable,
	(available) => {
		if (!available && contentSource.value === 'mcarchive') {
			contentSource.value = 'all'
			setLastBrowseContentSource('all')
		}
	},
	{ immediate: !route.query.i },
)

function selectContentSource(source: string) {
	if (isWorldMapBrowse.value) return
	if (
		source === 'all' ||
		source === 'modrinth' ||
		source === 'curseforge' ||
		(source === 'mcarchive' && projectType.value === 'mod' && mcArchiveAvailable.value) ||
		(source === 'planet_minecraft' && projectType.value === 'mod' && planetMinecraftAvailable.value)
	) {
		contentSource.value = source
		setLastBrowseContentSource(source)
	}
}

watch(
	[
		() => searchState.query.value,
		() => searchState.currentFilters.value,
		() => searchState.serverCurrentFilters.value,
		() => projectType.value,
	],
	() => {
		if (isServerContext.value) {
			syncHiddenServerContentProjectIds()
		} else if (activeInstance.value) {
			syncHiddenInstanceProjectIds()
		}
	},
	{ deep: true },
)

watch(queuedServerInstallCount, (count) => {
	if (count === 0) {
		hideSelectedServerInstalls.value = false
	}
})

type UnlistenFn = () => void

let isUnmounted = false
let unlistenInstances: UnlistenFn | null = null
let pendingBrowseReturnScroll = browseReturnSnapshot !== null

async function restoreBrowseReturnScroll() {
	if (!browseReturnSnapshot) return

	await nextTick()
	await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))
	if (isUnmounted) return
	document.querySelector<HTMLElement>('.app-viewport')?.scrollTo({
		top: browseReturnSnapshot.scrollTop,
	})
	completeBrowseReturnNavigation(route.fullPath)
}

watch(
	searchState.loading,
	(isLoading) => {
		if (isLoading || !pendingBrowseReturnScroll) return

		pendingBrowseReturnScroll = false
		void restoreBrowseReturnScroll()
	},
	{ flush: 'post' },
)

onMounted(() => {
	if (pendingRouteInstanceSwitch.value) {
		instanceSelector.value?.requestSwitch(pendingRouteInstanceSwitch.value)
		pendingRouteInstanceSwitch.value = null
	}
	const initialSearchDependencies = [initialInstanceFilterPromise]
	if (instanceHideInstalled.value) initialSearchDependencies.push(initialInstalledProjectsPromise)
	void Promise.allSettled(initialSearchDependencies).then(() => {
		if (!isUnmounted) void searchState.refreshSearch()
	})

	instance_listener(async (event: { event: string; instance_id: string }) => {
		if (
			activeInstance.value &&
			event.instance_id === activeInstance.value.id &&
			['synced', 'content_install_finished', 'content_install_failed'].includes(event.event)
		) {
			await refreshInstalledProjectIds()
			await searchState.refreshSearch()
		}
	})
		.then((unlisten) => {
			if (isUnmounted) {
				unlisten()
				return
			}

			unlistenInstances = unlisten
		})
		.catch(handleError)
})

onUnmounted(() => {
	isUnmounted = true
	unlistenInstances?.()
})

function getProjectBrowseQuery() {
	return {
		...route.query,
		b: route.fullPath,
	}
}

const advancedFiltersCollapsed = computed({
	get: () => themeStore.getFeatureFlag('advanced_filters_collapsed'),
	set: (value) => {
		themeStore.featureFlags['advanced_filters_collapsed'] = value
		getSettings()
			.then((settings) => {
				settings.feature_flags['advanced_filters_collapsed'] = value
				return setSettings(settings)
			})
			.catch(handleError)
	},
})

provideBrowseManager({
	tags,
	projectType,
	...searchState,
	advancedFiltersCollapsed,
	getProjectLink: (
		result: Labrinth.Search.v2.ResultSearchProject & {
			provider: 'modrinth' | 'curseforge' | 'mcarchive' | 'planet_minecraft'
			provider_project_id?: string
		},
	) => ({
		path:
			result.provider === 'curseforge'
				? `/project/curseforge/${result.provider_project_id}`
				: result.provider === 'mcarchive'
					? `/project/mcarchive/${result.slug}`
					: result.provider === 'planet_minecraft'
						? `/project/planet-minecraft/${result.provider_project_id}`
						: result.provider === 'modrinth'
							? `/project/${result.project_id ?? result.slug}`
							: route.path,
		query: getProjectBrowseQuery(),
	}),
	getServerProjectLink: (result: Labrinth.Search.v3.ResultSearchProject) => ({
		path: `/project/${result.slug ?? result.project_id}`,
		query: getProjectBrowseQuery(),
	}),
	selectableProjectTypes,
	showProjectTypeTabs: computed(() => !isServerContext.value),
	variant: 'app',
	getCardActions,
	installContext,
	providedFilters: combinedProvidedFilters,
	hideInstalled: computed({
		get: () => (isServerContext.value ? serverHideInstalled.value : instanceHideInstalled.value),
		set: (val: boolean) => {
			if (isServerContext.value) {
				serverHideInstalled.value = val
				if (val) syncHiddenServerContentProjectIds()
			} else {
				instanceHideInstalled.value = val
				if (val) syncHiddenInstanceProjectIds()
			}
		},
	}),
	showHideInstalled: computed(
		() =>
			!isWorldMapBrowse.value &&
			((isServerContext.value && projectType.value !== 'modpack') || !!activeInstance.value),
	),
	hideInstalledLabel: computed(() =>
		formatMessage(
			isFromWorlds.value ? messages.hideAddedServers : commonMessages.hideInstalledContentLabel,
		),
	),
	hideSelected: hideSelectedServerInstalls,
	showHideSelected: computed(
		() =>
			isServerContext.value &&
			projectType.value !== 'modpack' &&
			queuedServerInstallCount.value > 0,
	),
	hideSelectedLabel: computed(() => formatMessage(commonMessages.hideSelectedContentLabel)),
	onInstalled: onSearchResultInstalled,
	serverPings,
	getServerModpackContent,
	onContextMenu: (event, result) => {
		if ('provider' in result && result.provider !== 'modrinth') return
		if (!('provider' in result) && projectType.value !== 'server') return
		handleRightClick(event, result)
	},
	offline,
	lockedFilterMessages,
	displayMode,
	displayModeOptions,
	displayModeTooltip,
	setDisplayMode,
})
</script>

<template>
	<div data-onboarding-id="browse-content" class="flex flex-col gap-3 p-6">
		<BrowsePageLayout v-if="!isWorldMapBrowse || curseForgeCapability.configured">
			<template #nav-tabs-actions>
				<ButtonStyled size="large" type="transparent">
					<button :disabled="translationLoading" @click="toggleTranslation">
						<SpinnerIcon v-if="translationLoading" class="animate-spin" />
						<LanguagesIcon v-else />
						{{
							formatMessage(
								translationLoading
									? messages.translating
									: translationActive
										? messages.showOriginal
										: messages.translateProject,
							)
						}}
					</button>
				</ButtonStyled>
			</template>
			<template #search-bar-actions>
				<ButtonStyled
					v-if="!isServerContext && projectType !== 'server' && projectType !== 'modpack'"
					size="standard"
					type="standard"
				>
					<button class="flex min-w-0 items-center gap-2" @click="instanceSelector?.show()">
						<InstanceIcon
							v-if="activeInstance"
							class="shrink-0"
							size="1.25rem"
							:icon-path="activeInstance.icon_path"
							:instance-id="activeInstance.id"
							:loader="activeInstance.loader"
						/>
						<PlusIcon v-else class="size-5 shrink-0" />
						<span class="max-w-40 truncate font-medium">
							{{ activeInstance?.name ?? formatMessage(messages.chooseInstance) }}
						</span>
						<span
							aria-hidden="true"
							class="flex size-4 shrink-0 items-center justify-center text-secondary"
						>
							<ChevronDownIcon class="size-4" />
						</span>
					</button>
				</ButtonStyled>
				<PopoutMenu v-if="projectType !== 'server' && !isWorldMapBrowse" placement="bottom-end">
					<ButtonStyled size="standard" type="standard">
						<button class="flex items-center gap-2">
							<component :is="sourceIcon" class="h-5 w-5" />
							<span>{{ formatMessage(currentSourceLabel) }}</span>
						</button>
					</ButtonStyled>
					<template #menu>
						<div class="flex w-min flex-col gap-1 p-1">
							<ButtonStyled
								v-for="option in sourceOptions"
								:key="option.id"
								:type="contentSource === option.id ? 'filled' : 'transparent'"
							>
								<button
									class="flex w-full items-center gap-2 !justify-start text-left"
									@click="selectContentSource(option.id)"
								>
									<component :is="option.icon" class="h-4 w-4" />
									{{ formatMessage(option.label) }}
								</button>
							</ButtonStyled>
						</div>
					</template>
				</PopoutMenu>
			</template>
			<template #after>
				<ContextMenu ref="contextMenuRef" @option-clicked="handleOptionsClick">
					<template #open_link>
						<GlobeIcon /> {{ formatMessage(commonMessages.openInModrinthButton) }} <ExternalIcon />
					</template>
					<template #copy_link>
						<ClipboardCopyIcon /> {{ formatMessage(commonMessages.copyLinkButton) }}
					</template>
				</ContextMenu>
			</template>
			<template #above-results>
				<div
					v-if="searchNotice"
					class="flex items-center gap-2 px-1 pb-1 text-sm"
					aria-live="polite"
				>
					<SparklesIcon class="size-3.5 shrink-0 text-brand" />
					<p class="m-0 text-secondary">
						{{ formatMessage(messages.fuzzySearchPrefix, searchNotice) }}
						<button
							type="button"
							class="font-medium text-contrast underline-offset-2 hover:text-brand hover:underline"
							@click="searchState.query.value = searchNotice.used"
						>
							{{ searchNotice.used }}
						</button>
						{{ formatMessage(messages.fuzzySearchSuffix) }}
					</p>
				</div>
			</template>
		</BrowsePageLayout>
		<EmptyState
			v-else
			type="empty-inbox"
			:heading="formatMessage(messages.mapsUnavailable)"
			:description="formatMessage(messages.mapsUnavailableDescription)"
		/>
		<CreationFlowModal
			v-if="isServerContext && projectType === 'modpack'"
			ref="serverSetupModalRef"
			:type="serverFlowFrom === 'reset-server' ? 'reset-server' : 'server-onboarding'"
			:available-loaders="['vanilla', 'fabric', 'neoforge', 'forge', 'quilt', 'paper', 'purpur']"
			:show-snapshot-toggle="true"
			:on-back="onServerFlowBack"
			:search-modpacks="searchServerModpacks"
			:get-project-versions="getServerProjectVersions"
			:get-loader-manifest="getLoaderManifest"
			@hide="() => {}"
			@browse-modpacks="() => {}"
			@create="handleServerModpackFlowCreate"
		/>
		<BrowseInstanceSelector
			ref="instanceSelector"
			:instances="contentSelection.instances.value"
			:selected-instance="activeInstance"
			:selected-count="contentSelection.selectedCount.value"
			:install-current="contentSelection.installSelected"
			:clear-current="contentSelection.clear"
			@select="selectTargetInstance"
			@cancel-switch="cancelTargetInstanceSwitch"
		/>
		<Teleport to="#sidebar-teleport-target">
			<BrowseSidebar />
		</Teleport>
	</div>
</template>
