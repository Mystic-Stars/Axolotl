import assert from 'node:assert/strict'
import test from 'node:test'

import type { ContentItem } from '@modrinth/ui'

import type { PendingContentChange } from '@/providers/download-manager.ts'

import {
	activeContentChangeJobs,
	contentChangeAffectsItem,
	hasActiveContentChange,
	pendingContentChangeAffectsItem,
} from './content-change-jobs.ts'
import type { InstallJobSnapshot } from './install.ts'

function job(
	status: InstallJobSnapshot['status'],
	intent: NonNullable<InstallJobSnapshot['content_change']>['intent'],
	contentIds: string[] = [],
): InstallJobSnapshot {
	return {
		job_id: `${status}-${intent.type}`,
		instance_id: 'instance-a',
		kind: 'change_content',
		status,
		target: { type: 'existing_instance', instance_id: 'instance-a' },
		content_change: { intent, content_ids: contentIds },
	} as InstallJobSnapshot
}

const userItem = {
	instanceEntryId: 'entry-a',
	instanceOwnershipKind: 'user_added',
} as ContentItem
const packItem = {
	instanceEntryId: 'entry-b',
	instanceOwnershipKind: 'pack_managed',
} as ContentItem

test('active content jobs are restored by kind, instance, and active status', () => {
	const active = job('running', { type: 'update_one', content_id: 'entry-a' }, ['entry-a'])
	const paused = job('waiting_for_user', { type: 'update_one', content_id: 'entry-a' }, ['entry-a'])
	const finished = job('succeeded', { type: 'update_one', content_id: 'entry-a' }, ['entry-a'])
	const otherInstance = {
		...active,
		job_id: 'other-instance',
		instance_id: 'instance-b',
		target: { type: 'existing_instance', instance_id: 'instance-b' },
	} as InstallJobSnapshot
	const install = { ...active, job_id: 'install', kind: 'install_content' } as InstallJobSnapshot

	assert.deepEqual(
		activeContentChangeJobs([active, paused, finished, otherInstance, install], 'instance-a'),
		[active, paused],
	)
})

test('resolved jobs affect only their persisted stable content IDs', () => {
	const update = job('queued', { type: 'update_one', content_id: 'entry-a' }, ['entry-a'])
	assert.equal(contentChangeAffectsItem(update, userItem), true)
	assert.equal(contentChangeAffectsItem(update, packItem), false)
	assert.equal(hasActiveContentChange([update], userItem), true)
})

test('unresolved update-all marks user-added content without blocking pack content', () => {
	const updateAll = job('queued', { type: 'update_all_user_added' })
	assert.equal(contentChangeAffectsItem(updateAll, userItem), true)
	assert.equal(contentChangeAffectsItem(updateAll, packItem), false)
})

test('submission reservations synchronously cover their exact content scope', () => {
	const selected: PendingContentChange = {
		id: 'pending-selected',
		instanceId: 'instance-a',
		intent: {
			type: 'update_selected',
			targets: [{ content_id: 'entry-a', target_release_id: 'release-a' }],
		},
		contentIds: ['entry-a'],
		updateAll: false,
	}
	const updateAll: PendingContentChange = {
		id: 'pending-all',
		instanceId: 'instance-a',
		intent: { type: 'update_all_user_added' },
		contentIds: [],
		updateAll: true,
	}

	assert.equal(pendingContentChangeAffectsItem(selected, userItem), true)
	assert.equal(pendingContentChangeAffectsItem(selected, packItem), false)
	assert.equal(pendingContentChangeAffectsItem(updateAll, userItem), true)
	assert.equal(pendingContentChangeAffectsItem(updateAll, packItem), false)
})
