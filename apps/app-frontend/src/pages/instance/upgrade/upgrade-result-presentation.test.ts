import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

import type { InstanceUpgradeSolution } from '@/helpers/instance-upgrade'

import {
	filterUpgradeDetailItems,
	paginateUpgradeDetailItems,
	UPGRADE_RESULT_PAGE_SIZE,
	upgradeDetailItems,
	upgradeDetailProjectIdentities,
	upgradeDetailReleaseIdentities,
} from './upgrade-result-presentation.ts'

function largeSolution(): InstanceUpgradeSolution {
	return {
		kind: 'custom',
		warnings: [],
		selections: Array.from({ length: 500 }, (_, index) => ({
			contentId: `example-${index}`,
			provider: 'modrinth',
			projectId: `project-${index}`,
			currentReleaseId: `old-${index}`,
			targetReleaseId: `new-${index}`,
			action: index % 3 === 0 ? 'keep' : index % 3 === 1 ? 'disable' : 'upgrade',
			enabled: index % 3 !== 1,
		})),
		dependencyChanges: [],
	}
}

test('500-item result paginates to 25 real visible rows per page', () => {
	const all = upgradeDetailItems(largeSolution())
	const first = paginateUpgradeDetailItems(all, 1)
	const second = paginateUpgradeDetailItems(all, 2)
	assert.equal(UPGRADE_RESULT_PAGE_SIZE, 25)
	assert.equal(first.items.length, 25)
	assert.deepEqual(
		first.items.map((item) => item.contentId),
		Array.from({ length: 25 }, (_, index) => `example-${index}`),
	)
	assert.deepEqual(
		second.items.map((item) => item.contentId),
		Array.from({ length: 25 }, (_, index) => `example-${index + 25}`),
	)
})

test('search and status filters happen before pagination', () => {
	const all = upgradeDetailItems(largeSolution())
	const match = filterUpgradeDetailItems(all, 'all', 'example-487')
	assert.deepEqual(
		match.map((item) => item.contentId),
		['example-487'],
	)
	const updated = filterUpgradeDetailItems(all, 'updated', '')
	assert.ok(updated.every((item) => item.action === 'upgrade'))
	assert.equal(
		paginateUpgradeDetailItems(updated, 99).page <=
			paginateUpgradeDetailItems(updated, 99).pageCount,
		true,
	)
})

test('metadata scope contains only identities from the current visible page', () => {
	const visible = paginateUpgradeDetailItems(upgradeDetailItems(largeSolution()), 1).items
	assert.equal(upgradeDetailProjectIdentities(visible).length, 25)
	assert.equal(upgradeDetailReleaseIdentities(visible).length, 50)
})

test('component resets page on filter/search and lazily mounts paginated rows', () => {
	const source = readFileSync(new URL('./UpgradeResultCollections.vue', import.meta.url), 'utf8')
	assert.match(source, /watch\(\[search, filter\],[\s\S]*?page\.value = 1[\s\S]*?\)/)
	assert.match(source, /v-if="detailsOpen"/)
	assert.match(source, /v-for="item in visibleRows"/)
	assert.doesNotMatch(source, /v-for="item in allItems"/)
	assert.match(source, /upgradeDetailProjectIdentities\(pageData\.value\.items\)/)
})
