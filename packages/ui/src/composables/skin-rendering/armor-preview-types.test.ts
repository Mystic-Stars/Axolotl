import assert from 'node:assert/strict'
import test from 'node:test'

import { ref } from 'vue'

import {
    ARMOR_SLOTS,
    ARMOR_TRIM_MATERIALS,
    ARMOR_TRIM_PATTERNS,
    armorMaterialsForSlot,
    cloneArmorPreviewConfig,
    createDefaultArmorPreviewConfig,
} from './armor-preview-types.ts'

test('armor preview exposes every supported slot, trim pattern, and trim material', () => {
    assert.deepEqual(ARMOR_SLOTS, ['helmet', 'chestplate', 'leggings', 'boots'])
    assert.equal(ARMOR_TRIM_PATTERNS.length, 18)
    assert.equal(ARMOR_TRIM_MATERIALS.length, 11)
})

test('armor slots start unequipped and keep independent selections', () => {
    const config = createDefaultArmorPreviewConfig()

    for (const slot of ARMOR_SLOTS) {
        assert.equal(config[slot].material, null)
        assert.equal(config[slot].trimPattern, null)
        assert.equal(config[slot].trimMaterial, 'iron')
    }

    config.helmet.material = 'diamond'
    assert.equal(config.chestplate.material, null)
})

test('turtle armor is only available for the helmet slot', () => {
    assert.ok(armorMaterialsForSlot('helmet').includes('turtle'))
    assert.ok(!armorMaterialsForSlot('chestplate').includes('turtle'))
    assert.ok(!armorMaterialsForSlot('leggings').includes('turtle'))
    assert.ok(!armorMaterialsForSlot('boots').includes('turtle'))
})

test('clones a config that lives behind a reactive ref, deeply', () => {
    const config = createDefaultArmorPreviewConfig()
    config.helmet.material = 'iron'
    config.helmet.trimPattern = 'bolt'

    // The value a caller holds has been through `ref()`, which `structuredClone`
    // refuses to copy at all.
    const clone = cloneArmorPreviewConfig(ref(config).value)

    assert.deepEqual(clone, config)
    assert.notEqual(clone, config)
    assert.notEqual(clone.helmet, config.helmet)

    clone.helmet.material = null
    assert.equal(config.helmet.material, 'iron')
})
