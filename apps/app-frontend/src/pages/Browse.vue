<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	CheckIcon,
	ClipboardCopyIcon,
	ExternalIcon,
	GlobeIcon,
	PlusIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import type { BrowseInstallContentType, CardAction, ProjectType, Tags } from '@modrinth/ui'
import {
	BrowsePageLayout,
	BrowseSidebar,
	ButtonStyled,
	commonMessages,
	CreationFlowModal,
	defineMessages,
	getLatestMatchingInstallVersion,
	getSelectedInstallPreferences,
	getTargetInstallPreferences,
	injectNotificationManager,
	preferencesDiffer,
	provideBrowseManager,
	requestInstall,
	stripServerRuntimeInstallFilters,
	stripServerRuntimeInstallOverrides,
	useBrowseSearch,
	useDebugLogger,
	useVIntl,
} from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { Ref } from 'vue'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import type { LocationQuery } from 'vue-router'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import { useAppServerBrowse } from '@/composables/browse/use-app-server-browse'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { mergeProviderResults } from '@/helpers/browse-merge'
import {
	get_project,
	get_project_v3,
	get_project_v3_many,
	get_search_results_v3,
	get_version_many,
} from '@/helpers/cache.js'
import {
	bilingualTitle,
	type ChineseSearchResolution,
	type ChineseSearchTranslation,
	containsChineseSearchText,
	resolveChineseContentSearch,
} from '@/helpers/content-search'
import {
	type CurseForgeCategory,
	getCurseForgeCapability,
	getCurseForgeCategories,
	getCurseForgeImageUrl,
	searchCurseForgeProjects,
	type UnifiedSearchHit,
} from '@/helpers/curseforge'
import {
	CF_EXTRA_CATEGORY_HEADER,
	curseForgeCategoryValue,
	findUnmappedCurseForgeCategories,
	isCurseForgeOnlyCategoryName,
	localizeCurseForgeCategoryLabels,
	localizeCurseForgeCategoryName,
	localizeCurseForgeLabel,
	resolveCurseForgeCategoryIdsFromFilterValues,
} from '@/helpers/curseforge-category-map'
import { instance_listener } from '@/helpers/events.js'
import {
	get as getInstance,
	get_installed_project_ids as getInstalledProjectIds,
} from '@/helpers/instance'
import { isBuiltInInstanceIcon } from '@/helpers/instance-icon-frame'
import { get_loader_versions as getLoaderManifest } from '@/helpers/metadata'
import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { get_categories, get_game_versions, get_loaders } from '@/helpers/tags'
import { get_instance_worlds } from '@/helpers/worlds'
import { injectContentInstall } from '@/providers/content-install'
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
const queryClient = useQueryClient()
const debugLog = useDebugLogger('Browse')
const router = useRouter()
const route = useRoute()
const projectType = ref<ProjectType>(route.params.projectType as ProjectType)

const curseForgeClassIds: Partial<Record<ProjectType, number>> = {
	mod: 6,
	plugin: 5,
	resourcepack: 12,
	datapack: 6945,
	shader: 6552,
	modpack: 4471,
}

const curseForgeCapability = ref(
	await getCurseForgeCapability().catch(() => ({
		status: 'missing_key' as const,
		configured: false,
	})),
)
const contentSource = ref<'all' | 'modrinth' | 'curseforge'>(
	curseForgeCapability.value.configured && route.query.source === 'curseforge'
		? 'curseforge'
		: route.query.source === 'modrinth'
			? 'modrinth'
			: curseForgeCapability.value.configured
				? 'all'
				: 'modrinth',
)
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

if (
	curseForgeCapability.value.configured &&
	(contentSource.value === 'curseforge' || contentSource.value === 'all')
) {
	await ensureCurseForgeCategories(projectType.value).catch(handleError)
}

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
const [categories, loaders, availableGameVersions] = await Promise.all([
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
	gameVersions: availableGameVersions.value ?? [],
	loaders: loaders.value ?? [],
	categories:
		contentSource.value === 'curseforge'
			? curseForgeCategoryTags.value
			: contentSource.value === 'all'
				? allSourceCategoryTags.value
				: (categories.value ?? []),
}))

type Instance = {
	game_version: string
	loader: string
	path: string
	install_stage: string
	icon_path?: string
	name: string
	link?: {
		type: string
		project_id: string
		version_id: string
	}
}

const instance: Ref<Instance | null> = ref(null)
const installedProjectIds: Ref<string[] | null> = ref(null)
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

const allInstalledIds = computed(
	() => new Set([...newlyInstalled.value, ...(installedProjectIds.value ?? [])]),
)

function syncHiddenInstanceProjectIds() {
	hiddenInstanceProjectIds.value = new Set([
		...(installedProjectIds.value ?? []),
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

await initInstanceContext()

async function refreshInstalledProjectIds() {
	if (!route.query.i) return

	if (route.query.from === 'worlds') {
		const worlds = await get_instance_worlds(route.query.i as string).catch(handleError)
		if (!worlds) return

		const serverProjectIds = worlds
			.filter((w) => w.type === 'server' && 'project_id' in w && w.project_id)
			.map((w) => (w as { project_id: string }).project_id)
		debugLog('installedServerProjectIds loaded', { count: serverProjectIds.length })
		installedProjectIds.value = serverProjectIds
		return
	}

	const ids = await getInstalledProjectIds(route.query.i as string).catch(handleError)
	if (!ids) return

	debugLog('installedProjectIds loaded', { count: ids.length })
	installedProjectIds.value = ids
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

	if (route.query.i) {
		instance.value = (await getInstance(route.query.i as string).catch(handleError)) ?? null
		debugLog('instance loaded', {
			name: instance.value?.name,
			loader: instance.value?.loader,
			gameVersion: instance.value?.game_version,
		})

		await refreshInstalledProjectIds()

		if (instance.value?.link?.project_id) {
			debugLog('checking linked project for server status', instance.value.link.project_id)
			const projectV3 = await get_project_v3(
				instance.value.link.project_id,
				'must_revalidate',
			).catch(handleError)
			if (projectV3?.minecraft_server != null) {
				debugLog('instance is a server instance')
				isServerInstance.value = true
			}
		}
	}

	if (route.query.ai && !(route.params.projectType === 'modpack')) {
		debugLog('setting instanceHideInstalled from query', route.query.ai)
		instanceHideInstalled.value = route.query.ai === 'true'
	}
}

const instanceFilters = computed(() => {
	const filters = []

	if (instance.value) {
		const gameVersion = instance.value.game_version
		if (gameVersion) {
			filters.push({ type: 'game_version', option: gameVersion })
		}

		const platform = instance.value.loader
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
		if (gameVersion) filters.push({ type: 'game_version', option: gameVersion })

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

const combinedProvidedFilters = computed(() =>
	isServerContext.value ? serverContextFilters.value : instanceFilters.value,
)

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
	modrinthSource: {
		id: 'app.browse.source.modrinth',
		defaultMessage: 'Modrinth',
	},
	curseForgeSource: {
		id: 'app.browse.source.curseforge',
		defaultMessage: 'CurseForge',
	},
})

const breadcrumbs = useBreadcrumbs()
const browseTitle = computed(() =>
	formatMessage(isFromWorlds.value ? messages.discoverServers : messages.discoverContent),
)
breadcrumbs.setName('BrowseTitle', browseTitle.value)
if (instance.value) {
	const instanceLink = `/instance/${encodeURIComponent(instance.value.id)}`
	breadcrumbs.setContext({
		name: instance.value.name,
		link: isFromWorlds.value ? `${instanceLink}/worlds` : instanceLink,
	})
} else {
	breadcrumbs.setContext(null)
}

onBeforeRouteLeave(() => {
	breadcrumbs.setContext({
		name: browseTitle.value,
		link: `/browse/${projectType.value}`,
		query: route.query,
	})
})

function resetInstanceContext() {
	if (!instance.value) return

	debugLog('instance context removed, resetting')
	instance.value = null
	installedProjectIds.value = null
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
	let dataPacks = false,
		mods = false,
		modpacks = false

	if (instance.value) {
		if (
			availableGameVersions.value &&
			availableGameVersions.value.findIndex((x) => x.version === instance.value?.game_version) <=
				availableGameVersions.value.findIndex((x) => x.version === '1.13') &&
			!isServerInstance.value
		) {
			dataPacks = true
		}

		if (instance.value.loader !== 'vanilla') {
			mods = true
		}
	} else {
		dataPacks = true
		mods = true
		modpacks = true
	}

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
		return [{ label: 'Servers', href: `/browse/server${suffix}` }]
	}

	return [
		{
			label: formatMessage(messages.modpacksProjectType),
			href: `/browse/modpack${suffix}`,
			shown: modpacks,
		},
		{ label: formatMessage(messages.modsProjectType), href: `/browse/mod${suffix}`, shown: mods },
		{
			label: formatMessage(messages.resourcepacksProjectType),
			href: `/browse/resourcepack${suffix}`,
		},
		{
			label: formatMessage(messages.datapacksProjectType),
			href: `/browse/datapack${suffix}`,
			shown: dataPacks,
		},
		{ label: formatMessage(messages.shadersProjectType), href: `/browse/shader${suffix}` },
		{
			label: formatMessage(messages.serversProjectType),
			href: `/browse/server${suffix}`,
			shown: !instance.value,
		},
	]
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
	if (instance.value) {
		return {
			name: instance.value.name,
			loader: instance.value.loader,
			gameVersion: instance.value.game_version,
			iconSrc: instance.value.icon_path ? convertFileSrc(instance.value.icon_path) : null,
			iconFrameless: isBuiltInInstanceIcon(instance.value.icon_path),
			backUrl: `/instance/${encodeURIComponent(instance.value.id)}${isFromWorlds.value ? '/worlds' : ''}`,
			backLabel: formatMessage(messages.backToInstance),
			heading: formatMessage(
				isFromWorlds.value ? messages.addServersToInstance : commonMessages.installingContentLabel,
			),
			warning:
				isServerInstance.value && !isFromWorlds.value
					? formatMessage(messages.serverInstanceContentWarning)
					: undefined,
		}
	}
	return null
})

const installingProjectIds = ref<Set<string>>(new Set())

function setProjectInstalling(projectId: string, installing: boolean) {
	const next = new Set(installingProjectIds.value)
	if (installing) {
		next.add(projectId)
	} else {
		next.delete(projectId)
	}
	installingProjectIds.value = next
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
			gameVersion: instance.value?.game_version,
			loader: instance.value?.loader,
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
	const targetInstance = instance.value
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
		provider?: 'modrinth' | 'curseforge'
		provider_project_id?: string
	}
	const isInstalled =
		projectResult.installed ||
		allInstalledIds.value.has(projectResult.project_id || '') ||
		serverContentProjectIds.value.has(projectResult.project_id || '') ||
		serverContextServerData.value?.upstream?.project_id === projectResult.project_id
	const isInstalling = installingProjectIds.value.has(projectResult.project_id)

	if (
		isServerContext.value &&
		projectResult.provider !== 'curseforge' &&
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
					if (isQueued) {
						removeQueuedServerInstall(projectResult.project_id)
						return
					}

					const contentType = currentProjectType as BrowseInstallContentType
					const isModpack = contentType === 'modpack'
					const shouldShowInstalling = isModpack || !isQueued
					if (shouldShowInstalling) {
						setProjectInstalling(projectResult.project_id, true)
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
							setProjectInstalling(projectResult.project_id, false)
						}
					}
				},
			},
		]
	}

	const isModpack =
		projectResult.project_types?.includes('modpack') || projectResult.project_type === 'modpack'
	const shouldUseInstallIcon = !!instance.value || isModpack

	return [
		{
			key: 'install',
			label: formatMessage(
				isInstalling
					? messages.installingToServer
					: isInstalled
						? commonMessages.installedLabel
						: shouldUseInstallIcon
							? commonMessages.installButton
							: messages.addToAnInstance,
			),
			icon: isInstalling ? SpinnerIcon : isInstalled ? CheckIcon : PlusIcon,
			iconClass: isInstalling ? 'animate-spin' : undefined,
			disabled: isInstalled || isInstalling,
			color: 'brand',
			type: 'outlined',
			onClick: async () => {
				setProjectInstalling(projectResult.project_id, true)
				try {
					const selectedInstall =
						instance.value && projectResult.provider !== 'curseforge'
							? await chooseInstanceInstallVersion(projectResult, currentProjectType)
							: { versionId: null as string | null }
					if (selectedInstall === null) {
						setProjectInstalling(projectResult.project_id, false)
						return
					}
					const selectedPreferences = getCurrentSelectedInstallPreferences(currentProjectType)
					const installContent =
						projectResult.provider === 'curseforge' ? installCurseForge : installVersion
					await installContent(
						projectResult.provider_project_id ?? projectResult.project_id,
						selectedInstall.versionId,
						instance.value ? instance.value.id : null,
						'SearchCard',
						(versionId, installedProjectIds) => {
							setProjectInstalling(projectResult.project_id, false)
							if (versionId) {
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
					setProjectInstalling(projectResult.project_id, false)
					handleError(err)
				}
			},
		},
	]
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

function mapCurseForgeHit(hit: UnifiedSearchHit) {
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
		categories: localizeCurseForgeCategoryLabels(hit.categories),
		display_categories: localizeCurseForgeCategoryLabels(hit.categories),
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
	provider?: 'modrinth' | 'curseforge'
	provider_project_id?: string
	installed?: boolean
	chinese_search_score?: number
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
	const provider = hit.provider === 'curseforge' ? 'curseforge' : 'modrinth'
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
		const key = `${hit.provider ?? 'modrinth'}:${hit.project_id}`
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

async function search(requestParams: string) {
	debugLog('searching v3', requestParams)
	const isServer = projectType.value === 'server'
	const params = new URLSearchParams(requestParams)
	const limit = Math.min(Number(params.get('limit') ?? 20), 50)
	const offset = Number(params.get('offset') ?? 0)
	const rawQuery = params.get('query') ?? ''
	let chineseResolution: ChineseSearchResolution | null = null
	if (containsChineseSearchText(rawQuery)) {
		chineseResolution = await resolveChineseContentSearch(rawQuery).catch((error) => {
			debugLog('chinese search resolution failed, using original query', error)
			return null
		})
	}
	const filters = params.get('new_filters') ?? ''
	const categoryValues = getSearchFilterValues(filters, 'categories')
	const hasOnlyCurseForgeExclusiveCategories =
		categoryValues.length > 0 &&
		categoryValues.every(
			(value) => isCurseForgeOnlyCategoryName(value) || curseForgeLoaderTypes[value] !== undefined,
		) &&
		categoryValues.some((value) => isCurseForgeOnlyCategoryName(value))

	const includeModrinth =
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

	const gameVersion = getFirstSearchFilter(filters, 'game_versions')
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
	const modrinthRequest = includeModrinth
		? queryClient.fetchQuery({
				queryKey: ['search', 'v3', modrinthRequestParams],
				queryFn: () =>
					get_search_results_v3(modrinthRequestParams, 'must_revalidate') as Promise<{
						result: Labrinth.Search.v3.SearchResults & {
							hits: (Labrinth.Search.v3.ResultSearchProject & { installed?: boolean })[]
						}
					} | null>,
				staleTime: 30_000,
			})
		: Promise.resolve(null)
	if (includeCurseForge) {
		debugLog('curseforge filters', {
			filters,
			categoryValues,
			categoryIds: curseForgeCategoryIds,
			gameVersion,
			loader,
		})
	}
	const curseForgeRequest = includeCurseForge
		? searchCurseForgeProjects({
				classId: curseForgeClassIds[projectType.value]!,
				categoryIds: curseForgeCategoryIds,
				searchFilter: (chineseResolution?.curseforgeQuery ?? rawQuery) || undefined,
				gameVersion: gameVersion || undefined,
				modLoaderType: loader ? curseForgeLoaderTypes[loader] : undefined,
				sortField: getCurseForgeSortField(params.get('index')),
				sortOrder: 'desc',
				index: offset,
				pageSize: limit,
			})
		: Promise.resolve(null)
	const directModrinthRequest =
		includeModrinth &&
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
	const rawResults = modrinthResult.status === 'fulfilled' ? modrinthResult.value : null
	const rawCurseForge = curseForgeResult.status === 'fulfilled' ? curseForgeResult.value : null
	const rawDirectModrinth =
		directModrinthResult.status === 'fulfilled'
			? (directModrinthResult.value as DirectModrinthProject[])
			: []

	if (modrinthResult.status === 'rejected') {
		debugLog('modrinth search failed', modrinthResult.reason)
	}
	if (curseForgeResult.status === 'rejected') {
		debugLog('curseforge search failed', curseForgeResult.reason)
	}
	if (directModrinthResult.status === 'rejected') {
		debugLog('direct modrinth chinese candidates failed', directModrinthResult.reason)
	}

	if (!rawResults && !rawCurseForge && rawDirectModrinth.length === 0) {
		const error =
			modrinthResult.status === 'rejected'
				? modrinthResult.reason
				: curseForgeResult.status === 'rejected'
					? curseForgeResult.reason
					: new Error('No content providers are available')
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
			provider?: 'modrinth' | 'curseforge'
		}

		if (instance.value || isServerContext.value) {
			const installedIds = instance.value
				? new Set([...newlyInstalled.value, ...(installedProjectIds.value ?? [])])
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
			if (instance.value || isServerContext.value) {
				const installedIds = instance.value
					? new Set([...newlyInstalled.value, ...(installedProjectIds.value ?? [])])
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
		.map((hit) => applyChineseTranslation(hit as ChineseSearchHit, chineseResolution))
	return {
		projectHits:
			contentSource.value === 'all'
				? mergeProviderResults({
						modrinthHits,
						curseForgeHits,
						sort: params.get('index'),
						query: params.get('query'),
						limit,
					})
				: contentSource.value === 'curseforge'
					? curseForgeHits
					: modrinthHits.slice(0, limit),
		serverHits: [],
		total_hits:
			contentSource.value === 'all'
				? (rawResults?.result.total_hits ?? 0) +
					(rawCurseForge?.total_hits ?? 0) +
					injectedModrinthCount
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

const searchState = useBrowseSearch({
	projectType,
	tags,
	providedFilters: combinedProvidedFilters,
	search,
	persistentQueryParams: ['i', 'ai', 'shi', 'sid', 'wid', 'from', 'source'],
	getExtraQueryParams: () => ({
		sid: serverIdQuery.value || undefined,
		wid: effectiveServerWorldId.value || undefined,
		ai: instanceHideInstalled.value ? 'true' : undefined,
		shi: serverHideInstalled.value ? 'true' : undefined,
		source: contentSource.value === 'all' ? undefined : contentSource.value,
	}),
})

watch(contentSource, async (source) => {
	searchState.projectHits.value = []
	searchState.totalHits.value = 0
	searchState.loading.value = true
	searchState.currentFilters.value = searchState.currentFilters.value.filter(
		(filter) => !filter.type.startsWith('category_'),
	)
	if (source === 'curseforge' || source === 'all') {
		await ensureCurseForgeCategories(projectType.value).catch(handleError)
	}
	await searchState.refreshSearch()
})

watch(projectType, async (type) => {
	if (contentSource.value === 'curseforge' || contentSource.value === 'all') {
		await ensureCurseForgeCategories(type).catch(handleError)
	}
})

function selectContentSource(source: string) {
	if (source === 'all' || source === 'modrinth' || source === 'curseforge') {
		contentSource.value = source
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
		} else if (instance.value) {
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

if (instance.value?.game_version) {
	const gv = instance.value.game_version
	const alreadyHasGv = searchState.serverCurrentFilters.value.some(
		(f) => f.type === 'server_game_version' && f.option === gv,
	)
	if (!alreadyHasGv) {
		searchState.serverCurrentFilters.value.push({ type: 'server_game_version', option: gv })
	}
}

void searchState.refreshSearch()

type UnlistenFn = () => void

let isUnmounted = false
let unlistenInstances: UnlistenFn | null = null

onMounted(() => {
	instance_listener(async (event: { event: string; instance_id: string }) => {
		if (instance.value && event.instance_id === instance.value.id && event.event === 'synced') {
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
	if (!installContext.value) return undefined
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
			provider?: 'modrinth' | 'curseforge'
			provider_project_id?: string
		},
	) => ({
		path:
			result.provider === 'curseforge'
				? `/project/curseforge/${result.provider_project_id}`
				: `/project/${result.project_id ?? result.slug}`,
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
		() => (isServerContext.value && projectType.value !== 'modpack') || !!instance.value,
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
		if ('provider' in result && result.provider === 'curseforge') return
		handleRightClick(event, result)
	},
	offline,
	lockedFilterMessages,
})
</script>

<template>
	<div data-onboarding-id="browse-content" class="flex flex-col gap-3 p-6">
		<div v-if="curseForgeCapability.configured && projectType !== 'server'" class="flex gap-2">
			<ButtonStyled
				v-for="source in [
					{ id: 'all', label: messages.allSources },
					{ id: 'modrinth', label: messages.modrinthSource },
					{ id: 'curseforge', label: messages.curseForgeSource },
				]"
				:key="source.id"
				:type="contentSource === source.id ? 'outlined' : 'transparent'"
			>
				<button @click="selectContentSource(source.id)">
					{{ formatMessage(source.label) }}
				</button>
			</ButtonStyled>
		</div>
		<BrowsePageLayout>
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
		</BrowsePageLayout>
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
		<Teleport to="#sidebar-teleport-target">
			<BrowseSidebar />
		</Teleport>
	</div>
</template>
