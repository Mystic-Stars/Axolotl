import assert from 'node:assert/strict'
import test from 'node:test'

import { projectSchematicBlockPreviewPosition } from './block-preview.ts'

test('full block previews are not vertically compressed', () => {
    const corners = Array.from({ length: 8 }, (_, index) =>
        projectSchematicBlockPreviewPosition([index & 1, (index >> 1) & 1, (index >> 2) & 1]),
    )
    const width = Math.max(...corners.map(({ x }) => x)) - Math.min(...corners.map(({ x }) => x))
    const height = Math.max(...corners.map(({ y }) => y)) - Math.min(...corners.map(({ y }) => y))

    assert.ok(height > width)
})
