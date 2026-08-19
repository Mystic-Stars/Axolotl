import { invoke } from '@tauri-apps/api/core'

import { install_job_listener } from './events'
import type { InstanceLink, InstanceLoader } from './types'

export interface PackLocationVersionId {
	type: 'fromVersionId'
	project_id: string
	version_id: string
	title: string
	icon_url?: string | null
}

export interface PackLocationFile {
	type: 'fromFile'
	path: string
}

export type CreatePackLocation = PackLocationVersionId | PackLocationFile

export interface InstallModpackPreview {
	name: string
	gameVersion: string
	modloader: InstanceLoader
	loaderVersion: string | null
	icon?: string | null
	iconUrl?: string | null
	link?: InstanceLink | null
	unknownFile: boolean
}

export interface InstallCreateInstanceRequest {
	name: string
	gameVersion: string
	loader: InstanceLoader
	loaderVersion: string | null
	iconPath: string | null
	link?: InstanceLink | null
}

export interface InstallPostInstallEdit {
	name?: string | null
	iconPath?: string | null
	link?: InstanceLink | null
}

export type InstallJobStatus =
	| 'queued'
	| 'running'
	| 'canceling'
	| 'waiting_for_user'
	| 'succeeded'
	| 'failed'
	| 'interrupted'
	| 'canceled'

export type InstallPhaseId =
	| 'preparing_instance'
	| 'resolving_pack'
	| 'downloading_pack_file'
	| 'reading_pack_manifest'
	| 'downloading_content'
	| 'extracting_overrides'
	| 'resolving_minecraft'
	| 'resolving_loader'
	| 'preparing_java'
	| 'downloading_minecraft'
	| 'running_loader_processors'
	| 'finalizing'
	| 'rolling_back'

export interface InstallProgress {
	current: number
	total: number
	secondary?: InstallProgressSecondary | null
}

export interface InstallProgressSecondary {
	current: number
	total: number
}

export interface InstallParallelProgress {
	phase: InstallPhaseId
	current: number
	total: number
	details: InstallJobSnapshot['details']
}

export type InstallJavaStep =
	| 'resolving'
	| 'fetching_metadata'
	| 'downloading'
	| 'extracting'
	| 'validating'

export interface InstallErrorView {
	code: string
	phase?: InstallPhaseId | null
	message: string
	api?: {
		error: string
		status?: number | null
		method?: string | null
		url?: string | null
		route?: string | null
	} | null
	context?: {
		operation: string
		source_path?: string | null
		target_path?: string | null
		file_path?: string | null
		entry_path?: string | null
		urls?: string[]
		cache_types?: string[]
		sqlite_code?: string | null
		expected_hash?: string | null
		expected_size?: number | null
		project_id?: string | null
		version_id?: string | null
		minecraft_version?: string | null
		loader?: string | null
		java_version?: number | null
		os?: string | null
		arch?: string | null
	} | null
}

export type InstallPauseReason = {
	type: 'missing_required_content'
	failed_files: number
	paths: string[]
}

export interface InstallJobSnapshot {
	job_id: string
	instance_id?: string | null
	instance_deleted: boolean
	kind:
		| 'create_instance'
		| 'create_modpack_instance'
		| 'import_instance'
		| 'duplicate_instance'
		| 'install_existing_instance'
		| 'install_pack_to_existing_instance'
		| 'install_content'
		| 'download_java'
	status: InstallJobStatus
	execution_mode: 'normal' | 'recovery_validation'
	provider: 'modrinth' | 'curse_forge' | 'minecraft' | 'java' | 'application' | 'local'
	target:
		| { type: 'new_instance'; instance_id?: string | null }
		| { type: 'existing_instance'; instance_id: string }
	phase: InstallPhaseId
	progress?: InstallProgress | null
	details:
		| { type: 'empty' }
		| { type: 'instance'; name: string }
		| { type: 'minecraft'; game_version: string; loader: InstanceLoader }
		| { type: 'java'; major_version: number; step: InstallJavaStep }
		| {
				type: 'modpack'
				project_id?: string | null
				version_id?: string | null
				title?: string | null
		  }
		| { type: 'import'; launcher_type: string; instance_folder: string }
	parallel?: InstallParallelProgress | null
	display?: { title: string; icon?: string | null } | null
	error?: InstallErrorView | null
	rollback_error?: InstallErrorView | null
	pause_reason?: InstallPauseReason | null
	created: string
	modified: string
	finished?: string | null
	summary: {
		files_completed: number
		files_total?: number | null
		bytes_downloaded: number
		bytes_total?: number | null
		speed_bytes_per_second?: number | null
		eta_seconds?: number | null
		source?: string | null
		fallback_count: number
	}
	items: Array<{
		id: string
		name: string
		project_id?: string | null
		version_id?: string | null
		status:
			| 'queued'
			| 'downloading'
			| 'verifying'
			| 'writing'
			| 'waiting_for_user'
			| 'completed'
			| 'skipped'
			| 'failed'
			| 'canceled'
		bytes_downloaded: number
		bytes_total?: number | null
		attempt?: number | null
		max_attempts?: number | null
		error?: string | null
		manual_url?: string | null
		request_url?: string | null
		source?: string | null
	}>
}

export interface DownloadJobListRequest {
	status?: InstallJobStatus
	provider?: InstallJobSnapshot['provider']
	query?: string
	cursor?: string
	limit?: number
}

export interface DownloadJobPage {
	jobs: InstallJobSnapshot[]
	nextCursor?: string | null
}

export interface MissingModpackContentView {
	remaining: number
	files: Array<{
		itemId: string
		path: string
		expectedSize: number
		status: InstallJobSnapshot['items'][number]['status']
		lastError?: string | null
		browserUrls: string[]
		attempt?: number | null
		maxAttempts?: number | null
	}>
}

export interface MissingModpackScanResult {
	downloadDirectory?: string | null
	content: MissingModpackContentView
	importedItemIds: string[]
	mismatchedItemIds: string[]
	rejectedItemIds: string[]
	checkedCandidates: number
	pendingCandidates: number
	errors: Array<{ itemId: string; message: string }>
	job: InstallJobSnapshot
}

export type DownloadRequestUpdate =
	| {
			type: 'started'
			job_id: string
			id: string
			name: string
			url: string
			source: string
			bytes_total?: number | null
			attempt: number
			max_attempts: number
	  }
	| {
			type: 'progress'
			job_id: string
			id: string
			bytes: number
			status: 'downloading' | 'writing' | 'verifying'
			speed_bytes_per_second?: number | null
			eta_seconds?: number | null
	  }
	| { type: 'finished'; job_id: string; id: string; bytes: number }
	| { type: 'failed'; job_id: string; id: string }

export type ImportPlanStage = 'resolving' | 'scanning' | 'done' | 'error'

export interface ImportPlanCounts {
	files: number
	bytes: number
}

export interface ImportPlanSnapshot {
	requestId: string
	stage: ImportPlanStage
	gameVersion: string | null
	loader: string | null
	loaderVersion: string | null
	importPath: string
	minecraftRoot: string
	modCount: number
	cache: ImportPlanCounts
	local: ImportPlanCounts
	network: ImportPlanCounts
	migrate: ImportPlanCounts
	error?: string | null
}

export interface ImportPlanRequest {
	requestId: string
	launcherType: string
	basePath: string
	instanceFolder: string
	instancePath?: string | null
	gameVersion?: string | null
	loader?: string | null
	loaderVersion?: string | null
}

export async function install_get_modpack_preview(location: CreatePackLocation) {
	return await invoke<InstallModpackPreview>('plugin:install|install_get_modpack_preview', {
		location,
	})
}

export async function install_create_instance(request: InstallCreateInstanceRequest) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_create_instance', { request })
}

export async function install_create_modpack_instance(
	location: CreatePackLocation,
	postInstallEdit?: InstallPostInstallEdit | null,
) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_create_modpack_instance', {
		location,
		postInstallEdit,
	})
}

export async function install_import_instance(
	launcherType: string,
	basePath: string,
	instanceFolder: string,
	symlink?: boolean,
	instancePath?: string,
	gameVersion?: string | null,
	loader?: string | null,
	loaderVersion?: string | null,
) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_import_instance', {
		launcherType,
		basePath,
		instanceFolder,
		instancePath,
		symlink,
		gameVersion,
		loader,
		loaderVersion,
	})
}

export async function install_start_import_plan(request: ImportPlanRequest) {
	return await invoke<string>('plugin:install|install_start_import_plan', { request })
}

export async function install_cancel_import_plan(requestId: string) {
	return await invoke<void>('plugin:install|install_cancel_import_plan', { requestId })
}

export async function install_duplicate_instance(sourceInstanceId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_duplicate_instance', {
		sourceInstanceId,
	})
}

export async function install_existing_instance(instanceId: string, force: boolean) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_existing_instance', {
		instanceId,
		force,
	})
}

export async function install_pack_to_existing_instance(
	instanceId: string,
	location: CreatePackLocation,
	postInstallEdit?: InstallPostInstallEdit | null,
) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_pack_to_existing_instance', {
		instanceId,
		location,
		postInstallEdit,
	})
}

export async function install_job_list(includeFinished: boolean) {
	return await invoke<InstallJobSnapshot[]>('plugin:install|install_job_list', { includeFinished })
}

export async function install_job_get(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_job_get', { jobId })
}

export async function install_job_retry(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_job_retry', { jobId })
}

export async function install_job_repair_cache_and_retry(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_job_repair_cache_and_retry', {
		jobId,
	})
}

export async function install_job_resume(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_job_resume', { jobId })
}

export async function install_job_skip_missing_content(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_job_skip_missing_content', {
		jobId,
	})
}

export async function install_job_missing_files(jobId: string) {
	return await invoke<MissingModpackContentView>('plugin:install|install_job_missing_files', {
		jobId,
	})
}

export async function install_job_scan_missing_files(jobId: string, scanDirectory?: string | null) {
	return await invoke<MissingModpackScanResult>('plugin:install|install_job_scan_missing_files', {
		jobId,
		scanDirectory: scanDirectory ?? null,
	})
}

export async function install_job_retry_missing_file(jobId: string, itemId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_job_retry_missing_file', {
		jobId,
		itemId,
	})
}

export async function install_job_import_missing_file(
	jobId: string,
	itemId: string,
	selectedFilePath: string,
) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_job_import_missing_file', {
		jobId,
		itemId,
		selectedFilePath,
	})
}

export async function install_job_cancel(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|install_job_cancel', { jobId })
}

export async function install_job_dismiss(jobId: string) {
	return await invoke<void>('plugin:install|install_job_dismiss', { jobId })
}

export async function install_job_support_details(jobId: string) {
	return await invoke<string>('plugin:install|install_job_support_details', { jobId })
}

export async function download_job_list(request: DownloadJobListRequest = {}) {
	return await invoke<DownloadJobPage>('plugin:install|download_job_list', { request })
}

export async function download_job_get(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|download_job_get', { jobId })
}

export async function download_job_retry(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|download_job_retry', { jobId })
}

export async function download_job_resume(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|download_job_resume', { jobId })
}

export async function download_job_cancel(jobId: string) {
	return await invoke<InstallJobSnapshot>('plugin:install|download_job_cancel', { jobId })
}

export async function download_job_delete(jobId: string) {
	return await invoke<void>('plugin:install|download_job_delete', { jobId })
}

export async function download_history_clear() {
	return await invoke<number>('plugin:install|download_history_clear')
}

export async function download_job_support_details(jobId: string) {
	return await invoke<string>('plugin:install|download_job_support_details', { jobId })
}

export function installJobInstanceId(job: InstallJobSnapshot): string | null {
	return job.instance_id ?? job.target.instance_id ?? null
}

export function isInstallJobFinished(status: InstallJobStatus) {
	return (
		status === 'succeeded' ||
		status === 'failed' ||
		status === 'interrupted' ||
		status === 'canceled'
	)
}

function settleInstallJob(job: InstallJobSnapshot) {
	if (job.status === 'succeeded') return job

	throw new Error(job.error?.message ?? `Install job ${job.job_id} ${job.status}`)
}

export async function wait_for_install_job(jobId: string) {
	const current = await install_job_get(jobId)
	if (isInstallJobFinished(current.status)) return settleInstallJob(current)

	return await new Promise<InstallJobSnapshot>((resolve, reject) => {
		let finished = false
		let unlisten: (() => void) | null = null

		const cleanup = () => {
			if (unlisten) {
				unlisten()
				unlisten = null
			}
		}

		const resolveJob = (job: InstallJobSnapshot) => {
			if (finished || job.job_id !== jobId || !isInstallJobFinished(job.status)) return

			finished = true
			cleanup()

			try {
				resolve(settleInstallJob(job))
			} catch (err) {
				reject(err)
			}
		}

		const rejectWait = (err: unknown) => {
			if (finished) return
			finished = true
			cleanup()
			reject(err)
		}

		install_job_listener(resolveJob)
			.then((listener) => {
				if (finished) {
					listener()
					return
				}

				unlisten = listener
				install_job_get(jobId).then(resolveJob).catch(rejectWait)
			})
			.catch(rejectWait)
	})
}
