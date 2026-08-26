import type {
	ContentItem,
	ContentModpackCardProject,
	ContentModpackCardVersion,
	ContentOwner,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'

import {
	get_content_snapshot,
	type InstanceContentSnapshot,
	type LinkedModpackInfo,
	refresh_content,
} from '@/helpers/instance'
import type { CacheBehaviour } from '@/helpers/types'

export type InstanceContentData = {
	path: string
	snapshot: InstanceContentSnapshot
	contentItems: ContentItem[]
	linkedContentItems: ContentItem[]
	modpack: InstanceContentModpackData | null
}

export type InstanceContentModpackData = {
	project: ContentModpackCardProject
	version: ContentModpackCardVersion | null
	owner: ContentOwner | null
	hasUpdate: boolean
	updateVersionId: string | null
}

export function isWorldSaveContentItem(item: Pick<ContentItem, 'project_type'>): boolean {
	return ['world', 'worldsave', 'world_save'].includes(item.project_type)
}

export function localContentIconUrl(iconUrl?: string | null): string {
	if (!iconUrl) return ''
	return /^(https?:|data:|blob:|asset:|tauri:)/.test(iconUrl) ? iconUrl : convertFileSrc(iconUrl)
}

export async function loadInstanceContentData(
	path: string,
	cacheBehaviour?: CacheBehaviour,
	onError?: (error: Error) => unknown,
): Promise<InstanceContentData | null> {
	try {
		const snapshot =
			cacheBehaviour === 'bypass' || cacheBehaviour === 'must_revalidate'
				? await refresh_content(path)
				: await get_content_snapshot(path)
		const normalizedItems = snapshot.items.map((item) => {
			const fileName = item.expectedRelativePath.split('/').pop() ?? item.expectedRelativePath
			const requiresManualDownload =
				item.ownershipKind === 'pack_managed' &&
				item.required &&
				item.provider === 'curseforge' &&
				item.materializationState === 'pending_manual'
			const curseForgeProjectId =
				item.provider === 'curseforge' && /^\d+$/.test(item.providerProjectId ?? '')
					? Number(item.providerProjectId)
					: null
			const projectId =
				item.provider === 'curseforge' && curseForgeProjectId != null
					? `curseforge:${curseForgeProjectId}`
					: item.providerProjectId
			const fallbackContent: ContentItem = {
				id: item.memberId ?? item.entryId ?? item.fileId ?? item.expectedRelativePath,
				file_name: fileName,
				file_path: item.expectedRelativePath,
				size: 0,
				enabled: false,
				project_type: item.projectType,
				project: {
					id: projectId ?? `local:${item.expectedRelativePath}`,
					slug: projectId ?? item.expectedRelativePath,
					title: fileName,
					icon_url: undefined,
				},
				version: item.providerReleaseId
					? {
							id: item.providerReleaseId,
							version_number: item.providerReleaseId,
							file_name: fileName,
						}
					: undefined,
				update: null,
				origin_provider: item.provider,
				provider_refs:
					curseForgeProjectId != null
						? [
								{
									provider: 'curseforge' as const,
									project_id: curseForgeProjectId,
									file_id:
										item.providerReleaseId && /^\d+$/.test(item.providerReleaseId)
											? Number(item.providerReleaseId)
											: null,
								},
							]
						: item.provider === 'modrinth' && item.providerProjectId
							? [
									{
										provider: 'modrinth' as const,
										project_id: item.providerProjectId,
										version_id: item.providerReleaseId,
									},
								]
							: [],
				pendingManualDownload: requiresManualDownload,
			}
			return {
				...(item.content ?? fallbackContent),
				instanceFileId: item.fileId ?? undefined,
				instanceEntryId: item.entryId ?? undefined,
				instanceMemberId: item.memberId ?? undefined,
				instanceOwnershipKind: item.ownershipKind,
				instanceCapabilities: item.capabilities,
				instanceMaterializationState: item.materializationState,
				instanceOverrideKind: item.overrideKind,
				pendingManualDownload: requiresManualDownload,
				dependency: item.dependency ?? null,
			}
		}) satisfies ContentItem[]
		const contentItems = normalizedItems.filter(
			(item) => item.instanceOwnershipKind !== 'pack_managed',
		)
		const linkedContentItems = normalizedItems.filter(
			(item) => item.instanceOwnershipKind === 'pack_managed',
		)

		return {
			path,
			snapshot,
			contentItems,
			linkedContentItems,
			modpack: normalizePack(snapshot),
		}
	} catch (error) {
		if (onError) {
			onError(error as Error)
			return null
		}
		throw error
	}
}

function normalizePack(snapshot: InstanceContentSnapshot): InstanceContentModpackData | null {
	const pack = snapshot.pack
	if (!pack) return null
	const metadata = pack.metadata
	const project = metadata
		? normalizeProject(metadata, pack.iconPath)
		: {
				id: pack.projectId ?? snapshot.instanceId,
				slug: pack.projectId ?? snapshot.instanceId,
				title: pack.name,
				icon_url: pack.iconPath ?? undefined,
				description: '',
			}
	const version = metadata
		? ({
				...metadata.version,
				date_published: metadata.version.date_published.toString(),
			} as ContentModpackCardVersion)
		: null

	return {
		project,
		version,
		owner: metadata?.owner
			? {
					...metadata.owner,
					avatar_url: metadata.owner.avatar_url ?? undefined,
				}
			: null,
		hasUpdate: pack.canUpdate && metadata?.update != null,
		updateVersionId:
			metadata?.update?.provider === 'modrinth'
				? metadata.update.target_version_id
				: metadata?.update?.provider === 'curseforge'
					? String(metadata.update.target_file_id)
					: null,
	}
}

function normalizeProject(
	metadata: LinkedModpackInfo,
	fallbackIconPath?: string | null,
): ContentModpackCardProject {
	return {
		...metadata.project,
		slug: metadata.project.slug ?? metadata.project.id,
		icon_url: metadata.project.icon_url || fallbackIconPath || undefined,
	}
}
