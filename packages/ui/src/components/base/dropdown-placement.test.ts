import assert from 'node:assert/strict'
import test from 'node:test'

import { dropdownPlacement, shouldRenderDropdownUp } from './dropdown-placement.ts'

const baseGeometry = {
	viewportHeight: 800,
	controlTop: 400,
	controlBottom: 440,
	floatingActionBarClearance: 0,
	safeGap: 8,
	expectedMenuHeight: 300,
}

test('dropdown stays down when expected menu fits below', () => {
	assert.equal(shouldRenderDropdownUp(baseGeometry), false)
})

test('floating action bar clearance moves a colliding dropdown upward', () => {
	assert.equal(shouldRenderDropdownUp({ ...baseGeometry, floatingActionBarClearance: 180 }), true)
})

test('dropdown chooses side with more space when neither side fits', () => {
	assert.equal(
		shouldRenderDropdownUp({
			...baseGeometry,
			controlTop: 450,
			controlBottom: 490,
			expectedMenuHeight: 500,
		}),
		true,
	)
	assert.equal(
		shouldRenderDropdownUp({
			...baseGeometry,
			controlTop: 250,
			controlBottom: 290,
			expectedMenuHeight: 600,
		}),
		false,
	)
})

test('zero floating action bar clearance uses normal viewport space', () => {
	assert.equal(
		shouldRenderDropdownUp({
			...baseGeometry,
			controlTop: 300,
			controlBottom: 340,
			floatingActionBarClearance: 0,
		}),
		false,
	)
})

test('placement caps menu height to selected side above the floating bar', () => {
	assert.deepEqual(
		dropdownPlacement({
			...baseGeometry,
			controlTop: 260,
			controlBottom: 300,
			floatingActionBarClearance: 180,
		}),
		{ renderUp: false, availableHeight: 312 },
	)
	assert.deepEqual(
		dropdownPlacement({
			...baseGeometry,
			controlTop: 450,
			controlBottom: 490,
			floatingActionBarClearance: 80,
			expectedMenuHeight: 500,
		}),
		{ renderUp: true, availableHeight: 442 },
	)
})
