import assert from 'node:assert/strict'
import test from 'node:test'

import { createProjectBrowseLocation, getExternalLinkUrl } from './project-links.ts'

test('project sidebar loader links target the app browse route and loader filter', () => {
    assert.deepEqual(createProjectBrowseLocation('mod', 'loader', 'forge'), {
        path: '/browse/mod',
        query: { g: 'categories:forge' },
    })
})

test('project sidebar category links target the app browse route and category filter', () => {
    assert.deepEqual(createProjectBrowseLocation('mod', 'category', 'library-api'), {
        path: '/browse/mod',
        query: { f: 'categories:library-api' },
    })
})

test('server categories use the server browse route and server category filter', () => {
    assert.deepEqual(createProjectBrowseLocation('minecraft_java_server', 'category', 'vanilla'), {
        path: '/browse/server',
        query: { sc: 'vanilla' },
    })
})

test('bare external domains are recovered from the Tauri app origin', () => {
    assert.equal(
        getExternalLinkUrl('www.example.com/docs', 'https://tauri.localhost/www.example.com/docs'),
        'https://www.example.com/docs',
    )
})

test('CurseForge linkout links are decoded and opened externally', () => {
    const href = '/linkout?remoteUrl=https%253a%252f%252fwww.akliz.net%252fallthemods'
    assert.equal(
        getExternalLinkUrl(href, `https://tauri.localhost${href}`),
        'https://www.akliz.net/allthemods',
    )
})

test('absolute external links are passed through unchanged', () => {
    assert.equal(
        getExternalLinkUrl('https://example.com/docs', 'https://example.com/docs'),
        'https://example.com/docs',
    )
})

test('launcher-relative and local links stay in the web view', () => {
    assert.equal(getExternalLinkUrl('/browse/mod', 'https://tauri.localhost/browse/mod'), null)
    assert.equal(
        getExternalLinkUrl('https://tauri.localhost/settings', 'https://tauri.localhost/settings'),
        null,
    )
    assert.equal(getExternalLinkUrl('http://localhost:8000', 'http://localhost:8000'), null)
})
