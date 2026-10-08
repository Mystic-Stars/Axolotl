import assert from 'node:assert/strict'
import test from 'node:test'

import {
    createTranslationBatches,
    prepareTranslationText,
    translateInBatches,
    type TranslationRequest,
    type TranslationResponse,
} from './translation-batching.ts'

function request(ids: string[]): TranslationRequest {
    return {
        source_language: 'auto',
        target_language: 'zh-CN',
        context: { title: '', description: '' },
        segments: ids.map((id) => ({ id, text: id, format: 'plain' })),
    }
}

test('cleans invisible translation characters and trims surrounding whitespace', () => {
    assert.equal(prepareTranslationText(' \u200BHello\uFEFF '), 'Hello')
})

test('uses Read Frog batch limits for item count and character count', () => {
    const itemBatches = createTranslationBatches(request(['a', 'b', 'c', 'd', 'e']).segments)
    assert.deepEqual(
        itemBatches.map((batch) => batch.map(({ id }) => id)),
        [['a', 'b', 'c', 'd'], ['e']],
    )

    const characterBatches = createTranslationBatches([
        { id: 'a', text: 'a'.repeat(600), format: 'plain' },
        { id: 'b', text: 'b'.repeat(401), format: 'plain' },
    ])
    assert.deepEqual(
        characterBatches.map((batch) => batch.map(({ id }) => id)),
        [['a'], ['b']],
    )
})

test('keeps Read Frog batching semantics for empty and oversized segments', () => {
    const batches = createTranslationBatches([
        { id: 'empty', text: ' \u200B\uFEFF ', format: 'plain' },
        { id: 'oversized', text: 'x'.repeat(1001), format: 'plain' },
        { id: 'next', text: ' next ', format: 'plain' },
    ])

    assert.deepEqual(
        batches.map((batch) => batch.map(({ id, text }) => ({ id, text }))),
        [[{ id: 'oversized', text: 'x'.repeat(1001) }], [{ id: 'next', text: 'next' }]],
    )
})

test('falls back to individual requests only when a batch result is incomplete', async () => {
    const calls: string[][] = []
    const execute = async (input: TranslationRequest): Promise<TranslationResponse> => {
        const ids = input.segments.map(({ id }) => id)
        calls.push(ids)
        if (ids.length > 1) return { segments: [{ id: ids[0], text: `translated-${ids[0]}` }] }
        return { segments: [{ id: ids[0], text: `translated-${ids[0]}` }] }
    }

    const result = await translateInBatches(request(['a', 'b']), undefined, execute)
    assert.deepEqual(calls, [['a', 'b'], ['a'], ['b']])
    assert.deepEqual(result.segments, [
        { id: 'a', text: 'translated-a' },
        { id: 'b', text: 'translated-b' },
    ])
})

test('does not retry provider errors as individual requests', async () => {
    let calls = 0
    await assert.rejects(
        translateInBatches(request(['a', 'b']), undefined, async () => {
            calls++
            throw new Error('AI_RATE_LIMITED')
        }),
        /AI_RATE_LIMITED/,
    )
    assert.equal(calls, 1)
})

test('translates Read Frog batches concurrently while preserving result order', async () => {
    let active = 0
    let maxActive = 0
    const execute = async (input: TranslationRequest): Promise<TranslationResponse> => {
        active++
        maxActive = Math.max(maxActive, active)
        await new Promise((resolve) => setTimeout(resolve, input.segments[0].id === 'a' ? 10 : 0))
        active--
        return {
            segments: input.segments.map(({ id }) => ({ id, text: `translated-${id}` })),
        }
    }

    const result = await translateInBatches(request(['a', 'b', 'c', 'd', 'e']), undefined, execute)
    assert.equal(maxActive, 2)
    assert.deepEqual(
        result.segments.map(({ id }) => id),
        ['a', 'b', 'c', 'd', 'e'],
    )
})

test('falls back when a batch returns duplicate segment ids', async () => {
    const calls: string[][] = []
    const execute = async (input: TranslationRequest): Promise<TranslationResponse> => {
        const ids = input.segments.map(({ id }) => id)
        calls.push(ids)
        if (ids.length > 1) {
            return {
                segments: [
                    { id: ids[0], text: 'first' },
                    { id: ids[0], text: 'duplicate' },
                ],
            }
        }
        return { segments: [{ id: ids[0], text: `translated-${ids[0]}` }] }
    }

    const result = await translateInBatches(request(['a', 'b']), undefined, execute)
    assert.deepEqual(calls, [['a', 'b'], ['a'], ['b']])
    assert.deepEqual(result.segments, [
        { id: 'a', text: 'translated-a' },
        { id: 'b', text: 'translated-b' },
    ])
})
