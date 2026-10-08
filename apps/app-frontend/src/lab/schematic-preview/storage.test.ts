import assert from 'node:assert/strict'
import test from 'node:test'

import {
    clearRecentSchematics,
    loadRecentSchematics,
    recordRecentSchematic,
    removeRecentSchematic,
} from './storage.ts'

function createStorage() {
    const data = new Map<string, string>()
    return {
        getItem(key: string) {
            return data.get(key) ?? null
        },
        setItem(key: string, value: string) {
            data.set(key, value)
        },
        removeItem(key: string) {
            data.delete(key)
        },
    }
}

test('recent schematics deduplicate sources and retain at most five entries', () => {
    Object.defineProperty(globalThis, 'localStorage', {
        value: createStorage(),
        configurable: true,
    })
    for (let index = 0; index < 12; index += 1) {
        recordRecentSchematic(
            { kind: 'external', path: `/tmp/${index}.litematic` },
            `${index}.litematic`,
        )
    }
    recordRecentSchematic({ kind: 'external', path: '/tmp/5.litematic' }, 'renamed.litematic')
    const records = loadRecentSchematics()
    assert.equal(records.length, 5)
    assert.equal(records[0].fileName, 'renamed.litematic')
    assert.equal(
        records.filter(
            (record) =>
                record.source.kind === 'external' && record.source.path === '/tmp/5.litematic',
        ).length,
        1,
    )
})

test('recent schematics can be removed individually or cleared', () => {
    Object.defineProperty(globalThis, 'localStorage', {
        value: createStorage(),
        configurable: true,
    })
    const records = recordRecentSchematic(
        { kind: 'instance', instanceId: 'demo', relativePath: 'house.schem' },
        'house.schem',
    )
    assert.equal(removeRecentSchematic(records[0].id).length, 0)
    recordRecentSchematic({ kind: 'external', path: '/tmp/house.schem' }, 'house.schem')
    assert.deepEqual(clearRecentSchematics(), [])
    assert.deepEqual(loadRecentSchematics(), [])
})

test('recent schematics retain instance-root file sources', () => {
    Object.defineProperty(globalThis, 'localStorage', {
        value: createStorage(),
        configurable: true,
    })
    const source = {
        kind: 'instance_file' as const,
        instanceId: 'demo',
        relativePath: 'config/worldedit/schematics/house.schem',
    }
    recordRecentSchematic(source, 'house.schem')
    assert.deepEqual(loadRecentSchematics()[0].source, source)
    recordRecentSchematic(
        { kind: 'instance', instanceId: 'demo', relativePath: source.relativePath },
        'house.schem',
    )
    assert.equal(loadRecentSchematics().length, 2)
})
