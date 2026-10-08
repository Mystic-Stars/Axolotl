import assert from 'node:assert/strict'
import test from 'node:test'

import { shouldShowMinecraftCrash } from './process.js'

test('only shows the crash dialog for an explicitly crashed process', () => {
    assert.equal(shouldShowMinecraftCrash(true), true)
    assert.equal(shouldShowMinecraftCrash(false), false)
    assert.equal(shouldShowMinecraftCrash(undefined), false)
    assert.equal(shouldShowMinecraftCrash(null), false)
    assert.equal(shouldShowMinecraftCrash(1), false)
})
