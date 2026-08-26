import assert from 'node:assert/strict'
import test from 'node:test'

import { ref } from 'vue'

import type { InstallJobSnapshot } from '@/helpers/install'
import type { InstanceUpgradePlan, InstanceUpgradeResult } from '@/helpers/instance-upgrade'

import {
	attachUpgradeJobToFlow,
	type InstanceUpgradeFlow,
	isUpgradeRouteAvailable,
	isUpgradeRouteRecoveryPending,
	upgradeDownloadsLocation,
	upgradeProgressDestination,
} from './flow.ts'

function selectionFlow(plan: InstanceUpgradePlan | null): InstanceUpgradeFlow {
	return { plan: ref(plan) } as InstanceUpgradeFlow
}

test('selection route requires an unblocked plan with a selected solution', () => {
	const selectedSolution = { kind: 'newest', selections: [], dependencyChanges: [], warnings: [] }
	assert.equal(isUpgradeRouteAvailable('selection', selectionFlow(null)), false)
	assert.equal(
		isUpgradeRouteAvailable(
			'selection',
			selectionFlow({ blockingIssues: [], selectedSolution: null } as InstanceUpgradePlan),
		),
		false,
	)
	assert.equal(
		isUpgradeRouteAvailable(
			'selection',
			selectionFlow({
				blockingIssues: [{ code: 'dependency_conflict' }],
				selectedSolution,
			} as InstanceUpgradePlan),
		),
		false,
	)
	assert.equal(
		isUpgradeRouteAvailable(
			'selection',
			selectionFlow({ blockingIssues: [], selectedSolution } as InstanceUpgradePlan),
		),
		true,
	)
})

test('upgrade execution and Progress recovery target focused Downloads', () => {
	assert.deepEqual(upgradeDownloadsLocation('job/a'), {
		path: '/downloads',
		query: { job: 'job/a' },
	})
	assert.equal(upgradeProgressDestination('loading', null, 'instance/a'), null)
	assert.deepEqual(upgradeProgressDestination('ready', 'job/a', 'instance/a'), {
		path: '/downloads',
		query: { job: 'job/a' },
	})
	assert.deepEqual(upgradeProgressDestination('ready', null, 'instance/a'), {
		path: '/instance/instance%2Fa/upgrade',
	})
})

test('accepted upgrade job sets ownership, preserves backend result, and returns Downloads target', () => {
	let jobId: string | null = null
	let result: unknown = null
	const location = attachUpgradeJobToFlow(
		{
			setJob: (value) => (jobId = value),
			setResult: (value) => (result = value),
		},
		{
			job_id: 'job-a',
			status: 'succeeded',
			upgrade_result: { planId: 'plan-a' } as InstanceUpgradeResult,
		} as InstallJobSnapshot,
	)
	assert.equal(jobId, 'job-a')
	assert.deepEqual(result, { planId: 'plan-a' })
	assert.deepEqual(location, { path: '/downloads', query: { job: 'job-a' } })
})

test('job route waits only while persisted job recovery is loading', () => {
	const loading = {
		jobRecoveryState: ref('loading'),
		activeJobId: ref(null),
	} as InstanceUpgradeFlow
	assert.equal(isUpgradeRouteRecoveryPending('job', loading), true)
	assert.equal(isUpgradeRouteRecoveryPending('result', loading), true)
	loading.jobRecoveryState.value = 'ready'
	assert.equal(isUpgradeRouteRecoveryPending('job', loading), false)
	assert.equal(isUpgradeRouteAvailable('job', loading), false)
	loading.activeJobId.value = 'job-a'
	assert.equal(isUpgradeRouteAvailable('job', loading), true)
})
