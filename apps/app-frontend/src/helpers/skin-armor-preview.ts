import {
	ARMOR_SLOTS,
	ARMOR_TRIM_MATERIALS,
	ARMOR_TRIM_PATTERNS,
	armorMaterialsForSlot,
	createDefaultArmorPreviewConfig,
	type ArmorPreviewConfig,
} from '@modrinth/ui'

const STORAGE_KEY = 'axolotl:skin-armor-preview'

function includes<T extends string>(values: readonly T[], value: unknown): value is T {
	return typeof value === 'string' && values.some((item) => item === value)
}

export function loadSkinArmorPreview(): ArmorPreviewConfig {
	const config = createDefaultArmorPreviewConfig()
	try {
		const saved = JSON.parse(globalThis.localStorage?.getItem(STORAGE_KEY) ?? 'null')
		if (!saved || typeof saved !== 'object' || Array.isArray(saved)) return config

		for (const slot of ARMOR_SLOTS) {
			const piece = saved[slot]
			if (!piece || typeof piece !== 'object' || Array.isArray(piece)) continue
			if (piece.material === null || includes(armorMaterialsForSlot(slot), piece.material)) {
				config[slot].material = piece.material
			}
			if (piece.trimPattern === null || includes(ARMOR_TRIM_PATTERNS, piece.trimPattern)) {
				config[slot].trimPattern = piece.trimPattern
			}
			if (includes(ARMOR_TRIM_MATERIALS, piece.trimMaterial)) {
				config[slot].trimMaterial = piece.trimMaterial
			}
		}
	} catch {
		// Invalid or unavailable local storage falls back to a fresh preview.
	}
	return config
}

export function saveSkinArmorPreview(config: ArmorPreviewConfig): void {
	try {
		globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(config))
	} catch {
		// The preview remains usable when local storage is unavailable.
	}
}
