import assert from 'node:assert/strict'
import test from 'node:test'

import type { InstanceSchematicFile } from './backend.ts'
import {
    buildInstanceSchematicRows,
    collectSchematicFolders,
    type InstanceSchematicFolderRow,
} from './instance-files.ts'

function schematicFile(relativePath: string): InstanceSchematicFile {
    const segments = relativePath.split(/[\\/]/)
    return {
        relativePath,
        fileName: segments[segments.length - 1] ?? relativePath,
        format: relativePath.toLocaleLowerCase().endsWith('.schem') ? 'schem' : 'litematic',
        size: 1,
    }
}

function fileRows(rows: ReturnType<typeof buildInstanceSchematicRows>) {
    return rows.filter((row) => row.kind === 'file')
}

function folderRows(rows: ReturnType<typeof buildInstanceSchematicRows>) {
    return rows.filter((row): row is InstanceSchematicFolderRow => row.kind === 'folder')
}

test('root files and nested folders produce collapsible folder rows', () => {
    const files = [
        schematicFile('house.litematic'),
        schematicFile('redstone/clock.litematic'),
        schematicFile('redstone/contraptions/gear.schem'),
        schematicFile('builds/castle.schem'),
    ]
    const rows = buildInstanceSchematicRows(files, new Set(collectSchematicFolders(files)), '')

    assert.deepEqual(
        rows.map((row) =>
            row.kind === 'folder'
                ? `folder:${row.name}(${row.fileCount})`
                : `file:${row.file.fileName}`,
        ),
        [
            'folder:builds(1)',
            'file:castle.schem',
            'file:house.litematic',
            'folder:redstone(2)',
            'file:clock.litematic',
            'folder:contraptions(1)',
            'file:gear.schem',
        ],
    )
})

test('collapsed folders hide nested rows but keep their file count', () => {
    const files = [
        schematicFile('redstone/clock.litematic'),
        schematicFile('redstone/contraptions/gear.schem'),
    ]
    const rows = buildInstanceSchematicRows(files, new Set(), '')

    assert.deepEqual(
        rows.map((row) =>
            row.kind === 'folder'
                ? `folder:${row.name}(${row.fileCount})`
                : `file:${row.file.fileName}`,
        ),
        ['folder:redstone(2)'],
    )
})

test('nested rows track their depth and parent folder', () => {
    const files = [schematicFile('a/b/c/tower.litematic')]
    const rows = buildInstanceSchematicRows(files, new Set(collectSchematicFolders(files)), '')

    const folders = folderRows(rows)
    assert.deepEqual(
        folders.map((row) => [row.path, row.depth]),
        [
            ['a', 0],
            ['a/b', 1],
            ['a/b/c', 2],
        ],
    )
    const [file] = fileRows(rows)
    assert.equal(file.file.fileName, 'tower.litematic')
    assert.equal(file.depth, 3)
    assert.equal(file.parentPath, 'a/b/c')
})

test('search flattens matching files and filters by relative path', () => {
    const files = [
        schematicFile('house.litematic'),
        schematicFile('redstone/clock.litematic'),
        schematicFile('builds/house.schem'),
    ]
    const rows = buildInstanceSchematicRows(files, new Set(), 'house')

    assert.deepEqual(
        rows.map((row) => row.kind === 'file' && row.file.relativePath),
        ['house.litematic', 'builds/house.schem'],
    )
    assert.equal(folderRows(rows).length, 0)
})

test('windows-style backslash paths are normalized into folders', () => {
    const files = [schematicFile('redstone\\clock.litematic')]
    const folders = collectSchematicFolders(files)

    assert.deepEqual(folders, ['redstone'])
    const rows = buildInstanceSchematicRows(files, new Set(folders), '')
    assert.deepEqual(
        folderRows(rows).map((row) => [row.path, row.name, row.depth]),
        [['redstone', 'redstone', 0]],
    )
    assert.equal(fileRows(rows)[0].file.fileName, 'clock.litematic')
})

test('folder paths are sorted and deduplicated', () => {
    const files = [
        schematicFile('b/x.litematic'),
        schematicFile('a/y.schem'),
        schematicFile('a/b/z.litematic'),
        schematicFile('b/x.litematic'),
    ]

    assert.deepEqual(collectSchematicFolders(files), ['a', 'a/b', 'b'])
})
