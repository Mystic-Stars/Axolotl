import assert from 'node:assert/strict'
import test from 'node:test'

import { mergeLiveLogHistory } from './console-history.ts'

test('retains the persisted latest.log history and appends a live-only tail', () => {
    assert.equal(
        mergeLiveLogHistory('first\nsecond\nthird', 'second\nthird\nfourth'),
        'first\nsecond\nthird\nfourth',
    )
})

test('does not duplicate a buffer already persisted in latest.log', () => {
    assert.equal(
        mergeLiveLogHistory('first\nsecond\nthird', 'second\nthird'),
        'first\nsecond\nthird',
    )
})

test('keeps live output when it is not present in latest.log', () => {
    assert.equal(mergeLiveLogHistory('minecraft', 'launcher'), 'minecraft\nlauncher')
})

test('accepts an unavailable latest.log while the instance is starting', () => {
    assert.equal(mergeLiveLogHistory('', 'launcher output'), 'launcher output')
})
