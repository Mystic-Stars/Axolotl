import assert from 'node:assert/strict'
import test from 'node:test'

import { escapeSchematicCsvCell, normalizeSchematicLayerRange } from './utils.ts'

test('layer ranges remain ordered and inside structure bounds', () => {
    assert.deepEqual(normalizeSchematicLayerRange(20, 5, -4, 16), [5, 16])
    assert.deepEqual(normalizeSchematicLayerRange(-20, -10, -4, 16), [-4, -4])
})

test('material CSV cells escape quotes, commas, and line breaks', () => {
    assert.equal(escapeSchematicCsvCell('minecraft:stone'), 'minecraft:stone')
    assert.equal(escapeSchematicCsvCell('name,"quoted"'), '"name,""quoted"""')
    assert.equal(escapeSchematicCsvCell('two\nlines'), '"two\nlines"')
})
