<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	BookmarkFilledIcon,
	BookmarkIcon,
	CheckIcon,
	ChevronDownIcon,
	GenericListIcon,
	GridIcon,
	ListIcon,
	PlusIcon,
	SearchIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import {
	type BrowseInstallContext,
	BrowseInstallHeader,
	ButtonStyled,
	commonMessages,
	defineMessages,
	EmptyState,
	getLatestMatchingInstallVersion,
	getTargetInstallPreferences,
	injectNotificationManager,
	LoadingIndicator,
	NavTabs,
	Pagination,
	PopoutMenu,
	ProjectCard,
	ProjectCardList,
	SelectedProjectsFloatingBar,
	StyledInput,
	useStickyObserver,
	useVIntl,
} from '@modrinth/ui'
import { computed, nextTick, onMounted, ref, shallowRef, watch } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

import BrowseInstanceSelector from '@/components/browse/BrowseInstanceSelector.vue'
import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import { useContentFavorites } from '@/composables/useContentFavorites'
import { createBrowseProjectTabs, getBrowseProjectTabOptions } from '@/helpers/browse-project-tabs'
import {
	completeBrowseReturnNavigation,
	consumeBrowseReturnSnapshot,
	isBrowseReturnSourcePath,
	saveBrowseReturnSnapshot,
} from '@/helpers/browse-return-state.ts'
import { get_project, get_project_many, get_version_many } from '@/helpers/cache.js'
import {
	type ContentFavorite,
	contentFavoriteKey,
	type FavoriteContentType,
	type FavoriteProvider,
} from '@/helpers/content-favorites'
import {
	type CurseForgeProject,
	getCurseForgeFiles,
	getCurseForgeImageUrl,
	getCurseForgeProjects,
} from '@/helpers/curseforge'
import { getDisplayInstanceIcon } from '@/helpers/instance-icons'
import {
	getLastBrowseContentDisplayMode,
	setLastBrowseContentDisplayMode,
} from '@/helpers/settings'
import type { GameInstance } from '@/helpers/types'
import { injectContentSelection, makeContentSelectionKey } from '@/providers/content-selection'
import { useBreadcrumbs } from '@/store/breadcrumbs'

type FavoriteFilter = 'all' | FavoriteContentType
type FavoriteDisplayMode = 'list' | 'compact' | 'grid'

type FavoriteProject = {
	favorite: ContentFavorite
	provider: FavoriteProvider
	projectId: string
	title: string
	description: string
	slug?: string
	iconUrl?: string
	downloads?: number
	categories: string[]
	dateCreated?: string
	dateModified?: string
	banner?: string
	color?: string | number
	environment?: {
		clientSide: Labrinth.Projects.v2.Environment
		serverSide: Labrinth.Projects.v2.Environment
	}
	unavailable: boolean
}

type FavoritesReturnState = {
	projects: FavoriteProject[]
}

const PAGE_SIZE = 20
const curseForgeLoaderTypes: Record<string, number> = {
	forge: 1,
	fabric: 4,
	quilt: 5,
	neoforge: 6,
}

const route = useRoute()
const router = useRouter()
const breadcrumbs = useBreadcrumbs()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const contentSelection = injectContentSelection()
const contentFavorites = useContentFavorites()
const instanceSelector = ref<InstanceType<typeof BrowseInstanceSelector>>()
const stickyInstallHeaderRef = ref<HTMLElement | null>(null)
const { isStuck: isInstallHeaderStuck } = useStickyObserver(
	stickyInstallHeaderRef,
	'FavoritesInstallHeader',
)
const projects = shallowRef<FavoriteProject[]>([])
const loadingProjects = ref(false)
const installingKeys = ref(new Set<string>())
const displayMode = ref<FavoriteDisplayMode>(getLastBrowseContentDisplayMode())
let projectRequestId = 0
const browseReturnSnapshot = consumeBrowseReturnSnapshot<FavoritesReturnState>(route.fullPath)
if (browseReturnSnapshot) projects.value = browseReturnSnapshot.state.projects

const messages = defineMessages({
	title: {
		id: 'app.content-favorites.title',
		defaultMessage: 'Favorites',
	},
	search: {
		id: 'app.content-favorites.search',
		defaultMessage: 'Search favorites',
	},
	allContentTypes: {
		id: 'app.content-favorites.type.all',
		defaultMessage: 'All content types',
	},
	mods: {
		id: 'app.browse.project-type.mods',
		defaultMessage: 'Mods',
	},
	resourcepacks: {
		id: 'app.browse.project-type.resourcepacks',
		defaultMessage: 'Resource Packs',
	},
	datapacks: {
		id: 'app.browse.project-type.datapacks',
		defaultMessage: 'Data Packs',
	},
	shaders: {
		id: 'app.browse.project-type.shaders',
		defaultMessage: 'Shaders',
	},
	modpacks: {
		id: 'app.browse.project-type.modpacks',
		defaultMessage: 'Modpacks',
	},
	maps: {
		id: 'app.browse.project-type.maps',
		defaultMessage: 'Maps',
	},
	servers: {
		id: 'app.browse.project-type.servers',
		defaultMessage: 'Servers',
	},
	chooseInstance: {
		id: 'app.browse.choose-instance',
		defaultMessage: 'Choose instance',
	},
	backToInstance: {
		id: 'app.browse.back-to-instance',
		defaultMessage: 'Back to instance',
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
	noCompatibleVersion: {
		id: 'app.browse.no-compatible-version',
		defaultMessage: 'No compatible version was found for the selected instance.',
	},
	remove: {
		id: 'app.content-favorites.remove',
		defaultMessage: 'Remove from favorites',
	},
	view: {
		id: 'app.browse.display-mode.switch',
		defaultMessage: 'Switch view',
	},
	listView: {
		id: 'app.browse.display-mode.list',
		defaultMessage: 'List',
	},
	compactView: {
		id: 'app.browse.display-mode.compact-list',
		defaultMessage: 'Compact list',
	},
	gridView: {
		id: 'app.browse.display-mode.grid',
		defaultMessage: 'Grid',
	},
	recentlySaved: {
		id: 'app.content-favorites.sort.recently-saved',
		defaultMessage: 'Recently saved',
	},
	emptyTitle: {
		id: 'app.content-favorites.empty-title',
		defaultMessage: 'No favorites yet',
	},
	emptyDescription: {
		id: 'app.content-favorites.empty-description',
		defaultMessage: 'Bookmark mods, resource packs, data packs, and shaders to install them later.',
	},
	noMatchesTitle: {
		id: 'app.content-favorites.no-matches-title',
		defaultMessage: 'No matching favorites',
	},
	noMatchesDescription: {
		id: 'app.content-favorites.no-matches-description',
		defaultMessage: 'Try a different search or content type.',
	},
	unavailableTitle: {
		id: 'app.content-favorites.unavailable-title',
		defaultMessage: '{provider} project {projectId}',
	},
	unavailableDescription: {
		id: 'app.content-favorites.unavailable-description',
		defaultMessage: 'This project is currently unavailable. You can remove it from favorites.',
	},
})

function queryValue(value: unknown): string {
	return typeof value === 'string' ? value : ''
}

function parseFavoriteFilter(value: unknown): FavoriteFilter {
	return value === 'mod' || value === 'resourcepack' || value === 'datapack' || value === 'shader'
		? value
		: 'all'
}

function parsePage(value: unknown): number {
	const page = Number.parseInt(queryValue(value), 10)
	return Number.isFinite(page) && page > 0 ? page : 1
}

function updateQuery(values: Record<string, string | undefined>) {
	void router.replace({
		query: {
			...route.query,
			...values,
		},
	})
}

const search = computed({
	get: () => queryValue(route.query.q),
	set: (value: string) => updateQuery({ q: value || undefined, page: undefined }),
})

const filter = computed<FavoriteFilter>({
	get: () => parseFavoriteFilter(route.query.kind),
	set: (value) => updateQuery({ kind: value === 'all' ? undefined : value, page: undefined }),
})

const currentPage = computed({
	get: () => parsePage(route.query.page),
	set: (page: number) => updateQuery({ page: page > 1 ? String(page) : undefined }),
})

const favoriteTypeOptions = computed(() => [
	{ id: 'all' as const, label: formatMessage(messages.allContentTypes) },
	{ id: 'mod' as const, label: formatMessage(messages.mods) },
	{ id: 'resourcepack' as const, label: formatMessage(messages.resourcepacks) },
	{ id: 'datapack' as const, label: formatMessage(messages.datapacks) },
	{ id: 'shader' as const, label: formatMessage(messages.shaders) },
])

const currentFavoriteTypeLabel = computed(
	() => favoriteTypeOptions.value.find((option) => option.id === filter.value)?.label,
)

const displayModeOptions = computed(() => [
	{ id: 'list' as const, label: formatMessage(messages.listView), icon: ListIcon },
	{ id: 'compact' as const, label: formatMessage(messages.compactView), icon: GenericListIcon },
	{ id: 'grid' as const, label: formatMessage(messages.gridView), icon: GridIcon },
])

const currentDisplayMode = computed(() =>
	displayModeOptions.value.find((option) => option.id === displayMode.value),
)

const projectTabs = computed(() => {
	const query = new URLSearchParams()
	for (const [key, value] of Object.entries(route.query)) {
		if (key === 'kind' || key === 'q' || key === 'page') continue
		if (Array.isArray(value)) {
			for (const entry of value) {
				if (entry != null) query.append(key, entry)
			}
		} else if (value) {
			query.set(key, value)
		}
	}
	const suffix = query.size ? `?${query.toString()}` : ''
	return createBrowseProjectTabs(
		{
			modpacks: formatMessage(messages.modpacks),
			mods: formatMessage(messages.mods),
			resourcepacks: formatMessage(messages.resourcepacks),
			datapacks: formatMessage(messages.datapacks),
			maps: formatMessage(messages.maps),
			shaders: formatMessage(messages.shaders),
			servers: formatMessage(messages.servers),
			favorites: formatMessage(messages.title),
		},
		suffix,
		getBrowseProjectTabOptions({
			instance: contentSelection.targetInstance.value,
			hasInstanceContext: !!route.query.i,
			isServerInstance:
				contentSelection.targetInstance.value?.link?.type === 'server_project' ||
				contentSelection.targetInstance.value?.link?.type === 'server_project_modpack',
		}),
	)
})

function typeLabel(type: FavoriteContentType): string {
	return formatMessage(
		{
			mod: messages.mods,
			resourcepack: messages.resourcepacks,
			datapack: messages.datapacks,
			shader: messages.shaders,
		}[type],
	)
}

function toUnavailable(favorite: ContentFavorite): FavoriteProject {
	const provider =
		favorite.provider === 'curseforge'
			? 'CurseForge'
			: favorite.provider === 'mcarchive'
				? 'MCArchive'
				: 'Modrinth'
	return {
		favorite,
		provider: favorite.provider,
		projectId: favorite.project_id,
		title: formatMessage(messages.unavailableTitle, {
			provider,
			projectId: favorite.project_id,
		}),
		description: formatMessage(messages.unavailableDescription),
		categories: [typeLabel(favorite.content_type)],
		unavailable: true,
	}
}

function toModrinthFavorite(
	project: Labrinth.Projects.v2.Project,
	favorite: ContentFavorite,
): FavoriteProject {
	return {
		favorite,
		provider: 'modrinth',
		projectId: project.id,
		title: project.title,
		description: project.description,
		slug: project.slug,
		iconUrl: project.icon_url,
		downloads: project.downloads,
		categories: [typeLabel(favorite.content_type), ...project.categories],
		dateCreated: project.published,
		dateModified: project.updated,
		banner: project.gallery?.find((image) => image.featured)?.url,
		color: project.color,
		environment: {
			clientSide: project.client_side,
			serverSide: project.server_side,
		},
		unavailable: false,
	}
}

function toCurseForgeFavorite(
	project: CurseForgeProject,
	favorite: ContentFavorite,
): FavoriteProject {
	return {
		favorite,
		provider: 'curseforge',
		projectId: project.id.toString(),
		title: project.name,
		description: project.summary,
		slug: project.slug,
		iconUrl: getCurseForgeImageUrl(project.logo?.thumbnailUrl),
		downloads: project.downloadCount,
		categories: [
			typeLabel(favorite.content_type),
			...project.categories.map((category) => category.slug),
		],
		dateCreated: project.dateCreated,
		dateModified: project.dateModified,
		banner: getCurseForgeImageUrl(project.screenshots[0]?.thumbnailUrl, 960),
		unavailable: false,
	}
}

async function loadModrinthFavorites(projectIds: string[]) {
	try {
		return (await get_project_many(projectIds, 'bypass')) as Labrinth.Projects.v2.Project[]
	} catch (error) {
		handleError(error)
		return await get_project_many(projectIds, 'cache_only').catch((cacheError) => {
			handleError(cacheError)
			return [] as Labrinth.Projects.v2.Project[]
		})
	}
}

async function loadCurseForgeFavorites(projectIds: number[]) {
	try {
		return await getCurseForgeProjects(projectIds, 'bypass')
	} catch (error) {
		handleError(error)
		return await getCurseForgeProjects(projectIds, 'cache_only').catch((cacheError) => {
			handleError(cacheError)
			return [] as CurseForgeProject[]
		})
	}
}

async function refreshProjects() {
	const requestId = ++projectRequestId
	loadingProjects.value = true
	try {
		await contentFavorites.load(true)
		const favorites = [...contentFavorites.favorites.value]
		if (favorites.length === 0) {
			if (requestId === projectRequestId) projects.value = []
			return
		}

		const modrinthIds = favorites
			.filter((favorite) => favorite.provider === 'modrinth')
			.map((favorite) => favorite.project_id)
		const curseForgeIds = favorites
			.filter((favorite) => favorite.provider === 'curseforge')
			.map((favorite) => Number(favorite.project_id))
			.filter(Number.isSafeInteger)
		const [modrinthResult, curseForgeResult] = await Promise.all([
			modrinthIds.length
				? loadModrinthFavorites(modrinthIds)
				: Promise.resolve([] as Labrinth.Projects.v2.Project[]),
			curseForgeIds.length
				? loadCurseForgeFavorites(curseForgeIds)
				: Promise.resolve([] as CurseForgeProject[]),
		])
		if (requestId !== projectRequestId) return

		const modrinthById = new Map(modrinthResult.map((project) => [project.id, project]))
		const curseForgeById = new Map(
			curseForgeResult.map((project) => [project.id.toString(), project]),
		)
		projects.value = favorites.map((favorite) => {
			if (favorite.provider === 'modrinth') {
				const project = modrinthById.get(favorite.project_id)
				return project ? toModrinthFavorite(project, favorite) : toUnavailable(favorite)
			}
			if (favorite.provider === 'curseforge') {
				const project = curseForgeById.get(favorite.project_id)
				return project ? toCurseForgeFavorite(project, favorite) : toUnavailable(favorite)
			}
			return toUnavailable(favorite)
		})
	} catch (error) {
		handleError(error)
	} finally {
		if (requestId === projectRequestId) loadingProjects.value = false
	}
}

const activeFavoriteKeys = computed(
	() =>
		new Set(
			contentFavorites.favorites.value.map((favorite) =>
				contentFavoriteKey(favorite.provider, favorite.project_id),
			),
		),
)

const availableProjects = computed(() =>
	projects.value
		.filter((project) =>
			activeFavoriteKeys.value.has(contentFavoriteKey(project.provider, project.projectId)),
		)
		.sort((left, right) => right.favorite.saved_at - left.favorite.saved_at),
)

const filteredProjects = computed(() => {
	const normalizedSearch = search.value.trim().toLocaleLowerCase()
	return availableProjects.value.filter((project) => {
		if (filter.value !== 'all' && project.favorite.content_type !== filter.value) return false
		if (!normalizedSearch) return true
		return [project.title, project.description, project.provider, project.projectId]
			.join('\n')
			.toLocaleLowerCase()
			.includes(normalizedSearch)
	})
})

const pageCount = computed(() => Math.max(1, Math.ceil(filteredProjects.value.length / PAGE_SIZE)))
const pagedProjects = computed(() => {
	const page = Math.min(currentPage.value, pageCount.value)
	const offset = (page - 1) * PAGE_SIZE
	return filteredProjects.value.slice(offset, offset + PAGE_SIZE)
})

watch(pageCount, (count) => {
	if (currentPage.value > count) currentPage.value = count
})

const installContext = computed<BrowseInstallContext | null>(() => {
	const target = contentSelection.targetInstance.value
	if (!target) return null
	const icon = getDisplayInstanceIcon(target.icon_path, target.loader)
	const processing = ['validating', 'reviewing', 'queueing'].includes(contentSelection.state.value)
	return {
		showInstallHeader: !!route.query.i,
		name: target.name,
		loader: target.loader,
		gameVersion: target.game_version,
		iconSrc: icon.url,
		iconFrameless: icon.frameless,
		backUrl: route.query.i ? `/instance/${encodeURIComponent(target.id)}` : route.fullPath,
		backLabel: formatMessage(messages.backToInstance),
		heading: formatMessage(commonMessages.installingContentLabel),
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
})

function isInstalling(project: FavoriteProject) {
	const key = makeContentSelectionKey(project.provider, project.projectId)
	return installingKeys.value.has(key) || contentSelection.isInstalling(key)
}

function isSelected(project: FavoriteProject) {
	return contentSelection.isSelected(makeContentSelectionKey(project.provider, project.projectId))
}

function installLabel(project: FavoriteProject) {
	if (isInstalling(project)) return formatMessage(commonMessages.validatingLabel)
	if (isSelected(project)) return formatMessage(messages.selected)
	return formatMessage(commonMessages.installButton)
}

function setInstalling(key: string, installing: boolean) {
	const next = new Set(installingKeys.value)
	if (installing) next.add(key)
	else next.delete(key)
	installingKeys.value = next
}

function getInstallPreferences(target: GameInstance, type: FavoriteContentType) {
	return getTargetInstallPreferences(
		{ gameVersion: target.game_version, loader: target.loader },
		type,
	)
}

async function getModrinthVersions(projectId: string) {
	const project = (await get_project(projectId, 'must_revalidate')) as Labrinth.Projects.v2.Project
	return (await get_version_many(
		project.versions,
		'must_revalidate',
	)) as Labrinth.Versions.v2.Version[]
}

async function toggleProjectSelection(project: FavoriteProject) {
	if (project.unavailable) return
	if (!contentSelection.targetInstance.value) {
		await contentSelection.refreshInstances(queryValue(route.query.i) || undefined)
		instanceSelector.value?.show()
		return
	}

	const target = contentSelection.targetInstance.value
	const key = makeContentSelectionKey(project.provider, project.projectId)
	if (contentSelection.isSelected(key)) {
		contentSelection.remove(key)
		return
	}

	setInstalling(key, true)
	try {
		const preferences = getInstallPreferences(target, project.favorite.content_type)
		const versionId =
			project.provider === 'modrinth'
				? getLatestMatchingInstallVersion(await getModrinthVersions(project.projectId), preferences)
						?.id
				: (
						await getCurseForgeFiles(Number(project.projectId), {
							gameVersion: target.game_version,
							modLoaderType:
								project.favorite.content_type === 'mod'
									? curseForgeLoaderTypes[target.loader]
									: undefined,
						})
					).files
						.find((file) => file.isAvailable)
						?.id.toString()
		if (!versionId) throw new Error(formatMessage(messages.noCompatibleVersion))

		await contentSelection.add({
			key,
			provider: project.provider,
			projectId: project.projectId,
			providerProjectId: project.projectId,
			versionId,
			contentType: project.favorite.content_type,
			title: project.title,
			iconUrl: project.iconUrl,
			slug: project.slug,
			preferences,
		})
	} catch (error) {
		handleError(error)
	} finally {
		setInstalling(key, false)
	}
}

async function removeFavorite(project: FavoriteProject) {
	try {
		await contentFavorites.remove(project.provider, project.projectId)
	} catch (error) {
		handleError(error)
	}
}

async function selectTargetInstance(target: GameInstance) {
	contentSelection.setTarget(target)
	await contentSelection.refreshInstalledIdentities()
	await router.replace({ query: { ...route.query, i: target.id } })
}

function setDisplayMode(mode: FavoriteDisplayMode) {
	displayMode.value = mode
	setLastBrowseContentDisplayMode(mode)
}

function getProjectLink(project: FavoriteProject) {
	if (project.unavailable) return undefined
	return {
		path:
			project.provider === 'curseforge'
				? `/project/curseforge/${project.projectId}`
				: `/project/${project.slug ?? project.projectId}`,
		query: { ...route.query, b: route.fullPath },
	}
}

onBeforeRouteLeave((to) => {
	if (isBrowseReturnSourcePath(to.path)) {
		const viewport = document.querySelector<HTMLElement>('.app-viewport')
		saveBrowseReturnSnapshot<FavoritesReturnState>({
			url: route.fullPath,
			scrollTop: viewport?.scrollTop ?? 0,
			state: { projects: projects.value },
		})
	}

	breadcrumbs.setContext({
		name: formatMessage(messages.title),
		link: '/browse/favorites',
		query: route.query,
	})
})

watch(
	() =>
		contentFavorites.favorites.value
			.map((favorite) => `${favorite.provider}:${favorite.project_id}`)
			.join('|'),
	() => void refreshProjects(),
)

onMounted(async () => {
	breadcrumbs.setName('FavoritesTitle', formatMessage(messages.title))
	await contentSelection.refreshInstances(queryValue(route.query.i) || undefined)
	if (browseReturnSnapshot) {
		void nextTick().then(
			() =>
				new Promise<void>((resolve) => {
					requestAnimationFrame(() => {
						document.querySelector<HTMLElement>('.app-viewport')?.scrollTo({
							top: browseReturnSnapshot.scrollTop,
						})
						completeBrowseReturnNavigation(route.fullPath)
						resolve()
					})
				}),
		)
	}
	await refreshProjects()
})
</script>

<template>
	<div data-onboarding-id="browse-favorites-content" class="flex flex-col gap-3 p-6">
		<div
			v-if="installContext?.showInstallHeader"
			ref="stickyInstallHeaderRef"
			class="browse-install-header sticky top-0 z-20 -mx-6 -mt-6 rounded-tl-[--radius-xl] border-0 border-b border-solid bg-surface-1 p-3 border-surface-5"
			:class="[isInstallHeaderStuck ? 'border-t' : '']"
		>
			<BrowseInstallHeader :install-context="installContext" />
		</div>

		<div class="flex min-h-12 items-center justify-between">
			<NavTabs :links="projectTabs" />
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<StyledInput
				v-model="search"
				:icon="SearchIcon"
				type="text"
				autocomplete="off"
				clearable
				:placeholder="formatMessage(messages.search)"
				wrapper-class="flex-1"
				input-class="h-12"
			/>
			<ButtonStyled size="standard" type="standard">
				<button class="flex min-w-0 items-center gap-2" @click="instanceSelector?.show()">
					<InstanceIcon
						v-if="contentSelection.targetInstance.value"
						class="shrink-0"
						size="1.25rem"
						:icon-path="contentSelection.targetInstance.value.icon_path"
						:instance-id="contentSelection.targetInstance.value.id"
						:loader="contentSelection.targetInstance.value.loader"
					/>
					<PlusIcon v-else class="size-5 shrink-0" />
					<span class="max-w-40 truncate font-medium">
						{{
							contentSelection.targetInstance.value?.name ?? formatMessage(messages.chooseInstance)
						}}
					</span>
					<span
						aria-hidden="true"
						class="flex size-4 shrink-0 items-center justify-center text-secondary"
					>
						<ChevronDownIcon class="size-4" />
					</span>
				</button>
			</ButtonStyled>
			<PopoutMenu placement="bottom-end">
				<ButtonStyled size="standard" type="standard">
					<button class="flex items-center gap-2">
						<BookmarkIcon class="size-5" />
						<span>{{ currentFavoriteTypeLabel }}</span>
					</button>
				</ButtonStyled>
				<template #menu>
					<div class="flex w-48 flex-col gap-1 p-1">
						<ButtonStyled
							v-for="option in favoriteTypeOptions"
							:key="option.id"
							:type="filter === option.id ? 'filled' : 'transparent'"
						>
							<button
								class="flex w-full !justify-start text-left"
								:aria-pressed="filter === option.id"
								@click="filter = option.id"
							>
								{{ option.label }}
							</button>
						</ButtonStyled>
					</div>
				</template>
			</PopoutMenu>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<span class="text-sm font-medium text-secondary">{{
				formatMessage(messages.recentlySaved)
			}}</span>
			<PopoutMenu :tooltip="formatMessage(messages.view)" placement="bottom-end" class="ml-auto">
				<ButtonStyled circular>
					<button :aria-label="formatMessage(messages.view)">
						<component :is="currentDisplayMode?.icon" />
					</button>
				</ButtonStyled>
				<template #menu>
					<div class="flex w-44 flex-col gap-1 p-1">
						<ButtonStyled
							v-for="option in displayModeOptions"
							:key="option.id"
							:type="displayMode === option.id ? 'filled' : 'transparent'"
						>
							<button
								class="flex w-full items-center gap-2 !justify-start text-left"
								:aria-pressed="displayMode === option.id"
								@click="setDisplayMode(option.id)"
							>
								<component :is="option.icon" class="size-4" />
								{{ option.label }}
							</button>
						</ButtonStyled>
					</div>
				</template>
			</PopoutMenu>
			<Pagination :page="currentPage" :count="pageCount" @switch-page="currentPage = $event" />
		</div>

		<section
			v-if="loadingProjects && availableProjects.length === 0"
			class="flex min-h-64 items-center justify-center"
		>
			<LoadingIndicator />
		</section>
		<EmptyState
			v-else-if="contentFavorites.loaded.value && contentFavorites.favorites.value.length === 0"
			type="empty-inbox"
			:heading="formatMessage(messages.emptyTitle)"
			:description="formatMessage(messages.emptyDescription)"
		/>
		<EmptyState
			v-else-if="filteredProjects.length === 0"
			type="no-search-result"
			:heading="formatMessage(messages.noMatchesTitle)"
			:description="formatMessage(messages.noMatchesDescription)"
		/>
		<ProjectCardList v-else :layout="displayMode">
			<ProjectCard
				v-for="project in pagedProjects"
				:key="`${project.provider}:${project.projectId}`"
				:layout="displayMode"
				:link="getProjectLink(project)"
				:title="project.title"
				:summary="project.description"
				:icon-url="project.iconUrl"
				:downloads="project.downloads"
				:tags="project.categories"
				:date-published="project.dateCreated"
				:date-updated="project.dateModified"
				:banner="project.banner"
				:color="project.color"
				:environment="project.environment"
				:provider="project.provider"
			>
				<template #actions>
					<div class="flex gap-2">
						<ButtonStyled
							v-if="!project.unavailable"
							color="brand"
							type="outlined"
							:size="displayMode === 'compact' ? 'small' : 'standard'"
						>
							<button
								:disabled="isInstalling(project)"
								@click.stop="toggleProjectSelection(project)"
							>
								<SpinnerIcon v-if="isInstalling(project)" class="animate-spin" />
								<CheckIcon v-else-if="isSelected(project)" />
								<PlusIcon v-else />
								{{ installLabel(project) }}
							</button>
						</ButtonStyled>
						<ButtonStyled
							circular
							color="brand"
							type="transparent"
							:size="displayMode === 'compact' ? 'small' : 'standard'"
						>
							<button
								v-tooltip="formatMessage(messages.remove)"
								:disabled="contentFavorites.isPending(project.provider, project.projectId)"
								:aria-label="formatMessage(messages.remove)"
								@click.stop="removeFavorite(project)"
							>
								<SpinnerIcon
									v-if="contentFavorites.isPending(project.provider, project.projectId)"
									class="animate-spin"
								/>
								<BookmarkFilledIcon v-else />
							</button>
						</ButtonStyled>
					</div>
				</template>
			</ProjectCard>
		</ProjectCardList>

		<div class="flex justify-end">
			<Pagination :page="currentPage" :count="pageCount" @switch-page="currentPage = $event" />
		</div>

		<SelectedProjectsFloatingBar :install-context="installContext" />
		<BrowseInstanceSelector
			ref="instanceSelector"
			:instances="contentSelection.instances.value"
			:selected-instance="contentSelection.targetInstance.value"
			:selected-count="contentSelection.selectedCount.value"
			:install-current="contentSelection.installSelected"
			:clear-current="contentSelection.clear"
			@select="selectTargetInstance"
		/>
	</div>
</template>
