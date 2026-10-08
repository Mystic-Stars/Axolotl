import assert from 'node:assert/strict'
import test from 'node:test'

import { applySettingChanges } from './deep-link-settings.ts'
import type { AppSettings } from './settings.ts'
import { createSettingsPatchSaver, diffSettings } from './settings-patch.ts'

test('stale drafts save only edits and preserve changes from another settings region', async () => {
    const original = { theme: 'dark', developer_mode: false, feature_flags: { a: false, b: false } }
    const backend = structuredClone(original)
    const patches: Record<string, unknown>[] = []
    const save = async (patch: Record<string, unknown>) => {
        patches.push(patch)
        Object.assign(backend, patch)
    }
    const appearance = createSettingsPatchSaver(original, save)
    const developer = createSettingsPatchSaver(original, save)
    await appearance({ ...original, theme: 'light' })
    await developer({ ...original, developer_mode: true })
    assert.equal(backend.theme, 'light')
    assert.equal(backend.developer_mode, true)
    assert.deepEqual(patches, [{ theme: 'light' }, { developer_mode: true }])
    assert.deepEqual(
        diffSettings(original, { ...original, feature_flags: { a: true, b: false } }),
        { feature_flags: { a: true } },
    )
})

test('captures rapid saves and retries failed edits without advancing the baseline', async () => {
    let release!: () => void
    const pending = new Promise<void>((resolve) => {
        release = resolve
    })
    const patches: Record<string, unknown>[] = []
    const draft = { theme: 'dark', locale: 'en-US' }
    const saver = createSettingsPatchSaver(draft, async (patch) => {
        patches.push(patch)
        if (patches.length === 1) {
            await pending
            throw new Error('write failed')
        }
    })
    draft.theme = 'light'
    const first = saver(draft)
    draft.locale = 'zh-CN'
    const second = saver(draft)
    draft.theme = 'oled'
    release()
    await assert.rejects(first, /write failed/)
    await second
    assert.deepEqual(patches, [{ theme: 'light' }, { theme: 'light', locale: 'zh-CN' }])
    await saver(draft)
    assert.deepEqual(patches[2], { theme: 'oled' })
})

test('replaces structured configuration when keys are removed and supports clearing values', () => {
    assert.deepEqual(
        diffSettings(
            { home_widgets: { a: 1, b: 2 }, ui_font: 'Font' },
            { home_widgets: { b: 2 }, ui_font: null },
        ),
        { home_widgets: { b: 2 }, ui_font: null },
    )
})

test('clearing a privileged launch hook survives JSON patch serialization', () => {
    const settings = {
        hooks: { pre_launch: 'old command', wrapper: 'keep wrapper' },
    } as AppSettings
    const before = structuredClone(settings)
    assert.equal(applySettingChanges(settings, [{ key: 'hooks_pre_launch', value: '' }]), true)
    const patch = JSON.parse(JSON.stringify(diffSettings(before, settings)))
    assert.deepEqual(patch, { hooks: { pre_launch: null } })
    assert.equal(settings.hooks.wrapper, 'keep wrapper')
})
