import assert from 'node:assert/strict'
import test from 'node:test'

import { placeContextMenu } from './context-menu-placement.ts'

const boundary = { left: 100, top: 50, right: 700, bottom: 550 }

test('places a menu down and right when it fits', () => {
    assert.deepEqual(
        placeContextMenu({
            clientX: 300,
            clientY: 200,
            menuWidth: 160,
            menuHeight: 120,
            boundary,
            safeGap: 10,
        }),
        { left: 310, top: 210 },
    )
})

test('flips left and up near the content edges', () => {
    assert.deepEqual(
        placeContextMenu({
            clientX: 680,
            clientY: 530,
            menuWidth: 160,
            menuHeight: 120,
            boundary,
            safeGap: 10,
        }),
        { left: 510, top: 400 },
    )
})

test('keeps the menu inside a narrowed content pane', () => {
    const result = placeContextMenu({
        clientX: 680,
        clientY: 200,
        menuWidth: 240,
        menuHeight: 120,
        boundary: { ...boundary, right: 470 },
        safeGap: 10,
    })
    assert.equal(result.left + 240, 460)
})

test('clamps oversized menus to the available boundary', () => {
    assert.deepEqual(
        placeContextMenu({
            clientX: 300,
            clientY: 200,
            menuWidth: 800,
            menuHeight: 700,
            boundary,
            safeGap: 10,
        }),
        { left: 110, top: 60 },
    )
})
