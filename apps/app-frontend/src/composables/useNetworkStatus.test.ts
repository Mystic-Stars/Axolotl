import assert from 'node:assert/strict'
import test from 'node:test'

import { createNetworkStatus } from './useNetworkStatus.ts'

test('network probe failures recover when a later probe succeeds', () => {
	const status = createNetworkStatus()

	status.setNetworkReachable(false)
	assert.equal(status.offline.value, true)

	status.setNetworkReachable(true)
	assert.equal(status.offline.value, false)
})

test('browser online clears a temporary probe failure', () => {
	const status = createNetworkStatus()
	status.setNetworkReachable(false)
	status.setBrowserOffline(true)
	assert.equal(status.offline.value, true)

	status.markBrowserOnline()
	assert.equal(status.browserOffline.value, false)
	assert.equal(status.offline.value, false)
})

test('browser offline remains authoritative over a successful probe', () => {
	const status = createNetworkStatus()
	status.setNetworkReachable(true)
	status.setBrowserOffline(true)

	assert.equal(status.offline.value, true)
})
