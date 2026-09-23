import type { ContentItem } from '@modrinth/ui'

import { isActiveInstallJobStatus } from './install-job-status.ts'
import type { InstallJobSnapshot } from './install.ts'
import type { PendingContentChange } from '@/providers/download-manager.ts'

export function activeContentChangeJobs(
	jobs: InstallJobSnapshot[],
	instanceId: string,
): InstallJobSnapshot[] {
	return jobs.filter(
		(job) =>
			job.kind === 'change_content' &&
			(job.instance_id ?? job.target.instance_id ?? null) === instanceId &&
			isActiveInstallJobStatus(job.status),
	)
}

export function contentItemStableId(item: ContentItem): string | null {
	return item.instanceEntryId ?? item.instanceMemberId ?? item.instanceFileId ?? null
}

export function contentChangeAffectsItem(job: InstallJobSnapshot, item: ContentItem): boolean {
	const change = job.content_change
	if (!change) return false
	if (change.intent.type === 'update_all_user_added' && change.content_ids.length === 0) {
		return item.instanceOwnershipKind === 'user_added'
	}
	const contentId = contentItemStableId(item)
	return contentId != null && change.content_ids.includes(contentId)
}

export function hasActiveContentChange(jobs: InstallJobSnapshot[], item: ContentItem): boolean {
	return jobs.some((job) => contentChangeAffectsItem(job, item))
}

export function pendingContentChangeAffectsItem(
	pending: PendingContentChange,
	item: ContentItem,
): boolean {
	if (pending.updateAll) return item.instanceOwnershipKind === 'user_added'
	const contentId = contentItemStableId(item)
	return contentId != null && pending.contentIds.includes(contentId)
}
