/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank instance object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import type { Labrinth } from '@modrinth/api-client'
import type { ContentItem, ContentOwner } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'

import { isOfflineMode } from '@/composables/useNetworkStatus'
import { buildGcCandidateChain, resolveAutoGcStrategy } from '@/helpers/gc/auto-selector'
import { collectGcContext } from '@/helpers/gc/context'
import { detectGcStrategy, GC_STRATEGY_DEFINITIONS } from '@/helpers/gc/strategies'
import type { GcContext, ResolvedGcStrategyId } from '@/helpers/gc/types'
import { setLastGcLaunchReport } from '@/helpers/gc-notice'
import { AUTO_GC_PRESET_ARG } from '@/helpers/java-arguments'
import { get_jre, get_memory_status } from '@/helpers/jre.js'
import { get as getSettings } from '@/helpers/settings'

import type { InstallJobSnapshot } from './install'
import { removeInstanceCache } from './instance-cache'
import type {
	CacheBehaviour,
	ContentFile,
	ContentFileProjectType,
	GameInstance,
	InstanceLoader,
} from './types'

export async function remove(instanceId: string): Promise<void> {
	removeInstanceCache(instanceId)
	return await invoke('plugin:instance|instance_remove', { instanceId })
}

export async function get(instanceId: string): Promise<GameInstance | null> {
	return await invoke('plugin:instance|instance_get', { instanceId })
}

export async function get_many(instanceIds: string[]): Promise<GameInstance[]> {
	return await invoke('plugin:instance|instance_get_many', { instanceIds })
}

export async function get_projects(
	instanceId: string,
	cacheBehaviour?: CacheBehaviour,
): Promise<Record<string, ContentFile>> {
	return await invoke('plugin:instance|instance_get_projects', { instanceId, cacheBehaviour })
}

export async function get_installed_project_ids(instanceId: string): Promise<string[]> {
	return await invoke('plugin:instance|instance_get_installed_project_ids', { instanceId })
}

export type InstanceInstallTarget = {
	game_version: string
	loader: string
}

export type InstanceInstallCandidate = {
	id: string
	name: string
	icon_path?: string | null
	game_version: string
	loader: InstanceLoader
	installed: boolean
	compatible: boolean
}

export async function get_install_candidates(
	projectId: string,
	projectType: string,
	targets: InstanceInstallTarget[],
): Promise<InstanceInstallCandidate[]> {
	return await invoke('plugin:instance|instance_get_install_candidates', {
		projectId,
		projectType,
		targets,
	})
}

// Get content items with rich metadata for an instance
// Returns content items filtered to exclude modpack files (if linked),
// sorted alphabetically by project name
export async function get_content_items(
	instanceId: string,
	cacheBehaviour?: CacheBehaviour,
): Promise<ContentItem[]> {
	const items = await invoke<ContentItem[]>('plugin:instance|instance_get_content_items', {
		instanceId,
		cacheBehaviour,
	})
	return items
}

export async function get_content_items_by_paths(
	instanceId: string,
	paths: string[],
	cacheBehaviour?: CacheBehaviour,
): Promise<ContentItem[]> {
	return await invoke<ContentItem[]>('plugin:instance|instance_get_content_items_by_paths', {
		instanceId,
		paths,
		cacheBehaviour,
	})
}

export type ContentOwnershipKind = 'pack_managed' | 'user_added' | 'local_discovered'
export type PackMemberMaterializationState = 'present' | 'pending_manual' | 'missing' | 'removed'
export type PackMemberOverrideKind = 'none' | 'disabled' | 'removed' | 'version'

export interface InstanceContentSnapshotItem {
	fileId: string | null
	entryId: string | null
	memberId: string | null
	ownershipKind: ContentOwnershipKind
	materializationState: PackMemberMaterializationState
	overrideKind: PackMemberOverrideKind
	expectedRelativePath: string
	required: boolean
	projectType: string
	provider: 'modrinth' | 'curseforge' | null
	providerProjectId: string | null
	providerReleaseId: string | null
	content: ContentItem | null
	capabilities: {
		canToggle: boolean
		canDelete: boolean
		canUpdate: boolean
		canChangeVersion: boolean
		canRestorePackDefault: boolean
	}
	dependency: {
		autoDependency: boolean
		requiredBy: Array<{
			provider: 'modrinth' | 'curseforge' | 'local'
			projectId: string
			releaseId: string
		}>
		requires: Array<{
			provider: 'modrinth' | 'curseforge' | 'local'
			projectId: string
			releaseId: string
		}>
		orphaned: boolean
	} | null
}

export interface PendingManualDownload {
	id: string
	instanceId: string
	packMemberId: string | null
	contentEntryId: string | null
	operationKind: 'pack_install' | 'pack_update' | 'content_install' | 'content_update'
	operationTargetId: string | null
	projectType: string
	provider: 'modrinth' | 'curseforge'
	providerProjectId: string
	providerReleaseId: string
	fileName: string
	websiteUrl: string | null
	targetRelativePath: string
	expectedSha1: string | null
	expectedSize: number | null
	expectedFingerprint: number | null
	state: 'waiting' | 'matched' | 'imported' | 'error' | 'cancelled'
	context: Record<string, unknown>
	createdAt: string
	modifiedAt: string
}

export interface InstanceContentSnapshot {
	instanceId: string
	revision: number
	pack: {
		name: string
		iconPath: string | null
		provider: 'modrinth' | 'curseforge' | null
		projectId: string | null
		versionId: string | null
		reconciled: boolean
		canUpdate: boolean
		metadata: LinkedModpackInfo | null
	} | null
	items: InstanceContentSnapshotItem[]
	pendingManualDownloads: PendingManualDownload[]
	warnings: Array<{
		code: string
		message: string
		provider: 'modrinth' | 'curseforge' | null
	}>
}

export async function get_content_snapshot(instanceId: string): Promise<InstanceContentSnapshot> {
	return await invoke('plugin:instance|instance_get_content_snapshot', { instanceId })
}

export async function refresh_content(instanceId: string): Promise<InstanceContentSnapshot> {
	return await invoke('plugin:instance|instance_refresh_content', { instanceId })
}

export type ContentUpdateScope = 'user_added' | 'pack' | 'item'

export interface ContentUpdatePlan {
	id: string
	instanceId: string
	revision: number
	scope: ContentUpdateScope
	actions: Array<{
		contentId: string
		relativePath: string | null
		ownershipKind: ContentOwnershipKind
		provider: 'modrinth' | 'curseforge'
		currentReleaseId: string | null
		targetReleaseId: string
	}>
	warnings: string[]
}

export interface ContentUpdateResolution {
	contentId: string
	choice: 'keep_override' | 'restore_pack_default'
}

export async function plan_content_updates(
	instanceId: string,
	scope: ContentUpdateScope,
	target?: string,
): Promise<ContentUpdatePlan> {
	return await invoke('plugin:instance|instance_plan_content_updates', {
		instanceId,
		scope,
		target,
	})
}

export async function apply_content_update_plan(
	planId: string,
	resolutions: ContentUpdateResolution[] = [],
): Promise<InstanceContentSnapshot> {
	return await invoke('plugin:instance|instance_apply_content_update_plan', {
		planId,
		resolutions,
	})
}

// Linked modpack info returned from backend
export interface LinkedModpackInfo {
	project: Labrinth.Projects.v2.Project
	version: Labrinth.Versions.v2.Version
	owner: ContentOwner | null
	update:
		| {
				provider: 'modrinth'
				project_id: string
				current_version_id: string
				target_version_id: string
		  }
		| {
				provider: 'curseforge'
				project_id: number
				current_file_id: number
				target_file_id: number
		  }
		| null
	update_version: Labrinth.Versions.v2.Version | null
}

// Get linked modpack info for an instance
// Returns project, version, and owner information for the linked modpack,
// or null if the instance is not linked to a modpack
export async function get_linked_modpack_info(
	instanceId: string,
	cacheBehaviour?: CacheBehaviour,
): Promise<LinkedModpackInfo | null> {
	return await invoke('plugin:instance|instance_get_linked_modpack_info', {
		instanceId,
		cacheBehaviour,
	})
}

// Get content items that are part of the linked modpack
// Returns the modpack's dependencies as ContentItem list
// Returns empty array if the instance is not linked to a modpack
export async function get_linked_modpack_content(
	instanceId: string,
	cacheBehaviour?: CacheBehaviour,
): Promise<ContentItem[]> {
	const items = await invoke<ContentItem[]>('plugin:instance|instance_get_linked_modpack_content', {
		instanceId,
		cacheBehaviour,
	})
	return items
}

// Convert a list of dependencies into ContentItems with rich metadata
export async function get_dependencies_as_content_items(
	dependencies: Labrinth.Versions.v3.Dependency[],
	cacheBehaviour?: CacheBehaviour,
): Promise<ContentItem[]> {
	const items = await invoke<ContentItem[]>(
		'plugin:instance|instance_get_dependencies_as_content_items',
		{
			dependencies,
			cacheBehaviour,
		},
	)
	return items
}

export async function get_full_path(instanceId: string): Promise<string> {
	return await invoke('plugin:instance|instance_get_full_path', { instanceId })
}

export async function get_mod_full_path(instanceId: string, projectPath: string): Promise<string> {
	return await invoke('plugin:instance|instance_get_mod_full_path', { instanceId, projectPath })
}

export interface JavaVersion {
	parsed_version: number
	version: string
	architecture: string
	path: string
}

export async function get_optimal_jre_key(instanceId: string): Promise<JavaVersion | null> {
	return await invoke('plugin:instance|instance_get_optimal_jre_key', { instanceId })
}

export async function list(): Promise<GameInstance[]> {
	return await invoke('plugin:instance|instance_list')
}

export type DailyPlaytime = {
	date: string
	played_seconds: number
	session_count: number
	top_instance_name?: string | null
}

export async function set_pinned(instanceId: string, pinned: boolean): Promise<GameInstance> {
	return await invoke('plugin:instance|instance_set_pinned', { instanceId, pinned })
}

export async function get_daily_playtime(
	startDate: string,
	endDate: string,
): Promise<DailyPlaytime[]> {
	return await invoke('plugin:instance|instance_get_daily_playtime', { startDate, endDate })
}

export type DailyPlaytimeEntry = {
	instance_id: string
	instance_name: string
	played_seconds: number
	session_count: number
}

export async function get_daily_playtime_details(date: string): Promise<DailyPlaytimeEntry[]> {
	return await invoke('plugin:instance|instance_get_daily_playtime_details', { date })
}

export async function check_installed(instanceId: string, projectId: string): Promise<boolean> {
	return await invoke('plugin:instance|instance_check_installed', { instanceId, projectId })
}

export async function update_all(instanceId: string): Promise<Record<string, string>> {
	return await invoke('plugin:instance|instance_update_all', { instanceId })
}

// Updates a specified project
export async function update_project(instanceId: string, projectPath: string): Promise<string> {
	return await invoke('plugin:instance|instance_update_project', { instanceId, projectPath })
}

// Add a project to an instance from a version
// Returns a path to the new project file
export type DownloadReason = 'standalone' | 'dependency' | 'modpack' | 'update'

export interface ResolutionPreferences {
	game_versions?: string[]
	loaders?: string[]
}

export interface ResolveContentRequest {
	project_id: string
	version_id?: string | null
	content_type: Labrinth.Content.v3.ContentType
	selected?: ResolutionPreferences
	excluded_project_ids?: string[]
}

export interface ResolvedContent {
	project_id: string
	version_id: string
	dependent_on_version_id?: string | null
}

export interface ResolveContentPlan {
	primary: ResolvedContent
	dependencies: ResolvedContent[]
	skipped: Array<{
		project_id: string
		version_id?: string | null
		dependent_on_version_id?: string | null
		reason: string
	}>
}

export async function add_project_from_version(
	instanceId: string,
	versionId: string,
	reason: DownloadReason,
	dependentOnVersionId?: string,
): Promise<string> {
	return await invoke('plugin:instance|instance_add_project_from_version', {
		instanceId,
		versionId,
		reason,
		dependentOnVersionId,
	})
}

export async function install_project_with_dependencies(
	instanceId: string,
	request: ResolveContentRequest,
): Promise<ResolveContentPlan> {
	return await invoke('plugin:instance|instance_install_project_with_dependencies', {
		instanceId,
		request,
	})
}

export async function preview_project_with_dependencies(
	instanceId: string,
	request: ResolveContentRequest,
): Promise<ResolveContentPlan> {
	return await invoke('plugin:instance|instance_preview_project_with_dependencies', {
		instanceId,
		request,
	})
}

export async function preview_project_with_dependencies_for_target(
	request: ResolveContentRequest,
	gameVersion: string,
	loader: string,
): Promise<ResolveContentPlan> {
	return await invoke('plugin:instance|instance_preview_project_with_dependencies_for_target', {
		request,
		gameVersion,
		loader,
	})
}

export async function queue_project_with_dependencies(
	instanceId: string,
	request: ResolveContentRequest,
	display: { title: string; iconUrl?: string | null },
): Promise<InstallJobSnapshot> {
	return await invoke('plugin:instance|instance_queue_project_with_dependencies', {
		instanceId,
		request,
		displayTitle: display.title,
		displayIcon: display.iconUrl ?? null,
	})
}

export async function switch_project_version_with_dependencies(
	instanceId: string,
	projectPath: string,
	versionId: string,
): Promise<string> {
	return await invoke('plugin:instance|instance_switch_project_version_with_dependencies', {
		instanceId,
		projectPath,
		versionId,
	})
}

// Import a world save into an instance
// Returns the imported world name
export async function import_world_save(
	instanceId: string,
	sourcePath: string,
	innerBase?: string,
): Promise<string> {
	return await invoke('plugin:instance|instance_import_world_save', {
		instanceId,
		sourcePath,
		innerBase,
	})
}

export async function install_datapack_to_world(
	instanceId: string,
	worldPath: string,
	sourcePath: string,
	innerBase?: string,
): Promise<string> {
	return await invoke('plugin:instance|instance_install_datapack_to_world', {
		instanceId,
		worldPath,
		sourcePath,
		innerBase,
	})
}

// Add a project to an instance from a path + project_type
// Returns a path to the new project file
export async function add_project_from_path(
	instanceId: string,
	projectPath: string,
	projectType?: ContentFileProjectType,
	innerBase?: string,
): Promise<string> {
	return await invoke('plugin:instance|instance_add_project_from_path', {
		instanceId,
		projectPath,
		projectType,
		innerBase,
	})
}

// Toggle disabling a project
export async function toggle_disable_project(
	instanceId: string,
	projectPath: string,
	desiredEnabled?: boolean,
): Promise<string> {
	return await invoke('plugin:instance|instance_toggle_disable_project', {
		instanceId,
		projectPath,
		desiredEnabled,
	})
}

export async function toggle_content_entry(
	instanceId: string,
	contentId: string,
	desiredEnabled?: boolean,
): Promise<string> {
	return await invoke('plugin:instance|instance_toggle_content_entry', {
		instanceId,
		contentId,
		desiredEnabled,
	})
}

export type ContentToggleResult = {
	contentId: string
	path: string
	enabled: boolean
}

export async function toggle_content_entries(
	instanceId: string,
	contentIds: string[],
	desiredEnabled?: boolean,
): Promise<ContentToggleResult[]> {
	return await invoke('plugin:instance|instance_toggle_content_entries', {
		instanceId,
		contentIds,
		desiredEnabled,
	})
}

// Roll back an updated project to its previous file (kept as a .old backup)
export async function rollback_project(instanceId: string, projectPath: string): Promise<string> {
	return await invoke('plugin:instance|instance_rollback_project', {
		instanceId,
		projectPath,
	})
}

// Remove a project
export async function remove_project(instanceId: string, projectPath: string): Promise<void> {
	return await invoke('plugin:instance|instance_remove_project', { instanceId, projectPath })
}

export async function remove_content_entry(instanceId: string, contentId: string): Promise<void> {
	return await invoke('plugin:instance|instance_remove_content_entry', { instanceId, contentId })
}

export async function update_content_entry(instanceId: string, contentId: string): Promise<string> {
	return await invoke('plugin:instance|instance_update_content_entry', { instanceId, contentId })
}

export async function switch_content_entry_version(
	instanceId: string,
	contentId: string,
	versionId: string,
): Promise<string> {
	return await invoke('plugin:instance|instance_switch_content_entry_version', {
		instanceId,
		contentId,
		versionId,
	})
}

export async function restore_pack_member_default(
	instanceId: string,
	memberId: string,
): Promise<string | null> {
	return await invoke('plugin:instance|instance_restore_pack_member_default', {
		instanceId,
		memberId,
	})
}

// Update a managed Modrinth instance to a specific version
export async function update_managed_modrinth_version(
	instanceId: string,
	versionId: string,
): Promise<InstallJobSnapshot> {
	return await invoke('plugin:instance|instance_update_managed_modrinth_version', {
		instanceId,
		versionId,
	})
}

// Repair a managed Modrinth instance
export async function update_repair_modrinth(instanceId: string): Promise<InstallJobSnapshot> {
	return await invoke('plugin:instance|instance_repair_managed_modrinth', { instanceId })
}

// Export an instance to .mrpack
// included_overrides is an array of paths to override folders to include (ie: 'mods', 'resource_packs')
// Version id is optional (ie: 1.1.5)
export async function export_instance_mrpack(
	instanceId: string,
	exportLocation: string,
	includedOverrides: string[],
	versionId?: string,
	description?: string,
	name?: string,
): Promise<void> {
	return await invoke('plugin:instance|instance_export_mrpack', {
		instanceId,
		exportLocation,
		includedOverrides,
		versionId,
		description,
		name,
	})
}

// Given a folder path, populate an array of all the subfolders
// Intended to be used for finding potential override folders
// profile
// -- mods
// -- resourcepacks
// -- file1
// => [mods, resourcepacks]
// allows selection for 'included_overrides' in export_instance_mrpack
export async function get_pack_export_candidates(instanceId: string): Promise<string[]> {
	return await invoke('plugin:instance|instance_get_pack_export_candidates', { instanceId })
}

export interface GcLaunchIntent {
	active_preset_id: string
	block_tokens: string[]
	candidate_ids: string[]
	candidates: string[][]
}

export interface GcLaunchReport {
	preferred_strategy: string
	chosen_strategy: string
	chosen_args: string[]
	pruned_args: string[]
	reason_chain: string[]
}

export interface InstanceRunResult {
	process: unknown
	gc_notice: GcLaunchReport | null
}

export function gcReportFellBack(report: GcLaunchReport): boolean {
	return (
		report.chosen_strategy !== report.preferred_strategy || report.pruned_args.length > 0
	)
}

/**
 * The Java that will actually run the instance: a per-instance `java_path`
 * wins, otherwise the optimal JRE for the game version.
 */
async function resolveEffectiveJava(
	instance: Pick<GameInstance, 'java_path'>,
	instanceId: string,
): Promise<JavaVersion | null> {
	if (instance.java_path) {
		try {
			return await get_jre(instance.java_path)
		} catch {
			// Invalid override; fall back to the optimal JRE.
		}
	}
	return await get_optimal_jre_key(instanceId)
}

function normalizeJvmToken(token: string): string {
	const stripped = token.replace(/^-+/, '')
	const withoutPrefix = stripped.startsWith('XX:') ? stripped.slice(3) : stripped
	const withoutModifier =
		withoutPrefix.startsWith('+') || withoutPrefix.startsWith('-')
			? withoutPrefix.slice(1)
			: withoutPrefix
	return withoutModifier.split('=')[0] ?? withoutModifier
}

function strategyTokenKeys(strategy: ResolvedGcStrategyId, context: GcContext): Set<string> {
	const tokens = GC_STRATEGY_DEFINITIONS[strategy]
		.buildArgs(context)
		.split(/\s+/)
		.filter(Boolean)
	return new Set(tokens.map(normalizeJvmToken))
}

/**
 * Determine whether the effective Java args contain an auto GC marker or an
 * active GC preset block, and if so build the ordered candidate chain for the
 * backend to verify against the actual JVM. The args are returned untouched so
 * the backend replaces exactly the preset block.
 */
export async function resolveGcLaunchIntent(
	instanceId: string,
): Promise<{ args: string[]; gcIntent: GcLaunchIntent | null }> {
	const instance = await get(instanceId)
	if (!instance) return { args: [], gcIntent: null }

	const settings = await getSettings()
	const effectiveArgs = instance.extra_launch_args ?? settings.extra_launch_args

	const java = await resolveEffectiveJava(instance, instanceId)
	const javaMajorVersion = java?.parsed_version ?? null
	let modCount = 0
	try {
		const snapshot = await get_content_snapshot(instanceId)
		modCount = snapshot.items.filter(
			(item) => item.projectType === 'mod' && item.materializationState === 'present',
		).length
	} catch {
		modCount = 0
	}

	const memory = instance.memory ?? settings.memory
	let allocatedMemoryMb = memory.maximum
	if (memory.automatic) {
		try {
			const status = await get_memory_status(instanceId, memory.maximum, true)
			allocatedMemoryMb = status.allocated_mb
		} catch {
			// Fall back to the configured value when automatic memory info is unavailable.
		}
	}
	const context = await collectGcContext(
		allocatedMemoryMb,
		instance.loader,
		javaMajorVersion,
		modCount,
	)

	let activePresetId: string | null = null
	let blockTokens: string[] = []
	if (effectiveArgs.includes(AUTO_GC_PRESET_ARG)) {
		activePresetId = 'gc-auto'
		blockTokens = [AUTO_GC_PRESET_ARG]
	} else {
		const detected = detectGcStrategy(effectiveArgs.join(' '))
		if (detected) {
			activePresetId = `gc-${detected}`
			const presetKeys = strategyTokenKeys(detected, context)
			blockTokens = effectiveArgs.filter((arg) => presetKeys.has(normalizeJvmToken(arg)))
		}
	}

	if (!activePresetId || !blockTokens.length) {
		return { args: effectiveArgs, gcIntent: null }
	}

	const preferred: ResolvedGcStrategyId =
		activePresetId === 'gc-auto'
			? resolveAutoGcStrategy(context).resolvedStrategy
			: (activePresetId.slice('gc-'.length) as ResolvedGcStrategyId)

	const { ids, args: candidates } = buildGcCandidateChain(context, preferred)

	return {
		args: effectiveArgs,
		gcIntent: {
			active_preset_id: activePresetId,
			block_tokens: blockTokens,
			candidate_ids: ids,
			candidates,
		},
	}
}

// Run Minecraft using an instance
// Returns the process metadata plus an optional GC fallback report.
export async function run(
	instanceId: string,
	serverAddress: string | null = null,
): Promise<InstanceRunResult> {
	const { args, gcIntent } = await resolveGcLaunchIntent(instanceId)
	const result = await invoke<InstanceRunResult>('plugin:instance|instance_run', {
		instanceId,
		serverAddress,
		offlineMode: isOfflineMode(),
		extraLaunchArgs: args,
		gcIntent,
	})
	setLastGcLaunchReport(result.gc_notice)
	return result
}

export async function kill(instanceId: string): Promise<void> {
	return await invoke('plugin:instance|instance_kill', { instanceId })
}

// Edits an instance
export async function edit(instanceId: string, editInstance: Partial<GameInstance>): Promise<void> {
	return await invoke('plugin:instance|instance_edit', { instanceId, editInstance })
}

export async function cache_icon(iconName: string, bytes: number[]): Promise<string> {
	return await invoke('plugin:instance|instance_cache_icon', { iconName, bytes })
}

// Edits an instance's icon
export async function edit_icon(instanceId: string, iconPath: string | null): Promise<void> {
	return await invoke('plugin:instance|instance_edit_icon', { instanceId, iconPath })
}

export type SymlinkCapability = 'supported' | 'requires_admin' | 'unsupported'

export async function check_symlink_capability(): Promise<SymlinkCapability> {
	return await invoke('check_symlink_capability')
}

export async function allow_symlink_target(path: string): Promise<void> {
	return await invoke('allow_symlink_target', { path })
}
