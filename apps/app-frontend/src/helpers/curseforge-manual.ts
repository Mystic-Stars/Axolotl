export interface CurseForgeManualDownloadItem {
	projectId: number
	fileId: number
	fileName: string
	websiteUrl?: string
}

export interface InstalledCurseForgeContentItem {
	file_name: string
	provider_refs?: Array<{
		provider: string
		project_id: string
		version_id?: string | null
	}>
}

const STORAGE_KEY = 'axolotl.curseforge.manual-downloads.v1'

type ManualDownloadMap = Record<string, CurseForgeManualDownloadItem[]>

function readStore(): ManualDownloadMap {
	try {
		const raw = localStorage.getItem(STORAGE_KEY)
		if (!raw) return {}
		const parsed = JSON.parse(raw) as ManualDownloadMap
		return parsed && typeof parsed === 'object' ? parsed : {}
	} catch {
		return {}
	}
}

function writeStore(store: ManualDownloadMap) {
	localStorage.setItem(STORAGE_KEY, JSON.stringify(store))
}

function modFileFamily(fileName: string) {
	const extension = fileName.match(/\.([^.]+)$/)?.[1]?.toLowerCase()
	if (!extension) return undefined

	const stem = fileName
		.toLowerCase()
		.replace(/\.(?:jar|zip|litemod|mrpack)$/i, '')
		.replace(/\s*\(\d+\)$/, '')
	const versionStart = stem.search(/[-_. ]+v?\d/)
	if (versionStart <= 0) return undefined

	const family = stem.slice(0, versionStart).replace(/[^a-z0-9]+/g, '')
	return family.length >= 3 ? `${extension}:${family}` : undefined
}

export function getCurseForgeManualDownloads(instanceId: string): CurseForgeManualDownloadItem[] {
	return readStore()[instanceId] ?? []
}

export function setCurseForgeManualDownloads(
	instanceId: string,
	items: CurseForgeManualDownloadItem[],
) {
	const store = readStore()
	if (!items.length) {
		const { [instanceId]: _removed, ...rest } = store
		writeStore(rest)
		return
	}

	const deduped = new Map<string, CurseForgeManualDownloadItem>()
	for (const item of items) {
		deduped.set(`${item.projectId}:${item.fileId}`, item)
	}
	store[instanceId] = [...deduped.values()]
	writeStore(store)
}

export function clearCurseForgeManualDownloads(instanceId: string) {
	setCurseForgeManualDownloads(instanceId, [])
}

export function filterInstalledCurseForgeManualDownloads(
	manualDownloads: CurseForgeManualDownloadItem[],
	installedItems: InstalledCurseForgeContentItem[],
) {
	const installedFileNames = new Set(installedItems.map((item) => item.file_name.toLowerCase()))
	const installedFileFamilies = new Set(
		installedItems
			.map((item) => modFileFamily(item.file_name))
			.filter((family): family is string => !!family),
	)
	const installedCurseForgeProjects = new Set(
		installedItems.flatMap((item) =>
			(item.provider_refs ?? [])
				.filter((reference) => reference.provider === 'curseforge')
				.map((reference) => reference.project_id),
		),
	)
	const installedCurseForgeFiles = new Set(
		installedItems.flatMap((item) =>
			(item.provider_refs ?? [])
				.filter((reference) => reference.provider === 'curseforge' && reference.version_id)
				.map((reference) => `${reference.project_id}:${reference.version_id}`),
		),
	)
	return manualDownloads.filter((item) => {
		const fileFamily = modFileFamily(item.fileName)
		return (
			!installedCurseForgeProjects.has(String(item.projectId)) &&
			!installedCurseForgeFiles.has(`${item.projectId}:${item.fileId}`) &&
			!installedFileNames.has(item.fileName.toLowerCase()) &&
			(!fileFamily || !installedFileFamilies.has(fileFamily))
		)
	})
}

export function removeInstalledCurseForgeManualDownloads(
	instanceId: string,
	manualDownloads: CurseForgeManualDownloadItem[],
	installedItems: InstalledCurseForgeContentItem[],
) {
	const remaining = filterInstalledCurseForgeManualDownloads(manualDownloads, installedItems)
	setCurseForgeManualDownloads(instanceId, remaining)
	return remaining
}
