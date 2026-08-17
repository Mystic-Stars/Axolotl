import { invoke } from '@tauri-apps/api/core'

import type { InstallJobSnapshot } from './install'

export type ContentProvider = 'modrinth' | 'curseforge'

export interface CurseForgeCapability {
	status: 'missing_key' | 'ready' | 'unauthorized'
	configured: boolean
}

export interface CurseForgeSearchRequest {
	classId: number
	categoryId?: number
	categoryIds?: number[]
	searchFilter?: string
	gameVersion?: string
	modLoaderType?: number
	sortField?: number
	sortOrder?: 'asc' | 'desc'
	index?: number
	pageSize?: number
}

export interface UnifiedSearchHit {
	provider: 'curseforge'
	project_id: string
	slug?: string
	author: string
	author_url?: string
	title: string
	description: string
	project_type: string
	categories: string[]
	versions: string[]
	downloads: number
	icon_url?: string
	date_created: string
	date_modified: string
	latest_version?: string
	gallery: string[]
	website_url?: string
	source_url?: string
	allow_mod_distribution?: boolean
}

export interface UnifiedSearchResponse {
	provider: 'curseforge'
	hits: UnifiedSearchHit[]
	offset: number
	limit: number
	total_hits: number
}

export interface CurseForgeFilesRequest {
	gameVersion?: string
	modLoaderType?: number
	gameVersionTypeId?: number
	index?: number
	pageSize?: number
}

export interface CurseForgeProject {
	id: number
	name: string
	slug: string
	summary: string
	downloadCount: number
	mainFileId: number
	classId?: number
	dateCreated: string
	dateModified: string
	dateReleased: string
	allowModDistribution?: boolean
	gamePopularityRank?: number
	logo?: { thumbnailUrl: string; url: string }
	authors: Array<{ id: number; name: string; url: string }>
	categories: Array<{ id: number; name: string; slug: string; iconUrl?: string }>
	screenshots: Array<{ id: number; title: string; url: string; thumbnailUrl: string }>
	latestFilesIndexes: Array<{
		gameVersion: string
		fileId: number
		filename: string
		releaseType: number
		gameVersionTypeId?: number
		modLoader?: number
	}>
	links: {
		websiteUrl?: string
		wikiUrl?: string
		issuesUrl?: string
		sourceUrl?: string
	}
}

export interface CurseForgeCategory {
	id: number
	gameId: number
	name: string
	slug: string
	url: string
	iconUrl?: string
	dateModified: string
	isClass?: boolean | null
	classId?: number
	parentCategoryId?: number
	displayIndex?: number
}

export interface CurseForgeFile {
	id: number
	modId: number
	isAvailable: boolean
	displayName: string
	fileName: string
	releaseType: number
	fileDate: string
	fileLength: number
	hashes: Array<{ value: string; algo: number }>
	fileFingerprint: number
	downloadCount: number
	downloadUrl?: string
	gameVersions: string[]
	dependencies: Array<{ modId: number; relationType: number }>
}

export function getCurseForgeImageUrl(source?: string | null, width = 256): string | undefined {
	if (!source) return undefined

	try {
		const url = new URL(source)
		if (url.protocol !== 'https:' || !url.hostname.endsWith('forgecdn.net')) return source

		const proxy = new URL('https://images.weserv.nl/')
		proxy.searchParams.set('url', source)
		proxy.searchParams.set('w', String(width))
		proxy.searchParams.set('fit', 'contain')
		proxy.searchParams.set('output', 'webp')
		return proxy.toString()
	} catch {
		return source
	}
}

export interface CurseForgeFilesResponse {
	files: CurseForgeFile[]
	pagination: {
		index: number
		pageSize: number
		resultCount: number
		totalCount: number
	}
}

export interface CurseForgeInstallRequest {
	instanceId: string
	projectId: number
	fileId: number
	projectType: string
	ownershipKind?: 'pack_managed' | 'user_added'
	manualOperationKind?: 'pack_install' | 'pack_update' | 'content_install' | 'content_update'
	gameVersion?: string
	modLoaderType?: number
	worldName?: string
	installDependencies?: boolean
	excludedDependencyProjectIds?: number[]
	dependencyPlanId?: string
}

export interface CurseForgeWorldInstallRequest {
	instanceId: string
	projectId: number
	fileId: number
}

export interface CurseForgeInstallResult {
	installed: Array<{
		projectId: number
		fileId: number
		relativePath: string
		dependency: boolean
	}>
	manualDownloads: Array<{
		projectId: number
		fileId: number
		fileName: string
		ownershipKind: 'pack_managed' | 'user_added'
		operationKind: 'pack_install' | 'pack_update' | 'content_install' | 'content_update'
		websiteUrl?: string
		projectType: string
		projectSlug: string
		targetFolder: string
		hashes: Array<{ value: string; algo: number }>
		fileLength: number
		fileFingerprint: number
	}>
	failedDownloads: Array<{
		projectId: number
		fileId: number
		fileName: string
		reason: string
	}>
	optionalDependencies: number[]
	incompatibleDependencies: number[]
	skippedDependencies?: Array<{
		projectId: number
		fileId: number | null
		reason: string
	}>
}

export interface CurseForgeInstallPreview {
	planId: string
	primary: {
		projectId: number
		fileId: number
		title: string
		versionNumber: string
		fileName: string
		size: number
		requiredByProjectIds: number[]
		iconUrl?: string | null
	}
	dependencies: Array<{
		projectId: number
		fileId: number
		title: string
		versionNumber: string
		fileName: string
		size: number
		requiredByProjectIds: number[]
		iconUrl?: string | null
		versionMismatch?: boolean
		selectionReason?: 'native_strict_match' | 'sha1_verified_modrinth_fallback'
	}>
	modrinthFallbacks?: Array<{
		projectId: string
		versionId: string
		title: string
		versionNumber: string
		parentProjectId: number
		iconUrl?: string | null
	}>
	skipped: Array<{
		projectId: number
		fileId: number | null
		reason: string
	}>
	optionalDependencies: number[]
	incompatibleDependencies: number[]
}

export interface CurseForgeManualDownloadImport {
	projectId: number
	fileId: number
	relativePath: string
}

export type CurseForgeManualDownloadImportErrorKind =
	| 'not_pending'
	| 'verification_failed'
	| 'other'

export interface CurseForgeManualDownloadScanResult {
	downloadDirectory?: string | null
	imported: CurseForgeManualDownloadImport[]
	errors: Array<{
		projectId: number
		fileId: number
		message: string
	}>
}

export interface CurseForgeModpackInstallResult {
	content: CurseForgeInstallResult
	overridesWritten: number
	minecraftVersion: string
	loader?: string
}

export function summarizeCurseForgeInstall(result: CurseForgeInstallResult) {
	const installed = result.installed?.length ?? 0
	const manual = result.manualDownloads?.length ?? 0
	const failed = result.failedDownloads?.length ?? 0
	const optional = result.optionalDependencies?.length ?? 0
	const incompatible = result.incompatibleDependencies?.length ?? 0
	return { installed, manual, failed, optional, incompatible }
}

function getErrorMessage(error: unknown): string | null {
	if (error instanceof Error) return error.message
	if (typeof error === 'string') return error
	if (typeof error !== 'object' || error === null || !('message' in error)) return null
	return String(error.message)
}

export function classifyCurseForgeManualDownloadImportError(
	error: unknown,
): CurseForgeManualDownloadImportErrorKind {
	const message = getErrorMessage(error)
	if (message?.includes('The selected CurseForge file is not pending for this instance')) {
		return 'not_pending'
	}
	if (message?.includes('The selected file does not match the required CurseForge file')) {
		return 'verification_failed'
	}
	return 'other'
}

export function getCurseForgeDownloadFailureDetails(error: unknown): string | null {
	const message = getErrorMessage(error)
	if (!message) return null

	const normalized = message.toLowerCase()
	const isDownloadFailure = normalized.includes('download failed after')
	const isCurseForge =
		normalized.includes('curseforge') ||
		normalized.includes('forgecdn.net') ||
		normalized.includes('forgecdn')

	return isDownloadFailure && isCurseForge ? message : null
}

export function getCurseForgeCapability() {
	return invoke<CurseForgeCapability>('plugin:curseforge|curseforge_capability')
}

export function validateCurseForgeCredentials() {
	return invoke<CurseForgeCapability>('plugin:curseforge|curseforge_validate_credentials')
}

export function searchCurseForgeProjects(request: CurseForgeSearchRequest, requestId?: string) {
	return invoke<UnifiedSearchResponse>('plugin:curseforge|curseforge_search_projects', {
		request,
		requestId,
	})
}

export function getCurseForgeProject(projectId: number) {
	return invoke<CurseForgeProject>('plugin:curseforge|curseforge_get_project', { projectId })
}

export function getCurseForgeProjects(projectIds: number[]) {
	return invoke<CurseForgeProject[]>('plugin:curseforge|curseforge_get_projects', { projectIds })
}

export function getCurseForgeChangelog(projectId: number, fileId: number) {
	return invoke<string>('plugin:curseforge|curseforge_get_changelog', {
		projectId,
		fileId,
	})
}

export function getCurseForgeDescription(projectId: number) {
	return invoke<string>('plugin:curseforge|curseforge_get_description', {
		projectId,
	})
}

export function getCurseForgeFiles(projectId: number, request: CurseForgeFilesRequest) {
	return invoke<CurseForgeFilesResponse>('plugin:curseforge|curseforge_get_files', {
		projectId,
		request,
	})
}

export function getCurseForgeFile(projectId: number, fileId: number) {
	return invoke<CurseForgeFile>('plugin:curseforge|curseforge_get_file', {
		projectId,
		fileId,
	})
}

export function getCurseForgeDownloadUrl(projectId: number, fileId: number) {
	return invoke<string | null>('plugin:curseforge|curseforge_get_download_url', {
		projectId,
		fileId,
	})
}

export function getCurseForgeCategories(classId?: number) {
	return invoke<CurseForgeCategory[]>('plugin:curseforge|curseforge_get_categories', { classId })
}

export function installCurseForgeFile(request: CurseForgeInstallRequest) {
	return invoke<CurseForgeInstallResult>('plugin:curseforge|curseforge_install_file', { request })
}

export function previewCurseForgeFile(request: CurseForgeInstallRequest) {
	return invoke<CurseForgeInstallPreview>('plugin:curseforge|curseforge_preview_install_file', {
		request,
	})
}

export function queueCurseForgeFile(
	request: CurseForgeInstallRequest,
	display: { title: string; iconUrl?: string | null },
) {
	return invoke<InstallJobSnapshot>('plugin:instance|instance_queue_curseforge_content', {
		request,
		displayTitle: display.title,
		displayIcon: display.iconUrl ?? null,
	})
}

export function queueCurseForgeWorld(
	request: CurseForgeWorldInstallRequest,
	display: { title: string; iconUrl?: string | null },
) {
	return invoke<InstallJobSnapshot>('plugin:instance|instance_queue_curseforge_world', {
		request,
		displayTitle: display.title,
		displayIcon: display.iconUrl ?? null,
	})
}

export function updateCurseForgeFile(instanceId: string, relativePath: string) {
	return invoke<CurseForgeInstallResult>('plugin:curseforge|curseforge_update_installed_file', {
		instanceId,
		relativePath,
	})
}

export function switchCurseForgeFileVersion(
	instanceId: string,
	relativePath: string,
	fileId: number,
) {
	return invoke<CurseForgeInstallResult>(
		'plugin:curseforge|curseforge_switch_installed_file_version',
		{
			instanceId,
			relativePath,
			fileId,
		},
	)
}

export function recognizeCurseForgeFiles(instanceId: string) {
	return invoke<{
		scanned: number
		matched: number
		linked: CurseForgeInstallResult['installed']
		unmatchedPaths: string[]
	}>('plugin:curseforge|curseforge_recognize_instance_files', { instanceId })
}

export function importCurseForgeManualDownloads(instanceId: string, scanDirectory?: string | null) {
	return invoke<CurseForgeManualDownloadScanResult>(
		'plugin:curseforge|curseforge_import_manual_downloads',
		{ instanceId, scanDirectory: scanDirectory ?? null },
	)
}

export function listPendingCurseForgeManualDownloads(instanceId: string) {
	return invoke<CurseForgeInstallResult['manualDownloads']>(
		'plugin:curseforge|curseforge_list_pending_manual_downloads',
		{ instanceId },
	)
}

export function importPendingCurseForgeManualDownloadFile(
	instanceId: string,
	projectId: number,
	fileId: number,
	sourcePath: string,
) {
	return invoke<CurseForgeManualDownloadImport>(
		'plugin:curseforge|curseforge_import_pending_manual_download_file',
		{ instanceId, projectId, fileId, sourcePath },
	)
}

export function configureCurseForgeManualDownloadWatcher(
	enabled: boolean,
	scanDirectory?: string | null,
) {
	return invoke<string | null>('plugin:curseforge|curseforge_configure_manual_download_watcher', {
		enabled,
		scanDirectory: scanDirectory ?? null,
	})
}

export function installCurseForgeModpack(request: {
	instanceId: string
	projectId: number
	fileId: number
	installOptional?: boolean
}) {
	return invoke<CurseForgeModpackInstallResult>('plugin:curseforge|curseforge_install_modpack', {
		request,
	})
}

export function updateManagedCurseForgeModpack(instanceId: string, fileId: number) {
	return invoke<InstallJobSnapshot>('plugin:curseforge|curseforge_update_managed_modpack', {
		instanceId,
		fileId,
	})
}
