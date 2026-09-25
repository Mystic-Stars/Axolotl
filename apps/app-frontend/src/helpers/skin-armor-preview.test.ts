import assert from 'node:assert/strict'
import test from 'node:test'

import { loadSkinArmorPreview, saveSkinArmorPreview } from './skin-armor-preview.ts'

const originalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')

function withStorage(run: (values: Map<string, string>) => void): void {
	const values = new Map<string, string>()
	Object.defineProperty(globalThis, 'localStorage', {
		configurable: true,
		value: {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => values.set(key, value),
		},
	})
	try {
		run(values)
	} finally {
		if (originalStorage) Object.defineProperty(globalThis, 'localStorage', originalStorage)
		else delete (globalThis as { localStorage?: Storage }).localStorage
	}
}

test('armor preview selections survive storage round trips', () => {
	withStorage(() => {
		const config = loadSkinArmorPreview()
		config.helmet.material = 'diamond'
		config.helmet.trimPattern = 'sentry'
		config.helmet.trimMaterial = 'gold'
		saveSkinArmorPreview(config)

		assert.deepEqual(loadSkinArmorPreview(), config)
	})
})

test('invalid stored armor values fall back per slot and field', () => {
	withStorage((values) => {
		values.set(
			'axolotl:skin-armor-preview',
			JSON.stringify({
				helmet: { material: 'diamond', trimPattern: 'invalid', trimMaterial: 'gold' },
				boots: { material: 'turtle', trimPattern: 'sentry', trimMaterial: 'invalid' },
			}),
		)

		const config = loadSkinArmorPreview()
		assert.deepEqual(config.helmet, {
			material: 'diamond',
			trimPattern: null,
			trimMaterial: 'gold',
		})
		assert.deepEqual(config.boots, {
			material: null,
			trimPattern: 'sentry',
			trimMaterial: 'iron',
		})
		assert.equal(config.chestplate.material, null)
	})
})
