import assert from 'node:assert/strict'
import test from 'node:test'

import { ServerConsoleBuffer } from './server-console-buffer.ts'

test('bounds buffered console output without rescanning prior chunks', () => {
    const buffer = new ServerConsoleBuffer(5)
    buffer.push(Uint8Array.from([1, 2]))
    buffer.push(Uint8Array.from([3, 4]))
    buffer.push(Uint8Array.from([5, 6]))

    assert.equal(buffer.size, 4)
    assert.deepEqual(
        [...buffer.values()].map((chunk) => [...chunk]),
        [
            [3, 4],
            [5, 6],
        ],
    )
})

test('keeps only the tail of a chunk larger than the capacity', () => {
    const buffer = new ServerConsoleBuffer(3)
    buffer.push(Uint8Array.from([1, 2, 3, 4, 5]))

    assert.equal(buffer.size, 3)
    assert.deepEqual(
        [...buffer.values()].map((chunk) => [...chunk]),
        [[3, 4, 5]],
    )
})
