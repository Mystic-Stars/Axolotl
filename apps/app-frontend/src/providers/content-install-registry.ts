import type { Labrinth } from '@modrinth/api-client'
import type { ContentItem } from '@modrinth/ui'
import type { Ref } from 'vue'

import { get_organization, get_team } from '@/helpers/cache.js'

export interface InstallingProject {
	id: string
	slug?: string | null
	title: string
	icon_url?: string | null
	project_type?: string
	organization?: string | null
	team?: string
}

/**
 * Placeholder rows shown in the instance content list while an install is in flight.
 */
export function createInstallingItemsRegistry(
	installingItems: Ref<Map<string, ContentItem[]>>,
	debugState: (label: string, data?: unknown) => void,
) {
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

	function addInstallingItem(
		instanceId: string,
		project: InstallingProject,
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

	return {
		addInstallingItem,
		updateInstallingItem,
		removeInstallingItems,
	}
}
