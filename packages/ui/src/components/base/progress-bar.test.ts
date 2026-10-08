import assert from 'node:assert/strict'
import test from 'node:test'

import { progressRatio } from './progress-bar.ts'

test('converts ratios, percentage pairs, and natural units to the same ratio', () => {
    assert.equal(progressRatio(0.5, 1), 0.5)
    assert.equal(progressRatio(50, 100), 0.5)
    assert.equal(progressRatio(256, 512), 0.5)
})

test('clamps finite progress to the inclusive 0..1 range', () => {
    assert.equal(progressRatio(-25, 100), 0)
    assert.equal(progressRatio(125, 100), 1)
})

test('treats non-positive maxima as zero progress', () => {
    assert.equal(progressRatio(1, 0), 0)
    assert.equal(progressRatio(1, -1), 0)
})

test('treats non-finite progress or maxima as zero progress', () => {
    for (const progress of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
        assert.equal(progressRatio(progress, 100), 0)
    }
    for (const max of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
        assert.equal(progressRatio(50, max), 0)
    }
})
