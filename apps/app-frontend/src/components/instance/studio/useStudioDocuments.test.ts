import assert from 'node:assert/strict'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

import { type StudioDocument, useStudioDocuments } from './useStudioDocuments.ts'

function document(path: string, kind: StudioDocument['kind'] = 'text'): StudioDocument {
    return { kind, path, name: path, content: 'saved', savedContent: 'saved', saving: false }
}

function deferred() {
    let resolve!: () => void
    const promise = new Promise<void>((done) => {
        resolve = done
    })
    return { promise, resolve }
}

for (const kind of ['text', 'nbt'] as const) {
    test(`deleting a dirty ${kind} document cannot recreate its file`, async () => {
        const directory = await mkdtemp(join(tmpdir(), 'axolotl-studio-'))
        const path = join(directory, 'document')
        try {
            await writeFile(path, 'saved')
            const studio = useStudioDocuments(async (_, content) => {
                await writeFile(path, content)
            }, assert.fail)
            await studio.open(document('document', kind))
            studio.updateActiveContent('unsaved')
            const old = studio.activeDocument.value!
            assert.equal(await studio.deletePath('document', () => rm(path)), true)
            assert.equal(studio.documents.value.length, 0)
            assert.equal(studio.activePath.value, '')
            assert.equal(await studio.saveDocument(old), false)
            await studio.saveAll()
            await assert.rejects(readFile(path), { code: 'ENOENT' })
        } finally {
            await rm(directory, { recursive: true, force: true })
        }
    })
}

test('deletion waits for an in-flight write and blocks saves until it finishes', async () => {
    const write = deferred()
    const deletion = deferred()
    const events: string[] = []
    const studio = useStudioDocuments(async () => {
        events.push('write')
        await write.promise
        events.push('written')
    }, assert.fail)
    await studio.open(document('level.dat', 'nbt'))
    studio.updateActiveContent('edited')
    const saving = studio.saveActive()
    const deleting = studio.deletePath('level.dat', async () => {
        events.push('delete')
        await deletion.promise
    })
    assert.equal(await studio.saveActive(), false)
    assert.deepEqual(events, ['write'])
    write.resolve()
    await saving
    await Promise.resolve()
    assert.deepEqual(events, ['write', 'written', 'delete'])
    assert.equal(await studio.close('level.dat'), false)
    deletion.resolve()
    await deleting
    assert.deepEqual(events, ['write', 'written', 'delete'])
    assert.equal(studio.documents.value.length, 0)
})

test('directory deletion removes active and inactive descendants but keeps adjacent paths', async () => {
    const studio = useStudioDocuments(async () => {}, assert.fail)
    for (const path of ['config/a', 'config/b', 'configuration/keep'])
        await studio.open(document(path))
    studio.documents.value[0].content = 'dirty inactive'
    studio.documents.value[1].content = 'dirty active'
    await studio.activate('config/b')
    await studio.deletePath('config', async () => {})
    assert.deepEqual(
        studio.documents.value.map((document) => document.path),
        ['configuration/keep'],
    )
    assert.equal(studio.activePath.value, 'configuration/keep')
})

test('failed deletion retains dirty content and allows a later save', async () => {
    const writes: string[] = []
    const studio = useStudioDocuments(async (_, content) => {
        writes.push(content)
    }, assert.fail)
    await studio.open(document('settings.json'))
    studio.updateActiveContent('unsaved content')
    await assert.rejects(
        studio.deletePath('settings.json', async () => {
            throw new Error('delete failed')
        }),
        /delete failed/,
    )
    assert.equal(studio.activeDocument.value?.content, 'unsaved content')
    assert.equal(studio.hasUnsavedChanges.value, true)
    assert.equal(await studio.saveActive(), true)
    assert.deepEqual(writes, ['unsaved content'])
})

test('a read started before deletion cannot reopen a deleted descendant', async () => {
    const studio = useStudioDocuments(async () => {}, assert.fail)
    const version = studio.pathVersion('config/new.json')
    await studio.deletePath('config', async () => {})
    assert.equal(await studio.open(document('config/new.json'), version), false)
    assert.equal(studio.documents.value.length, 0)
    assert.equal(await studio.open(document('config/new.json')), true)
})

test('ordinary close saves edits and concurrent closes do not remove another tab', async () => {
    const pending = deferred()
    const writes: string[] = []
    const studio = useStudioDocuments(async (document, content) => {
        writes.push(`${document.path}:${content}`)
        await pending.promise
    }, assert.fail)
    await studio.open(document('a'))
    await studio.open(document('b'))
    studio.documents.value[0].content = 'edited'
    const first = studio.close('a')
    const second = studio.close('a')
    pending.resolve()
    await Promise.all([first, second])
    assert.deepEqual(writes, ['a:edited'])
    assert.deepEqual(
        studio.documents.value.map((document) => document.path),
        ['b'],
    )
    assert.equal(studio.activePath.value, 'b')
})

test('a deletion finishing after a workspace reset does not close the new workspace document', async () => {
    const pending = deferred()
    const studio = useStudioDocuments(async () => {}, assert.fail)
    await studio.open(document('same-path'))
    const version = studio.pathVersion('same-path')
    const deleting = studio.deletePath('same-path', () => pending.promise)
    await Promise.resolve()
    studio.reset()
    await studio.open(document('same-path'))
    studio.updateActiveContent('new workspace edits')
    pending.resolve()
    await deleting
    assert.equal(studio.activeDocument.value?.content, 'new workspace edits')
    assert.equal(studio.canOpen('same-path', version), false)
    assert.equal(studio.isDeleting('same-path'), false)
})
