import assert from 'node:assert/strict'
import test from 'node:test'

import { upgradeVersionCacheKey, upgradeVersionDisplayLabel } from './upgrade-version-display.ts'

test('release identity remains separate from display version', () => {
	const identity = { provider: 'modrinth', projectId: 'project', releaseId: 'raw-id' }
	const metadata = new Map([
		[upgradeVersionCacheKey('modrinth', 'project', 'raw-id'), { version: '1.2.3' }],
	])
	assert.equal(upgradeVersionDisplayLabel(metadata, identity), '1.2.3')
	assert.equal(identity.releaseId, 'raw-id')
})

test('current, target, candidate, and dependency releases resolve independently', () => {
	const metadata = new Map([
		[upgradeVersionCacheKey('modrinth', 'root', 'current-id'), { version: '1.0.0' }],
		[upgradeVersionCacheKey('modrinth', 'root', 'target-id'), { version: '2.0.0' }],
		[upgradeVersionCacheKey('modrinth', 'root', 'candidate-id'), { version: '2.1.0-beta' }],
		[upgradeVersionCacheKey('modrinth', 'dependency', 'dependency-id'), { version: '3.0.0' }],
	])
	const label = (projectId: string, releaseId: string) =>
		upgradeVersionDisplayLabel(metadata, { provider: 'modrinth', projectId, releaseId })
	assert.equal(label('root', 'current-id'), '1.0.0')
	assert.equal(label('root', 'target-id'), '2.0.0')
	assert.equal(label('root', 'candidate-id'), '2.1.0-beta')
	assert.equal(label('dependency', 'dependency-id'), '3.0.0')
})

test('current-only release metadata does not require a target identity', () => {
	const metadata = new Map([
		[upgradeVersionCacheKey('modrinth', 'project', 'current-id'), { version: '1.8' }],
	])
	assert.equal(
		upgradeVersionDisplayLabel(metadata, {
			provider: 'modrinth',
			projectId: 'project',
			releaseId: 'current-id',
		}),
		'1.8',
	)
})
