import assert from 'node:assert/strict'
import test from 'node:test'

import type { ContentItem } from '@modrinth/ui'

import { applyContentItemUpdates } from './content-item-state.ts'

function contentItem(path: string): ContentItem {
    const fileName = path.split('/').pop() ?? path
    return {
        id: 'content',
        file_name: fileName,
        file_path: path,
        size: 1,
        enabled: true,
        project_type: 'mod',
        project: {
            id: 'local:content',
            slug: 'content',
            title: 'Content',
            icon_url: 'C:/icons/content.png',
        },
        version: {
            id: 'local:content',
            version_number: '1.0.0',
            file_name: fileName,
        },
        update: null,
        provider_refs: [],
    } as ContentItem
}

test('toggle updates survive recomputing an icon-bearing display clone', () => {
    const source = contentItem('mods/content.jar')
    const rendered = {
        ...source,
        project: { ...source.project!, icon_url: 'asset://localhost/icons/content.png' },
    }

    applyContentItemUpdates([source], rendered, source.file_name, source.file_path, {
        file_name: 'content.jar.disabled',
        file_path: 'mods/content.jar.disabled',
        enabled: false,
    })

    assert.equal(source.enabled, false)
    assert.equal(source.file_name, 'content.jar.disabled')
    assert.equal(source.file_path, 'mods/content.jar.disabled')
    assert.equal(rendered.enabled, false)
    assert.equal(rendered.file_path, 'mods/content.jar.disabled')
})
