import type { Labrinth } from '@modrinth/api-client'
import type { ContentInstallInstance, ContentInstallProjectInfo, ContentItem } from '@modrinth/ui'
import { createContext, defineMessage, useVIntl } from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import { nextTick, type Ref, ref } from 'vue'
import type { Router } from 'vue-router'

import { trackEvent } from '@/helpers/analytics'
import {
	get_organization,
	get_project,
	get_project_many,
	get_team,
	get_version_many,
} from '@/helpers/cache.js'
import {
	type CurseForgeFile,
	type CurseForgeInstallResult,
	type CurseForgeProject,
	getCurseForgeFiles,
	getCurseForgeProject,
	installCurseForgeFile,
	installCurseForgeModpack,
	summarizeCurseForgeInstall,
} from '@/helpers/curseforge'
import {
	type CurseForgeManualDownloadItem,
	setCurseForgeManualDownloads,
} from '@/helpers/curseforge-manual'
import { instance_listener } from '@/helpers/events.js'
import {
	install_create_instance,
	install_create_modpack_instance,
	installJobInstanceId,
	wait_for_install_job,
} from '@/helpers/install'
import {
	add_project_from_version,
	edit,
	get,
	get_content_items,
	get_install_candidates,
	get_projects,
	install_project_with_dependencies,
	list,
	remove_project,
	type ResolveContentPlan,
} from '@/helpers/instance'
import { isBuiltInInstanceIcon } from '@/helpers/instance-icon-frame'
import { get_game_versions } from '@/helpers/tags'
import type { GameInstance, InstanceLoader } from '@/helpers/types'
import { useTheming } from '@/store/state'
interface ModalRef {
	show: (initialVersionId?: string) => void
	hide: () => void
}

interface ModpackAlreadyInstalledModalRef {
	show: (instanceName: string, instanceId: string) => void
}

interface CurseForgeManualDownloadsModalRef {
	show: (payload: {
		items: CurseForgeManualDownloadItem[]
		installed: number
		instanceId?: string | null
	}) => void
}

export type ContentInstallCallback = (versionId?: string, installedProjectIds?: string[]) => void
type InstallingProjectDisplay = {
	id?: string
	slug?: string | null
	title?: string
	name?: string
	icon_url?: string | null
	project_type?: string
	type?: string
	organization?: string | null
	team?: string
}
type ContentInstallInstanceEvent = {
	event: string
	instance_id: string
	project_ids?: string[]
	message?: string
}

const LOADER_ORDER = ['vanilla', 'fabric', 'quilt', 'neoforge', 'forge']
const SUPPORTED_LOADERS: Set<string> = new Set(['vanilla', 'forge', 'fabric', 'quilt', 'neoforge'])
const VANILLA_COMPATIBLE_LOADERS: Set<string> = new Set(['minecraft', 'datapack'])
type InstallProvider = 'modrinth' | 'curseforge'
const noCompatibleVersionsMessage = defineMessage({
	id: 'app.content-install.no-compatible-versions',
	defaultMessage:
		'No available versions match {compatibilityLabel}. Select a version to install anyway. Dependencies will not be installed automatically.',
})
const manualDownloadsTitleMessage = defineMessage({
	id: 'app.curseforge.manual-downloads.notification-title',
	defaultMessage: 'Some CurseForge files need manual download',
})
const manualDownloadsPartialMessage = defineMessage({
	id: 'app.curseforge.manual-downloads.notification-partial',
	defaultMessage:
		'Installed {installed, number} files automatically, but {manual, number} could not be downloaded ({list}). Open the download list to finish those files.',
})
const manualDownloadsFailedMessage = defineMessage({
	id: 'app.curseforge.manual-downloads.notification-failed',
	defaultMessage:
		'{manual, number} CurseForge files could not be downloaded automatically ({list}). Open the download list to install them manually.',
})
const manualDownloadsListAndMoreMessage = defineMessage({
	id: 'app.curseforge.manual-downloads.list-and-more',
	defaultMessage: '{list}, and {count, number} more',
})
const manualDownloadsFilesCountMessage = defineMessage({
	id: 'app.curseforge.manual-downloads.files-count',
	defaultMessage: '{count, number} files',
})
const modpackInstalledTitleMessage = defineMessage({
	id: 'app.curseforge.modpack-installed.title',
	defaultMessage: 'CurseForge modpack installed',
})
const modpackInstalledBodyMessage = defineMessage({
	id: 'app.curseforge.modpack-installed.body',
	defaultMessage: 'Installed {count, number} content files from CurseForge.',
})

const RESOLVABLE_PROJECT_TYPES = new Set<Labrinth.Content.v3.ContentType>([
	'mod',
	'plugin',
	'datapack',
	'resourcepack',
	'shader',
	'modpack',
])

function resolveContentType(projectType?: Labrinth.Projects.v2.ProjectType) {
	return projectType && RESOLVABLE_PROJECT_TYPES.has(projectType) ? projectType : 'mod'
}

function isVersionCompatible(
	version: Labrinth.Versions.v2.Version,
	project: Labrinth.Projects.v2.Project,
	instance: GameInstance,
) {
	return (
		version.game_versions.includes(instance.game_version) &&
		(project.project_type === 'mod'
			? version.loaders.includes(instance.loader) || version.loaders.includes('datapack')
			: true)
	)
}

function findPreferredVersion(
	versions: Labrinth.Versions.v2.Version[],
	project: Labrinth.Projects.v2.Project,
	instance: GameInstance,
) {
	const projectType = project.project_type ?? 'mod'

	return (
		versions.find(
			(v) =>
				v.game_versions.includes(instance.game_version) &&
				(projectType === 'mod' ? v.loaders.includes(instance.loader) : true),
		) ?? versions.find((v) => isVersionCompatible(v, project, instance))
	)
}

function sortLoaders(loaders: string[]): string[] {
	return loaders.slice().sort((a, b) => {
		const aIdx = LOADER_ORDER.indexOf(a)
		const bIdx = LOADER_ORDER.indexOf(b)
		if (aIdx === -1 && bIdx === -1) return a.localeCompare(b)
		if (aIdx === -1) return 1
		if (bIdx === -1) return -1
		return aIdx - bIdx
	})
}

function curseForgeProjectType(classId?: number): Labrinth.Projects.v2.ProjectType {
	switch (classId) {
		case 5:
			return 'plugin'
		case 12:
			return 'resourcepack'
		case 6945:
			return 'datapack'
		case 4471:
			return 'modpack'
		case 6552:
			return 'shader'
		default:
			return 'mod'
	}
}

function curseForgeLoader(value: string): string | null {
	switch (value.toLowerCase().replaceAll(' ', '')) {
		case 'forge':
			return 'forge'
		case 'fabric':
		case 'fabricloader':
			return 'fabric'
		case 'quilt':
			return 'quilt'
		case 'neoforge':
			return 'neoforge'
		default:
			return null
	}
}

function curseForgeGameVersions(file: CurseForgeFile): string[] {
	return file.gameVersions.filter(
		(value) =>
			!curseForgeLoader(value) &&
			(/^(?:\d+\.\d+(?:\.\d+)?(?:-(?:pre|rc)\d+)?|\d{2}w\d{2}[a-z])$/i.test(value) ||
				value.toLowerCase().includes('snapshot')),
	)
}

function mapCurseForgeVersion(
	file: CurseForgeFile,
	projectId: number,
	projectType: Labrinth.Projects.v2.ProjectType,
): Labrinth.Versions.v2.Version {
	const loaders = [...new Set(file.gameVersions.map(curseForgeLoader).filter(Boolean))] as string[]
	return {
		id: file.id.toString(),
		project_id: `curseforge:${projectId}`,
		name: file.displayName,
		version_number: file.displayName,
		game_versions: curseForgeGameVersions(file),
		loaders:
			loaders.length > 0 && (projectType === 'mod' || projectType === 'modpack')
				? loaders
				: ['minecraft'],
		date_published: file.fileDate,
		version_type: file.releaseType === 1 ? 'release' : file.releaseType === 2 ? 'beta' : 'alpha',
		files: [
			{
				filename: file.fileName,
				url: file.downloadUrl ?? '',
				primary: true,
				size: file.fileLength,
				hashes: {},
			},
		],
	} as unknown as Labrinth.Versions.v2.Version
}

function mapCurseForgeProject(
	project: CurseForgeProject,
	files: CurseForgeFile[],
): Labrinth.Projects.v2.Project {
	const projectType = curseForgeProjectType(project.classId)
	const versions = files.map((file) => mapCurseForgeVersion(file, project.id, projectType))
	return {
		id: `curseforge:${project.id}`,
		slug: project.slug,
		title: project.name,
		description: project.summary,
		project_type: projectType,
		icon_url: project.logo?.thumbnailUrl ?? project.logo?.url ?? null,
		versions: versions.map((version) => version.id),
		game_versions: [...new Set(versions.flatMap((version) => version.game_versions))],
		loaders: [...new Set(versions.flatMap((version) => version.loaders))],
		organization: null,
		team: '',
	} as unknown as Labrinth.Projects.v2.Project
}

function curseForgeLoaderType(loader: string): number | undefined {
	switch (loader) {
		case 'forge':
			return 1
		case 'fabric':
			return 4
		case 'quilt':
			return 5
		case 'neoforge':
			return 6
		default:
			return undefined
	}
}

type InstallTargetInstance = Pick<
	GameInstance,
	'id' | 'name' | 'icon_path' | 'game_version' | 'loader'
>

export interface ContentInstallContext {
	instances: Ref<ContentInstallInstance[]>
	compatibleLoaders: Ref<string[]>
	gameVersions: Ref<string[]>
	loading: Ref<boolean>
	defaultTab: Ref<'existing' | 'new'>
	preferredLoader: Ref<string | null>
	preferredGameVersion: Ref<string | null>
	releaseGameVersions: Ref<Set<string>>
	projectInfo: Ref<ContentInstallProjectInfo | null>
	handleInstallToInstance: (instance: ContentInstallInstance) => Promise<void>
	handleCreateAndInstall: (data: {
		name: string
		iconPath: string | null
		iconPreviewUrl: string | null
		loader: string
		gameVersion: string
	}) => Promise<void>
	handleNavigate: (instance: ContentInstallInstance) => void
	handleCancel: () => void
	setContentInstallModal: (ref: ModalRef) => void
	setModpackAlreadyInstalledModal: (ref: ModpackAlreadyInstalledModalRef) => void
	handleModpackDuplicateCreateAnyway: () => Promise<void>
	handleModpackDuplicateGoToInstance: (instanceId: string) => void
	setCurseForgeManualDownloadsModal: (ref: CurseForgeManualDownloadsModalRef) => void
	setIncompatibilityWarningModal: (ref: ModalRef) => void
	incompatibilityWarningVersions: Ref<Labrinth.Versions.v2.Version[]>
	incompatibilityWarningCurrentGameVersion: Ref<string>
	incompatibilityWarningCurrentLoader: Ref<string>
	incompatibilityWarningProjectType: Ref<string | undefined>
	incompatibilityWarningProjectIconUrl: Ref<string | undefined>
	incompatibilityWarningProjectName: Ref<string | undefined>
	incompatibilityWarningMessage: Ref<string | undefined>
	incompatibilityWarningInstalling: Ref<boolean>
	handleIncompatibilityWarningInstall: (version: Labrinth.Versions.v2.Version) => Promise<void>
	handleIncompatibilityWarningCancel: () => void
	install: (
		projectId: string,
		versionId?: string | null,
		instanceId?: string | null,
		source?: string,
		callback?: ContentInstallCallback,
		createInstanceCallback?: (instanceId: string) => void,
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) => Promise<void>
	installCurseForge: (
		projectId: string,
		versionId?: string | null,
		instanceId?: string | null,
		source?: string,
		callback?: ContentInstallCallback,
		createInstanceCallback?: (instanceId: string) => void,
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) => Promise<void>
	installingItems: Ref<Map<string, ContentItem[]>>
	pendingManualDownloadsByInstance: Ref<Map<string, CurseForgeManualDownloadItem[]>>
	installRevisionByInstance: Ref<Map<string, number>>
	installFailureRevisionByInstance: Ref<Map<string, number>>
}

export const [injectContentInstall, provideContentInstall] = createContext<ContentInstallContext>(
	'root',
	'contentInstall',
)

export function createContentInstall(opts: {
	router: Router
	handleError: (err: unknown) => void
	addNotification: (notification: {
		title: string
		text?: string
		type?: 'error' | 'warning' | 'success' | 'info'
	}) => void
}): ContentInstallContext {
	const { formatMessage } = useVIntl()
	const themeStore = useTheming()
	const instances = ref<ContentInstallInstance[]>([])
	const compatibleLoaders = ref<string[]>([])
	const gameVersions = ref<string[]>([])
	const loading = ref(false)
	const defaultTab = ref<'existing' | 'new'>('existing')
	const preferredLoader = ref<string | null>(null)
	const preferredGameVersion = ref<string | null>(null)
	const releaseGameVersions = ref<Set<string>>(new Set())

	const projectInfo = ref<ContentInstallProjectInfo | null>(null)
	const installingItems = ref<Map<string, ContentItem[]>>(new Map())
	const pendingManualDownloadsByInstance = ref<Map<string, CurseForgeManualDownloadItem[]>>(
		new Map(),
	)
	const installRevisionByInstance = ref<Map<string, number>>(new Map())
	const installFailureRevisionByInstance = ref<Map<string, number>>(new Map())
	const incompatibilityWarningVersions = ref<Labrinth.Versions.v2.Version[]>([])
	const incompatibilityWarningCurrentGameVersion = ref('')
	const incompatibilityWarningCurrentLoader = ref('')
	const incompatibilityWarningProjectType = ref<string | undefined>(undefined)
	const incompatibilityWarningProjectIconUrl = ref<string | undefined>(undefined)
	const incompatibilityWarningProjectName = ref<string | undefined>(undefined)
	const incompatibilityWarningMessage = ref<string | undefined>(undefined)
	const incompatibilityWarningInstalling = ref(false)

	function addInstallingItem(
		instanceId: string,
		project: {
			id: string
			slug?: string | null
			title: string
			icon_url?: string | null
			project_type?: string
			organization?: string | null
			team?: string
		},
		version?: Labrinth.Versions.v2.Version,
	) {
		const primaryFile = version?.files?.find((f) => f.primary) ?? version?.files?.[0]
		const placeholder: ContentItem = {
			id: `__installing_${project.id}`,
			file_name: `__installing_${project.id}`,
			project: {
				id: project.id,
				slug: project.slug ?? '',
				title: project.title,
				icon_url: project.icon_url ?? undefined,
			},
			version: version
				? {
						id: version.id,
						version_number: version.version_number,
						file_name: primaryFile?.filename ?? '',
					}
				: undefined,
			project_type: project.project_type ?? 'mod',
			has_update: false,
			update_version_id: null,
			enabled: true,
			installing: true,
		}
		const next = new Map(installingItems.value)
		const items = next.get(instanceId) ?? []
		if (items.some((i) => i.file_name === placeholder.file_name)) return
		next.set(instanceId, [...items, placeholder])
		installingItems.value = next

		if (project.organization) {
			get_organization(project.organization)
				.then((org: { id: string; slug: string; name: string; icon_url?: string }) => {
					updateInstallingItem(instanceId, placeholder.file_name, {
						owner: {
							id: org.id,
							name: org.name,
							avatar_url: org.icon_url,
							type: 'organization',
						},
					})
				})
				.catch(() => {})
		} else if (project.team) {
			get_team(project.team)
				.then(
					(
						members: {
							user: { id: string; username: string; avatar_url?: string }
							is_owner: boolean
						}[],
					) => {
						const owner = members.find((m) => m.is_owner)
						if (owner) {
							updateInstallingItem(instanceId, placeholder.file_name, {
								owner: {
									id: owner.user.id,
									name: owner.user.username,
									avatar_url: owner.user.avatar_url,
									type: 'user',
								},
							})
						}
					},
				)
				.catch(() => {})
		}
	}

	function updateInstallingItem(
		instanceId: string,
		fileName: string,
		updates: Partial<ContentItem>,
	) {
		const next = new Map(installingItems.value)
		const items = next.get(instanceId)
		if (!items) return
		const index = items.findIndex((i) => i.file_name === fileName)
		if (index === -1) return
		const updated = [...items]
		updated[index] = { ...updated[index], ...updates }
		next.set(instanceId, updated)
		installingItems.value = next
	}

	function removeInstallingItems(instanceId: string, projectIds: string[]) {
		const next = new Map(installingItems.value)
		const items = next.get(instanceId)
		if (items) {
			const idsToRemove = new Set(projectIds.map((id) => `__installing_${id}`))
			const filtered = items.filter((i) => !idsToRemove.has(i.file_name))
			if (filtered.length > 0) {
				next.set(instanceId, filtered)
			} else {
				next.delete(instanceId)
			}
			installingItems.value = next
		}
	}

	function resolvedProjectIds(plan: ResolveContentPlan) {
		return [
			plan.primary.project_id,
			...plan.dependencies.map((dependency) => dependency.project_id),
		]
	}

	async function addInstallingItemsForPlan(
		instanceId: string,
		plan: ResolveContentPlan,
		primaryProject: Labrinth.Projects.v2.Project,
		primaryVersion: Labrinth.Versions.v2.Version,
	) {
		const entries = [plan.primary, ...plan.dependencies]
		const projectIds = [...new Set(entries.map((entry) => entry.project_id))]
		const versionIds = [...new Set(entries.map((entry) => entry.version_id))]
		const projectMap = new Map<string, InstallingProjectDisplay>([
			[primaryProject.id, primaryProject],
		])
		const versionMap = new Map<string, Labrinth.Versions.v2.Version>([
			[primaryVersion.id, primaryVersion],
		])

		const [projects, versions] = await Promise.all([
			get_project_many(projectIds, 'bypass').catch(() => []),
			get_version_many(versionIds, 'bypass').catch(() => []),
		])

		for (const project of projects as InstallingProjectDisplay[]) {
			if (project?.id) projectMap.set(project.id, project)
		}
		for (const version of versions as Labrinth.Versions.v2.Version[]) {
			if (version?.id) versionMap.set(version.id, version)
		}

		for (const entry of entries) {
			const project = projectMap.get(entry.project_id)
			const version = versionMap.get(entry.version_id)
			addInstallingItem(
				instanceId,
				{
					id: entry.project_id,
					slug: project?.slug ?? entry.project_id,
					title: project?.title ?? project?.name ?? entry.project_id,
					icon_url: project?.icon_url ?? null,
					project_type: project?.project_type ?? project?.type ?? primaryProject.project_type,
					organization: project?.organization ?? null,
					team: project?.team,
				},
				version,
			)
		}
	}

	function markInstanceContentChanged(instanceId: string) {
		const next = new Map(installRevisionByInstance.value)
		next.set(instanceId, (next.get(instanceId) ?? 0) + 1)
		installRevisionByInstance.value = next
	}

	function markInstanceContentInstallFailed(instanceId: string) {
		const next = new Map(installFailureRevisionByInstance.value)
		next.set(instanceId, (next.get(instanceId) ?? 0) + 1)
		installFailureRevisionByInstance.value = next
	}

	void instance_listener((event: ContentInstallInstanceEvent) => {
		if (event.event === 'content_install_finished') {
			markInstanceContentChanged(event.instance_id)
			removeInstallingItems(event.instance_id, event.project_ids ?? [])
		} else if (event.event === 'content_install_failed') {
			removeInstallingItems(event.instance_id, event.project_ids ?? [])
			markInstanceContentInstallFailed(event.instance_id)
			markInstanceContentChanged(event.instance_id)
			opts.handleError(event.message ?? 'Failed to install content')
		}
	}).catch(opts.handleError)

	let modalRef: ModalRef | null = null
	let modpackAlreadyInstalledModalRef: ModpackAlreadyInstalledModalRef | null = null
	let curseForgeManualDownloadsModalRef: CurseForgeManualDownloadsModalRef | null = null
	let incompatibilityWarningModalRef: ModalRef | null = null
	let currentProvider: InstallProvider = 'modrinth'
	let currentProject: Labrinth.Projects.v2.Project | null = null
	let currentVersions: Labrinth.Versions.v2.Version[] = []
	let currentCurseForgeProject: CurseForgeProject | null = null
	let currentCurseForgeFiles = new Map<string, CurseForgeFile>()
	let currentCallback: ContentInstallCallback = () => {}
	let instanceMap: Record<string, InstallTargetInstance> = {}
	let incompatibilityWarningInstance: InstallTargetInstance | null = null
	let incompatibilityWarningProject: Labrinth.Projects.v2.Project | null = null
	let incompatibilityWarningCallback: ContentInstallCallback = () => {}
	let incompatibilityWarningInstalled = false

	let pendingModpackInstall: {
		project: Labrinth.Projects.v2.Project
		version: string
		source: string
		callback: ContentInstallCallback
		createInstanceCallback: (instanceId: string) => void
		provider: InstallProvider
	} | null = null

	async function createAndInstallCurseForgeModpack(
		project: Labrinth.Projects.v2.Project,
		version: Labrinth.Versions.v2.Version,
		numericProjectId: number,
		gameVersion: string,
		loader: InstanceLoader,
		source: string,
		callback: ContentInstallCallback,
		createInstanceCallback: (instanceId: string) => void,
	) {
		const curseForgeLink = {
			type: 'curseforge_modpack' as const,
			project_id: numericProjectId.toString(),
			version_id: version.id,
		}
		// Match Modrinth managed packs: associate the instance as soon as it is created
		// so Installation settings can show the linked-modpack controls immediately.
		const job = await install_create_instance({
			name: project.title,
			gameVersion,
			loader,
			loaderVersion: 'latest',
			iconPath: project.icon_url ?? null,
			link: curseForgeLink,
		})
		const createdInstanceId = installJobInstanceId(job)
		if (!createdInstanceId) return
		createInstanceCallback(createdInstanceId)
		addInstallingItem(createdInstanceId, project, version)
		try {
			await wait_for_install_job(job.job_id)
			await edit(createdInstanceId, {
				link: curseForgeLink,
			})
			removeInstallingItems(createdInstanceId, [project.id])
			markInstanceContentChanged(createdInstanceId)
			trackEvent('PackInstall', {
				id: project.id,
				version_id: version.id,
				title: project.title,
				source,
			})
			callback(version.id, [project.id])
		} catch (err) {
			// Best-effort: still keep the managed association for settings/update UI.
			await edit(createdInstanceId, {
				link: curseForgeLink,
			}).catch(() => {})
			removeInstallingItems(createdInstanceId, [project.id])
			markInstanceContentInstallFailed(createdInstanceId)
			throw err
		}
	}

	async function showModInstallModal(
		project: Labrinth.Projects.v2.Project,
		versions: Labrinth.Versions.v2.Version[],
		onInstall: ContentInstallCallback,
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) {
		currentProject = project
		currentVersions = versions
		currentCallback = onInstall

		instances.value = []
		loading.value = true
		defaultTab.value = 'existing'

		if (hints?.showProjectInfo) {
			projectInfo.value = {
				title: project.title,
				iconUrl: project.icon_url,
				link:
					currentProvider === 'curseforge'
						? `/project/curseforge/${currentCurseForgeProject?.id}`
						: `/project/${project.slug ?? project.id}`,
			}
			if (currentProvider === 'curseforge' && currentCurseForgeProject?.authors[0]) {
				const author = currentCurseForgeProject.authors[0]
				projectInfo.value = {
					...projectInfo.value,
					owner: {
						name: author.name,
						circle: true,
						link: () => openUrl(author.url),
					},
				}
			} else if (project.organization) {
				get_organization(project.organization)
					.then((org: { id: string; slug: string; name: string; icon_url?: string }) => {
						if (projectInfo.value) {
							const orgSlug = org.slug ?? org.id
							projectInfo.value = {
								...projectInfo.value,
								owner: {
									name: org.name,
									iconUrl: org.icon_url,
									circle: false,
									link: () => openUrl(`https://modrinth.com/organization/${orgSlug}`),
								},
							}
						}
					})
					.catch(() => {})
			} else if (project.team) {
				get_team(project.team)
					.then(
						(
							members: {
								user: { id: string; username: string; avatar_url?: string }
								is_owner: boolean
							}[],
						) => {
							const owner = members.find((m) => m.is_owner)
							if (owner && projectInfo.value) {
								projectInfo.value = {
									...projectInfo.value,
									owner: {
										name: owner.user.username,
										iconUrl: owner.user.avatar_url,
										circle: true,
										link: () => openUrl(`https://modrinth.com/user/${owner.user.username}`),
									},
								}
							}
						},
					)
					.catch(() => {})
			}
		} else {
			projectInfo.value = null
		}

		const loaderSet = new Set<string>()
		const gameVersionSet = new Set<string>()
		for (const v of versions) {
			for (const l of v.loaders) loaderSet.add(l)
			for (const gv of v.game_versions) gameVersionSet.add(gv)
		}
		const mappedLoaders = new Set<string>()
		for (const l of loaderSet) {
			if (SUPPORTED_LOADERS.has(l)) mappedLoaders.add(l)
			else if (VANILLA_COMPATIBLE_LOADERS.has(l)) mappedLoaders.add('vanilla')
		}
		compatibleLoaders.value = sortLoaders([...mappedLoaders])
		gameVersions.value = [...gameVersionSet]
		releaseGameVersions.value = new Set(gameVersionSet)

		preferredLoader.value =
			hints?.preferredLoader && loaderSet.has(hints.preferredLoader) ? hints.preferredLoader : null
		preferredGameVersion.value =
			hints?.preferredGameVersion && gameVersionSet.has(hints.preferredGameVersion)
				? hints.preferredGameVersion
				: null

		await nextTick()
		modalRef?.show()
		trackEvent('ProjectInstallStart', { source: 'ProjectInstallModal' })

		get_game_versions()
			.then((allGameVersions) => {
				const releases = new Set<string>()
				const ordered: string[] = []
				for (const gv of allGameVersions) {
					if (gameVersionSet.has(gv.version)) {
						ordered.push(gv.version)
						if (gv.version_type === 'release') {
							releases.add(gv.version)
						}
					}
				}
				gameVersions.value = ordered
				releaseGameVersions.value = releases
			})
			.catch(() => {})

		try {
			const candidates = await get_install_candidates(
				project.id,
				project.project_type,
				getInstallTargets(versions),
			)
			const newInstanceMap: Record<string, InstallTargetInstance> = {}
			const newInstances: ContentInstallInstance[] = candidates.map((instance) => {
				newInstanceMap[instance.id] = instance
				return {
					id: instance.id,
					name: instance.name,
					iconUrl: instance.icon_path ? convertFileSrc(instance.icon_path) : null,
					iconFrameless: isBuiltInInstanceIcon(instance.icon_path),
					installed: instance.installed,
					compatible: instance.compatible,
					installing: false,
				}
			})

			instanceMap = newInstanceMap
			instances.value = newInstances

			if (!newInstances.some((i) => i.compatible && !i.installed)) {
				defaultTab.value = 'new'
			}
		} catch (err) {
			opts.handleError(err)
		} finally {
			loading.value = false
		}
	}

	function getInstallTargets(versions: Labrinth.Versions.v2.Version[]) {
		const targets: { game_version: string; loader: string }[] = []
		const seen = new Set<string>()

		for (const version of versions) {
			for (const gameVersion of version.game_versions) {
				for (const loader of version.loaders) {
					const key = `${gameVersion}\0${loader}`
					if (seen.has(key)) continue
					seen.add(key)
					targets.push({ game_version: gameVersion, loader })
				}
			}
		}

		return targets
	}

	async function removeInstalledCurseForgeProject(instanceId: string, projectId: number) {
		const content = await get_content_items(instanceId).catch(() => [])
		for (const item of content) {
			if (
				item.provider_refs?.some(
					(reference) =>
						reference.provider === 'curseforge' && reference.project_id === projectId.toString(),
				)
			) {
				await remove_project(instanceId, item.file_path ?? item.file_name)
			}
		}
	}

	function rememberManualDownloads(
		instanceId: string,
		result: CurseForgeInstallResult,
	): CurseForgeManualDownloadItem[] {
		const manualItems: CurseForgeManualDownloadItem[] = (result.manualDownloads ?? []).map(
			(item) => ({
				projectId: item.projectId,
				fileId: item.fileId,
				fileName: item.fileName,
				websiteUrl:
					item.websiteUrl ??
					currentCurseForgeProject?.links.websiteUrl ??
					`https://www.curseforge.com/minecraft/mc-mods/${item.projectId}/files/${item.fileId}`,
			}),
		)
		setCurseForgeManualDownloads(instanceId, manualItems)
		const next = new Map(pendingManualDownloadsByInstance.value)
		if (manualItems.length > 0) {
			next.set(instanceId, manualItems)
		} else {
			next.delete(instanceId)
		}
		pendingManualDownloadsByInstance.value = next
		return manualItems
	}

	function showManualCurseForgeDownloads(instanceId: string, result: CurseForgeInstallResult) {
		const summary = summarizeCurseForgeInstall(result)
		const manualItems = rememberManualDownloads(instanceId, result)
		if (manualItems.length === 0) return

		const manualNames = manualItems
			.slice(0, 5)
			.map((item) => item.fileName)
			.filter(Boolean)
		const extra = summary.manual > manualNames.length ? summary.manual - manualNames.length : 0
		const listText = manualNames.length
			? extra > 0
				? formatMessage(manualDownloadsListAndMoreMessage, {
						list: manualNames.join(', '),
						count: extra,
					})
				: manualNames.join(', ')
			: formatMessage(manualDownloadsFilesCountMessage, { count: summary.manual })

		opts.addNotification({
			title: formatMessage(manualDownloadsTitleMessage),
			text:
				summary.installed > 0
					? formatMessage(manualDownloadsPartialMessage, {
							installed: summary.installed,
							manual: summary.manual,
							list: listText,
						})
					: formatMessage(manualDownloadsFailedMessage, {
							manual: summary.manual,
							list: listText,
						}),
			type: summary.installed > 0 ? 'warning' : 'error',
		})

		curseForgeManualDownloadsModalRef?.show({
			items: manualItems,
			installed: summary.installed,
			instanceId,
		})
	}

	async function installCurrentCurseForgeVersion(
		instance: InstallTargetInstance,
		project: Labrinth.Projects.v2.Project,
		version: Labrinth.Versions.v2.Version,
		installDependencies: boolean,
	) {
		const curseForgeProject = currentCurseForgeProject
		const file = currentCurseForgeFiles.get(version.id)
		if (!curseForgeProject || !file) {
			throw new Error('CurseForge project or file was not loaded')
		}

		let result: CurseForgeInstallResult
		if (project.project_type === 'modpack') {
			result = (
				await installCurseForgeModpack({
					instanceId: instance.id,
					projectId: curseForgeProject.id,
					fileId: file.id,
				})
			).content
		} else {
			await removeInstalledCurseForgeProject(instance.id, curseForgeProject.id)
			result = await installCurseForgeFile({
				instanceId: instance.id,
				projectId: curseForgeProject.id,
				fileId: file.id,
				projectType: project.project_type,
				gameVersion: instance.game_version,
				modLoaderType: curseForgeLoaderType(instance.loader),
				installDependencies,
			})
		}

		showManualCurseForgeDownloads(instance.id, result)
		const installedProjectIds = [
			...new Set(result.installed.map((installed) => `curseforge:${installed.projectId}`)),
		]
		const primaryInstalled =
			project.project_type === 'modpack' ||
			result.installed.some(
				(installed) => !installed.dependency && installed.projectId === curseForgeProject.id,
			)
		if (primaryInstalled && !installedProjectIds.includes(project.id)) {
			installedProjectIds.unshift(project.id)
		}
		markInstanceContentChanged(instance.id)
		if (project.project_type === 'modpack') {
			const summary = summarizeCurseForgeInstall(result)
			if (summary.manual === 0 && summary.installed > 0) {
				setCurseForgeManualDownloads(instance.id, [])
				const next = new Map(pendingManualDownloadsByInstance.value)
				next.delete(instance.id)
				pendingManualDownloadsByInstance.value = next
				opts.addNotification({
					title: formatMessage(modpackInstalledTitleMessage),
					text: formatMessage(modpackInstalledBodyMessage, {
						count: summary.installed,
					}),
					type: 'success',
				})
			}
		}
		return { installedProjectIds, primaryInstalled }
	}

	async function handleInstallToInstance(instance: ContentInstallInstance) {
		const selectedInstance = instanceMap[instance.id]
		const storeInstance = instances.value.find((i) => i.id === instance.id)
		if (!currentProject || !selectedInstance) {
			opts.handleError('No project or instance found')
			return
		}

		const version = findPreferredVersion(currentVersions, currentProject, selectedInstance)
		if (!version) {
			if (currentVersions.length > 0 && incompatibilityWarningModalRef) {
				const onIncompatibleInstall = (versionId?: string) => {
					if (versionId && storeInstance) {
						storeInstance.installed = true
					}
					currentCallback(versionId, versionId && currentProject ? [currentProject.id] : undefined)
				}
				await showIncompatibilityWarning(
					selectedInstance,
					currentProject,
					currentVersions,
					currentVersions[0],
					onIncompatibleInstall,
				)
			} else {
				opts.handleError('No version found')
			}
			return
		}

		if (storeInstance) storeInstance.installing = true

		const installedProjectIds: string[] = [currentProject.id]
		let plannedProjectIds: string[] = [currentProject.id]
		addInstallingItem(instance.id, currentProject, version)

		try {
			let primaryInstalled = true
			if (currentProvider === 'curseforge') {
				const result = await installCurrentCurseForgeVersion(
					selectedInstance,
					currentProject,
					version,
					true,
				)
				installedProjectIds.splice(0, installedProjectIds.length, ...result.installedProjectIds)
				primaryInstalled = result.primaryInstalled
				removeInstallingItems(instance.id, plannedProjectIds)
			} else {
				const request = {
					project_id: currentProject.id,
					version_id: version.id,
					content_type: resolveContentType(currentProject.project_type),
				}
				const plan = await install_project_with_dependencies(instance.id, request)
				plannedProjectIds = resolvedProjectIds(plan)
				await addInstallingItemsForPlan(instance.id, plan, currentProject, version)
				installedProjectIds.splice(
					0,
					installedProjectIds.length,
					plan.primary.project_id,
					...plan.dependencies.map((dependency) => dependency.project_id),
				)
			}
			if (storeInstance) {
				storeInstance.installed = primaryInstalled
				storeInstance.installing = false
			}
			trackEvent('ProjectInstall', {
				loader: selectedInstance.loader,
				game_version: selectedInstance.game_version,
				id: currentProject!.id,
				version_id: version.id,
				project_type: currentProject!.project_type,
				title: currentProject!.title,
				source: 'ProjectInstallModal',
			})
			currentCallback(primaryInstalled ? version.id : undefined, installedProjectIds)
		} catch (err) {
			if (storeInstance) storeInstance.installing = false
			removeInstallingItems(instance.id, plannedProjectIds)
			markInstanceContentInstallFailed(instance.id)
			opts.handleError(err)
		}
	}

	async function showIncompatibilityWarning(
		instance: InstallTargetInstance,
		project: Labrinth.Projects.v2.Project,
		versions: Labrinth.Versions.v2.Version[],
		version: Labrinth.Versions.v2.Version,
		callback: ContentInstallCallback,
	) {
		incompatibilityWarningInstance = instance
		incompatibilityWarningProject = project
		incompatibilityWarningCallback = callback
		incompatibilityWarningInstalled = false
		incompatibilityWarningInstalling.value = false
		incompatibilityWarningVersions.value = versions
		incompatibilityWarningCurrentGameVersion.value = instance.game_version ?? ''
		incompatibilityWarningCurrentLoader.value = instance.loader ?? ''
		incompatibilityWarningProjectType.value = project.project_type
		incompatibilityWarningProjectIconUrl.value = project.icon_url ?? undefined
		incompatibilityWarningProjectName.value = project.title

		const compatibilityLabel =
			project.project_type === 'resourcepack' || project.project_type === 'datapack'
				? (instance.game_version ?? '')
				: `${instance.loader ?? ''} ${instance.game_version ?? ''}`.trim()
		incompatibilityWarningMessage.value = formatMessage(noCompatibleVersionsMessage, {
			compatibilityLabel,
		})

		await nextTick()
		incompatibilityWarningModalRef?.show(version.id)
		trackEvent('ProjectInstallStart', { source: 'ProjectIncompatibilityWarningModal' })
	}

	async function handleIncompatibilityWarningInstall(version: Labrinth.Versions.v2.Version) {
		if (!incompatibilityWarningInstance || !incompatibilityWarningProject) return

		incompatibilityWarningInstalling.value = true
		addInstallingItem(incompatibilityWarningInstance.id, incompatibilityWarningProject, version)
		try {
			if (currentProvider === 'curseforge') {
				const result = await installCurrentCurseForgeVersion(
					incompatibilityWarningInstance,
					incompatibilityWarningProject,
					version,
					false,
				)
				if (!result.primaryInstalled) {
					incompatibilityWarningInstalling.value = false
					removeInstallingItems(incompatibilityWarningInstance.id, [
						incompatibilityWarningProject.id,
					])
					incompatibilityWarningCallback()
					return
				}
			} else {
				await add_project_from_version(incompatibilityWarningInstance.id, version.id, 'standalone')
			}
		} catch (err) {
			opts.handleError(err)
			incompatibilityWarningInstalling.value = false
			removeInstallingItems(incompatibilityWarningInstance.id, [incompatibilityWarningProject.id])
			markInstanceContentInstallFailed(incompatibilityWarningInstance.id)
			return
		}

		incompatibilityWarningInstalling.value = false
		incompatibilityWarningInstalled = true
		incompatibilityWarningCallback(version.id, [incompatibilityWarningProject.id])
		markInstanceContentChanged(incompatibilityWarningInstance.id)
		incompatibilityWarningModalRef?.hide()
		removeInstallingItems(incompatibilityWarningInstance.id, [incompatibilityWarningProject.id])

		trackEvent('ProjectInstall', {
			loader: incompatibilityWarningInstance.loader,
			game_version: incompatibilityWarningInstance.game_version,
			id: incompatibilityWarningProject.id,
			version_id: version.id,
			project_type: incompatibilityWarningProject.project_type,
			title: incompatibilityWarningProject.title,
			source: 'ProjectIncompatibilityWarningModal',
		})
	}

	function handleIncompatibilityWarningCancel() {
		if (!incompatibilityWarningInstalled) {
			incompatibilityWarningCallback()
		}
		incompatibilityWarningInstalled = false
	}

	async function handleCreateAndInstall(data: {
		name: string
		iconPath: string | null
		iconPreviewUrl: string | null
		loader: string
		gameVersion: string
	}) {
		const loaderCandidates =
			data.loader === 'vanilla' ? ['vanilla', 'datapack', 'minecraft'] : [data.loader]
		const version =
			currentVersions.find(
				(v) =>
					v.game_versions.includes(data.gameVersion) &&
					loaderCandidates.some((l) => v.loaders.includes(l)),
			) ?? currentVersions[0]

		let createdInstanceId: string | null = null
		try {
			const job = await install_create_instance({
				name: data.name,
				gameVersion: data.gameVersion,
				loader: data.loader as InstanceLoader,
				loaderVersion: 'latest',
				iconPath: data.iconPath,
			})
			const id = installJobInstanceId(job)
			if (!id) return
			createdInstanceId = id
			addInstallingItem(id, currentProject!, version)

			let installedProjectIds: string[]
			if (currentProvider === 'curseforge') {
				const result = await installCurrentCurseForgeVersion(
					{
						id,
						name: data.name,
						icon_path: data.iconPath ?? undefined,
						game_version: data.gameVersion,
						loader: data.loader as InstanceLoader,
					},
					currentProject!,
					version,
					true,
				)
				installedProjectIds = result.installedProjectIds
				removeInstallingItems(id, [currentProject!.id])
			} else {
				const plan = await install_project_with_dependencies(id, {
					project_id: currentProject!.id,
					version_id: version.id,
					content_type: resolveContentType(currentProject!.project_type),
				})
				await addInstallingItemsForPlan(id, plan, currentProject!, version)
				installedProjectIds = resolvedProjectIds(plan)
			}
			await opts.router.push(
				currentProvider === 'curseforge' && currentProject?.project_type === 'modpack'
					? '/downloads'
					: `/instance/${encodeURIComponent(id)}`,
			)

			trackEvent('InstanceCreate', {
				source: 'ProjectInstallModal',
			})
			trackEvent('ProjectInstall', {
				loader: data.loader,
				game_version: data.gameVersion,
				id: currentProject!.id,
				version_id: version.id,
				project_type: currentProject!.project_type,
				title: currentProject!.title,
				source: 'ProjectInstallModal',
			})

			currentCallback(version.id, installedProjectIds)
			modalRef?.hide()
		} catch (err) {
			if (createdInstanceId && currentProject) {
				removeInstallingItems(createdInstanceId, [currentProject.id])
				markInstanceContentInstallFailed(createdInstanceId)
			}
			opts.handleError(err)
		}
	}

	function handleNavigate(instance: ContentInstallInstance) {
		modalRef?.hide()
		opts.router.push(`/instance/${encodeURIComponent(instance.id)}`)
	}

	function handleCancel() {
		currentCallback?.()
	}

	async function install(
		projectId: string,
		versionId?: string | null,
		instanceId?: string | null,
		source: string = 'unknown',
		callback: ContentInstallCallback = () => {},
		createInstanceCallback: (instanceId: string) => void = () => {},
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) {
		currentProvider = 'modrinth'
		currentCurseForgeProject = null
		currentCurseForgeFiles = new Map()
		const project: Labrinth.Projects.v2.Project = await get_project(projectId, 'must_revalidate')

		if (project.project_type === 'modpack') {
			const version = versionId ?? project.versions[project.versions.length - 1]
			const packs = await list()
			const existingPack = packs.find((pack) => pack.link?.project_id === project.id)

			if (existingPack && !themeStore.getFeatureFlag('skip_non_essential_warnings')) {
				pendingModpackInstall = {
					project,
					version,
					source,
					callback,
					createInstanceCallback,
					provider: 'modrinth',
				}
				modpackAlreadyInstalledModalRef?.show(existingPack.name, existingPack.id)
				return
			}

			const job = await install_create_modpack_instance({
				type: 'fromVersionId',
				project_id: project.id,
				version_id: version,
				title: project.title,
				icon_url: project.icon_url,
			})
			const instanceId = installJobInstanceId(job)
			if (instanceId) {
				createInstanceCallback(instanceId)
			}
			trackEvent('PackInstall', {
				id: project.id,
				version_id: version,
				title: project.title,
				source,
			})
			callback(version)
		} else if (instanceId) {
			const [instanceOrNull, instanceProjects, versions] = await Promise.all([
				get(instanceId),
				get_projects(instanceId),
				get_version_many(project.versions, 'must_revalidate') as Promise<
					Labrinth.Versions.v2.Version[]
				>,
			])
			if (!instanceOrNull) return

			const instance = instanceOrNull
			const projectVersions = versions.sort(
				(a, b) => dayjs(b.date_published).valueOf() - dayjs(a.date_published).valueOf(),
			)

			let version = versionId
				? projectVersions.find((v) => v.id === versionId)
				: findPreferredVersion(projectVersions, project, instance)
			if (!version) version = projectVersions[0]

			if (isVersionCompatible(version, project, instance)) {
				for (const [path, file] of Object.entries(instanceProjects)) {
					if (file.metadata?.project_id === project.id) {
						await remove_project(instance.id, path)
					}
				}

				const installedProjectIds: string[] = [project.id]
				let plannedProjectIds: string[] = [project.id]
				addInstallingItem(instanceId, project, version)
				try {
					const request = {
						project_id: project.id,
						version_id: version.id,
						content_type: resolveContentType(project.project_type),
					}
					const plan = await install_project_with_dependencies(instance.id, request)
					plannedProjectIds = resolvedProjectIds(plan)
					await addInstallingItemsForPlan(instanceId, plan, project, version)
					installedProjectIds.splice(
						0,
						installedProjectIds.length,
						plan.primary.project_id,
						...plan.dependencies.map((dependency) => dependency.project_id),
					)

					trackEvent('ProjectInstall', {
						loader: instance.loader,
						game_version: instance.game_version,
						id: project.id,
						project_type: project.project_type,
						version_id: version.id,
						title: project.title,
						source,
					})
					callback(version.id, installedProjectIds)
				} catch (err) {
					removeInstallingItems(instanceId, plannedProjectIds)
					markInstanceContentInstallFailed(instanceId)
					throw err
				}
			} else {
				await showIncompatibilityWarning(instance, project, projectVersions, version, callback)
			}
		} else {
			let versions = (
				(await get_version_many(project.versions)) as Labrinth.Versions.v2.Version[]
			).sort((a, b) => dayjs(b.date_published).valueOf() - dayjs(a.date_published).valueOf())
			if (versionId) versions = versions.filter((v) => v.id === versionId)
			await showModInstallModal(project, versions, callback, hints)
		}
	}

	async function installCurseForge(
		projectId: string,
		versionId?: string | null,
		instanceId?: string | null,
		source: string = 'unknown',
		callback: ContentInstallCallback = () => {},
		createInstanceCallback: (instanceId: string) => void = () => {},
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) {
		const numericProjectId = Number(projectId.replace(/^curseforge:/, ''))
		if (!Number.isFinite(numericProjectId)) {
			throw new Error('Invalid CurseForge project ID')
		}
		const [curseForgeProject, fileResponse] = await Promise.all([
			getCurseForgeProject(numericProjectId),
			getCurseForgeFiles(numericProjectId, { index: 0, pageSize: 50 }),
		])
		const availableFiles = fileResponse.files.filter((file) => file.isAvailable)
		const project = mapCurseForgeProject(curseForgeProject, availableFiles)
		let versions = availableFiles
			.map((file) => mapCurseForgeVersion(file, numericProjectId, project.project_type))
			.sort((a, b) => dayjs(b.date_published).valueOf() - dayjs(a.date_published).valueOf())
		if (versionId) versions = versions.filter((version) => version.id === versionId)
		if (versions.length === 0) {
			throw new Error('No CurseForge files are available for this project')
		}

		currentProvider = 'curseforge'
		currentProject = project
		currentVersions = versions
		currentCurseForgeProject = curseForgeProject
		currentCurseForgeFiles = new Map(availableFiles.map((file) => [file.id.toString(), file]))

		if (project.project_type === 'modpack') {
			const version = versions[0]
			const gameVersion =
				(hints?.preferredGameVersion &&
					version.game_versions.includes(hints.preferredGameVersion) &&
					hints.preferredGameVersion) ||
				version.game_versions[0]
			if (!gameVersion) {
				throw new Error('The CurseForge modpack does not declare a Minecraft version')
			}
			const loader =
				version.loaders.find((candidate) => SUPPORTED_LOADERS.has(candidate)) ?? 'vanilla'
			const packs = await list()
			const existingPack = packs.find(
				(pack) =>
					pack.link?.type === 'curseforge_modpack' &&
					(pack.link.project_id === numericProjectId.toString() ||
						pack.link.project_id === project.id),
			)
			if (existingPack && !themeStore.getFeatureFlag('skip_non_essential_warnings')) {
				pendingModpackInstall = {
					project,
					version: version.id,
					source,
					callback,
					createInstanceCallback,
					provider: 'curseforge',
				}
				modpackAlreadyInstalledModalRef?.show(existingPack.name, existingPack.id)
				return
			}
			await createAndInstallCurseForgeModpack(
				project,
				version,
				numericProjectId,
				gameVersion,
				loader as InstanceLoader,
				source,
				callback,
				createInstanceCallback,
			)
		} else if (instanceId) {
			const instance = await get(instanceId)
			if (!instance) return
			let version = versionId
				? versions.find((candidate) => candidate.id === versionId)
				: findPreferredVersion(versions, project, instance)
			if (!version) version = versions[0]
			if (isVersionCompatible(version, project, instance)) {
				addInstallingItem(instance.id, project, version)
				try {
					const result = await installCurrentCurseForgeVersion(instance, project, version, true)
					removeInstallingItems(instance.id, [project.id])
					trackEvent('ProjectInstall', {
						loader: instance.loader,
						game_version: instance.game_version,
						id: project.id,
						project_type: project.project_type,
						version_id: version.id,
						title: project.title,
						source,
					})
					callback(result.primaryInstalled ? version.id : undefined, result.installedProjectIds)
				} catch (err) {
					removeInstallingItems(instance.id, [project.id])
					markInstanceContentInstallFailed(instance.id)
					throw err
				}
			} else {
				await showIncompatibilityWarning(instance, project, versions, version, callback)
			}
		} else {
			await showModInstallModal(project, versions, callback, hints)
		}
	}

	return {
		instances,
		compatibleLoaders,
		gameVersions,
		loading,
		defaultTab,
		preferredLoader,
		preferredGameVersion,
		releaseGameVersions,
		projectInfo,
		handleInstallToInstance,
		handleCreateAndInstall,
		handleNavigate,
		handleCancel,
		setContentInstallModal(ref: ModalRef) {
			modalRef = ref
		},
		setModpackAlreadyInstalledModal(ref: ModpackAlreadyInstalledModalRef) {
			modpackAlreadyInstalledModalRef = ref
		},
		setCurseForgeManualDownloadsModal(ref: CurseForgeManualDownloadsModalRef) {
			curseForgeManualDownloadsModalRef = ref
		},
		async handleModpackDuplicateCreateAnyway() {
			if (!pendingModpackInstall) return
			const { project, version, source, callback, createInstanceCallback, provider } =
				pendingModpackInstall
			pendingModpackInstall = null
			if (provider === 'curseforge') {
				const numericProjectId = Number(project.id.replace(/^curseforge:/, ''))
				const selectedVersion =
					currentVersions.find((candidate) => candidate.id === version) ?? currentVersions[0]
				if (!selectedVersion || !Number.isFinite(numericProjectId)) {
					throw new Error('Unable to reinstall the CurseForge modpack')
				}
				const gameVersion = selectedVersion.game_versions[0]
				if (!gameVersion) {
					throw new Error('The CurseForge modpack does not declare a Minecraft version')
				}
				const loader =
					(selectedVersion.loaders.find((candidate) => SUPPORTED_LOADERS.has(candidate)) as
						| InstanceLoader
						| undefined) ?? 'vanilla'
				await createAndInstallCurseForgeModpack(
					project,
					selectedVersion,
					numericProjectId,
					gameVersion,
					loader,
					source,
					callback,
					createInstanceCallback,
				)
				return
			}
			const job = await install_create_modpack_instance({
				type: 'fromVersionId',
				project_id: project.id,
				version_id: version,
				title: project.title,
				icon_url: project.icon_url,
			})
			const instanceId = installJobInstanceId(job)
			if (instanceId) {
				createInstanceCallback(instanceId)
			}
			trackEvent('PackInstall', {
				id: project.id,
				version_id: version,
				title: project.title,
				source,
			})
			callback(version)
		},
		handleModpackDuplicateGoToInstance(instanceId: string) {
			pendingModpackInstall = null
			opts.router.push(`/instance/${encodeURIComponent(instanceId)}`)
		},
		setIncompatibilityWarningModal(ref: ModalRef) {
			incompatibilityWarningModalRef = ref
		},
		incompatibilityWarningVersions,
		incompatibilityWarningCurrentGameVersion,
		incompatibilityWarningCurrentLoader,
		incompatibilityWarningProjectType,
		incompatibilityWarningProjectIconUrl,
		incompatibilityWarningProjectName,
		incompatibilityWarningMessage,
		incompatibilityWarningInstalling,
		handleIncompatibilityWarningInstall,
		handleIncompatibilityWarningCancel,
		install,
		installCurseForge,
		installingItems,
		pendingManualDownloadsByInstance,
		installRevisionByInstance,
		installFailureRevisionByInstance,
	}
}
