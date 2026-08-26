import assert from 'node:assert/strict'
import test from 'node:test'

import { reactive } from 'vue'

import {
	clearUpgradeFlow,
	consumeUpgradeFlow,
	parkUpgradeFlow,
	peekUpgradeFlow,
	restoreUpgradeFlow,
	upgradeProjectPath,
} from '../../../helpers/upgrade-return-state.ts'

test('upgrade return snapshot is one-shot and instance-scoped', () => {
	clearUpgradeFlow()
	const snapshot = {
		instanceId: 'instance-a',
		returnFullPath: '/instance/instance-a/upgrade/compatibility',
		targetEnvironment: null,
		plan: null,
		createFullBackup: true,
		sharedUpgradeMode: null,
		activeJobId: null,
		result: null,
	}
	parkUpgradeFlow(snapshot)
	assert.equal(consumeUpgradeFlow('instance-b', snapshot.returnFullPath), null)
	assert.deepEqual(consumeUpgradeFlow('instance-a', snapshot.returnFullPath), snapshot)
	assert.equal(consumeUpgradeFlow('instance-a', snapshot.returnFullPath), null)
})

for (const route of ['compatibility', 'customize', 'confirm']) {
	test(`${route} return hydrates the parked plan before consuming it`, () => {
		clearUpgradeFlow()
		const snapshot = {
			instanceId: 'instance-a',
			returnFullPath: `/instance/instance-a/upgrade/${route}`,
			targetEnvironment: { gameVersion: '26.1.2' },
			plan: { id: 'same-plan' },
			createFullBackup: true,
			sharedUpgradeMode: null,
			activeJobId: null,
			result: null,
		} as never
		parkUpgradeFlow(snapshot)
		let hydratedPlanId: string | undefined
		const restored = restoreUpgradeFlow('instance-a', snapshot.returnFullPath, (value) => {
			hydratedPlanId = value.plan?.id
		})
		assert.equal(hydratedPlanId, 'same-plan')
		assert.equal(restored?.plan?.id, 'same-plan')
		assert.equal(peekUpgradeFlow('instance-a'), null)
	})
}

test('confirm project return restores plan and confirm choices without replanning', () => {
	clearUpgradeFlow()
	const snapshot = {
		instanceId: 'instance-a',
		returnFullPath: '/instance/instance-a/upgrade/confirm',
		targetEnvironment: { gameVersion: '26.1.2' },
		plan: {
			id: 'same-plan',
			selectedSolution: { kind: 'custom' },
			customConstraints: [{ contentId: 'root', versionId: 'fixed' }],
		},
		createFullBackup: false,
		directFullBackupPreference: false,
		sharedUpgradeMode: 'direct',
		activeJobId: null,
		result: null,
	} as never
	parkUpgradeFlow(snapshot)
	let restoredSnapshot: typeof snapshot | null = null
	restoreUpgradeFlow('instance-a', snapshot.returnFullPath, (value) => {
		restoredSnapshot = value as typeof snapshot
	})

	assert.equal(restoredSnapshot?.plan.id, 'same-plan')
	assert.deepEqual(restoredSnapshot?.targetEnvironment, snapshot.targetEnvironment)
	assert.deepEqual(restoredSnapshot?.plan.selectedSolution, snapshot.plan.selectedSolution)
	assert.deepEqual(restoredSnapshot?.plan.customConstraints, snapshot.plan.customConstraints)
	assert.equal(restoredSnapshot?.createFullBackup, false)
	assert.equal(restoredSnapshot?.sharedUpgradeMode, 'direct')
})

test('failed hydration leaves the parked snapshot available', () => {
	clearUpgradeFlow()
	const snapshot = {
		instanceId: 'instance-a',
		returnFullPath: '/instance/instance-a/upgrade/compatibility',
		targetEnvironment: null,
		plan: null,
		createFullBackup: true,
		sharedUpgradeMode: null,
		activeJobId: null,
		result: null,
	}
	parkUpgradeFlow(snapshot)
	assert.throws(() =>
		restoreUpgradeFlow('instance-a', snapshot.returnFullPath, () => {
			throw new Error('hydrate failed')
		}),
	)
	assert.deepEqual(peekUpgradeFlow('instance-a'), snapshot)
})

test('confirm project title routes match trusted provider routes only', () => {
	assert.equal(upgradeProjectPath('modrinth', 'P7dR8mSH'), '/project/P7dR8mSH')
	assert.equal(upgradeProjectPath('curseforge', '123'), '/project/curseforge/123')
	assert.equal(upgradeProjectPath('local', 'pack'), null)
	assert.equal(upgradeProjectPath(null, 'unidentified'), null)
})

test('upgrade return snapshot detaches reactive flow DTOs', () => {
	clearUpgradeFlow()
	const snapshot = reactive({
		instanceId: 'reactive-instance',
		returnFullPath: '/instance/reactive-instance/upgrade/compatibility',
		targetEnvironment: null,
		plan: null,
		createFullBackup: true,
		sharedUpgradeMode: null,
		activeJobId: null,
		result: null,
	})
	parkUpgradeFlow(snapshot)
	snapshot.createFullBackup = false
	assert.equal(peekUpgradeFlow('reactive-instance')?.createFullBackup, true)
	assert.equal(consumeUpgradeFlow('wrong-instance', snapshot.returnFullPath), null)
})
