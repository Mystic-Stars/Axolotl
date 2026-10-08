import assert from 'node:assert/strict'
import test from 'node:test'

import { createJavaFormatter } from './formatter.ts'
import { parseIdentifier } from './identifier.ts'

test('formats Java 1.12 ingredients and results with legacy data values', () => {
    const formatter = createJavaFormatter('1.12')
    const stone = parseIdentifier('minecraft:stone:1')
    assert.deepEqual(formatter.ingredient(stone, false), { item: 'minecraft:stone', data: 1 })
    assert.deepEqual(formatter.result(stone, 2), {
        item: 'minecraft:stone',
        data: 1,
        count: 2,
    })
    assert.equal(formatter.typeName('crafting_shaped'), 'crafting_shaped')
})

test('formats Java 1.13 ingredients, tags, and string cooking results', () => {
    const formatter = createJavaFormatter('1.13')
    const iron = parseIdentifier('minecraft:iron_ingot')
    assert.deepEqual(formatter.ingredient(iron, false), { item: 'minecraft:iron_ingot' })
    assert.deepEqual(formatter.ingredient(iron, true), { tag: 'minecraft:iron_ingot' })
    assert.deepEqual(formatter.result(iron), { item: 'minecraft:iron_ingot' })
    assert.equal(formatter.cookingResult(iron), 'minecraft:iron_ingot')
    assert.equal(formatter.typeName('smelting'), 'smelting')
})

test('namespaces recipe types from Java 1.14 onwards', () => {
    const formatter = createJavaFormatter('1.14')
    assert.equal(formatter.typeName('smelting'), 'minecraft:smelting')
})

test('switches results to id objects from Java 1.20', () => {
    const formatter = createJavaFormatter('1.20')
    const iron = parseIdentifier('minecraft:iron_ingot')
    assert.deepEqual(formatter.result(iron, 3), { id: 'minecraft:iron_ingot', count: 3 })
    assert.deepEqual(formatter.cookingResult(iron), { id: 'minecraft:iron_ingot' })
    assert.deepEqual(formatter.stonecutterResult(iron, 4), {
        result: { id: 'minecraft:iron_ingot', count: 4 },
    })
})

test('uses string ingredients and tag references from Java 1.21.2', () => {
    const formatter = createJavaFormatter('1.21.2')
    const iron = parseIdentifier('minecraft:iron_ingot')
    assert.equal(formatter.ingredient(iron, false), 'minecraft:iron_ingot')
    assert.equal(formatter.ingredient(iron, true), '#minecraft:iron_ingot')
})
