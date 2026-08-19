<template>
	<div v-if="loading" class="flex min-h-64 items-center justify-center gap-3 p-6 text-secondary">
		<SpinnerIcon class="animate-spin" />
		{{ formatMessage(messages.loading) }}
	</div>
	<div v-else-if="data">
		<Teleport to="#sidebar-teleport-target">
			<ProjectSidebarCompatibility
				:project="data"
				:tags="{ loaders: allLoaders, gameVersions: allGameVersions }"
				:platform-action="(platform) => browseByProjectFilter('loader', platform)"
				class="project-sidebar-section"
			/>
			<ProjectSidebarLinks
				link-target="_blank"
				:project="data"
				:mcmod-url="mcmodUrl"
				class="project-sidebar-section"
			/>
			<ProjectSidebarTags
				:project="data"
				:tag-action="(tag) => browseByProjectFilter('category', tag)"
				class="project-sidebar-section"
			/>
			<ProjectSidebarCreators
				:members="members"
				:org-link="() => data.links.website_url"
				:user-link="(username) => authorLinks[username] ?? data.links.website_url"
				link-target="_blank"
				class="project-sidebar-section"
			/>
			<ProjectSidebarDetails
				:project="data"
				:has-versions="versions.length > 0"
				hide-license
				link-target="_blank"
				class="project-sidebar-section"
			/>
		</Teleport>

		<div class="flex flex-col gap-4 p-6">
			<Teleport v-if="themeStore.featureFlags.project_background" to="#background-teleport-target">
				<ProjectBackgroundGradient :project="data" />
			</Teleport>
			<ProjectHeader
				:project="data"
				:show-followers="false"
				:translated-title="translationActive ? translations.title : undefined"
				:translated-description="translationActive ? translations.description : undefined"
				:translation-mode="translationMode"
				:translation-style="translationStyle"
			>
				<template #actions>
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
					<ButtonStyled v-if="managedProjectType || isWorldMap" size="large" color="brand">
						<button :disabled="installing || cartProjectInstalling" @click="installSelected(null)">
							<SpinnerIcon v-if="installing" class="animate-spin" />
							<PlusIcon v-else-if="isWorldMap || cartProjectSelected" />
							<DownloadIcon v-else />
							{{
								formatMessage(
									installing || cartProjectInstalling
										? commonMessages.installingLabel
										: cartProjectSelected
											? commonMessages.selectedLabel
										: isWorldMap
											? instanceId
												? commonMessages.installButton
												: messages.addToAnInstance
											: commonMessages.installButton,
								)
							}}
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="data.site_url || mcmodUrl" size="large" circular type="transparent">
						<OverflowMenu
							:tooltip="formatMessage(commonMessages.moreOptionsButton)"
							:options="[
								...(data.site_url
									? [
											{
												id: 'open-in-browser',
												link: data.site_url,
												external: true,
											},
										]
									: []),
								...(mcmodUrl
									? [
											{
												id: 'open-in-mcmod',
												link: mcmodUrl,
												external: true,
											},
										]
									: []),
							]"
							:aria-label="formatMessage(commonMessages.moreOptionsButton)"
						>
							<MoreVerticalIcon aria-hidden="true" />
							<template #open-in-browser>
								<ExternalIcon /> {{ formatMessage(commonMessages.openInBrowserButton) }}
							</template>
							<template #open-in-mcmod>
								<BookOpenIcon /> {{ formatMessage(messages.openInMcmod) }}
							</template>
						</OverflowMenu>
					</ButtonStyled>
				</template>
			</ProjectHeader>
			<SelectedProjectsFloatingBar
				v-if="cartInstallContext"
				:install-context="cartInstallContext"
			/>
			<BrowseInstanceSelector
				ref="browseInstanceSelector"
				:instances="contentSelection.instances.value"
				:selected-instance="contentSelection.targetInstance.value"
				:selected-count="contentSelection.selectedCount.value"
				:install-current="contentSelection.installSelected"
				:clear-current="contentSelection.clear"
				@select="contentSelection.setTarget"
			/>

			<NavTabs
				:links="[
					{
						label: formatMessage(messages.description),
						href: projectDescriptionHref,
					},
					{
						label: formatMessage(messages.versions),
						href: projectVersionsHref,
					},
					{
						label: formatMessage(messages.gallery),
						href: projectGalleryHref,
						shown: data.gallery.length > 0,
					},
				]"
			/>

			<Gallery v-if="activeTab === 'gallery'" :project="data" />
			<ProjectPageVersions
				v-else-if="activeTab === 'versions'"
				:loaders="allLoaders"
				:game-versions="allGameVersions"
				:versions="versions"
				:project="data"
				:show-environment-column="themeStore.featureFlags.show_version_environment_column"
			>
				<template #actions="{ version }">
					<ButtonStyled circular type="transparent" :color="isWorldMap ? 'brand' : 'green'">
						<button
							v-tooltip="formatMessage(isWorldMap ? messages.addToAnInstance : commonMessages.installButton)"
							:disabled="installing"
							@click.stop="installSelected(version.id)"
						>
							<PlusIcon v-if="isWorldMap" />
							<DownloadIcon v-else />
						</button>
					</ButtonStyled>
				</template>
			</ProjectPageVersions>
			<Card v-else>
				<TranslatedProjectDescription
					v-if="data.body"
					:description="data.body"
					:active="translationActive"
					:translations="translations"
					:mode="translationMode"
					:style="translationStyle"
					format="html"
				/>
				<p v-else class="m-0">{{ data.description }}</p>
			</Card>
		</div>
	</div>
	<div v-else class="p-6">
		<Card>
			<h2>{{ formatMessage(messages.unavailableTitle) }}</h2>
			<p class="mb-0">{{ formatMessage(messages.unavailableDescription) }}</p>
		</Card>
	</div>
</template>

<script setup lang="ts">
import {
	BookOpenIcon,
	DownloadIcon,
	ExternalIcon,
	LanguagesIcon,
	MoreVerticalIcon,
	PlusIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	Card,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NavTabs,
	OverflowMenu,
	ProjectBackgroundGradient,
	ProjectHeader,
	ProjectPageVersions,
	ProjectSidebarCompatibility,
	ProjectSidebarCreators,
	ProjectSidebarDetails,
	ProjectSidebarLinks,
	ProjectSidebarTags,
	SelectedProjectsFloatingBar,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import BrowseInstanceSelector from '@/components/browse/BrowseInstanceSelector.vue'
import TranslatedProjectDescription from '@/components/ui/TranslatedProjectDescription.vue'
import { resolveMcmodUrl } from '@/helpers/content-search'
import {
	type CurseForgeFile,
	type CurseForgeProject,
	getCurseForgeDescription,
	getCurseForgeFiles,
	getCurseForgeImageUrl,
	getCurseForgeProject,
} from '@/helpers/curseforge'
import { createProjectBrowseLocation, type ProjectBrowseFilter } from '@/helpers/project-links'
import { get_game_versions, get_loaders } from '@/helpers/tags'
import {
	getTranslationErrorKind,
	getTranslationSettings,
	prepareDescription,
	translateInBatches as translateContent,
	type TranslationStyle,
	validateTranslatedDescription,
} from '@/helpers/translation'
import i18n from '@/i18n.config'
import { injectContentInstall } from '@/providers/content-install'
import {
	injectContentSelection,
	makeContentSelectionKey,
} from '@/providers/content-selection'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { useTheming } from '@/store/state.js'

import Gallery from './Gallery.vue'

const route = useRoute()
const router = useRouter()
const breadcrumbs = useBreadcrumbs()
const themeStore = useTheming()
const { addNotification, handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const { installCurseForge, installCurseForgeWorld } = injectContentInstall()
const contentSelection = injectContentSelection()

const messages = defineMessages({
	loading: {
		id: 'app.project.curseforge.loading',
		defaultMessage: 'Loading CurseForge project…',
	},
	openInMcmod: {
		id: 'app.project.open-in-mcmod',
		defaultMessage: 'Open in MC Mod',
	},
	description: {
		id: 'project.description.title',
		defaultMessage: 'Description',
	},
	versions: {
		id: 'project.versions.title',
		defaultMessage: 'Versions',
	},
	gallery: {
		id: 'project.gallery.title',
		defaultMessage: 'Gallery',
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
	translationFailed: {
		id: 'app.project.translation.failed',
		defaultMessage: 'Translation failed. The original content was kept. Try again.',
	},
	translationFailedTitle: {
		id: 'app.project.translation.failed-title',
		defaultMessage: 'Translation failed',
	},
	translationRateLimited: {
		id: 'app.translation.error.rate-limited',
		defaultMessage: 'The translation service is temporarily rate limited. Please try again later.',
	},
	translationAuthenticationFailed: {
		id: 'app.translation.error.authentication',
		defaultMessage: 'The translation service could not authenticate. Please try again later.',
	},
	translationContentTooLong: {
		id: 'app.translation.error.content-too-long',
		defaultMessage: 'This content is too long for the selected translation service.',
	},
	translationNetworkFailed: {
		id: 'app.translation.error.network',
		defaultMessage: 'The translation service could not be reached. Check your network or proxy.',
	},
	unavailableTitle: {
		id: 'app.project.curseforge.unavailable-title',
		defaultMessage: 'Project unavailable',
	},
	unavailableDescription: {
		id: 'app.project.curseforge.unavailable-description',
		defaultMessage: 'The CurseForge project did not return any data.',
	},
	addToAnInstance: {
		id: 'app.browse.add-to-an-instance',
		defaultMessage: 'Add to an instance',
	},
	noCompatibleVersion: {
		id: 'app.project.install-button.no-compatible-version',
		defaultMessage: 'No compatible version was found for this instance.',
	},
})

const loading = ref(true)
const installing = ref(false)
const browseInstanceSelector = ref()
const project = shallowRef<CurseForgeProject | null>(null)
const mcmodUrl = ref<string | null>(null)
const description = ref('')
const files = shallowRef<CurseForgeFile[]>([])
const allLoaders = ref([])
const allGameVersions = ref([])
const translationActive = ref(false)
const translationLoading = ref(false)
const translations = ref<Record<string, string>>({})
const translationMode = ref<'bilingual' | 'translation-only'>('bilingual')
const translationStyle = ref<TranslationStyle>('weakened')
let projectRequestVersion = 0
let translationRequestVersion = 0

const projectType = computed(() => {
	switch (project.value?.classId) {
		case 5:
			return 'plugin'
		case 6:
			return 'mod'
		case 12:
			return 'resourcepack'
		case 17:
			return 'world'
		case 6945:
			return 'datapack'
		case 4471:
			return 'modpack'
		case 6552:
			return 'shader'
		default:
			return 'mod'
	}
})

const managedProjectType = computed(() =>
	['mod', 'resourcepack', 'shader', 'datapack', 'modpack'].includes(projectType.value),
)
const isWorldMap = computed(() => projectType.value === 'world')
const instanceId = computed(() => (typeof route.query.i === 'string' ? route.query.i : null))
const fromBrowse = computed(
	() => typeof route.query.b === 'string' && route.query.b.startsWith('/browse/'),
)
const cartEligible = computed(
	() =>
		fromBrowse.value &&
		['mod', 'resourcepack', 'shader', 'datapack', 'world'].includes(projectType.value),
)
const cartProjectKey = computed(() =>
	project.value ? makeContentSelectionKey('curseforge', project.value.id.toString()) : '',
)
const cartProjectSelected = computed(
	() => !!cartProjectKey.value && contentSelection.isSelected(cartProjectKey.value),
)
const cartProjectInstalling = computed(
	() => !!cartProjectKey.value && contentSelection.isInstalling(cartProjectKey.value),
)
const cartInstallContext = computed(() => {
	const target = contentSelection.targetInstance.value
	if (!cartEligible.value || !target) return null
	return {
		showInstallHeader: false,
		name: target.name,
		loader: target.loader,
		gameVersion: target.game_version,
		backUrl: typeof route.query.b === 'string' ? route.query.b : `/browse/${projectType.value}`,
		backLabel: '',
		heading: '',
		selectedProjects: contentSelection.selectedProjects.value,
		isInstallingSelected: ['validating', 'reviewing', 'queueing'].includes(
			contentSelection.state.value,
		),
		installProgress: contentSelection.progress.value,
		clearSelected: contentSelection.clear,
		installSelected: contentSelection.installSelected,
	}
})

const platformNames = [
	'forge',
	'fabric',
	'quilt',
	'neoforge',
	'liteloader',
	'rift',
	'iris',
	'optifine',
]
const loaderTypes: Record<number, string> = { 1: 'forge', 4: 'fabric', 5: 'quilt', 6: 'neoforge' }

function getFilePlatforms(file: CurseForgeFile) {
	const platforms = file.gameVersions
		.map((version) => version.toLowerCase().replaceAll(' ', ''))
		.filter((version) => platformNames.includes(version))

	if (platforms.length === 0 && projectType.value === 'resourcepack') {
		return ['minecraft']
	}

	return platforms
}

const projectLoaders = computed(() => {
	const loaders = new Set<string>()
	for (const file of files.value) {
		for (const platform of getFilePlatforms(file)) loaders.add(platform)
	}
	for (const index of project.value?.latestFilesIndexes ?? []) {
		if (index.modLoader && loaderTypes[index.modLoader]) loaders.add(loaderTypes[index.modLoader])
	}
	return [...loaders]
})

const minecraftVersions = computed(() => {
	const versions = new Set<string>()
	for (const file of files.value) {
		for (const version of file.gameVersions) {
			if (/^\d+\.\d+/.test(version)) versions.add(version)
		}
	}
	for (const index of project.value?.latestFilesIndexes ?? []) {
		if (/^\d+\.\d+/.test(index.gameVersion)) versions.add(index.gameVersion)
	}
	return [...versions]
})

const data = computed(() => {
	if (!project.value) return null
	const value = project.value
	return {
		id: value.id.toString(),
		slug: value.slug,
		title: value.name,
		description: value.summary,
		body: description.value,
		project_type: projectType.value,
		downloads: value.downloadCount,
		followers: 0,
		icon_url: getCurseForgeImageUrl(value.logo?.thumbnailUrl),
		color: null,
		status: 'approved',
		categories: value.categories.map((category) => category.slug),
		additional_categories: [],
		versions: files.value.map((file) => file.id.toString()),
		game_versions: minecraftVersions.value,
		loaders: projectLoaders.value,
		client_side: 'unknown',
		server_side: 'unknown',
		published: value.dateCreated,
		approved: value.dateReleased || value.dateCreated,
		updated: value.dateModified,
		queued: null,
		license: { id: 'LicenseRef-Unknown', name: 'Unknown', url: null },
		issues_url: value.links.issuesUrl ?? '',
		source_url: value.links.sourceUrl ?? '',
		wiki_url: value.links.wikiUrl ?? '',
		discord_url: '',
		site_url: value.links.websiteUrl ?? '',
		donation_urls: [],
		links: {
			website_url: value.links.websiteUrl ?? '',
		},
		gallery: value.screenshots.map((screenshot) => ({
			title: screenshot.title,
			description: '',
			created: value.dateModified,
			url: getCurseForgeImageUrl(screenshot.thumbnailUrl, 960),
			raw_url: getCurseForgeImageUrl(screenshot.url, 1920),
			featured: false,
		})),
	}
})

function browseByProjectFilter(filter: ProjectBrowseFilter, value: string) {
	if (!data.value?.project_type) return
	void router.push(createProjectBrowseLocation(data.value.project_type, filter, value))
}

const members = computed(() =>
	(project.value?.authors ?? []).map((author, index) => ({
		id: author.id.toString(),
		role: index === 0 ? 'Owner' : 'Author',
		is_owner: index === 0,
		accepted: true,
		user: {
			id: author.id.toString(),
			username: author.name,
			avatar_url: '',
		},
	})),
)

const authorLinks = computed(() =>
	Object.fromEntries((project.value?.authors ?? []).map((author) => [author.name, author.url])),
)

const versions = computed(() =>
	files.value.map((file) => {
		const loaders = getFilePlatforms(file)
		const gameVersions = file.gameVersions.filter((version) => /^\d+\.\d+/.test(version))
		return {
			id: file.id.toString(),
			project_id: project.value?.id.toString() ?? '',
			name: file.displayName,
			version_number: file.displayName,
			version_type: file.releaseType === 1 ? 'release' : file.releaseType === 2 ? 'beta' : 'alpha',
			date_published: file.fileDate,
			downloads: file.downloadCount,
			game_versions: gameVersions,
			loaders: loaders.length ? loaders : projectLoaders.value,
			files: [
				{
					filename: file.fileName,
					size: file.fileLength,
					url: file.downloadUrl ?? '',
					primary: true,
					hashes: {},
				},
			],
			featured: false,
			status: 'listed',
			changelog: '',
			dependencies: [],
			displayUrlEnding: file.id.toString(),
		}
	}),
)

const activeTab = computed(() => {
	if (route.path.endsWith('/versions')) return 'versions'
	if (route.path.endsWith('/gallery')) return 'gallery'
	return 'description'
})

function buildProjectHref(path: string) {
	const params = new URLSearchParams()
	for (const [key, value] of Object.entries(route.query)) {
		if (Array.isArray(value)) {
			for (const item of value) if (item) params.append(key, String(item))
		} else if (value) {
			params.append(key, String(value))
		}
	}
	const query = params.toString()
	return query ? `${path}?${query}` : path
}

const projectDescriptionHref = computed(() =>
	buildProjectHref(`/project/curseforge/${route.params.id}`),
)
const projectVersionsHref = computed(() =>
	buildProjectHref(`/project/curseforge/${route.params.id}/versions`),
)
const projectGalleryHref = computed(() =>
	buildProjectHref(`/project/curseforge/${route.params.id}/gallery`),
)

async function loadProject(projectId: number) {
	const requestVersion = ++projectRequestVersion
	translationRequestVersion++
	translationActive.value = false
	translationLoading.value = false
	translations.value = {}
	loading.value = true
	project.value = null
	mcmodUrl.value = null
	description.value = ''
	files.value = []
	allLoaders.value = []
	allGameVersions.value = []

	try {
		const supplementaryData = Promise.allSettled([
			getCurseForgeDescription(projectId),
			getCurseForgeFiles(projectId, { index: 0, pageSize: 50 }),
			get_loaders(),
			get_game_versions(),
		])
		const projectData = await getCurseForgeProject(projectId)
		if (requestVersion !== projectRequestVersion) return
		project.value = projectData
		breadcrumbs.setName('Project', projectData.name)
		loading.value = false
		void resolveMcmodUrl(projectData.slug, 'curseforge').then((url) => {
			if (requestVersion === projectRequestVersion) mcmodUrl.value = url
		})

		const [projectDescription, projectFiles, loaders, gameVersions] = await supplementaryData
		if (requestVersion !== projectRequestVersion) return
		if (projectDescription.status === 'fulfilled') {
			description.value = projectDescription.value
		} else {
			handleError(projectDescription.reason)
		}
		if (projectFiles.status === 'fulfilled') {
			files.value = projectFiles.value.files
		} else {
			handleError(projectFiles.reason)
		}
		if (loaders.status === 'fulfilled') allLoaders.value = loaders.value
		if (gameVersions.status === 'fulfilled') allGameVersions.value = gameVersions.value
		void maybeAutoTranslate()
	} catch (error) {
		if (requestVersion === projectRequestVersion) handleError(error)
	} finally {
		if (requestVersion === projectRequestVersion) loading.value = false
	}
}

watch(
	() => Number(route.params.id),
	(projectId) => {
		if (Number.isFinite(projectId)) void loadProject(projectId)
	},
	{ immediate: true },
)

watch(
	[fromBrowse, instanceId],
	async ([enabled, preferredInstanceId]) => {
		if (enabled) await contentSelection.refreshInstances(preferredInstanceId)
	},
	{ immediate: true },
)

async function installSelected(fileId: string | null) {
	if (!project.value) return
	if (cartEligible.value && !contentSelection.targetInstance.value) {
		await contentSelection.refreshInstances()
		browseInstanceSelector.value?.show()
		return
	}
	if (cartEligible.value && contentSelection.targetInstance.value) {
		if (cartProjectSelected.value) {
			contentSelection.remove(cartProjectKey.value)
			return
		}
		const target = contentSelection.targetInstance.value
		const expectedLoader = { forge: 1, fabric: 4, quilt: 5, neoforge: 6 }[target.loader]
		const resolvedFileId =
			fileId ??
			project.value.latestFilesIndexes.find(
				(index) =>
					index.gameVersion === target.game_version &&
					(projectType.value !== 'mod' ||
						!expectedLoader ||
						index.modLoader === expectedLoader),
			)?.fileId ??
			files.value.find(
				(file) =>
					file.gameVersions.includes(target.game_version) &&
					(projectType.value !== 'mod' ||
						getFilePlatforms(file).includes(target.loader)),
			)?.id
		if (!resolvedFileId) {
			handleError(new Error(formatMessage(messages.noCompatibleVersion)))
			return
		}
		await contentSelection.add({
			key: cartProjectKey.value,
			provider: 'curseforge',
			projectId: project.value.id.toString(),
			providerProjectId: project.value.id.toString(),
			versionId: resolvedFileId.toString(),
			contentType: projectType.value as 'mod' | 'resourcepack' | 'datapack' | 'shader' | 'world',
			title: project.value.name,
			iconUrl: getCurseForgeImageUrl(project.value.logo?.thumbnailUrl),
			slug: project.value.slug,
			preferences: {
				gameVersions: [target.game_version],
				loaders: projectType.value === 'mod' ? [target.loader] : [],
			},
		})
		return
	}
	if (isWorldMap.value) {
		installing.value = true
		await installCurseForgeWorld(
			project.value.id,
			fileId,
			instanceId.value,
			'ProjectPage',
			() => {
				installing.value = false
			},
		).catch((error) => {
			installing.value = false
			handleError(error)
		})
		return
	}

	installing.value = true
	await installCurseForge(
		project.value.id.toString(),
		fileId,
		instanceId.value,
		'ProjectPage',
		() => {
			installing.value = false
		},
		(instanceId) => {
			router.push(`/instance/${instanceId}`)
		},
	).catch((error) => {
		installing.value = false
		handleError(error)
	})
}

function translationFailureMessage(error: unknown) {
	return formatMessage(
		{
			'rate-limited': messages.translationRateLimited,
			authentication: messages.translationAuthenticationFailed,
			'content-too-long': messages.translationContentTooLong,
			network: messages.translationNetworkFailed,
			provider: messages.translationFailed,
		}[getTranslationErrorKind(error)],
	)
}

async function translateProject() {
	if (!data.value || translationLoading.value) return
	const requestVersion = ++translationRequestVersion
	const previousTranslationActive = translationActive.value
	const previousTranslations = translations.value
	translationLoading.value = true

	try {
		const settings = await getTranslationSettings()
		translationMode.value = settings.mode
		translationStyle.value = settings.style
		const prepared = prepareDescription(data.value.body ?? '', 'html')
		const targetLanguage = settings.target_language || i18n.global.locale.value || 'en-US'
		const baseRequest = {
			source_language: 'auto',
			target_language: targetLanguage,
			context: {
				title: data.value.title,
				description: data.value.description,
			},
		}
		const allSegments = [
			{ id: 'title', text: data.value.title, format: 'plain' },
			{ id: 'description', text: data.value.description, format: 'plain' },
			...prepared.segments,
		]

		translationActive.value = true
		const accumulated = { ...translations.value }
		await translateContent({ ...baseRequest, segments: allSegments }, (response) => {
			if (requestVersion !== translationRequestVersion) return
			for (const segment of response.segments) accumulated[segment.id] = segment.text
			translations.value = { ...accumulated }
		})
		if (requestVersion !== translationRequestVersion) return

		validateTranslatedDescription(prepared, accumulated)
	} catch (error) {
		if (requestVersion === translationRequestVersion) {
			translationActive.value = previousTranslationActive
			translations.value = previousTranslations
			addNotification({
				title: formatMessage(messages.translationFailedTitle),
				text: translationFailureMessage(error),
				type: 'error',
			})
		}
	} finally {
		if (requestVersion === translationRequestVersion) translationLoading.value = false
	}
}

async function maybeAutoTranslate() {
	try {
		const settings = await getTranslationSettings()
		if (settings.auto_translate) await translateProject()
	} catch (error) {
		handleError(error)
	}
}

function toggleTranslation() {
	if (translationActive.value) {
		translationRequestVersion++
		translationActive.value = false
		translationLoading.value = false
		return
	}
	void translateProject()
}
</script>

<style scoped>
.project-sidebar-section {
	@apply p-4 flex flex-col gap-2 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid;
}
</style>
