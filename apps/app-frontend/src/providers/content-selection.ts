import type { Labrinth } from '@modrinth/api-client'
import { type BrowseSelectedProject, usesTargetGameVersion, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import type ContentInstallPreviewModal from '@/components/ui/ContentInstallPreviewModal.vue'
import type { ContentInstallPreviewData } from '@/components/ui/ContentInstallPreviewModal.vue'
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
	getCurseForgeFile,
	getCurseForgeProjects,
	previewCurseForgeFile,
	queueCurseForgeFile,
	queueCurseForgeWorld,
} from '@/helpers/curseforge'
import {
	get_content_snapshot,
	type InstallContentBatchItem,
	list as listInstances,
	preview_project_with_dependencies,
	queue_content_batch,
	queue_project_with_dependencies,
} from '@/helpers/instance'
import { getBrowseDefaultInstanceId, setBrowseDefaultInstanceId } from '@/helpers/settings'
import type { GameInstance } from '@/helpers/types'
import { aggregateContentSelectionDependencies } from '@/providers/content-selection-logic'
import {
	type ContentSelectionContext,
	type ContentSelectionItem,
	contentSelectionMessages as messages,
	type ContentSelectionProvider,
	type ContentSelectionState,
	type ContentSelectionType,
	type CreateContentSelectionOptions,
	curseForgeLoaderType,
	injectContentSelection,
	type PreparedSelection,
	provideContentSelection,
	toModrinthContentType,
} from '@/providers/content-selection-types'
import { useTheming } from '@/store/state'

export {
	type ContentSelectionContext,
	type ContentSelectionItem,
	type ContentSelectionProvider,
	type ContentSelectionState,
	type ContentSelectionType,
	type CreateContentSelectionOptions,
	injectContentSelection,
	provideContentSelection,
}

export const activeJobStatuses = new Set(['queued', 'running', 'canceling', 'waiting_for_user'])

function dependencyKey(provider: ContentSelectionProvider, projectId: string, versionId: string) {
	return `${provider}:${projectId}:${versionId}`
}

function selectedItemKey(provider: ContentSelectionProvider, projectId: string) {
	return `${provider}:${projectId}`
}

// Install jobs are scoped to an instance. Keep the project key provider-qualified,
// but include the target instance when tracking the queued job so installing the
// same project in another instance does not lock its action.
function installJobKey(instanceId: string, itemKey: string) {
	return `${instanceId}\0${itemKey}`
}

function modrinthProjectUrl(project: {
	slug?: string | null
	project_type?: string | null
	id: string
}): string {
	if (project.slug && project.project_type)
		return `https://modrinth.com/${project.project_type}/${project.slug}`
	return `https://modrinth.com/mod/${project.id}`
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

	async function findConflicts(
		item: ContentSelectionItem,
		instance: GameInstance,
		{ skipInstalled = false }: { skipInstalled?: boolean } = {},
	) {
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
			...(skipInstalled
				? []
				: (await getInstalledIdentities(instance)).map((identity) => ({ item: null, identity }))),
		]
		return existing.flatMap(({ item: existingItem, identity }) => {
			const match = compareContentIdentities(candidate, identity)
			return match ? [{ candidate, existing: existingItem ?? identity, match }] : []
		})
	}

	async function validateSelectionConflicts(instance: GameInstance, preparedKeys?: Set<string>) {
		const rejected = new Set<string>()
		for (const item of items.value.values()) {
			if (preparedKeys && !preparedKeys.has(item.key)) continue
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
		const versionPending = !item.versionId
		const identity =
			item.identity ??
			(versionPending
				? contentIdentityFromInput({
						provider: item.provider,
						projectId: item.providerProjectId,
						contentType: item.contentType,
						slug: item.slug,
						title: item.title,
						fileName: item.fileName,
					})
				: await resolveSelectionIdentity(item))
		const conflicts = await findConflicts({ ...item, identity }, targetInstance.value, {
			skipInstalled: versionPending,
		})
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
			versionPending,
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

	const hasPendingVersions = computed(() =>
		[...items.value.values()].some((item) => item.versionPending),
	)

	function updateVersion(
		key: string,
		versionId: string,
		preferences?: BrowseInstallPreferences,
		sha1?: string,
	) {
		const current = items.value.get(key)
		if (!current) return
		const next = new Map(items.value)
		const updated: ContentSelectionItem = {
			...current,
			versionId,
			versionPending: false,
			preferences: preferences ?? current.preferences,
		}
		if (sha1) {
			updated.sha1 = sha1
		}
		next.set(key, updated)
		items.value = next
	}

	function isSelected(key: string) {
		return items.value.has(key)
	}

	function isInstalling(key: string) {
		const instanceId = targetInstance.value?.id
		if (!instanceId) return false
		const jobId = jobIdsByKey.value.get(installJobKey(instanceId, key))
		if (!jobId) return false
		const job = downloadManager.jobs.value.find((candidate) => candidate.job_id === jobId)
		return !job || activeJobStatuses.has(job.status)
	}

	async function prepareModrinth(item: ContentSelectionItem, instance: GameInstance) {
		const request = {
			project_id: item.projectId,
			version_id: item.versionId || null,
			content_type: toModrinthContentType(item.contentType),
			selected: {
				game_versions: item.preferences?.gameVersions ?? [],
				loaders: item.preferences?.loaders ?? [],
			},
		}
		const plan = await preview_project_with_dependencies(instance.id, request)

		// Rich metadata is attached by the backend - no extra HTTP requests needed
		const titleByVersion = new Map(
			[plan.primary, ...plan.dependencies, ...plan.skipped].flatMap((content) =>
				content.version_id
					? [[content.version_id, content.metadata?.title ?? content.project_id]]
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
			const meta = dependency.metadata
			return {
				id: dependencyKey('modrinth', dependency.project_id, dependency.version_id),
				title: meta?.title ?? dependency.project_id,
				iconUrl: meta?.icon_url,
				versionNumber: meta?.version_number,
				fileName: meta?.filename,
				description: undefined,
				projectUrl: modrinthProjectUrl({
					id: dependency.project_id,
					slug: undefined,
					project_type: undefined,
				}),
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
			dependencies.push({
				id: dependencyKey('modrinth', skipped.project_id, versionId),
				title: skipped.project_id,
				iconUrl: undefined,
				versionNumber: undefined,
				fileName: undefined,
				description: undefined,
				projectUrl: modrinthProjectUrl({
					id: skipped.project_id,
					slug: undefined,
					project_type: undefined,
				}),
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

		if (!item.versionId) {
			updateVersion(
				item.key,
				plan.primary.version_id,
				item.preferences,
				plan.primary.metadata?.sha1,
			)
		}

		return {
			item,
			primary: {
				key: item.key,
				title: item.title,
				iconUrl: item.iconUrl,
				versionNumber: plan.primary.metadata?.version_number,
				provider: 'Modrinth',
				contentType: item.contentType,
				removable: true,
			},
			dependencies,
			skipped: plan.skipped
				.filter((skipped) => skipped.reason !== 'already_installed')
				.map((skipped) => ({
					id: `${item.key}:skipped:${skipped.project_id}`,
					title: skipped.project_id,
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

	async function _queuePrepared(
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

	async function buildCurseForgeRequest(
		selection: PreparedSelection,
		instance: GameInstance,
		approvedIds: Set<string>,
	) {
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
		return {
			instanceId: instance.id,
			projectId: Number(selection.item.providerProjectId),
			fileId: Number(selection.item.versionId),
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
		}
	}

	async function installSelected() {
		const instance = targetInstance.value
		if (!instance || items.value.size === 0 || !previewModal) return false
		if ([...items.value.values()].some((item) => item.targetInstanceId !== instance.id)) {
			addNotification({ title: formatMessage(messages.targetChanged), type: 'error' })
			return false
		}
		// Reset browse-page installed indicators so they reflect the post-install state.
		// The identity cache itself is kept warm from the initial instance selection.
		installedIdentityKeys.value = new Set()
		installedIdentitySlugs.value = new Set()
		state.value = 'validating'
		progress.value = { completed: 0, total: items.value.size }
		const prepared: PreparedSelection[] = []
		const failed = new Set<string>()
		const preparedKeys = new Set<string>()
		const errors: unknown[] = []
		const itemsArray = [...items.value.values()]
		const PREPARE_CONCURRENCY = 4
		for (let i = 0; i < itemsArray.length; i += PREPARE_CONCURRENCY) {
			const batch = itemsArray.slice(i, i + PREPARE_CONCURRENCY)
			const results = await Promise.allSettled(
				batch.map(async (item) => {
					try {
						return item.provider === 'modrinth'
							? await prepareModrinth(item, instance)
							: await prepareCurseForge(item, instance)
					} catch (error) {
						failed.add(item.key)
						errors.push(error)
						return null
					}
				}),
			)
			for (const result of results) {
				if (result.status === 'fulfilled' && result.value !== null) {
					prepared.push(result.value)
					preparedKeys.add(result.value.item.key)
				}
			}
		}
		for (const error of errors) {
			handleError(error)
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

		const conflictKeys = await validateSelectionConflicts(instance, preparedKeys)
		if (conflictKeys.size) {
			errorKeys.value = new Set([...errorKeys.value, ...conflictKeys])
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
		const included = prepared.filter((candidate) => includedKeys.has(candidate.item.key))
		try {
			const batchItems: InstallContentBatchItem[] = []
			for (const selection of included) {
				if (selection.item.provider === 'modrinth') {
					const plan = selection.modrinthPlan
					if (!plan) throw new Error('Missing Modrinth install preview')
					batchItems.push({
						type: 'modrinth',
						project_id: selection.item.projectId,
						version_id: selection.item.versionId,
						content_type: toModrinthContentType(selection.item.contentType),
						selected: {
							game_versions: selection.item.preferences?.gameVersions ?? [],
							loaders: selection.item.preferences?.loaders ?? [],
						},
						excluded_project_ids: plan.dependencies
							.filter(
								(dependency) =>
									!approvedIds.has(
										dependencyKey('modrinth', dependency.project_id, dependency.version_id),
									),
							)
							.map((dependency) => dependency.project_id),
						force_project_ids: plan.skipped
							.filter(
								(skipped) =>
									skipped.reason === 'already_installed' &&
									!!skipped.version_id &&
									approvedIds.has(
										dependencyKey('modrinth', skipped.project_id, skipped.version_id),
									),
							)
							.map((skipped) => skipped.project_id),
					})
				} else if (selection.item.contentType === 'world') {
					batchItems.push({
						type: 'curse_forge_world',
						request: {
							instanceId: instance.id,
							projectId: Number(selection.item.providerProjectId),
							fileId: Number(selection.item.versionId),
						},
					})
				} else {
					const request = await buildCurseForgeRequest(selection, instance, approvedIds)
					batchItems.push({ type: 'curse_forge', request })
				}
			}
			const job = await queue_content_batch(instance.id, batchItems, {
				title: included.length === 1 ? included[0].item.title : `${included.length} items`,
				iconUrl: included.length === 1 ? included[0].item.iconUrl : null,
			})
			const nextJobs = new Map(jobIdsByKey.value)
			for (const selection of included) {
				nextJobs.set(installJobKey(instance.id, selection.item.key), job.job_id)
				remove(selection.item.key)
			}
			jobIdsByKey.value = nextJobs
		} catch (error) {
			for (const selection of included) queueFailures.add(selection.item.key)
			handleError(error)
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
		() => downloadManager.jobs.value.map(({ job_id, status }) => ({ job_id, status })),
		(jobs) => {
			const terminalJobIds = new Set(
				jobs.filter((job) => !activeJobStatuses.has(job.status)).map((job) => job.job_id),
			)
			if (!terminalJobIds.size) return
			let next: Map<string, string> | undefined
			for (const [key, jobId] of jobIdsByKey.value) {
				if (!terminalJobIds.has(jobId)) continue
				next ??= new Map(jobIdsByKey.value)
				next.delete(key)
			}
			if (next) jobIdsByKey.value = next
		},
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
		hasPendingVersions,
		updateVersion,
		installSelected,
		setPreviewModal(modal) {
			previewModal = modal
		},
	}
}

export function makeContentSelectionKey(provider: ContentSelectionProvider, projectId: string) {
	return selectedItemKey(provider, projectId)
}
