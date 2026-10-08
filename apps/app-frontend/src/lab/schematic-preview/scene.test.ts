import assert from 'node:assert/strict'
import test from 'node:test'

import { filterNativeWalkMouseDelta } from './scene.ts'

test('native walk mouse input ignores cursor-wrap spikes', () => {
    assert.equal(filterNativeWalkMouseDelta(24), 24)
    assert.equal(filterNativeWalkMouseDelta(-128), -128)
    assert.equal(filterNativeWalkMouseDelta(129), 0)
    assert.equal(filterNativeWalkMouseDelta(Number.POSITIVE_INFINITY), 0)
})
