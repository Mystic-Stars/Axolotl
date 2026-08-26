import assert from 'node:assert/strict'
import test from 'node:test'

import { isCurrentUpgradeSelectPlanning } from './planning-navigation.ts'

test('planner continuation navigates only while same Select request remains current', () => {
	assert.equal(isCurrentUpgradeSelectPlanning(false, 1, 1, 'InstanceUpgrade', 'a', 'a'), true)
	assert.equal(isCurrentUpgradeSelectPlanning(false, 1, 1, 'InstanceContent', 'a', 'a'), false)
	assert.equal(isCurrentUpgradeSelectPlanning(false, 1, 2, 'InstanceUpgrade', 'a', 'a'), false)
	assert.equal(isCurrentUpgradeSelectPlanning(true, 1, 1, 'InstanceUpgrade', 'a', 'a'), false)
})

test('pending planner completion does not navigate after route changes', async () => {
	let resolvePlanner!: () => void
	const planner = new Promise<void>((resolve) => {
		resolvePlanner = resolve
	})
	let routeName = 'InstanceUpgrade'
	let navigations = 0
	const continuation = planner.then(() => {
		if (isCurrentUpgradeSelectPlanning(false, 1, 1, routeName, 'a', 'a')) navigations += 1
	})

	routeName = 'InstanceContent'
	resolvePlanner()
	await continuation

	assert.equal(navigations, 0)
})

test('pending planner completion navigates once while Select remains current', async () => {
	let resolvePlanner!: () => void
	const planner = new Promise<void>((resolve) => {
		resolvePlanner = resolve
	})
	let navigations = 0
	const continuation = planner.then(() => {
		if (isCurrentUpgradeSelectPlanning(false, 1, 1, 'InstanceUpgrade', 'a', 'a')) {
			navigations += 1
		}
	})

	resolvePlanner()
	await continuation

	assert.equal(navigations, 1)
})
