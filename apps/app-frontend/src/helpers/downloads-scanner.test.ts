import assert from 'node:assert/strict'
import test from 'node:test'

import {
    createDownloadsScanLoop,
    createDownloadsScannerPresentationState,
    getMissingContentScannerSettings,
    reduceDownloadsScannerPresentation,
    setMissingContentScannerSettings,
} from './downloads-scanner.ts'

function memoryStorage(initial?: string) {
    let value = initial ?? null
    return {
        getItem: () => value,
        setItem: (_key: string, next: string) => {
            value = next
        },
    }
}

function deferred<T>() {
    let resolve!: (value: T) => void
    const promise = new Promise<T>((complete) => {
        resolve = complete
    })
    return { promise, resolve }
}

test('close and reopen ignores stale scan results', async () => {
    const first = deferred<string>()
    const second = deferred<string>()
    const results: string[] = []
    let scanCount = 0
    const loop = createDownloadsScanLoop({
        scan: () => (scanCount++ === 0 ? first.promise : second.promise),
        onResult: (result) => results.push(result),
        schedule: () => undefined,
        cancelSchedule: () => undefined,
    })

    loop.start()
    const staleRun = loop.runNow()
    loop.stop()
    loop.start()
    first.resolve('stale')
    await staleRun
    assert.deepEqual(results, [])

    const currentRun = loop.runNow()
    second.resolve('current')
    await currentRun
    assert.deepEqual(results, ['current'])
    loop.stop()
})

test('one scan stays in flight at a time', async () => {
    const pending = deferred<string>()
    let calls = 0
    const loop = createDownloadsScanLoop({
        scan: () => {
            calls += 1
            return pending.promise
        },
        onResult: () => undefined,
        schedule: () => undefined,
        cancelSchedule: () => undefined,
    })

    loop.start()
    const firstRun = loop.runNow()
    await loop.runNow()
    assert.equal(calls, 1)
    pending.resolve('done')
    await firstRun
    loop.stop()
})

test('empty scan activity keeps monitoring presentation stable', async () => {
    let state = createDownloadsScannerPresentationState()
    state = reduceDownloadsScannerPresentation(state, {
        type: 'scan_result',
        downloadDirectory: 'C:\\Downloads',
        importedItemIds: [],
        rejectedItemIds: [],
        pendingCandidates: 0,
        hasErrors: false,
        items: [],
    })
    const scan = deferred<string>()
    const observedPhases: string[] = []
    const loop = createDownloadsScanLoop({
        scan: () => scan.promise,
        onResult: () => {
            state = reduceDownloadsScannerPresentation(state, {
                type: 'scan_result',
                downloadDirectory: 'C:\\Downloads',
                importedItemIds: [],
                rejectedItemIds: [],
                pendingCandidates: 0,
                hasErrors: false,
                items: [],
            })
        },
        onScanningChange: () => observedPhases.push(state.phase),
        schedule: () => undefined,
        cancelSchedule: () => undefined,
    })

    loop.start()
    const running = loop.runNow()
    assert.equal(state.phase, 'monitoring')
    scan.resolve('done')
    await running
    assert.equal(state.phase, 'monitoring')
    assert.ok(observedPhases.every((phase) => phase === 'monitoring'))
    loop.stop()
})

test('empty interval scan keeps an unchanged same-name rejection visible', () => {
    let state = createDownloadsScannerPresentationState()
    state = reduceDownloadsScannerPresentation(state, {
        type: 'scan_result',
        downloadDirectory: 'C:\\Downloads',
        importedItemIds: [],
        rejectedItemIds: ['mods/example.jar'],
        pendingCandidates: 0,
        hasErrors: false,
        items: [],
    })
    assert.equal(state.phase, 'rejected')

    state = reduceDownloadsScannerPresentation(state, {
        type: 'scan_result',
        downloadDirectory: 'C:\\Downloads',
        importedItemIds: [],
        rejectedItemIds: ['mods/example.jar'],
        pendingCandidates: 0,
        hasErrors: false,
        items: [],
    })
    assert.equal(state.phase, 'rejected')
    assert.deepEqual(state.rejectedItemIds, ['mods/example.jar'])
})

test('candidate progresses from stability wait through verification and import', () => {
    let state = createDownloadsScannerPresentationState()
    state = reduceDownloadsScannerPresentation(state, {
        type: 'scan_result',
        downloadDirectory: 'C:\\Downloads',
        importedItemIds: [],
        rejectedItemIds: [],
        pendingCandidates: 1,
        hasErrors: false,
        items: [],
    })
    assert.equal(state.phase, 'waiting_for_stability')
    assert.deepEqual(state.rejectedItemIds, [])

    state = reduceDownloadsScannerPresentation(state, {
        type: 'items_updated',
        items: [{ id: 'mods/example.jar', status: 'verifying' }],
    })
    assert.equal(state.phase, 'verifying')
    assert.deepEqual(state.verifyingItemIds, ['mods/example.jar'])
    state = reduceDownloadsScannerPresentation(state, {
        type: 'items_updated',
        items: [{ id: 'mods/example.jar', status: 'writing' }],
    })
    assert.equal(state.phase, 'importing')

    state = reduceDownloadsScannerPresentation(state, {
        type: 'scan_result',
        downloadDirectory: 'C:\\Downloads',
        importedItemIds: ['mods/example.jar'],
        rejectedItemIds: [],
        pendingCandidates: 0,
        hasErrors: false,
        items: [],
    })
    assert.equal(state.phase, 'imported')
    assert.deepEqual(state.rejectedItemIds, [])
    assert.deepEqual(state.verifyingItemIds, [])
    assert.equal(state.importedCount, 1)
})

test('presentation keeps multiple concurrent candidate verifications visible', () => {
    let state = createDownloadsScannerPresentationState()
    state = reduceDownloadsScannerPresentation(state, {
        type: 'items_updated',
        items: [
            { id: 'mods/one.jar', status: 'verifying' },
            { id: 'mods/two.jar', status: 'verifying' },
        ],
    })

    assert.equal(state.phase, 'verifying')
    assert.deepEqual(state.verifyingItemIds, ['mods/one.jar', 'mods/two.jar'])
})

test('rejected candidate disappearance returns presentation to monitoring', () => {
    let state = createDownloadsScannerPresentationState()
    state = reduceDownloadsScannerPresentation(state, {
        type: 'scan_result',
        downloadDirectory: 'C:\\Downloads',
        importedItemIds: [],
        rejectedItemIds: ['mods/example.jar'],
        pendingCandidates: 0,
        hasErrors: false,
        items: [],
    })
    assert.equal(state.phase, 'rejected')

    state = reduceDownloadsScannerPresentation(state, {
        type: 'scan_result',
        downloadDirectory: 'C:\\Downloads',
        importedItemIds: [],
        rejectedItemIds: [],
        pendingCandidates: 0,
        hasErrors: false,
        items: [],
    })
    assert.equal(state.phase, 'monitoring')
    assert.deepEqual(state.rejectedItemIds, [])
})

test('presentation reset drops previous modal session result', () => {
    let state = createDownloadsScannerPresentationState()
    state = reduceDownloadsScannerPresentation(state, {
        type: 'scan_result',
        downloadDirectory: 'C:\\Downloads',
        importedItemIds: [],
        rejectedItemIds: ['mods/example.jar'],
        pendingCandidates: 0,
        hasErrors: false,
        items: [],
    })

    state = reduceDownloadsScannerPresentation(state, { type: 'reset' })
    assert.equal(state.phase, 'idle')
    assert.equal(state.downloadDirectory, null)
    assert.deepEqual(state.rejectedItemIds, [])
})

test('missing-content scanner settings default enabled and persist a custom folder', () => {
    const storage = memoryStorage()
    assert.deepEqual(getMissingContentScannerSettings(storage), {
        enabled: true,
        directory: null,
    })

    setMissingContentScannerSettings({ enabled: false, directory: 'D:\\Modpack Imports' }, storage)
    assert.deepEqual(getMissingContentScannerSettings(storage), {
        enabled: false,
        directory: 'D:\\Modpack Imports',
    })
})

test('invalid scanner settings fall back safely', () => {
    const storage = memoryStorage('{not-json')
    assert.deepEqual(getMissingContentScannerSettings(storage), {
        enabled: true,
        directory: null,
    })
})
