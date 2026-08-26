import assert from 'node:assert/strict'
import test from 'node:test'

import { computed, ref } from 'vue'

import {
	bulkResolutionAction,
	filterBulkResolutionIds,
	initialCustomizeStrategy,
	UPGRADE_ACTIVE_STEPS,
	upgradeControlEnabled,
	upgradeProgressModel,
} from './flow-controls.ts'

test('registered upgrade control reads live ref values without re-registration', () => {
	const canPlan = ref(false)
	const control = computed(() => canPlan.value)
	assert.equal(upgradeControlEnabled(control), false)
	canPlan.value = true
	assert.equal(upgradeControlEnabled(control), true)
})

test('missing controls remain disabled', () => {
	assert.equal(upgradeControlEnabled(undefined), false)
})

test('upgrade progress maps five active routes and terminal result', () => {
	assert.equal(UPGRADE_ACTIVE_STEPS.length, 5)
	for (const [index, route] of UPGRADE_ACTIVE_STEPS.entries()) {
		assert.deepEqual(upgradeProgressModel(`/instance/example/upgrade/${route}`), {
			currentIndex: index,
			complete: false,
			steps: UPGRADE_ACTIVE_STEPS,
		})
	}
	assert.equal(upgradeProgressModel('/instance/example/upgrade/result').complete, true)
})

test('customize strategy prefers flow UI state over selected backend solution', () => {
	assert.equal(initialCustomizeStrategy('custom', 'newest', 'custom'), 'custom')
	assert.equal(initialCustomizeStrategy(null, 'minimal_change', 'custom'), 'minimal_change')
	assert.equal(initialCustomizeStrategy(null, null, 'custom'), 'custom')
})

test('bulk resolution state and no-op filtering use authoritative actions', () => {
	assert.equal(bulkResolutionAction(['keep', 'keep']), 'keep')
	assert.equal(bulkResolutionAction(['disable', 'disable']), 'disable')
	assert.equal(bulkResolutionAction(['keep', 'disable']), null)
	assert.deepEqual(
		filterBulkResolutionIds(
			[
				{ contentId: 'a', action: 'keep' },
				{ contentId: 'b', action: 'disable' },
			],
			'keep',
		),
		['b'],
	)
})
