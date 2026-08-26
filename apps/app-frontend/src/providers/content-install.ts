import type { Labrinth } from '@modrinth/api-client'
import type { ContentInstallInstance, ContentInstallProjectInfo, ContentItem } from '@modrinth/ui'
import {
	createContext,
	defineMessage,
	useDebugLogger,
	usesTargetGameVersion,
	useVIntl,
} from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import { nextTick, type Ref, ref } from 'vue'
import type { Router } from 'vue-router'

import type ContentInstallPreviewModal from '@/components/ui/ContentInstallPreviewModal.vue'
import type { ContentInstallPreviewData } from '@/components/ui/ContentInstallPreviewModal.vue'
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
	type CurseForgeManualDownloadImport,
	type CurseForgeProject,
	getCurseForgeDownloadFailureDetails,
	getCurseForgeFile,
	getCurseForgeFiles,
	getCurseForgeProject,
	getCurseForgeProjects,
	installCurseForgeFile,
	installCurseForgeModpack,
	previewCurseForgeFile,
	queueCurseForgeFile,
	queueCurseForgeWorld,
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
	edit,
	get,
	get_content_items,
	get_install_candidates,
	get_projects,
	list,
	preview_project_with_dependencies,
	preview_project_with_dependencies_for_target,
	queue_project_with_dependencies,
	remove_project,
	type ResolveContentPlan,
} from '@/helpers/instance'
import { getDisplayInstanceIcon } from '@/helpers/instance-icons'
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
		installed?: number
		instanceId?: string | null
	}) => void
}

export type ContentInstallCallback = (versionId?: string, installedProjectIds?: string[]) => void
type ContentInstallInstanceEvent = {
	event: string
	instance_id: string
	project_ids?: string[]
	dependency_project_ids?: string[]
	message?: string
}

const LOADER_ORDER = ['vanilla', 'fabric', 'quilt', 'neoforge', 'forge']
const SUPPORTED_LOADERS: Set<string> = new Set(['vanilla', 'forge', 'fabric', 'quilt', 'neoforge'])
const VANILLA_COMPATIBLE_LOADERS: Set<string> = new Set(['minecraft', 'datapack'])
type InstallProvider = 'modrinth' | 'curseforge'
type ContentInstallTargetMode = 'content' | 'world'
const noCompatibleVersionsMessage = defineMessage({
	id: 'app.content-install.no-compatible-versions',
	defaultMessage:
		'No available versions match {compatibilityLabel}. Select a version to install anyway. Matching dependencies will still be installed.',
})
const curseForgeWorldInvalidProjectMessage = defineMessage({
	id: 'app.worlds.install-map.invalid-project',
	defaultMessage: 'The selected project is not a CurseForge map.',
})
const curseForgeWorldUnavailableMessage = defineMessage({
	id: 'app.browse.maps-no-installable-file',
	defaultMessage: 'The selected CurseForge map does not have an installable file.',
})
const curseForgeWorldInstanceNotReadyMessage = defineMessage({
	id: 'app.worlds.install-map.instance-not-ready',
	defaultMessage: 'Wait for this instance to finish installing before adding a map.',
})
const curseForgeWorldUnknownInstanceMessage = defineMessage({
	id: 'app.worlds.install-map.unknown-instance',
	defaultMessage: 'The selected instance is no longer available.',
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
const manualDownloadsImportedTitleMessage = defineMessage({
	id: 'app.curseforge.manual-downloads.imported-title',
	defaultMessage: 'CurseForge files imported',
})
const manualDownloadsImportedMessage = defineMessage({
	id: 'app.curseforge.manual-downloads.imported',
	defaultMessage: 'Imported {count, number} downloaded files into the instance.',
})
const automaticDownloadsFailedTitleMessage = defineMessage({
	id: 'app.curseforge.automatic-downloads-failed.notification-title',
	defaultMessage: 'Some CurseForge downloads failed',
})
const automaticDownloadsFailedMessage = defineMessage({
	id: 'app.curseforge.automatic-downloads-failed.notification-body',
	defaultMessage:
		'{failed, number} files failed after retrying ({list}). See Downloads for the recorded errors.',
})
const dependencyNotesTitleMessage = defineMessage({
	id: 'app.curseforge.dependency-notes.notification-title',
	defaultMessage: 'CurseForge dependency notes',
})
const dependencyNotesMessage = defineMessage({
	id: 'app.curseforge.dependency-notes.notification-body',
	defaultMessage:
		'{optional, plural, =0 {No optional dependencies were skipped.} one {# optional dependency was skipped.} other {# optional dependencies were skipped.}} {incompatible, plural, =0 {No incompatible dependencies were detected.} one {# incompatible dependency was detected.} other {# incompatible dependencies were detected.}} {skipped, plural, =0 {No other dependencies were skipped.} one {# dependency was skipped.} other {# dependencies were skipped.}}',
})
const dependenciesInstalledTitleMessage = defineMessage({
	id: 'app.content-install.dependencies-installed.notification-title',
	defaultMessage: 'Dependencies installed',
})
const dependenciesInstalledMessage = defineMessage({
	id: 'app.content-install.dependencies-installed.notification-body',
	defaultMessage:
		'Installed {count, plural, one {# dependency} other {# dependencies}} automatically: {list}',
})
const dependenciesSkippedTitleMessage = defineMessage({
	id: 'app.content-install.dependencies-skipped.notification-title',
	defaultMessage: 'Some dependencies were skipped',
})
const dependenciesSkippedMessage = defineMessage({
	id: 'app.content-install.dependencies-skipped.notification-body',
	defaultMessage: 'The following dependencies were not installed: {list}',
})
const sha1VerifiedModrinthFallbackMessage = defineMessage({
	id: 'app.content-install.preview.sha1-verified-modrinth-fallback',
	defaultMessage: 'Modrinth fallback verified by SHA-1',
})
const unavailableCurseForgeProjectMessage = defineMessage({
	id: 'app.content-install.preview.unavailable-curseforge-project',
	defaultMessage: 'Unavailable CurseForge project',
})
const skippedReasonMessages = {
	already_installed: defineMessage({
		id: 'app.content-install.preview.skip.already-installed',
		defaultMessage: 'Already installed',
	}),
	no_compatible_version: defineMessage({
		id: 'app.content-install.preview.skip.no-compatible-version',
		defaultMessage: 'No compatible version found',
	}),
	modrinth_lookup_failed: defineMessage({
		id: 'app.content-install.preview.skip.modrinth-lookup-failed',
		defaultMessage: 'Could not verify the Modrinth fallback',
	}),
	embedded: defineMessage({
		id: 'app.content-install.preview.skip.embedded',
		defaultMessage: 'Embedded in the project',
	}),
	tool: defineMessage({
		id: 'app.content-install.preview.skip.tool',
		defaultMessage: 'Development tool, not installed',
	}),
	unsupported_project_type: defineMessage({
		id: 'app.content-install.preview.skip.unsupported-project-type',
		defaultMessage: 'Unsupported project type',
	}),
	optional: defineMessage({
		id: 'app.content-install.preview.skip.optional',
		defaultMessage: 'Optional dependency',
	}),
	incompatible: defineMessage({
		id: 'app.content-install.preview.skip.incompatible',
		defaultMessage: 'Incompatible dependency',
	}),
	missing_version: defineMessage({
		id: 'app.content-install.preview.skip.missing-version',
		defaultMessage: 'Referenced version was not found',
	}),
	conflicting_dependency: defineMessage({
		id: 'app.content-install.preview.skip.conflicting-dependency',
		defaultMessage: 'Conflicting dependency version',
	}),
	duplicate_project: defineMessage({
		id: 'app.content-install.preview.skip.duplicate-project',
		defaultMessage: 'Already included',
	}),
	quilt_fabric_api: defineMessage({
		id: 'app.content-install.preview.skip.quilt-fabric-api',
		defaultMessage: 'Replaced for quilt',
	}),
	excluded_by_user: defineMessage({
		id: 'app.content-install.preview.skip.excluded-by-user',
		defaultMessage: 'Excluded by user',
	}),
	dependency_cycle: defineMessage({
		id: 'app.content-install.preview.skip.dependency-cycle',
		defaultMessage: 'Dependency cycle detected',
	}),
	dependency_depth_exceeded: defineMessage({
		id: 'app.content-install.preview.skip.dependency-depth-exceeded',
		defaultMessage: 'Dependency depth limit reached',
	}),
} as const
const curseForgeNetworkFailureTitleMessage = defineMessage({
	id: 'app.curseforge.network-download-failed.notification-title',
	defaultMessage: 'Could not download from CurseForge',
})
const curseForgeNetworkFailureMessage = defineMessage({
	id: 'app.curseforge.network-download-failed.notification-body',
	defaultMessage:
		'Could not connect to CurseForge to download this file. Your network or proxy may be blocking CurseForge. Turn off or change your VPN/proxy, try another network, then retry the download.',
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
	if (project.project_type === 'resourcepack') return true

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
	if (projectType === 'resourcepack') return versions[0]

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

function projectPageUrl(project: { project_type: string; slug: string }): string {
	return `https://modrinth.com/${project.project_type}/${project.slug}`
}

function curseForgePageUrl(project: { slug: string; links?: { websiteUrl?: string } }): string {
	if (project.links?.websiteUrl) return project.links.websiteUrl
	return `https://www.curseforge.com/minecraft/mc-mods/${project.slug}`
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
	symlinkTarget: Ref<string | null | undefined>
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
	setContentInstallPreviewModal: (
		ref: InstanceType<typeof ContentInstallPreviewModal> | null,
	) => void
	setModpackAlreadyInstalledModal: (ref: ModpackAlreadyInstalledModalRef) => void
	handleModpackDuplicateCreateAnyway: () => Promise<void>
	handleModpackDuplicateGoToInstance: (instanceId: string) => void
	handleModpackDuplicateCancel: () => void
	setCurseForgeManualDownloadsModal: (ref: CurseForgeManualDownloadsModalRef) => void
	showCurseForgeManualDownloads: (instanceId: string, items: CurseForgeManualDownloadItem[]) => void
	handleCurseForgeManualDownloadsImported: (
		instanceId: string,
		imported: CurseForgeManualDownloadImport[],
	) => void
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
	installCurseForgeWorld: (
		projectId: string | number,
		fileId?: string | number | null,
		instanceId?: string | null,
		source?: string,
		callback?: ContentInstallCallback,
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
		supportData?: Record<string, unknown>
	}) => void
}): ContentInstallContext {
	const { formatMessage } = useVIntl()
	const themeStore = useTheming()
	const debugState = useDebugLogger('content-install')
	const instances = ref<ContentInstallInstance[]>([])
	const compatibleLoaders = ref<string[]>([])
	const gameVersions = ref<string[]>([])
	const loading = ref(false)
	const defaultTab = ref<'existing' | 'new'>('existing')
	const preferredLoader = ref<string | null>(null)
	const preferredGameVersion = ref<string | null>(null)
	const releaseGameVersions = ref<Set<string>>(new Set())

	const projectInfo = ref<ContentInstallProjectInfo | null>(null)
	const symlinkTarget = ref<string | null | undefined>(undefined)
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
			provider_refs: [],
			origin_provider: null,
			update: null,
			enabled: true,
			installing: true,
		}
		const next = new Map(installingItems.value)
		const items = next.get(instanceId) ?? []
		if (items.some((i) => i.file_name === placeholder.file_name)) return
		next.set(instanceId, [...items, placeholder])
		installingItems.value = next
		debugState('addInstallingItem', {
			instanceId,
			projectId: project.id,
			fileName: placeholder.file_name,
		})

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
		debugState('removeInstallingItems call', {
			instanceId,
			projectIds,
			hadItems: !!items,
			count: items?.length,
		})
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

	async function notifyInstalledDependencies(instanceId: string, dependencyProjectIds: string[]) {
		if (dependencyProjectIds.length === 0) return
		const items = await get_content_items(instanceId).catch(() => [])
		const names = dependencyProjectIds
			.map((id) => {
				const curseForge = id.startsWith('curseforge:')
				const rawId = curseForge ? id.slice('curseforge:'.length) : id
				const item = items.find((candidate) =>
					candidate.provider_refs.some(
						(reference) =>
							reference.provider === (curseForge ? 'curseforge' : 'modrinth') &&
							String(reference.project_id) === rawId,
					),
				)
				return item?.project?.title ?? item?.file_name
			})
			.filter((name): name is string => !!name)
		if (names.length === 0) return
		const list = names.length > 5 ? `${names.slice(0, 5).join(', ')}, …` : names.join(', ')
		opts.addNotification({
			title: formatMessage(dependenciesInstalledTitleMessage),
			text: formatMessage(dependenciesInstalledMessage, {
				count: names.length,
				list,
			}),
			type: 'success',
		})
	}

	function markInstanceContentChanged(instanceId: string) {
		const next = new Map(installRevisionByInstance.value)
		const newRev = (next.get(instanceId) ?? 0) + 1
		next.set(instanceId, newRev)
		installRevisionByInstance.value = next
		debugState('markInstanceContentChanged', { instanceId, revision: newRev })
	}

	function markInstanceContentInstallFailed(instanceId: string) {
		const next = new Map(installFailureRevisionByInstance.value)
		const newRev = (next.get(instanceId) ?? 0) + 1
		next.set(instanceId, newRev)
		installFailureRevisionByInstance.value = next
		debugState('markInstanceContentInstallFailed', { instanceId, revision: newRev })
	}

	void instance_listener((event: ContentInstallInstanceEvent) => {
		debugState('instance_listener event', event)
		if (event.event === 'content_install_finished') {
			markInstanceContentChanged(event.instance_id)
			removeInstallingItems(event.instance_id, event.project_ids ?? [])
			void notifyInstalledDependencies(event.instance_id, event.dependency_project_ids ?? [])
		} else if (event.event === 'content_install_failed') {
			removeInstallingItems(event.instance_id, event.project_ids ?? [])
			markInstanceContentInstallFailed(event.instance_id)
			markInstanceContentChanged(event.instance_id)
			opts.handleError(event.message ?? 'Failed to install content')
		}
	}).catch(opts.handleError)

	let modalRef: ModalRef | null = null
	let contentInstallPreviewModalRef: InstanceType<typeof ContentInstallPreviewModal> | null = null
	let modpackAlreadyInstalledModalRef: ModpackAlreadyInstalledModalRef | null = null
	let curseForgeManualDownloadsModalRef: CurseForgeManualDownloadsModalRef | null = null
	let incompatibilityWarningModalRef: ModalRef | null = null
	let currentProvider: InstallProvider = 'modrinth'
	let currentTargetMode: ContentInstallTargetMode = 'content'
	let currentProject: Labrinth.Projects.v2.Project | null = null
	let currentVersions: Labrinth.Versions.v2.Version[] = []
	let currentCurseForgeProject: CurseForgeProject | null = null
	let currentCurseForgeFiles = new Map<string, CurseForgeFile>()
	let currentWorldFileId: string | null = null
	let currentCallback: ContentInstallCallback = () => {}
	let currentSessionId = 0
	let currentCallbackSettled = true
	let contentInstallModalOpen = false
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

	function beginInstallSession(callback: ContentInstallCallback) {
		if (!currentCallbackSettled) currentCallback()
		currentSessionId += 1
		currentCallback = callback
		currentCallbackSettled = false
		return currentSessionId
	}

	function settleCurrentCallback(...args: Parameters<ContentInstallCallback>) {
		if (currentCallbackSettled) return
		currentCallbackSettled = true
		currentCallback(...args)
	}

	function settleInstallSession(sessionId: number, ...args: Parameters<ContentInstallCallback>) {
		if (sessionId !== currentSessionId) return
		settleCurrentCallback(...args)
	}

	async function guardInstallRequest<T>(
		request: () => Promise<T>,
		callback: ContentInstallCallback,
	) {
		const previousSessionId = currentSessionId
		const promise = request()
		const sessionId = currentSessionId !== previousSessionId ? currentSessionId : null
		try {
			return await promise
		} catch (error) {
			if (sessionId !== null) {
				if (sessionId === currentSessionId) {
					hideContentInstallModal()
					settleInstallSession(sessionId)
				}
			} else {
				callback()
			}
			throw error
		}
	}

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
		debugState('createAndInstallCurseForgeModpack', {
			jobId: job.job_id,
			createdInstanceId,
		})
		if (!createdInstanceId) return
		createInstanceCallback(createdInstanceId)
		addInstallingItem(createdInstanceId, project, version)
		try {
			await wait_for_install_job(job.job_id)
			debugState('wait_for_install_job done', { jobId: job.job_id, createdInstanceId })
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
			debugState('createAndInstallCurseForgeModpack ERR', { err: String(err), createdInstanceId })
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
		modalAlreadyOpen = false,
	) {
		const sessionId = currentSessionId
		currentTargetMode = 'content'
		currentWorldFileId = null
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

		if (!modalAlreadyOpen) {
			await nextTick()
			if (sessionId !== currentSessionId) return
			contentInstallModalOpen = true
			modalRef?.show()
			trackEvent('ProjectInstallStart', { source: 'ProjectInstallModal' })
		}

		get_game_versions()
			.then((allGameVersions) => {
				if (sessionId !== currentSessionId) return
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
			if (sessionId !== currentSessionId) return
			const newInstanceMap: Record<string, InstallTargetInstance> = {}
			const newInstances: ContentInstallInstance[] = candidates.map((instance) => {
				newInstanceMap[instance.id] = instance
				const displayIcon = getDisplayInstanceIcon(instance.icon_path, instance.loader)
				return {
					id: instance.id,
					name: instance.name,
					iconUrl: displayIcon.url,
					iconFrameless: displayIcon.frameless,
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

	async function showContentInstallLoading(callback: ContentInstallCallback) {
		const sessionId = beginInstallSession(callback)
		currentTargetMode = 'content'
		currentWorldFileId = null
		instances.value = []
		compatibleLoaders.value = []
		gameVersions.value = []
		releaseGameVersions.value = new Set()
		projectInfo.value = null
		loading.value = true
		defaultTab.value = 'existing'

		await nextTick()
		contentInstallModalOpen = true
		modalRef?.show()
		trackEvent('ProjectInstallStart', { source: 'ProjectInstallModal' })
		return sessionId
	}

	async function showInstallPreview(
		instance: InstallTargetInstance,
		project: Labrinth.Projects.v2.Project,
		version: Labrinth.Versions.v2.Version,
		plan: ResolveContentPlan,
	): Promise<string[] | null> {
		if (!contentInstallPreviewModalRef) return []
		if (plan.dependencies.length === 0) {
			await notifySkippedPlanDependencies(plan.skipped)
			return []
		}
		const projectIds = [
			...new Set([
				plan.primary.project_id,
				...plan.dependencies.map((dependency) => dependency.project_id),
				...plan.skipped.map((skipped) => skipped.project_id),
			]),
		]
		const versionIds = [
			...new Set(
				[
					plan.primary.version_id,
					...plan.dependencies.map((dependency) => dependency.version_id),
				].filter((id): id is string => !!id),
			),
		]
		const [projects, versions] = await Promise.all([
			get_project_many(projectIds).catch(() => [] as Labrinth.Projects.v2.Project[]),
			get_version_many(versionIds).catch(() => [] as Labrinth.Versions.v2.Version[]),
		])
		const projectsById = new Map(projects.map((candidate) => [candidate.id, candidate]))
		const versionsById = new Map(versions.map((candidate) => [candidate.id, candidate]))
		const titleFor = (candidate: { project_id: string; version_id: string }) =>
			projectsById.get(candidate.project_id)?.title ??
			versionsById.get(candidate.version_id)?.version_number ??
			candidate.project_id
		const planned = [
			{ project_id: plan.primary.project_id, version_id: plan.primary.version_id },
			...plan.dependencies.map((dependency) => ({
				project_id: dependency.project_id,
				version_id: dependency.version_id,
			})),
		]
		const parentTitles = new Map(
			planned.map((candidate) => [candidate.version_id, titleFor(candidate)]),
		)
		const dependencies = plan.dependencies.map((dependency) => {
			const project = projectsById.get(dependency.project_id)
			const version = versionsById.get(dependency.version_id)
			return {
				id: dependency.project_id,
				title: project?.title ?? dependency.project_id,
				iconUrl: project?.icon_url ?? null,
				versionNumber: version?.version_number,
				fileName: version?.files?.[0]?.filename,
				description: project?.description,
				projectUrl: project ? projectPageUrl(project) : undefined,
				requiredBy: dependency.dependent_on_version_id
					? [parentTitles.get(dependency.dependent_on_version_id)].filter(
							(title): title is string => !!title,
						)
					: [],
				alreadyInstalled: false,
				required: dependency.required,
			}
		})
		const skipped = plan.skipped
			.filter((item) => item.project_id)
			.map((item) => ({
				id: item.project_id,
				title: projectsById.get(item.project_id)?.title ?? item.project_id,
				reason: formatMessage(
					skippedReasonMessages[item.reason as keyof typeof skippedReasonMessages] ??
						skippedReasonMessages.already_installed,
				),
			}))
		const data: ContentInstallPreviewData = {
			primary: {
				title: project.title,
				iconUrl: project.icon_url,
				versionNumber: version.version_number,
			},
			instanceName: instance.name,
			installDependencies: themeStore.getFeatureFlag('auto_install_dependencies'),
			dependencies,
			skipped,
		}
		const approvedIds = await contentInstallPreviewModalRef.show(data)
		if (approvedIds == null) return null
		const approved = new Set(approvedIds)
		return plan.dependencies
			.map((dependency) => dependency.project_id)
			.filter((projectId) => !approved.has(projectId))
	}

	async function notifySkippedPlanDependencies(
		skipped: Array<{ project_id: string; reason: string }>,
	) {
		if (skipped.length === 0) return
		const projectIds = [...new Set(skipped.map((item) => item.project_id).filter((id) => !!id))]
		const projects = await get_project_many(projectIds).catch(
			() => [] as Labrinth.Projects.v2.Project[],
		)
		const projectsById = new Map(projects.map((candidate) => [candidate.id, candidate]))
		const names = skipped.map((item) => {
			const title = projectsById.get(item.project_id)?.title ?? item.project_id
			const reason = formatMessage(
				skippedReasonMessages[item.reason as keyof typeof skippedReasonMessages] ??
					skippedReasonMessages.already_installed,
			)
			return `${title} (${reason})`
		})
		const list = names.slice(0, 5).join(', ') + (names.length > 5 ? ', …' : '')
		opts.addNotification({
			title: formatMessage(dependenciesSkippedTitleMessage),
			text: formatMessage(dependenciesSkippedMessage, { list }),
			type: 'info',
		})
	}

	function hideContentInstallModal() {
		contentInstallModalOpen = false
		modalRef?.hide()
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
				item.provider_refs.some(
					(reference) => reference.provider === 'curseforge' && reference.project_id === projectId,
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
				websiteUrl: item.websiteUrl,
				projectType: item.projectType,
				projectSlug: item.projectSlug,
				targetFolder: item.targetFolder,
				hashes: item.hashes,
				fileLength: item.fileLength,
				fileFingerprint: item.fileFingerprint,
				ownershipKind: item.ownershipKind,
				operationKind: item.operationKind,
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
		debugState('rememberManualDownloads', {
			instanceId,
			count: manualItems.length,
			items: manualItems.map((m) => m.fileName),
		})
		return manualItems
	}

	function handleCurseForgeManualDownloadsImported(
		instanceId: string,
		imported: CurseForgeManualDownloadImport[],
	) {
		if (imported.length === 0) return
		const importedKeys = new Set(imported.map((item) => `${item.projectId}:${item.fileId}`))
		const remaining = (pendingManualDownloadsByInstance.value.get(instanceId) ?? []).filter(
			(item) => !importedKeys.has(`${item.projectId}:${item.fileId}`),
		)
		setCurseForgeManualDownloads(instanceId, remaining)
		const next = new Map(pendingManualDownloadsByInstance.value)
		if (remaining.length > 0) next.set(instanceId, remaining)
		else next.delete(instanceId)
		pendingManualDownloadsByInstance.value = next
		markInstanceContentChanged(instanceId)
		opts.addNotification({
			title: formatMessage(manualDownloadsImportedTitleMessage),
			text: formatMessage(manualDownloadsImportedMessage, { count: imported.length }),
			type: 'success',
		})
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

	function showFailedCurseForgeDownloads(result: CurseForgeInstallResult) {
		const failedDownloads = result.failedDownloads ?? []
		if (failedDownloads.length === 0) return

		const names = failedDownloads
			.slice(0, 5)
			.map((item) => item.fileName)
			.filter(Boolean)
		const extra = failedDownloads.length - names.length
		const listText =
			extra > 0
				? formatMessage(manualDownloadsListAndMoreMessage, {
						list: names.join(', '),
						count: extra,
					})
				: names.join(', ')
		opts.addNotification({
			title: formatMessage(automaticDownloadsFailedTitleMessage),
			text: formatMessage(automaticDownloadsFailedMessage, {
				failed: failedDownloads.length,
				list: listText,
			}),
			type: 'error',
		})
	}

	function showCurseForgeDependencyNotes(result: CurseForgeInstallResult) {
		const optional = result.optionalDependencies?.length ?? 0
		const incompatible = result.incompatibleDependencies?.length ?? 0
		const skipped = result.skippedDependencies?.length ?? 0
		if (optional === 0 && incompatible === 0 && skipped === 0) return

		opts.addNotification({
			title: formatMessage(dependencyNotesTitleMessage),
			text: formatMessage(dependencyNotesMessage, { optional, incompatible, skipped }),
			type: 'info',
		})
	}

	function handleContentInstallError(error: unknown) {
		const technicalDetails = getCurseForgeDownloadFailureDetails(error)
		if (currentProvider === 'curseforge' && technicalDetails) {
			opts.addNotification({
				title: formatMessage(curseForgeNetworkFailureTitleMessage),
				text: formatMessage(curseForgeNetworkFailureMessage),
				type: 'error',
				supportData: {
					provider: 'curseforge',
					technicalDetails,
				},
			})
			return
		}

		opts.handleError(error)
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
			debugState('installCurrentCFVersion: modpack install', {
				instanceId: instance.id,
				projectId: curseForgeProject.id,
				fileId: file.id,
			})
			result = (
				await installCurseForgeModpack({
					instanceId: instance.id,
					projectId: curseForgeProject.id,
					fileId: file.id,
				})
			).content
		} else {
			debugState('installCurrentCFVersion: file install', {
				instanceId: instance.id,
				projectId: curseForgeProject.id,
				fileId: file.id,
			})
			await removeInstalledCurseForgeProject(instance.id, curseForgeProject.id)
			result = await installCurseForgeFile({
				instanceId: instance.id,
				projectId: curseForgeProject.id,
				fileId: file.id,
				projectType: project.project_type,
				gameVersion: usesTargetGameVersion(project.project_type)
					? instance.game_version
					: undefined,
				modLoaderType: curseForgeLoaderType(instance.loader),
				installDependencies,
			})
		}

		debugState('installCurrentCFVersion: result', {
			instanceId: instance.id,
			installedCount: result.installed.length,
			manualCount: (result.manualDownloads ?? []).length,
			failedCount: (result.failed ?? []).length,
		})

		showManualCurseForgeDownloads(instance.id, result)
		showFailedCurseForgeDownloads(result)
		showCurseForgeDependencyNotes(result)
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
			if (summary.manual === 0 && summary.failed === 0 && summary.installed > 0) {
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

	async function showCurseForgeInstallPreview(
		instance: InstallTargetInstance,
		preview: Awaited<ReturnType<typeof previewCurseForgeFile>>,
	): Promise<number[] | null> {
		if (!contentInstallPreviewModalRef) return []
		const titleById = new Map<number, string>()
		for (const item of [preview.primary, ...preview.dependencies]) {
			titleById.set(item.projectId, item.title)
		}
		const unresolvedProjectIds = [
			...preview.skipped.map((item) => item.projectId),
			...preview.optionalDependencies,
			...preview.incompatibleDependencies,
		].filter((projectId) => !titleById.has(projectId))
		const fetchProjectIds = [
			...new Set([...preview.dependencies.map((item) => item.projectId), ...unresolvedProjectIds]),
		]
		const projectById = new Map<number, CurseForgeProject>()
		if (fetchProjectIds.length > 0) {
			const projects = await getCurseForgeProjects(fetchProjectIds).catch(() => [])
			for (const project of projects) {
				projectById.set(project.id, project)
				titleById.set(project.id, project.name)
			}
		}
		const fallbackProjectById = new Map<string, Labrinth.Projects.v2.Project>()
		const fallbackProjectIds = [
			...new Set((preview.modrinthFallbacks ?? []).map((fallback) => fallback.projectId)),
		]
		if (fallbackProjectIds.length > 0) {
			const projects = await get_project_many(fallbackProjectIds).catch(
				() => [] as Labrinth.Projects.v2.Project[],
			)
			for (const project of projects) fallbackProjectById.set(project.id, project)
		}
		const dependencies = preview.dependencies.map((dependency) => {
			const project = projectById.get(dependency.projectId)
			return {
				id: String(dependency.projectId),
				title: dependency.title,
				iconUrl: dependency.iconUrl ?? null,
				versionNumber: dependency.versionNumber,
				fileName: dependency.fileName,
				description: project?.summary,
				projectUrl: project
					? curseForgePageUrl({
							slug: project.slug,
							links: project.links,
						})
					: undefined,
				requiredBy: dependency.requiredByProjectIds
					.map((projectId) => titleById.get(projectId))
					.filter((title): title is string => !!title),
				alreadyInstalled: false,
				versionMismatch: dependency.versionMismatch ?? false,
				selectionReason:
					dependency.selectionReason === 'sha1_verified_modrinth_fallback'
						? formatMessage(sha1VerifiedModrinthFallbackMessage)
						: undefined,
				required: dependency.required,
			}
		})
		for (const fallback of preview.modrinthFallbacks ?? []) {
			const fallbackProject = fallbackProjectById.get(fallback.projectId)
			dependencies.push({
				id: `modrinth:${fallback.versionId}`,
				title: fallback.title,
				iconUrl: fallback.iconUrl ?? null,
				versionNumber: fallback.versionNumber,
				description: fallbackProject?.description,
				projectUrl: fallbackProject ? projectPageUrl(fallbackProject) : undefined,
				requiredBy: [titleById.get(fallback.parentProjectId) ?? String(fallback.parentProjectId)],
				alreadyInstalled: false,
				selectionReason: formatMessage(sha1VerifiedModrinthFallbackMessage),
				required: fallback.required,
			})
		}
		const skipped = preview.skipped.map((item) => ({
			id: String(item.projectId),
			title: titleById.get(item.projectId) ?? formatMessage(unavailableCurseForgeProjectMessage),
			reason: formatMessage(
				skippedReasonMessages[item.reason as keyof typeof skippedReasonMessages] ??
					skippedReasonMessages.no_compatible_version,
			),
		}))
		for (const projectId of preview.optionalDependencies) {
			skipped.push({
				id: String(projectId),
				title: titleById.get(projectId) ?? formatMessage(unavailableCurseForgeProjectMessage),
				reason: formatMessage(
					skippedReasonMessages.optional ?? skippedReasonMessages.no_compatible_version,
				),
			})
		}
		for (const projectId of preview.incompatibleDependencies) {
			skipped.push({
				id: String(projectId),
				title: titleById.get(projectId) ?? formatMessage(unavailableCurseForgeProjectMessage),
				reason: formatMessage(
					skippedReasonMessages.incompatible ?? skippedReasonMessages.no_compatible_version,
				),
			})
		}
		const approvedIds = await contentInstallPreviewModalRef.show({
			primary: {
				title: preview.primary.title,
				iconUrl: preview.primary.iconUrl,
				versionNumber: preview.primary.versionNumber,
			},
			instanceName: instance.name,
			installDependencies: themeStore.getFeatureFlag('auto_install_dependencies'),
			dependencies,
			skipped,
		})
		if (approvedIds == null) return null
		const approved = new Set(approvedIds)
		const excludedProjectIds = preview.dependencies
			.map((dependency) => dependency.projectId)
			.filter((projectId) => !approved.has(String(projectId)))
		for (const fallback of preview.modrinthFallbacks ?? []) {
			if (!approved.has(`modrinth:${fallback.versionId}`)) {
				excludedProjectIds.push(fallback.parentProjectId)
			}
		}
		return [...new Set(excludedProjectIds)]
	}

	async function queueCurrentCurseForgeVersion(
		instance: InstallTargetInstance,
		project: Labrinth.Projects.v2.Project,
		version: Labrinth.Versions.v2.Version,
		precomputedExcludedProjectIds?: number[],
	) {
		const curseForgeProject = currentCurseForgeProject
		const file = currentCurseForgeFiles.get(version.id)
		if (!curseForgeProject || !file) {
			throw new Error('CurseForge project or file was not loaded')
		}
		if (project.project_type === 'modpack') {
			throw new Error('CurseForge modpacks use the modpack installer')
		}

		await removeInstalledCurseForgeProject(instance.id, curseForgeProject.id)
		const request = {
			instanceId: instance.id,
			projectId: curseForgeProject.id,
			fileId: file.id,
			projectType: project.project_type,
			ownershipKind: 'user_added' as const,
			manualOperationKind: 'content_install' as const,
			gameVersion: usesTargetGameVersion(project.project_type) ? instance.game_version : undefined,
			modLoaderType: curseForgeLoaderType(instance.loader),
			installDependencies: true,
		}
		const excludedProjectIds =
			precomputedExcludedProjectIds ??
			(() => {
				return null as number[] | null
			})()
		if (excludedProjectIds == null) {
			const preview = await previewCurseForgeFile(request)
			const computed = await showCurseForgeInstallPreview(instance, preview)
			if (computed == null) return null
			return await queueCurseForgeFile(
				{
					...request,
					excludedDependencyProjectIds: computed,
					dependencyPlanId: preview.planId,
				},
				{ title: project.title, iconUrl: project.icon_url },
			)
		}
		return await queueCurseForgeFile(
			{ ...request, excludedDependencyProjectIds: excludedProjectIds },
			{ title: project.title, iconUrl: project.icon_url },
		)
	}

	async function queueCurrentCurseForgeWorld(
		instance: InstallTargetInstance & Partial<Pick<GameInstance, 'install_stage'>>,
	) {
		const curseForgeProject = currentCurseForgeProject
		const file = currentWorldFileId ? currentCurseForgeFiles.get(currentWorldFileId) : null
		if (!curseForgeProject || !file || !currentProject) {
			throw new Error(formatMessage(curseForgeWorldUnavailableMessage))
		}
		if (curseForgeProject.classId !== 17) {
			throw new Error(formatMessage(curseForgeWorldInvalidProjectMessage))
		}
		if (instance.install_stage && instance.install_stage !== 'installed') {
			throw new Error(formatMessage(curseForgeWorldInstanceNotReadyMessage))
		}

		return await queueCurseForgeWorld(
			{
				instanceId: instance.id,
				projectId: curseForgeProject.id,
				fileId: file.id,
			},
			{ title: currentProject.title, iconUrl: currentProject.icon_url },
		)
	}

	async function handleInstallToInstance(instance: ContentInstallInstance) {
		const selectedInstance = instanceMap[instance.id]
		const storeInstance = instances.value.find((i) => i.id === instance.id)
		if (!currentProject || !selectedInstance) {
			opts.handleError('No project or instance found')
			settleCurrentCallback()
			return
		}

		if (currentTargetMode === 'world') {
			if (storeInstance) storeInstance.installing = true
			try {
				await queueCurrentCurseForgeWorld(selectedInstance)
				trackEvent('ProjectInstall', {
					loader: selectedInstance.loader,
					game_version: selectedInstance.game_version,
					id: currentProject.id,
					version_id: currentWorldFileId ?? '',
					project_type: 'world',
					title: currentProject.title,
					source: 'ProjectInstallModal',
				})
				settleCurrentCallback(currentWorldFileId ?? undefined, [currentProject.id])
				hideContentInstallModal()
			} catch (err) {
				if (storeInstance) storeInstance.installing = false
				handleContentInstallError(err)
				settleCurrentCallback()
			}
			return
		}

		const version = findPreferredVersion(currentVersions, currentProject, selectedInstance)
		if (!version) {
			if (currentVersions.length > 0 && incompatibilityWarningModalRef) {
				const onIncompatibleInstall = (versionId?: string) => {
					if (versionId && storeInstance) {
						storeInstance.installed = true
					}
					settleCurrentCallback(
						versionId,
						versionId && currentProject ? [currentProject.id] : undefined,
					)
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
				settleCurrentCallback()
			}
			return
		}

		if (currentProvider === 'modrinth') {
			if (storeInstance) storeInstance.installing = true
			try {
				const request = {
					project_id: currentProject.id,
					version_id: version.id,
					content_type: resolveContentType(currentProject.project_type),
				}
				const previewPlan = await preview_project_with_dependencies(instance.id, request)
				const excludedProjectIds = await showInstallPreview(
					selectedInstance,
					currentProject,
					version,
					previewPlan,
				)
				if (excludedProjectIds == null) {
					if (storeInstance) storeInstance.installing = false
					settleCurrentCallback()
					return
				}
				await queue_project_with_dependencies(
					instance.id,
					{ ...request, excluded_project_ids: excludedProjectIds },
					{ title: currentProject.title, iconUrl: currentProject.icon_url },
				)
				trackEvent('ProjectInstall', {
					loader: selectedInstance.loader,
					game_version: selectedInstance.game_version,
					id: currentProject.id,
					version_id: version.id,
					project_type: currentProject.project_type,
					title: currentProject.title,
					source: 'ProjectInstallModal',
				})
				settleCurrentCallback(version.id, [currentProject.id])
				hideContentInstallModal()
			} catch (err) {
				if (storeInstance) storeInstance.installing = false
				handleContentInstallError(err)
				settleCurrentCallback()
			}
			return
		}

		if (currentProvider === 'curseforge') {
			if (storeInstance) storeInstance.installing = true
			try {
				const job = await queueCurrentCurseForgeVersion(selectedInstance, currentProject, version)
				if (!job) {
					if (storeInstance) storeInstance.installing = false
					settleCurrentCallback()
					return
				}
				trackEvent('ProjectInstall', {
					loader: selectedInstance.loader,
					game_version: selectedInstance.game_version,
					id: currentProject.id,
					version_id: version.id,
					project_type: currentProject.project_type,
					title: currentProject.title,
					source: 'ProjectInstallModal',
				})
				settleCurrentCallback(version.id, [currentProject.id])
				hideContentInstallModal()
			} catch (err) {
				if (storeInstance) storeInstance.installing = false
				handleContentInstallError(err)
				settleCurrentCallback()
			}
			return
		}

		if (storeInstance) storeInstance.installing = true

		const installedProjectIds: string[] = [currentProject.id]
		const plannedProjectIds: string[] = [currentProject.id]
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
				throw new Error('Unexpected content provider')
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
			settleCurrentCallback(primaryInstalled ? version.id : undefined, installedProjectIds)
		} catch (err) {
			if (storeInstance) storeInstance.installing = false
			removeInstallingItems(instance.id, plannedProjectIds)
			markInstanceContentInstallFailed(instance.id)
			handleContentInstallError(err)
			settleCurrentCallback()
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
		const instance = incompatibilityWarningInstance
		const project = incompatibilityWarningProject
		const callback = incompatibilityWarningCallback
		if (!instance || !project) return

		incompatibilityWarningInstalling.value = true
		addInstallingItem(instance.id, project, version)
		try {
			if (currentProvider === 'curseforge') {
				const job = await queueCurrentCurseForgeVersion(instance, project, version)
				if (!job) {
					incompatibilityWarningInstalling.value = false
					removeInstallingItems(instance.id, [project.id])
					return
				}
			} else {
				const request = {
					project_id: project.id,
					version_id: version.id,
					content_type: resolveContentType(project.project_type),
				}
				const previewPlan = await preview_project_with_dependencies(instance.id, request)
				const excludedProjectIds = await showInstallPreview(instance, project, version, previewPlan)
				if (excludedProjectIds == null) {
					incompatibilityWarningInstalling.value = false
					removeInstallingItems(instance.id, [project.id])
					return
				}
				await queue_project_with_dependencies(
					instance.id,
					{ ...request, excluded_project_ids: excludedProjectIds },
					{ title: project.title, iconUrl: project.icon_url },
				)
			}
		} catch (err) {
			handleContentInstallError(err)
			incompatibilityWarningInstalling.value = false
			removeInstallingItems(instance.id, [project.id])
			markInstanceContentInstallFailed(instance.id)
			return
		}

		incompatibilityWarningInstalling.value = false
		incompatibilityWarningInstalled = true
		callback(version.id, [project.id])
		markInstanceContentChanged(instance.id)
		incompatibilityWarningModalRef?.hide()
		removeInstallingItems(instance.id, [project.id])

		trackEvent('ProjectInstall', {
			loader: instance.loader ?? '',
			game_version: instance.game_version ?? '',
			id: project.id,
			version_id: version.id,
			project_type: project.project_type,
			title: project.title,
			source: 'ProjectIncompatibilityWarningModal',
		})
	}

	function handleIncompatibilityWarningCancel() {
		if (incompatibilityWarningInstance && !incompatibilityWarningInstalled) {
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
		let excludedModrinthProjectIds: string[] = []
		let excludedCurseForgeProjectIds: number[] = []
		debugState('handleCreateAndInstall', {
			provider: currentProvider,
			projectType: currentProject?.project_type,
			projectTitle: currentProject?.title,
			isCurseforgeModpack:
				currentProvider === 'curseforge' && currentProject?.project_type === 'modpack',
		})
		if (currentProvider === 'modrinth' && currentProject) {
			const installRequest = {
				project_id: currentProject.id,
				version_id: version.id,
				content_type: resolveContentType(currentProject.project_type),
			}
			const previewPlan = await preview_project_with_dependencies_for_target(
				installRequest,
				data.gameVersion,
				data.loader,
			)
			const result = await showInstallPreview(
				{
					id: '',
					name: data.name,
					icon_path: data.iconPath ?? undefined,
					game_version: data.gameVersion,
					loader: data.loader as InstanceLoader,
				},
				currentProject,
				version,
				previewPlan,
			)
			if (result == null) return
			excludedModrinthProjectIds = result
		} else if (
			currentProvider === 'curseforge' &&
			currentProject &&
			currentProject.project_type !== 'modpack' &&
			currentCurseForgeProject &&
			currentCurseForgeFiles.get(version.id)
		) {
			const curseForgeProject = currentCurseForgeProject
			const file = currentCurseForgeFiles.get(version.id)!
			const preview = await previewCurseForgeFile({
				instanceId: '',
				projectId: curseForgeProject.id,
				fileId: file.id,
				projectType: currentProject.project_type,
				ownershipKind: 'user_added',
				manualOperationKind: 'content_install',
				gameVersion: data.gameVersion,
				modLoaderType: curseForgeLoaderType(data.loader),
				installDependencies: true,
			})
			const result = await showCurseForgeInstallPreview(
				{
					id: '',
					name: data.name,
					icon_path: data.iconPath ?? undefined,
					game_version: data.gameVersion,
					loader: data.loader as InstanceLoader,
				},
				preview,
			)
			if (result == null) return
			excludedCurseForgeProjectIds = result
		}
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

			if (currentProvider === 'modrinth') {
				await queue_project_with_dependencies(
					id,
					{
						project_id: currentProject!.id,
						version_id: version.id,
						content_type: resolveContentType(currentProject!.project_type),
						excluded_project_ids: excludedModrinthProjectIds,
					},
					{ title: currentProject!.title, iconUrl: currentProject!.icon_url },
				)
				trackEvent('InstanceCreate', { source: 'ProjectInstallModal' })
				trackEvent('ProjectInstall', {
					loader: data.loader,
					game_version: data.gameVersion,
					id: currentProject!.id,
					version_id: version.id,
					project_type: currentProject!.project_type,
					title: currentProject!.title,
					source: 'ProjectInstallModal',
				})
				settleCurrentCallback(version.id, [currentProject!.id])
				hideContentInstallModal()
				return
			}

			if (currentProvider === 'curseforge' && currentProject!.project_type !== 'modpack') {
				const job = await queueCurrentCurseForgeVersion(
					{
						id,
						name: data.name,
						icon_path: data.iconPath ?? undefined,
						game_version: data.gameVersion,
						loader: data.loader as InstanceLoader,
					},
					currentProject!,
					version,
					excludedCurseForgeProjectIds,
				)
				if (!job) return
				trackEvent('InstanceCreate', { source: 'ProjectInstallModal' })
				trackEvent('ProjectInstall', {
					loader: data.loader,
					game_version: data.gameVersion,
					id: currentProject!.id,
					version_id: version.id,
					project_type: currentProject!.project_type,
					title: currentProject!.title,
					source: 'ProjectInstallModal',
				})
				settleCurrentCallback(version.id, [currentProject!.id])
				hideContentInstallModal()
				return
			}

			addInstallingItem(id, currentProject!, version)

			let installedProjectIds: string[]
			if (currentProvider === 'curseforge') {
				debugState('handleCreateAndInstall: CF path start', {
					instanceId: id,
					projectId: currentProject?.id,
				})
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
				throw new Error('Unexpected content provider')
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

			settleCurrentCallback(version.id, installedProjectIds)
			modalRef?.hide()
		} catch (err) {
			if (createdInstanceId && currentProject) {
				removeInstallingItems(createdInstanceId, [currentProject.id])
				markInstanceContentInstallFailed(createdInstanceId)
			}
			handleContentInstallError(err)
		}
	}

	function handleNavigate(instance: ContentInstallInstance) {
		hideContentInstallModal()
		opts.router.push(`/instance/${encodeURIComponent(instance.id)}`)
	}

	function handleCancel() {
		if (!contentInstallModalOpen) return
		contentInstallModalOpen = false
		settleCurrentCallback()
	}

	async function installInternal(
		projectId: string,
		versionId?: string | null,
		instanceId?: string | null,
		source: string = 'unknown',
		callback: ContentInstallCallback = () => {},
		createInstanceCallback: (instanceId: string) => void = () => {},
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) {
		let modalSessionId: number | null = null
		currentProvider = 'modrinth'
		currentTargetMode = 'content'
		currentWorldFileId = null
		currentCurseForgeProject = null
		currentCurseForgeFiles = new Map()
		const shouldShowInstallTargetModal = !instanceId
		if (shouldShowInstallTargetModal) {
			modalSessionId = await showContentInstallLoading(callback)
		}
		const project: Labrinth.Projects.v2.Project = await get_project(projectId).catch((error) => {
			if (modalSessionId === currentSessionId) {
				hideContentInstallModal()
				settleCurrentCallback()
			}
			throw error
		})
		if (!project) {
			if (modalSessionId === currentSessionId) {
				hideContentInstallModal()
				settleCurrentCallback()
			}
			opts.handleError(`Project not found: '${projectId}'`)
			return
		}
		if (modalSessionId !== null && modalSessionId !== currentSessionId) return
		const requestCallback: ContentInstallCallback =
			modalSessionId === null
				? callback
				: (...args) => settleInstallSession(modalSessionId, ...args)

		if (project.project_type === 'modpack') {
			if (shouldShowInstallTargetModal) hideContentInstallModal()
			const version = versionId ?? project.versions[project.versions.length - 1]
			const packs = await list()
			const existingPack = packs.find((pack) => pack.link?.project_id === project.id)

			if (existingPack && !themeStore.getFeatureFlag('skip_non_essential_warnings')) {
				if (shouldShowInstallTargetModal) hideContentInstallModal()
				pendingModpackInstall = {
					project,
					version,
					source,
					callback: requestCallback,
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
			if (instanceId) createInstanceCallback(instanceId)
			trackEvent('PackInstall', {
				id: project.id,
				version_id: version,
				title: project.title,
				source,
			})
			requestCallback(version)
			return
		}

		if (instanceId) {
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
					if (
						file.provider_refs.some(
							(reference) =>
								reference.provider === 'modrinth' && String(reference.project_id) === project.id,
						)
					) {
						await remove_project(instance.id, path)
					}
				}

				const installRequest = {
					project_id: project.id,
					version_id: version.id,
					content_type: resolveContentType(project.project_type),
				}
				const previewPlan = await preview_project_with_dependencies(instance.id, installRequest)
				const excludedProjectIds = await showInstallPreview(instance, project, version, previewPlan)
				if (excludedProjectIds == null) return
				await queue_project_with_dependencies(
					instance.id,
					{ ...installRequest, excluded_project_ids: excludedProjectIds },
					{ title: project.title, iconUrl: project.icon_url },
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
				callback(version.id, [project.id])
			} else {
				await showIncompatibilityWarning(instance, project, projectVersions, version, callback)
			}
			return
		}

		let versions = (
			(await get_version_many(project.versions)) as Labrinth.Versions.v2.Version[]
		).sort((a, b) => dayjs(b.date_published).valueOf() - dayjs(a.date_published).valueOf())
		if (versionId) versions = versions.filter((v) => v.id === versionId)
		await showModInstallModal(project, versions, callback, hints, true)
	}

	async function installCurseForgeInternal(
		projectId: string,
		versionId?: string | null,
		instanceId?: string | null,
		source: string = 'unknown',
		callback: ContentInstallCallback = () => {},
		createInstanceCallback: (instanceId: string) => void = () => {},
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) {
		let modalSessionId: number | null = null
		currentTargetMode = 'content'
		currentWorldFileId = null
		const numericProjectId = Number(projectId.replace(/^curseforge:/, ''))
		if (!Number.isFinite(numericProjectId)) {
			throw new Error('Invalid CurseForge project ID')
		}
		const shouldShowInstallTargetModal = !instanceId
		if (shouldShowInstallTargetModal) {
			modalSessionId = await showContentInstallLoading(callback)
		}
		const [curseForgeProject, fileResponse] = await Promise.all([
			getCurseForgeProject(numericProjectId),
			getCurseForgeFiles(numericProjectId, { index: 0, pageSize: 50 }),
		]).catch((error) => {
			if (modalSessionId === currentSessionId) {
				hideContentInstallModal()
				settleCurrentCallback()
			}
			throw error
		})
		if (modalSessionId !== null && modalSessionId !== currentSessionId) return
		const requestCallback: ContentInstallCallback =
			modalSessionId === null
				? callback
				: (...args) => settleInstallSession(modalSessionId, ...args)
		const availableFiles = fileResponse.files.filter((file) => file.isAvailable)
		const project = mapCurseForgeProject(curseForgeProject, availableFiles)
		let versions = availableFiles
			.map((file) => mapCurseForgeVersion(file, numericProjectId, project.project_type))
			.sort((a, b) => dayjs(b.date_published).valueOf() - dayjs(a.date_published).valueOf())
		if (versionId) versions = versions.filter((version) => version.id === versionId)
		if (versions.length === 0) {
			if (shouldShowInstallTargetModal) hideContentInstallModal()
			throw new Error('No CurseForge files are available for this project')
		}

		currentProvider = 'curseforge'
		currentProject = project
		currentVersions = versions
		currentCurseForgeProject = curseForgeProject
		currentCurseForgeFiles = new Map(availableFiles.map((file) => [file.id.toString(), file]))

		if (project.project_type === 'modpack') {
			if (shouldShowInstallTargetModal) hideContentInstallModal()
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
					callback: requestCallback,
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
				requestCallback,
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
				const job = await queueCurrentCurseForgeVersion(instance, project, version)
				if (!job) return
				trackEvent('ProjectInstall', {
					loader: instance.loader,
					game_version: instance.game_version,
					id: project.id,
					project_type: project.project_type,
					version_id: version.id,
					title: project.title,
					source,
				})
				callback(version.id, [project.id])
			} else {
				await showIncompatibilityWarning(instance, project, versions, version, callback)
			}
		} else {
			await showModInstallModal(project, versions, callback, hints, true)
		}
	}

	function setCurseForgeWorldInstallState(
		curseForgeProject: CurseForgeProject,
		file: CurseForgeFile,
		callback: ContentInstallCallback,
	) {
		const project = mapCurseForgeProject(curseForgeProject, [file])
		currentProvider = 'curseforge'
		currentTargetMode = 'world'
		currentProject = project
		currentVersions = [mapCurseForgeVersion(file, curseForgeProject.id, project.project_type)]
		currentCurseForgeProject = curseForgeProject
		currentCurseForgeFiles = new Map([[file.id.toString(), file]])
		currentWorldFileId = file.id.toString()
		currentCallback = callback
	}

	async function showCurseForgeWorldInstallModal(
		curseForgeProject: CurseForgeProject,
		file: CurseForgeFile,
		callback: ContentInstallCallback,
	) {
		const sessionId = currentSessionId
		setCurseForgeWorldInstallState(curseForgeProject, file, callback)
		instances.value = []
		compatibleLoaders.value = []
		gameVersions.value = []
		releaseGameVersions.value = new Set()
		preferredLoader.value = null
		preferredGameVersion.value = null
		loading.value = true
		defaultTab.value = 'existing'
		projectInfo.value = null

		await nextTick()
		if (sessionId !== currentSessionId) return
		contentInstallModalOpen = true
		modalRef?.show()
		trackEvent('ProjectInstallStart', { source: 'ProjectInstallModal' })

		try {
			const candidates = (await list()).filter(
				(candidate) => candidate.install_stage === 'installed',
			)
			if (
				sessionId !== currentSessionId ||
				currentTargetMode !== 'world' ||
				currentCurseForgeProject?.id !== curseForgeProject.id ||
				currentWorldFileId !== file.id.toString()
			) {
				return
			}
			const newInstanceMap: Record<string, InstallTargetInstance> = {}
			instances.value = candidates.map((candidate) => {
				newInstanceMap[candidate.id] = candidate
				const displayIcon = getDisplayInstanceIcon(candidate.icon_path, candidate.loader)
				return {
					id: candidate.id,
					name: candidate.name,
					iconUrl: displayIcon.url,
					iconFrameless: displayIcon.frameless,
					installed: false,
					compatible: true,
					installing: false,
				}
			})
			instanceMap = newInstanceMap
		} catch (err) {
			opts.handleError(err)
		} finally {
			loading.value = false
		}
	}

	async function installCurseForgeWorldInternal(
		projectId: string | number,
		fileId?: string | number | null,
		instanceId?: string | null,
		source: string = 'unknown',
		callback: ContentInstallCallback = () => {},
	) {
		const modalSessionId = instanceId ? null : beginInstallSession(callback)
		const requestCallback: ContentInstallCallback =
			modalSessionId === null
				? callback
				: (...args) => settleInstallSession(modalSessionId, ...args)
		const numericProjectId = Number(String(projectId).replace(/^curseforge:/, ''))
		if (!Number.isSafeInteger(numericProjectId) || numericProjectId <= 0) {
			throw new Error(formatMessage(curseForgeWorldInvalidProjectMessage))
		}
		const numericFileId = fileId == null ? null : Number(fileId)
		if (numericFileId != null && (!Number.isSafeInteger(numericFileId) || numericFileId <= 0)) {
			throw new Error(formatMessage(curseForgeWorldUnavailableMessage))
		}

		const curseForgeProject = await getCurseForgeProject(numericProjectId)
		if (curseForgeProject.classId !== 17) {
			throw new Error(formatMessage(curseForgeWorldInvalidProjectMessage))
		}
		const file = numericFileId
			? await getCurseForgeFile(numericProjectId, numericFileId)
			: (await getCurseForgeFiles(numericProjectId, { index: 0, pageSize: 50 })).files.find(
					(candidate) => candidate.isAvailable,
				)
		if (!file?.isAvailable) {
			throw new Error(formatMessage(curseForgeWorldUnavailableMessage))
		}
		if (modalSessionId !== null && modalSessionId !== currentSessionId) return

		if (!instanceId) {
			await showCurseForgeWorldInstallModal(curseForgeProject, file, callback)
			return
		}

		setCurseForgeWorldInstallState(curseForgeProject, file, requestCallback)
		const instance = await get(instanceId)
		if (!instance) throw new Error(formatMessage(curseForgeWorldUnknownInstanceMessage))
		await queueCurrentCurseForgeWorld(instance)
		trackEvent('ProjectInstall', {
			loader: instance.loader,
			game_version: instance.game_version,
			id: currentProject!.id,
			version_id: file.id.toString(),
			project_type: 'world',
			title: currentProject!.title,
			source,
		})
		callback(file.id.toString(), [currentProject!.id])
	}

	function install(
		projectId: string,
		versionId?: string | null,
		instanceId?: string | null,
		source: string = 'unknown',
		callback: ContentInstallCallback = () => {},
		createInstanceCallback: (instanceId: string) => void = () => {},
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) {
		return guardInstallRequest(
			() =>
				installInternal(
					projectId,
					versionId,
					instanceId,
					source,
					callback,
					createInstanceCallback,
					hints,
				),
			callback,
		)
	}

	function installCurseForge(
		projectId: string,
		versionId?: string | null,
		instanceId?: string | null,
		source: string = 'unknown',
		callback: ContentInstallCallback = () => {},
		createInstanceCallback: (instanceId: string) => void = () => {},
		hints?: { preferredLoader?: string; preferredGameVersion?: string; showProjectInfo?: boolean },
	) {
		return guardInstallRequest(
			() =>
				installCurseForgeInternal(
					projectId,
					versionId,
					instanceId,
					source,
					callback,
					createInstanceCallback,
					hints,
				),
			callback,
		)
	}

	function installCurseForgeWorld(
		projectId: string | number,
		fileId?: string | number | null,
		instanceId?: string | null,
		source: string = 'unknown',
		callback: ContentInstallCallback = () => {},
	) {
		return guardInstallRequest(
			() => installCurseForgeWorldInternal(projectId, fileId, instanceId, source, callback),
			callback,
		)
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
		symlinkTarget,
		handleInstallToInstance,
		handleCreateAndInstall,
		handleNavigate,
		handleCancel,
		setContentInstallModal(ref: ModalRef) {
			modalRef = ref
		},
		setContentInstallPreviewModal(ref: InstanceType<typeof ContentInstallPreviewModal> | null) {
			contentInstallPreviewModalRef = ref
		},
		setModpackAlreadyInstalledModal(ref: ModpackAlreadyInstalledModalRef) {
			modpackAlreadyInstalledModalRef = ref
		},
		setCurseForgeManualDownloadsModal(ref: CurseForgeManualDownloadsModalRef) {
			curseForgeManualDownloadsModalRef = ref
		},
		showCurseForgeManualDownloads(instanceId: string, items: CurseForgeManualDownloadItem[]) {
			setCurseForgeManualDownloads(instanceId, items)
			const next = new Map(pendingManualDownloadsByInstance.value)
			next.set(instanceId, items)
			pendingManualDownloadsByInstance.value = next
			curseForgeManualDownloadsModalRef?.show({ items, instanceId })
		},
		handleCurseForgeManualDownloadsImported,
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
					callback()
					throw new Error('Unable to reinstall the CurseForge modpack')
				}
				const gameVersion = selectedVersion.game_versions[0]
				if (!gameVersion) {
					callback()
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
				).catch((error) => {
					callback()
					throw error
				})
				return
			}
			const job = await install_create_modpack_instance({
				type: 'fromVersionId',
				project_id: project.id,
				version_id: version,
				title: project.title,
				icon_url: project.icon_url,
			}).catch((error) => {
				callback()
				throw error
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
			const callback = pendingModpackInstall?.callback
			pendingModpackInstall = null
			callback?.()
			opts.router.push(`/instance/${encodeURIComponent(instanceId)}`)
		},
		handleModpackDuplicateCancel() {
			const callback = pendingModpackInstall?.callback
			pendingModpackInstall = null
			callback?.()
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
		installCurseForgeWorld,
		installingItems,
		pendingManualDownloadsByInstance,
		installRevisionByInstance,
		installFailureRevisionByInstance,
	}
}
