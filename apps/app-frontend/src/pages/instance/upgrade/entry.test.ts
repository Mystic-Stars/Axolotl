import assert from 'node:assert/strict'
import test from 'node:test'

import type { InstallJobSnapshot } from '@/helpers/install'
import type { GameInstance } from '@/helpers/types'

import { isActiveUpgradeJobForInstance, isUnmanagedUpgradeEligible } from './entry.ts'

const instance = (link: GameInstance['link'] = null): GameInstance => ({
	id: 'instance',
	path: 'path',
	install_stage: 'installed',
	launcher_feature_version: '1',
	name: 'Instance',
	game_version: '1.21.8',
	loader: 'fabric',
	loader_components: [],
	groups: [],
	link,
	update_channel: 'release',
	created: new Date(),
	modified: new Date(),
	submitted_time_played: 0,
	recent_time_played: 0,
	hooks: {},
})

test('eligibility allows local/shared and excludes managed packs', () => {
	assert.equal(isUnmanagedUpgradeEligible(instance()), true)
	assert.equal(
		isUnmanagedUpgradeEligible(instance({ type: 'shared_instance', shared_instance_id: 'shared' })),
		true,
	)
	assert.equal(
		isUnmanagedUpgradeEligible(
			instance({ type: 'modrinth_modpack', project_id: 'p', version_id: 'v' }),
		),
		false,
	)
	assert.equal(isUnmanagedUpgradeEligible({ ...instance(), install_stage: 'not_installed' }), false)
})

test('active upgrade job ownership is exact', () => {
	const job = {
		kind: 'upgrade_unmanaged_instance',
		status: 'running',
		instance_id: 'instance',
	} as InstallJobSnapshot
	assert.equal(isActiveUpgradeJobForInstance(job, 'instance'), true)
	assert.equal(isActiveUpgradeJobForInstance(job, 'other'), false)
	assert.equal(isActiveUpgradeJobForInstance({ ...job, status: 'succeeded' }, 'instance'), false)
})

test('active copy upgrade belongs to original source instance', () => {
	const job = {
		kind: 'upgrade_unmanaged_instance',
		status: 'running',
		instance_id: 'copy',
		source_instance_id: 'source',
	} as InstallJobSnapshot
	assert.equal(isActiveUpgradeJobForInstance(job, 'source'), true)
	assert.equal(isActiveUpgradeJobForInstance(job, 'copy'), false)
})
