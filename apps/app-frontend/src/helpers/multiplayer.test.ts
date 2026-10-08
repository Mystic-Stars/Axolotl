import assert from 'node:assert/strict'
import test from 'node:test'

import {
    type DetectedLanPort,
    type HongshiNode,
    multiplayerErrorText,
    selectedDetectedInstance,
    selectedNodePreference,
    storedMultiplayerProvider,
    validLocalPort,
} from './multiplayer.ts'

const detectedPort = (instanceId: string): DetectedLanPort => ({
    instance_id: instanceId,
    instance_name: `Instance ${instanceId}`,
    process_id: `process-${instanceId}`,
    port: 25565,
    detected_at: '2026-08-12 12:00:00',
})

const node = (name: string): HongshiNode => ({
    name,
    address: '203.0.113.1',
    latency_ms: 20,
    reachable: true,
    cached: false,
})

test('defaults the provider preference to Terracotta', () => {
    assert.equal(storedMultiplayerProvider(null), 'terracotta')
    assert.equal(storedMultiplayerProvider('invalid'), 'terracotta')
    assert.equal(storedMultiplayerProvider('hongshi'), 'hongshi')
})

test('accepts only complete ports in the Minecraft port range', () => {
    assert.equal(validLocalPort('1'), 1)
    assert.equal(validLocalPort('65535'), 65535)
    for (const value of ['', '0', '65536', '25565abc', ' 25565']) {
        assert.equal(validLocalPort(value), null)
    }
})

test('auto-selects exactly one detected instance and preserves valid choices', () => {
    assert.equal(selectedDetectedInstance('manual', [detectedPort('a')]), 'a')
    assert.equal(selectedDetectedInstance('a', [detectedPort('a'), detectedPort('b')]), 'a')
    assert.equal(
        selectedDetectedInstance('missing', [detectedPort('a'), detectedPort('b')]),
        'manual',
    )
})

test('falls back to automatic node selection when a cached preference disappears', () => {
    assert.equal(selectedNodePreference('Nanjing', [node('Nanjing')]), 'Nanjing')
    assert.equal(selectedNodePreference('Missing', [node('Nanjing')]), 'auto')
})

test('reads cancellation messages from serialized Tauri errors', () => {
    assert.equal(
        multiplayerErrorText({ field_name: 'Theseus', message: 'RedStone operation cancelled' }),
        'RedStone operation cancelled',
    )
    assert.equal(
        multiplayerErrorText('RedStone operation cancelled'),
        'RedStone operation cancelled',
    )
})
