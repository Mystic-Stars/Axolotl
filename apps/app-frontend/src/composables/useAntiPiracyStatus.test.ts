import assert from 'node:assert/strict'
import test from 'node:test'

import { createAntiPiracyStatus, isOfflineAccountRestrictedError } from './useAntiPiracyStatus.ts'

test('a late region event cannot undo a cleared official login record', () => {
    const { status, setStatus } = createAntiPiracyStatus()
    setStatus({ region: 'non_cn', restricted: false, revision: 1 })
    setStatus({ region: 'non_cn', restricted: true, revision: 2 })
    setStatus({ region: 'non_cn', restricted: false, revision: 1 })
    assert.equal(status.value.restricted, true)
    assert.equal(status.value.revision, 2)
})

test('a late eligibility read cannot undo a successful official login', () => {
    const { status, setStatus } = createAntiPiracyStatus()
    setStatus({ region: 'non_cn', restricted: true, revision: 1 })
    setStatus({ region: 'non_cn', restricted: false, revision: 2 })
    setStatus({ region: 'non_cn', restricted: true, revision: 1 })
    assert.equal(status.value.restricted, false)
})

test('restricted backend errors are recognized in Tauri and Error forms', () => {
    assert.equal(isOfflineAccountRestrictedError('Input error: OFFLINE_ACCOUNT_RESTRICTED'), true)
    assert.equal(isOfflineAccountRestrictedError(new Error('OFFLINE_ACCOUNT_RESTRICTED')), true)
    assert.equal(isOfflineAccountRestrictedError({ message: 'OFFLINE_ACCOUNT_RESTRICTED' }), true)
    assert.equal(isOfflineAccountRestrictedError(undefined), false)
    assert.equal(isOfflineAccountRestrictedError(new Error('Other failure')), false)
})
