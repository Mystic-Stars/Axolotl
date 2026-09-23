import assert from 'node:assert/strict'
import test from 'node:test'

import type { ContentItem } from '@modrinth/ui'

import { mergeContentItemMetadata } from './content-item-metadata.ts'

test('lazy update metadata enables the update action on the existing stable entry', () => {
	const item = {
		instanceEntryId: 'entry-a',
		instanceMaterializationState: 'present',
		instanceCapabilities: { canUpdate: false, canChangeVersion: true },
		update: null,
	} as ContentItem
	for (const update of [
		{ provider: 'modrinth', target_version_id: 'new-version' },
		{ provider: 'curseforge', target_file_id: 42 },
	]) {
		const refreshed = mergeContentItemMetadata(item, { update } as ContentItem)
		assert.equal(refreshed.instanceEntryId, 'entry-a')
		assert.equal(refreshed.instanceCapabilities?.canUpdate, true)
		assert.equal(refreshed.instanceCapabilities?.canChangeVersion, true)
		assert.equal(
			mergeContentItemMetadata(refreshed, { update: null } as ContentItem).instanceCapabilities
				?.canUpdate,
			false,
		)
		assert.equal(item.instanceCapabilities?.canUpdate, false)
	}
})

test('update metadata does not enable mutation of a missing or removed file', () => {
	for (const instanceMaterializationState of ['missing', 'removed', 'pending_manual']) {
		const item = {
			instanceMaterializationState,
			instanceCapabilities: { canUpdate: false },
		} as ContentItem
		const metadata = { update: { provider: 'modrinth', target_version_id: 'new' } } as ContentItem
		assert.equal(mergeContentItemMetadata(item, metadata).instanceCapabilities?.canUpdate, false)
	}
})
