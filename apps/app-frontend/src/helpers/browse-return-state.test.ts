import assert from 'node:assert/strict'
import test from 'node:test'

import {
    clearBrowseReturnSnapshot,
    completeBrowseReturnNavigation,
    consumeBrowseReturnSnapshot,
    hasBrowseReturnSnapshot,
    isBrowseReturnNavigation,
    isBrowseReturnSourcePath,
    prepareBrowseReturnNavigation,
    saveBrowseReturnSnapshot,
} from './browse-return-state.ts'

test('consumes a matching browse snapshot only once', () => {
    const url = '/browse/mod?m=100&o=100'
    saveBrowseReturnSnapshot({ url, scrollTop: 480, state: { currentPage: 2, hits: ['a'] } })
    assert.equal(prepareBrowseReturnNavigation(url, '/project/sodium'), true)

    assert.deepEqual(consumeBrowseReturnSnapshot(url), {
        url,
        scrollTop: 480,
        state: { currentPage: 2, hits: ['a'] },
    })
    assert.equal(consumeBrowseReturnSnapshot(url), null)
    assert.equal(isBrowseReturnNavigation(url), true)
    completeBrowseReturnNavigation(url)
    assert.equal(isBrowseReturnNavigation(url), false)
})

test('does not consume a snapshot for a different browse URL', () => {
    saveBrowseReturnSnapshot({ url: '/browse/mod?page=2', scrollTop: 480, state: {} })

    assert.equal(consumeBrowseReturnSnapshot('/browse/mod?page=3'), null)
    assert.equal(hasBrowseReturnSnapshot('/browse/mod?page=2'), true)
    clearBrowseReturnSnapshot()
})

test('consumes a matching snapshot without a route guard marker', () => {
    const url = '/browse/mod?source=modrinth'
    saveBrowseReturnSnapshot({ url, scrollTop: 480, state: {} })

    assert.deepEqual(consumeBrowseReturnSnapshot(url), { url, scrollTop: 480, state: {} })
})

test('clears snapshots for ordinary Browse navigation', () => {
    const url = '/browse/mod?page=2'
    saveBrowseReturnSnapshot({ url, scrollTop: 480, state: {} })

    assert.equal(prepareBrowseReturnNavigation(url, '/library'), false)
    assert.equal(hasBrowseReturnSnapshot(url), false)
})

test('recognizes only project, download, and instance return routes', () => {
    assert.equal(isBrowseReturnSourcePath('/project/sodium'), true)
    assert.equal(isBrowseReturnSourcePath('/project/sodium/versions'), true)
    assert.equal(isBrowseReturnSourcePath('/downloads'), true)
    assert.equal(isBrowseReturnSourcePath('/instance/example'), true)
    assert.equal(isBrowseReturnSourcePath('/library'), false)
})
