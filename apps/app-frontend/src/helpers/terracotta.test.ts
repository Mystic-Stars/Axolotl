import assert from 'node:assert/strict'
import test from 'node:test'

import { parseTerracottaPublicNodes } from './terracotta.ts'

test('parses Terracotta public nodes from lines and commas', () => {
    assert.deepEqual(
        parseTerracottaPublicNodes(
            'wss://center.node.1tmc.top\ntcp://example.com:11010, udp://example.net:11010',
        ),
        {
            nodes: [
                'wss://center.node.1tmc.top',
                'tcp://example.com:11010',
                'udp://example.net:11010',
            ],
            invalidNode: null,
        },
    )
})

test('allows an empty Terracotta public node list', () => {
    assert.deepEqual(parseTerracottaPublicNodes('  \n'), { nodes: [], invalidNode: null })
})

test('rejects unsupported or incomplete Terracotta public nodes', () => {
    assert.equal(parseTerracottaPublicNodes('ftp://example.com').invalidNode, 'ftp://example.com')
    assert.equal(parseTerracottaPublicNodes('wss://').invalidNode, 'wss://')
})
