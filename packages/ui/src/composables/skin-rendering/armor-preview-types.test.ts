import assert from 'node:assert/strict'
import test from 'node:test'

import {
	ARMOR_SLOTS,
	ARMOR_TRIM_MATERIALS,
	ARMOR_TRIM_PATTERNS,
	armorMaterialsForSlot,
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
