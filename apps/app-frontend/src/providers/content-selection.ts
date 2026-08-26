import type { Labrinth } from '@modrinth/api-client'
import {
	type BrowseInstallPreferences,
	type BrowseSelectedProject,
	createContext,
	defineMessages,
	usesTargetGameVersion,
	useVIntl,
} from '@modrinth/ui'
import { computed, type ComputedRef, type Ref, ref, watch } from 'vue'

import type ContentInstallPreviewModal from '@/components/ui/ContentInstallPreviewModal.vue'
import type {
	ContentInstallPreviewData,
	ContentInstallPreviewDependency,
	ContentInstallPreviewPrimary,
	ContentInstallPreviewSkipped,
} from '@/components/ui/ContentInstallPreviewModal.vue'
import { get_project_many, get_version_many } from '@/helpers/cache.js'
import {
	compareContentIdentities,
	type ContentIdentity,
	contentIdentityFromInput,
	type ContentIdentityInput,
	contentIdentityInputsFromSnapshot,
	resolveContentIdentities,
} from '@/helpers/content-identity'
import {
	type CurseForgeInstallPreview,
	getCurseForgeFile,
	getCurseForgeProjects,
	previewCurseForgeFile,
	queueCurseForgeFile,
	queueCurseForgeWorld,
} from '@/helpers/curseforge'
import {
	get_content_snapshot,
	list as listInstances,
	preview_project_with_dependencies,
	queue_project_with_dependencies,
	type ResolveContentPlan,
} from '@/helpers/instance'
import { getBrowseDefaultInstanceId, setBrowseDefaultInstanceId } from '@/helpers/settings'
import type { GameInstance } from '@/helpers/types'
import { aggregateContentSelectionDependencies } from '@/providers/content-selection-logic'
import type { DownloadManager } from '@/providers/download-manager'
import { useTheming } from '@/store/state'

export type ContentSelectionProvider = 'modrinth' | 'curseforge'
export type ContentSelectionType = 'mod' | 'resourcepack' | 'datapack' | 'shader' | 'world'
export type ContentSelectionState = 'idle' | 'validating' | 'reviewing' | 'queueing' | 'error'

export interface ContentSelectionItem {
	key: string
	provider: ContentSelectionProvider
	projectId: string
	providerProjectId: string
	versionId: string
	contentType: ContentSelectionType
	title: string
	iconUrl?: string | null
	preferences?: BrowseInstallPreferences
	targetInstanceId?: string
	slug?: string | null
	fileName?: string | null
	sha1?: string | null
	identity?: ContentIdentity
}

interface PreparedSelection {
	item: ContentSelectionItem
	primary: ContentInstallPreviewPrimary
	dependencies: ContentInstallPreviewDependency[]
	skipped: ContentInstallPreviewSkipped[]
	modrinthPlan?: ResolveContentPlan
	curseForgePreview?: CurseForgeInstallPreview
}

export interface ContentSelectionContext {
	instances: Ref<GameInstance[]>
	targetInstance: Ref<GameInstance | null>
	items: Ref<Map<string, ContentSelectionItem>>
	selectedProjects: ComputedRef<BrowseSelectedProject[]>
	selectedCount: ComputedRef<number>
	state: Ref<ContentSelectionState>
	progress: Ref<{ completed: number; total: number }>
	errorKeys: Ref<Set<string>>
	refreshInstances: (preferredId?: string | null) => Promise<GameInstance | null>
	refreshInstalledIdentities: () => Promise<void>
	setTarget: (instance: GameInstance | null) => void
	add: (item: ContentSelectionItem) => Promise<boolean>
	remove: (key: string) => void
	clear: () => void
	isSelected: (key: string) => boolean
	isInstalledIdentity: (
		provider: ContentSelectionProvider,
		projectId: string,
		slug?: string | null,
	) => boolean
	isInstalling: (key: string) => boolean
	installSelected: () => Promise<boolean>
	setPreviewModal: (modal: InstanceType<typeof ContentInstallPreviewModal> | null) => void
}

export interface CreateContentSelectionOptions {
	addNotification: (notification: { title: string; type: 'error' }) => void
	handleError: (error: unknown) => void
	downloadManager: DownloadManager
}

export const [injectContentSelection, provideContentSelection] =
	createContext<ContentSelectionContext>('App', 'contentSelection')

const messages = defineMessages({
	previewFailed: {
		id: 'app.content-selection.preview-failed',
		defaultMessage: 'Some selected content could not be prepared. Remove it or try again.',
	},
	queueFailed: {
		id: 'app.content-selection.queue-failed',
		defaultMessage: 'Some content could not be added to the install queue. It remains selected.',
	},
	dependencyConflict: {
		id: 'app.content-selection.dependency-conflict',
		defaultMessage: '{dependency} resolves to conflicting versions in this selection.',
	},
	unknownDependency: {
		id: 'app.content-selection.unknown-dependency',
		defaultMessage: 'Dependency {id}',
	},
	unknownReason: {
		id: 'app.content-selection.unknown-reason',
		defaultMessage: 'Could not be resolved',
	},
	targetChanged: {
		id: 'app.content-selection.target-changed',
		defaultMessage:
			'The selected content belongs to another instance. Switch back or clear it first.',
	},
	duplicateContent: {
		id: 'app.content-selection.duplicate-content',
		defaultMessage: '{project} is already installed or selected from another source.',
	},
	conflictUnavailable: {
		id: 'app.content-selection.conflict-unavailable',
		defaultMessage: 'Could not verify whether this content duplicates another source.',
	},
})

const activeJobStatuses = new Set(['queued', 'running', 'canceling', 'waiting_for_user'])

function curseForgeLoaderType(loader: string): number | undefined {
	if (loader === 'forge') return 1
	if (loader === 'fabric') return 4
	if (loader === 'quilt') return 5
	if (loader === 'neoforge') return 6
	return undefined
}

function toModrinthContentType(contentType: ContentSelectionType): Labrinth.Content.v3.ContentType {
	return contentType as Labrinth.Content.v3.ContentType
}

function dependencyKey(provider: ContentSelectionProvider, projectId: string, versionId: string) {
	return `${provider}:${projectId}:${versionId}`
}

function selectedItemKey(provider: ContentSelectionProvider, projectId: string) {
	return `${provider}:${projectId}`
}

function modrinthProjectUrl(project: Labrinth.Projects.v2.Project): string {
	return `https://modrinth.com/${project.project_type}/${project.slug}`
}

function curseForgeProjectUrl(project: { slug: string; links?: { websiteUrl?: string } }): string {
	if (project.links?.websiteUrl) return project.links.websiteUrl
	return `https://www.curseforge.com/minecraft/mc-mods/${project.slug}`
}

export function createContentSelection({
	addNotification,
	handleError,
	downloadManager,
}: CreateContentSelectionOptions): ContentSelectionContext {
	const { formatMessage } = useVIntl()
	const themeStore = useTheming()
	const instances = ref<GameInstance[]>([])
	const targetInstance = ref<GameInstance | null>(null)
	const items = ref<Map<string, ContentSelectionItem>>(new Map())
	const state = ref<ContentSelectionState>('idle')
	const progress = ref({ completed: 0, total: 0 })
	const errorKeys = ref<Set<string>>(new Set())
	const jobIdsByKey = ref<Map<string, string>>(new Map())
	const installedIdentityCache = new Map<string, ContentIdentity[]>()
	const installedIdentityKeys = ref<Set<string>>(new Set())
	const installedIdentitySlugs = ref<Set<string>>(new Set())
	let installedIdentityRequestId = 0
	const heuristicOverrides = new Set<string>()
	let previewModal: InstanceType<typeof ContentInstallPreviewModal> | null = null

	const selectedProjects = computed<BrowseSelectedProject[]>(() =>
		Array.from(items.value.values()).map((item) => ({
			id: item.key,
			name: item.title,
			iconUrl: item.iconUrl,
		})),
	)
	const selectedCount = computed(() => items.value.size)

	async function refreshInstances(preferredId?: string | null) {
		const available = (await listInstances()).filter(
			(instance) => instance.install_stage === 'installed',
		)
		instances.value = available
		const requestedId = preferredId ?? targetInstance.value?.id ?? getBrowseDefaultInstanceId()
		const current = available.find((instance) => instance.id === targetInstance.value?.id) ?? null
		const selected =
			items.value.size > 0 && current
				? current
				: (available.find((instance) => instance.id === requestedId) ?? null)
		if (selected?.id !== targetInstance.value?.id) {
			installedIdentityRequestId += 1
			installedIdentityCache.clear()
			installedIdentityKeys.value = new Set()
			installedIdentitySlugs.value = new Set()
			heuristicOverrides.clear()
		}
		targetInstance.value = selected
		setBrowseDefaultInstanceId(selected?.id ?? null)
		return selected
	}

	function setTarget(instance: GameInstance | null) {
		if (instance?.id !== targetInstance.value?.id) {
			installedIdentityRequestId += 1
			installedIdentityCache.clear()
			installedIdentityKeys.value = new Set()
			installedIdentitySlugs.value = new Set()
			heuristicOverrides.clear()
		}
		targetInstance.value = instance
		setBrowseDefaultInstanceId(instance?.id ?? null)
	}

	async function resolveSelectionIdentity(item: ContentSelectionItem) {
		let sha1 = item.sha1 ?? undefined
		if (!sha1 && item.provider === 'modrinth') {
			const version = (await get_version_many([item.versionId]).catch(() => []))[0]
			sha1 = version?.files?.[0]?.hashes?.sha1
		}
		if (!sha1 && item.provider === 'curseforge') {
			const file = await getCurseForgeFile(
				Number(item.providerProjectId),
				Number(item.versionId),
			).catch(() => null)
			sha1 = file?.hashes.find((hash) => hash.algo === 1)?.value
		}
		const input: ContentIdentityInput = {
			provider: item.provider,
			projectId: item.providerProjectId,
			contentType: item.contentType,
			slug: item.slug,
			title: item.title,
			fileName: item.fileName,
			sha1,
		}
		return (await resolveContentIdentities([input]))[0] ?? contentIdentityFromInput(input)
	}

	async function getInstalledIdentities(instance: GameInstance) {
		const cached = installedIdentityCache.get(instance.id)
		if (cached) return cached
		const requestId = ++installedIdentityRequestId
		const canCommit = () =>
			requestId === installedIdentityRequestId && targetInstance.value?.id === instance.id
		const snapshot = await get_content_snapshot(instance.id).catch(() => null)
		if (!snapshot) {
			// A stale/offline instance snapshot must not turn an add operation into a
			// false authoritative conflict. Provider-qualified cart de-duplication
			// still applies while the failed lookup remains unknown.
			installedIdentityCache.set(instance.id, [])
			if (canCommit()) {
				installedIdentityKeys.value = new Set()
				installedIdentitySlugs.value = new Set()
			}
			return []
		}
		const unresolvedInputs = contentIdentityInputsFromSnapshot(snapshot.items)
		const modrinthIds = unresolvedInputs
			.filter((input) => input.provider === 'modrinth')
			.map((input) => input.projectId)
		const curseForgeIds = unresolvedInputs
			.filter((input) => input.provider === 'curseforge')
			.map((input) => Number(input.projectId))
			.filter((id) => Number.isSafeInteger(id))
		const [modrinthProjects, curseForgeProjects] = await Promise.all([
			get_project_many([...new Set(modrinthIds)])
				.catch(() => [])
				.then((projects) => (projects ?? []) as Labrinth.Projects.v2.Project[]),
			getCurseForgeProjects([...new Set(curseForgeIds)]).catch(() => []),
		])
		const inputs = contentIdentityInputsFromSnapshot(snapshot.items, {
			modrinth: new Map(
				modrinthProjects.map((project) => [
					project.id,
					{ slug: project.slug, title: project.title },
				]),
			),
			curseforge: new Map(
				curseForgeProjects.map((project) => [
					String(project.id),
					{ slug: project.slug, title: project.name },
				]),
			),
		})
		const identities = await resolveContentIdentities(inputs)
		installedIdentityCache.set(instance.id, identities)
		if (!canCommit()) return identities
		const keys = new Set<string>()
		const slugs = new Set<string>()
		for (const identity of identities) {
			keys.add(selectedItemKey(identity.provider, identity.projectId))
			if (identity.slug) {
				slugs.add(selectedItemKey(identity.provider, identity.slug.toLowerCase()))
			}
			for (const counterpart of identity.counterparts ?? []) {
				keys.add(selectedItemKey(counterpart.provider, counterpart.projectId))
				if (counterpart.slug) {
					slugs.add(selectedItemKey(counterpart.provider, counterpart.slug.toLowerCase()))
				}
			}
		}
		installedIdentityKeys.value = keys
		installedIdentitySlugs.value = slugs
		return identities
	}

	async function refreshInstalledIdentities() {
		if (!targetInstance.value) {
			installedIdentityKeys.value = new Set()
			installedIdentitySlugs.value = new Set()
			return
		}
		installedIdentityCache.delete(targetInstance.value.id)
		await getInstalledIdentities(targetInstance.value)
	}

	async function findConflicts(item: ContentSelectionItem, instance: GameInstance) {
		const candidate = item.identity ?? (await resolveSelectionIdentity(item))
		const existing = [
			...(await Promise.all(
				[...items.value.values()]
					.filter((selected) => selected.key !== item.key)
					.map(async (selected) => ({
						item: selected,
						identity: selected.identity ?? (await resolveSelectionIdentity(selected)),
					})),
			)),
			...(await getInstalledIdentities(instance)).map((identity) => ({ item: null, identity })),
		]
		return existing.flatMap(({ item: existingItem, identity }) => {
			const match = compareContentIdentities(candidate, identity)
			return match ? [{ candidate, existing: existingItem ?? identity, match }] : []
		})
	}

	async function validateSelectionConflicts(instance: GameInstance) {
		const rejected = new Set<string>()
		for (const item of items.value.values()) {
			const conflicts = await findConflicts(item, instance)
			const hardConflict = conflicts.find((conflict) => conflict.match.source !== 'heuristic')
			if (hardConflict) {
				rejected.add(item.key)
				continue
			}
			const heuristicConflict = conflicts.find((conflict) => conflict.match.source === 'heuristic')
			if (!heuristicConflict || !previewModal || !heuristicConflict.existing) continue
			const overrideKey = `${instance.id}:${item.key}:${heuristicConflict.existing.provider}:${heuristicConflict.existing.projectId}`
			if (heuristicOverrides.has(overrideKey)) continue
			const confirmed = await previewModal.showConflict({
				candidate: {
					title: item.title,
					provider: item.provider,
					contentType: item.contentType,
					iconUrl: item.iconUrl,
				},
				existing: [
					{
						title: heuristicConflict.existing.title ?? '',
						provider: heuristicConflict.existing.provider,
						fileName: heuristicConflict.existing.fileName ?? undefined,
					},
				],
				source: 'heuristic',
				confidence: heuristicConflict.match.confidence === 'high' ? 'high' : 'possible',
			})
			if (!confirmed) rejected.add(item.key)
			else heuristicOverrides.add(overrideKey)
		}
		return rejected
	}

	async function add(item: ContentSelectionItem) {
		if (!targetInstance.value) throw new Error('No target instance selected')
		const identity = item.identity ?? (await resolveSelectionIdentity(item))
		const conflicts = await findConflicts({ ...item, identity }, targetInstance.value)
		const hardConflict = conflicts.find((conflict) => conflict.match.source !== 'heuristic')
		if (hardConflict) {
			addNotification({
				title: formatMessage(messages.duplicateContent, { project: item.title }),
				type: 'error',
			})
			return false
		}
		const heuristicConflict = conflicts.find((conflict) => conflict.match.source === 'heuristic')
		if (heuristicConflict) {
			if (!previewModal || !heuristicConflict.existing) {
				addNotification({ title: formatMessage(messages.conflictUnavailable), type: 'error' })
				return false
			}
			const confirmed = await previewModal.showConflict({
				candidate: {
					title: item.title,
					provider: item.provider,
					contentType: item.contentType,
					iconUrl: item.iconUrl,
				},
				existing: [
					{
						title: heuristicConflict.existing.title ?? '',
						provider: heuristicConflict.existing.provider,
						fileName: heuristicConflict.existing.fileName ?? undefined,
					},
				],
				source: 'heuristic',
				confidence: heuristicConflict.match.confidence === 'high' ? 'high' : 'possible',
			})
			if (!confirmed) return false
			for (const conflict of conflicts.filter(
				(candidate) => candidate.match.source === 'heuristic',
			)) {
				if (conflict.existing) {
					heuristicOverrides.add(
						`${targetInstance.value.id}:${item.key}:${conflict.existing.provider}:${conflict.existing.projectId}`,
					)
				}
			}
		}
		const next = new Map(items.value)
		next.set(item.key, {
			...item,
			identity,
			sha1: identity.sha1,
			targetInstanceId: targetInstance.value.id,
		})
		items.value = next
		const nextErrors = new Set(errorKeys.value)
		nextErrors.delete(item.key)
		errorKeys.value = nextErrors
		return true
	}

	function remove(key: string) {
		const next = new Map(items.value)
		next.delete(key)
		items.value = next
		const nextErrors = new Set(errorKeys.value)
		nextErrors.delete(key)
		errorKeys.value = nextErrors
	}

	function clear() {
		items.value = new Map()
		errorKeys.value = new Set()
		installedIdentityRequestId += 1
		installedIdentityCache.clear()
		installedIdentityKeys.value = new Set()
		installedIdentitySlugs.value = new Set()
		heuristicOverrides.clear()
	}

	function isSelected(key: string) {
		return items.value.has(key)
	}

	function isInstalling(key: string) {
		const jobId = jobIdsByKey.value.get(key)
		if (!jobId) return false
		const job = downloadManager.jobs.value.find((candidate) => candidate.job_id === jobId)
		return !job || activeJobStatuses.has(job.status)
	}

	async function prepareModrinth(item: ContentSelectionItem, instance: GameInstance) {
		const request = {
			project_id: item.projectId,
			version_id: item.versionId,
			content_type: toModrinthContentType(item.contentType),
			selected: {
				game_versions: item.preferences?.gameVersions ?? [],
				loaders: item.preferences?.loaders ?? [],
			},
		}
		const plan = await preview_project_with_dependencies(instance.id, request)
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
					...plan.skipped.map((skipped) => skipped.version_id),
				].filter((id): id is string => !!id),
			),
		]
		const [projects, versions] = await Promise.all([
			get_project_many(projectIds)
				.catch(() => [])
				.then((projects) => (projects ?? []) as Labrinth.Projects.v2.Project[]),
			get_version_many(versionIds)
				.catch(() => [])
				.then((versions) => (versions ?? []) as Labrinth.Versions.v2.Version[]),
		])
		const projectsById = new Map(projects.map((project) => [project.id, project]))
		const versionsById = new Map(versions.map((version) => [version.id, version]))
		const primaryVersion = versionsById.get(plan.primary.version_id)
		const titleByVersion = new Map(
			[plan.primary, ...plan.dependencies, ...plan.skipped].flatMap((content) =>
				content.version_id
					? [
							[
								content.version_id,
								projectsById.get(content.project_id)?.title ?? content.project_id,
							],
						]
					: [],
			),
		)
		const selectedProjectIds = new Set(
			[...items.value.values()]
				.filter((selected) => selected.key !== item.key && selected.provider === 'modrinth')
				.map((selected) => selected.projectId),
		)
		const dependencies = plan.dependencies.map((dependency) => {
			const included = selectedProjectIds.has(dependency.project_id)
			const project = projectsById.get(dependency.project_id)
			return {
				id: dependencyKey('modrinth', dependency.project_id, dependency.version_id),
				title: project?.title ?? dependency.project_id,
				iconUrl: project?.icon_url,
				versionNumber: versionsById.get(dependency.version_id)?.version_number,
				fileName: versionsById.get(dependency.version_id)?.files[0]?.filename,
				description: project?.description,
				projectUrl: project ? modrinthProjectUrl(project) : undefined,
				requiredBy: dependency.dependent_on_version_id
					? [titleByVersion.get(dependency.dependent_on_version_id)].filter(
							(title): title is string => !!title,
						)
					: [item.title],
				requiredByKeys: [item.key],
				alreadyInstalled: included,
				status: included ? ('included' as const) : undefined,
				required: dependency.required,
			}
		})
		for (const skipped of plan.skipped) {
			if (skipped.reason !== 'already_installed') continue
			const versionId = skipped.version_id ?? `skipped-${skipped.project_id}`
			const project = projectsById.get(skipped.project_id)
			dependencies.push({
				id: dependencyKey('modrinth', skipped.project_id, versionId),
				title: project?.title ?? skipped.project_id,
				iconUrl: project?.icon_url,
				versionNumber: skipped.version_id
					? versionsById.get(skipped.version_id)?.version_number
					: undefined,
				fileName: skipped.version_id
					? versionsById.get(skipped.version_id)?.files[0]?.filename
					: undefined,
				description: project?.description,
				projectUrl: project ? modrinthProjectUrl(project) : undefined,
				requiredBy: skipped.dependent_on_version_id
					? [titleByVersion.get(skipped.dependent_on_version_id)].filter(
							(title): title is string => !!title,
						)
					: [item.title],
				requiredByKeys: [item.key],
				alreadyInstalled: true,
				status: 'installed' as const,
				required: true,
			})
		}

		return {
			item,
			primary: {
				key: item.key,
				title: item.title,
				iconUrl: item.iconUrl,
				versionNumber: primaryVersion?.version_number,
				provider: 'Modrinth',
				contentType: item.contentType,
				removable: true,
			},
			dependencies,
			skipped: plan.skipped
				.filter((skipped) => skipped.reason !== 'already_installed')
				.map((skipped) => ({
					id: `${item.key}:skipped:${skipped.project_id}`,
					title: projectsById.get(skipped.project_id)?.title ?? skipped.project_id,
					reason: skipped.reason.replaceAll('_', ' '),
					requiredByKeys: [item.key],
				})),
			modrinthPlan: plan,
		} satisfies PreparedSelection
	}

	async function prepareCurseForge(item: ContentSelectionItem, instance: GameInstance) {
		const projectId = Number(item.providerProjectId)
		const fileId = Number(item.versionId)
		if (!Number.isSafeInteger(projectId) || !Number.isSafeInteger(fileId)) {
			throw new Error('Invalid CurseForge project or file ID')
		}
		if (item.contentType === 'world') {
			return {
				item,
				primary: {
					key: item.key,
					title: item.title,
					iconUrl: item.iconUrl,
					versionNumber: item.versionId,
					provider: 'CurseForge',
					contentType: item.contentType,
					removable: true,
				},
				dependencies: [],
				skipped: [],
			} satisfies PreparedSelection
		}

		const preview = await previewCurseForgeFile({
			instanceId: instance.id,
			projectId,
			fileId,
			projectType: item.contentType,
			ownershipKind: 'user_added',
			manualOperationKind: 'content_install',
			gameVersion: usesTargetGameVersion(item.contentType) ? instance.game_version : undefined,
			modLoaderType: curseForgeLoaderType(instance.loader),
			installDependencies: true,
		})
		const titleById = new Map<number, string>()
		for (const candidate of [preview.primary, ...preview.dependencies]) {
			titleById.set(candidate.projectId, candidate.title)
		}
		const missingIds = [
			...preview.skipped.map((skipped) => skipped.projectId),
			...preview.optionalDependencies,
			...preview.incompatibleDependencies,
		].filter((id) => !titleById.has(id))
		if (missingIds.length) {
			const projects = await getCurseForgeProjects([...new Set(missingIds)]).catch(() => [])
			for (const project of projects) titleById.set(project.id, project.name)
		}
		const dependencyProjectIds = [
			...new Set([
				...preview.dependencies.map((dependency) => dependency.projectId),
				...preview.skipped
					.filter((skipped) => skipped.reason === 'already_installed')
					.map((skipped) => skipped.projectId),
			]),
		]
		const projectById = new Map<number, { summary: string; slug: string; websiteUrl?: string }>()
		if (dependencyProjectIds.length) {
			const projects = await getCurseForgeProjects(dependencyProjectIds).catch(() => [])
			for (const project of projects) {
				projectById.set(project.id, {
					summary: project.summary,
					slug: project.slug,
					websiteUrl: project.links?.websiteUrl,
				})
			}
		}
		const fallbackProjectsById = new Map<string, Labrinth.Projects.v2.Project>()
		const fallbackProjectIds = [
			...new Set((preview.modrinthFallbacks ?? []).map((fallback) => fallback.projectId)),
		]
		if (fallbackProjectIds.length) {
			const projects = await get_project_many(fallbackProjectIds)
				.catch(() => [])
				.then((projects) => (projects ?? []) as Labrinth.Projects.v2.Project[])
			for (const project of projects) fallbackProjectsById.set(project.id, project)
		}
		const dependencies: ContentInstallPreviewDependency[] = preview.dependencies.map(
			(dependency) => {
				const included = [...items.value.values()].some(
					(selected) =>
						selected.key !== item.key &&
						selected.provider === 'curseforge' &&
						selected.providerProjectId === String(dependency.projectId),
				)
				const project = projectById.get(dependency.projectId)
				return {
					id: dependencyKey('curseforge', String(dependency.projectId), String(dependency.fileId)),
					title: dependency.title,
					iconUrl: dependency.iconUrl,
					versionNumber: dependency.versionNumber,
					fileName: dependency.fileName,
					description: project?.summary,
					projectUrl: project ? curseForgeProjectUrl(project) : undefined,
					requiredBy: dependency.requiredByProjectIds
						.map((id) => titleById.get(id))
						.filter((title): title is string => !!title),
					requiredByKeys: [item.key],
					alreadyInstalled: included,
					status: included ? ('included' as const) : undefined,
					versionMismatch: dependency.versionMismatch,
					required: dependency.required,
				}
			},
		)
		for (const skippedItem of preview.skipped) {
			if (skippedItem.reason !== 'already_installed') continue
			const projectId = String(skippedItem.projectId)
			const project = projectById.get(skippedItem.projectId)
			dependencies.push({
				id: dependencyKey('curseforge', projectId, String(skippedItem.fileId ?? 'skipped')),
				title: titleById.get(skippedItem.projectId) ?? projectId,
				description: project?.summary,
				projectUrl: project ? curseForgeProjectUrl(project) : undefined,
				requiredBy: [item.title],
				requiredByKeys: [item.key],
				alreadyInstalled: true,
				status: 'installed',
				required: true,
			})
		}
		for (const fallback of preview.modrinthFallbacks ?? []) {
			const included = [...items.value.values()].some(
				(selected) =>
					selected.key !== item.key &&
					selected.provider === 'modrinth' &&
					selected.projectId === fallback.projectId,
			)
			const fallbackProject = fallbackProjectsById.get(fallback.projectId)
			dependencies.push({
				id: dependencyKey('modrinth', fallback.projectId, fallback.versionId),
				title: fallback.title,
				iconUrl: fallback.iconUrl,
				versionNumber: fallback.versionNumber,
				description: fallbackProject?.description,
				projectUrl: fallbackProject ? modrinthProjectUrl(fallbackProject) : undefined,
				requiredBy: [titleById.get(fallback.parentProjectId) ?? item.title],
				requiredByKeys: [item.key],
				alreadyInstalled: included,
				status: included ? ('included' as const) : undefined,
				required: fallback.required,
			})
		}
		const skipped: ContentInstallPreviewSkipped[] = preview.skipped
			.filter((skippedItem) => skippedItem.reason !== 'already_installed')
			.map((skippedItem) => ({
				id: `${item.key}:skipped:${skippedItem.projectId}`,
				title:
					titleById.get(skippedItem.projectId) ??
					formatMessage(messages.unknownDependency, { id: skippedItem.projectId }),
				reason: skippedItem.reason || formatMessage(messages.unknownReason),
				requiredByKeys: [item.key],
			}))
		for (const projectId of preview.optionalDependencies) {
			skipped.push({
				id: `${item.key}:optional:${projectId}`,
				title:
					titleById.get(projectId) ?? formatMessage(messages.unknownDependency, { id: projectId }),
				reason: 'optional',
				requiredByKeys: [item.key],
			})
		}
		for (const projectId of preview.incompatibleDependencies) {
			skipped.push({
				id: `${item.key}:incompatible:${projectId}`,
				title:
					titleById.get(projectId) ?? formatMessage(messages.unknownDependency, { id: projectId }),
				reason: 'incompatible',
				requiredByKeys: [item.key],
			})
		}

		return {
			item,
			primary: {
				key: item.key,
				title: preview.primary.title,
				iconUrl: preview.primary.iconUrl ?? item.iconUrl,
				versionNumber: preview.primary.versionNumber,
				provider: 'CurseForge',
				contentType: item.contentType,
				removable: true,
			},
			dependencies,
			skipped,
			curseForgePreview: preview,
		} satisfies PreparedSelection
	}

	function mergePreview(prepared: PreparedSelection[], instance: GameInstance) {
		const { dependencies, conflicts, conflictIdentities } = aggregateContentSelectionDependencies(
			prepared.map((selection) => ({
				ownerKey: selection.item.key,
				dependencies: selection.dependencies,
			})),
			(dependency) => formatMessage(messages.dependencyConflict, { dependency: dependency.title }),
		)

		return {
			primaries: prepared.map((selection) => ({
				...selection.primary,
				error: conflicts.get(selection.item.key),
				conflictIdentities: conflictIdentities.get(selection.item.key),
			})),
			instanceName: instance.name,
			installDependencies: themeStore.getFeatureFlag('auto_install_dependencies'),
			dependencies,
			skipped: prepared.flatMap((selection) => selection.skipped),
		} satisfies ContentInstallPreviewData
	}

	async function queuePrepared(
		selection: PreparedSelection,
		instance: GameInstance,
		approvedIds: Set<string>,
	) {
		if (selection.item.provider === 'modrinth') {
			const plan = selection.modrinthPlan
			if (!plan) throw new Error('Missing Modrinth install preview')
			const excludedProjectIds = plan.dependencies
				.filter(
					(dependency) =>
						!approvedIds.has(
							dependencyKey('modrinth', dependency.project_id, dependency.version_id),
						),
				)
				.map((dependency) => dependency.project_id)
			const forceProjectIds = plan.skipped
				.filter(
					(skipped) =>
						skipped.reason === 'already_installed' &&
						!!skipped.version_id &&
						approvedIds.has(dependencyKey('modrinth', skipped.project_id, skipped.version_id)),
				)
				.map((skipped) => skipped.project_id)
			return await queue_project_with_dependencies(
				instance.id,
				{
					project_id: selection.item.projectId,
					version_id: selection.item.versionId,
					content_type: toModrinthContentType(selection.item.contentType),
					selected: {
						game_versions: selection.item.preferences?.gameVersions ?? [],
						loaders: selection.item.preferences?.loaders ?? [],
					},
					excluded_project_ids: excludedProjectIds,
					force_project_ids: forceProjectIds,
				},
				{ title: selection.item.title, iconUrl: selection.item.iconUrl },
			)
		}

		const projectId = Number(selection.item.providerProjectId)
		const fileId = Number(selection.item.versionId)
		if (selection.item.contentType === 'world') {
			return await queueCurseForgeWorld(
				{ instanceId: instance.id, projectId, fileId },
				{ title: selection.item.title, iconUrl: selection.item.iconUrl },
			)
		}
		const preview = selection.curseForgePreview
		if (!preview) throw new Error('Missing CurseForge install preview')
		const excludedDependencyProjectIds = preview.dependencies
			.filter(
				(dependency) =>
					!approvedIds.has(
						dependencyKey('curseforge', String(dependency.projectId), String(dependency.fileId)),
					),
			)
			.map((dependency) => dependency.projectId)
		const forceDependencyProjectIds = preview.skipped
			.filter(
				(skipped) =>
					skipped.reason === 'already_installed' &&
					approvedIds.has(
						dependencyKey(
							'curseforge',
							String(skipped.projectId),
							String(skipped.fileId ?? 'skipped'),
						),
					),
			)
			.map((skipped) => skipped.projectId)
		for (const fallback of preview.modrinthFallbacks ?? []) {
			if (!approvedIds.has(dependencyKey('modrinth', fallback.projectId, fallback.versionId))) {
				excludedDependencyProjectIds.push(fallback.parentProjectId)
			}
		}
		return await queueCurseForgeFile(
			{
				instanceId: instance.id,
				projectId,
				fileId,
				projectType: selection.item.contentType,
				ownershipKind: 'user_added',
				manualOperationKind: 'content_install',
				gameVersion: usesTargetGameVersion(selection.item.contentType)
					? instance.game_version
					: undefined,
				modLoaderType: curseForgeLoaderType(instance.loader),
				installDependencies: true,
				excludedDependencyProjectIds: [...new Set(excludedDependencyProjectIds)],
				forceDependencyProjectIds: [...new Set(forceDependencyProjectIds)],
			},
			{ title: selection.item.title, iconUrl: selection.item.iconUrl },
		)
	}

	async function installSelected() {
		const instance = targetInstance.value
		if (!instance || items.value.size === 0 || !previewModal) return false
		if ([...items.value.values()].some((item) => item.targetInstanceId !== instance.id)) {
			addNotification({ title: formatMessage(messages.targetChanged), type: 'error' })
			return false
		}
		// The instance may have changed after items were added to the cart.
		// Force the final check to observe the current snapshot.
		installedIdentityCache.delete(instance.id)
		installedIdentityKeys.value = new Set()
		installedIdentitySlugs.value = new Set()
		const conflictKeys = await validateSelectionConflicts(instance)
		if (conflictKeys.size) {
			errorKeys.value = conflictKeys
			state.value = 'error'
			addNotification({
				title: formatMessage(messages.duplicateContent, {
					project:
						[...items.value.values()].find((item) => conflictKeys.has(item.key))?.title ?? '',
				}),
				type: 'error',
			})
			return false
		}
		state.value = 'validating'
		progress.value = { completed: 0, total: items.value.size }
		const prepared: PreparedSelection[] = []
		const failed = new Set<string>()
		for (const item of items.value.values()) {
			try {
				prepared.push(
					item.provider === 'modrinth'
						? await prepareModrinth(item, instance)
						: await prepareCurseForge(item, instance),
				)
			} catch (error) {
				failed.add(item.key)
				handleError(error)
			}
		}
		errorKeys.value = failed
		if (prepared.length === 0) {
			state.value = 'error'
			addNotification({
				title: formatMessage(messages.previewFailed),
				type: 'error',
			})
			return false
		}

		state.value = 'reviewing'
		const result = await previewModal.showBatch(mergePreview(prepared, instance))
		if (!result) {
			state.value = failed.size ? 'error' : 'idle'
			return false
		}
		const includedKeys = new Set(result.primaryKeys)
		const approvedIds = new Set(result.approvedIds)
		for (const selection of prepared) {
			if (!includedKeys.has(selection.item.key)) remove(selection.item.key)
		}
		state.value = 'queueing'
		progress.value = { completed: 0, total: includedKeys.size }
		const queueFailures = new Set(failed)
		for (const selection of prepared.filter((candidate) => includedKeys.has(candidate.item.key))) {
			try {
				const job = await queuePrepared(selection, instance, approvedIds)
				installedIdentityCache.delete(instance.id)
				installedIdentityKeys.value = new Set()
				installedIdentitySlugs.value = new Set()
				const nextJobs = new Map(jobIdsByKey.value)
				nextJobs.set(selection.item.key, job.job_id)
				jobIdsByKey.value = nextJobs
				remove(selection.item.key)
			} catch (error) {
				queueFailures.add(selection.item.key)
				handleError(error)
			}
			progress.value = {
				completed: progress.value.completed + 1,
				total: progress.value.total,
			}
		}
		errorKeys.value = queueFailures
		heuristicOverrides.clear()
		state.value = queueFailures.size ? 'error' : 'idle'
		if (queueFailures.size) {
			addNotification({
				title: formatMessage(messages.queueFailed),
				type: 'error',
			})
		}
		return queueFailures.size === 0
	}

	watch(
		() => downloadManager.jobs.value,
		(jobs) => {
			const terminalJobIds = new Set(
				jobs.filter((job) => !activeJobStatuses.has(job.status)).map((job) => job.job_id),
			)
			if (!terminalJobIds.size) return
			const next = new Map(jobIdsByKey.value)
			for (const [key, jobId] of next) {
				if (terminalJobIds.has(jobId)) next.delete(key)
			}
			jobIdsByKey.value = next
		},
		{ deep: true },
	)

	return {
		instances,
		targetInstance,
		items,
		selectedProjects,
		selectedCount,
		state,
		progress,
		errorKeys,
		refreshInstances,
		refreshInstalledIdentities,
		setTarget,
		add,
		remove,
		clear,
		isSelected,
		isInstalledIdentity(provider, projectId, slug) {
			return (
				installedIdentityKeys.value.has(selectedItemKey(provider, projectId)) ||
				(!!slug && installedIdentitySlugs.value.has(selectedItemKey(provider, slug.toLowerCase())))
			)
		},
		isInstalling,
		installSelected,
		setPreviewModal(modal) {
			previewModal = modal
		},
	}
}

export function makeContentSelectionKey(provider: ContentSelectionProvider, projectId: string) {
	return selectedItemKey(provider, projectId)
}
