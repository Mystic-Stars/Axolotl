import { get_project_many, get_version_many } from './cache.js'
import { getCurseForgeChangelog, getCurseForgeFile, getCurseForgeProjects } from './curseforge'
import {
	type UpgradeReleaseIdentity,
	upgradeVersionCacheKey,
	type UpgradeVersionDisplayMetadata,
} from './upgrade-version-display'

export {
	type UpgradeReleaseIdentity,
	upgradeVersionCacheKey,
	upgradeVersionDisplayLabel,
	type UpgradeVersionDisplayMetadata,
} from './upgrade-version-display'

export interface UpgradeVersionMetadata extends UpgradeVersionDisplayMetadata {
	changelog: string | null
}

const displayCache = new Map<string, UpgradeVersionDisplayMetadata>()
const changelogCache = new Map<string, UpgradeVersionMetadata>()
const projectDisplayCache = new Map<string, UpgradeProjectDisplayMetadata>()

export interface UpgradeProjectIdentity {
	provider: 'modrinth' | 'curseforge'
	projectId: string
}

export interface UpgradeProjectDisplayMetadata {
	title: string
	iconUrl: string | null
}

export function upgradeProjectDisplayCacheKey(provider: string, projectId: string) {
	return `${provider}:${projectId}`
}

export async function loadUpgradeProjectDisplayMetadata(identities: UpgradeProjectIdentity[]) {
	const unique = [
		...new Map(
			identities.map((identity) => [
				upgradeProjectDisplayCacheKey(identity.provider, identity.projectId),
				identity,
			]),
		).values(),
	]
	const missing = unique.filter(
		(identity) =>
			!projectDisplayCache.has(
				upgradeProjectDisplayCacheKey(identity.provider, identity.projectId),
			),
	)
	const modrinthIds = missing
		.filter((identity) => identity.provider === 'modrinth')
		.map((identity) => identity.projectId)
	const curseForgeIds = missing
		.filter((identity) => identity.provider === 'curseforge')
		.map((identity) => Number(identity.projectId))
		.filter((projectId) => Number.isSafeInteger(projectId))
	const [modrinthProjects, curseForgeProjects] = await Promise.all([
		modrinthIds.length
			? (get_project_many(modrinthIds).catch(() => []) as Promise<
					Array<{ id: string; title: string; icon_url?: string | null }>
				>)
			: [],
		curseForgeIds.length ? getCurseForgeProjects(curseForgeIds).catch(() => []) : [],
	])
	for (const project of modrinthProjects) {
		projectDisplayCache.set(upgradeProjectDisplayCacheKey('modrinth', project.id), {
			title: project.title,
			iconUrl: project.icon_url ?? null,
		})
	}
	for (const project of curseForgeProjects) {
		projectDisplayCache.set(upgradeProjectDisplayCacheKey('curseforge', String(project.id)), {
			title: project.name,
			iconUrl: project.logo?.thumbnailUrl ?? project.logo?.url ?? null,
		})
	}
	return new Map(
		unique.flatMap((identity) => {
			const key = upgradeProjectDisplayCacheKey(identity.provider, identity.projectId)
			const value = projectDisplayCache.get(key)
			return value ? [[key, value] as const] : []
		}),
	)
}

async function loadCurseForgeDisplayMetadata(identities: UpgradeReleaseIdentity[]) {
	const queue = [...identities]
	await Promise.all(
		Array.from({ length: Math.min(4, queue.length) }, async () => {
			for (let identity = queue.shift(); identity; identity = queue.shift()) {
				const file = await getCurseForgeFile(Number(identity.projectId), Number(identity.releaseId))
				displayCache.set(
					upgradeVersionCacheKey(identity.provider, identity.projectId, identity.releaseId),
					{
						version: file.displayName ?? file.fileName ?? identity.releaseId,
						channel: file.releaseType,
					},
				)
			}
		}),
	)
}

export async function loadUpgradeVersionDisplayMetadata(identities: UpgradeReleaseIdentity[]) {
	const unique = [
		...new Map(
			identities.map((identity) => [
				upgradeVersionCacheKey(identity.provider, identity.projectId, identity.releaseId),
				identity,
			]),
		).values(),
	]
	const missing = unique.filter(
		(identity) =>
			!displayCache.has(
				upgradeVersionCacheKey(identity.provider, identity.projectId, identity.releaseId),
			),
	)
	const modrinth = missing.filter((identity) => identity.provider === 'modrinth')
	if (modrinth.length) {
		const versions = (await get_version_many([
			...new Set(modrinth.map((item) => item.releaseId)),
		])) as Array<{
			id: string
			version_number?: string
			version_type?: string
		}>
		const byId = new Map(versions.map((version) => [version.id, version]))
		for (const identity of modrinth) {
			const version = byId.get(identity.releaseId)
			displayCache.set(
				upgradeVersionCacheKey(identity.provider, identity.projectId, identity.releaseId),
				{
					version: version?.version_number ?? identity.releaseId,
					channel: version?.version_type,
				},
			)
		}
	}
	await loadCurseForgeDisplayMetadata(
		missing.filter((identity) => identity.provider === 'curseforge'),
	)
	return new Map(
		unique.flatMap((identity) => {
			const key = upgradeVersionCacheKey(identity.provider, identity.projectId, identity.releaseId)
			const value = displayCache.get(key)
			return value ? [[key, value] as const] : []
		}),
	)
}

export async function loadUpgradeVersionMetadata(
	provider: string,
	projectId: string,
	releaseId: string,
) {
	const key = upgradeVersionCacheKey(provider, projectId, releaseId)
	const cached = changelogCache.get(key)
	if (cached) return cached
	if (provider === 'modrinth') {
		const versions = (await get_version_many([releaseId])) as Array<{
			id: string
			version_number?: string
			version_type?: string
			changelog?: string | null
		}>
		const version = versions[0]
		const result = {
			version: version?.version_number ?? releaseId,
			channel: version?.version_type,
			changelog: version?.changelog ?? null,
		}
		displayCache.set(key, result)
		changelogCache.set(key, result)
		return result
	}
	if (provider === 'curseforge') {
		const [file, changelog] = await Promise.all([
			getCurseForgeFile(Number(projectId), Number(releaseId)),
			getCurseForgeChangelog(Number(projectId), Number(releaseId)),
		])
		const result = {
			version: file.displayName ?? file.fileName ?? releaseId,
			channel: file.releaseType,
			changelog: changelog || null,
		}
		displayCache.set(key, result)
		changelogCache.set(key, result)
		return result
	}
	return { version: releaseId, changelog: null }
}
