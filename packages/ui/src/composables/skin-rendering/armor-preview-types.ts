export const ARMOR_SLOTS = ['helmet', 'chestplate', 'leggings', 'boots'] as const
export type ArmorSlot = (typeof ARMOR_SLOTS)[number]

export const ARMOR_MATERIALS = [
	'leather',
	'chainmail',
	'copper',
	'gold',
	'iron',
	'diamond',
	'netherite',
	'turtle',
] as const
export type ArmorMaterial = (typeof ARMOR_MATERIALS)[number]

export const ARMOR_TRIM_PATTERNS = [
	'bolt',
	'coast',
	'dune',
	'eye',
	'flow',
	'host',
	'raiser',
	'rib',
	'sentry',
	'shaper',
	'silence',
	'snout',
	'spire',
	'tide',
	'vex',
	'ward',
	'wayfinder',
	'wild',
] as const
export type ArmorTrimPattern = (typeof ARMOR_TRIM_PATTERNS)[number]

export const ARMOR_TRIM_MATERIALS = [
	'amethyst',
	'copper',
	'diamond',
	'emerald',
	'gold',
	'iron',
	'lapis',
	'netherite',
	'quartz',
	'redstone',
	'resin',
] as const
export type ArmorTrimMaterial = (typeof ARMOR_TRIM_MATERIALS)[number]

export interface ArmorPiecePreview {
	material: ArmorMaterial | null
	trimPattern: ArmorTrimPattern | null
	trimMaterial: ArmorTrimMaterial
}

export type ArmorPreviewConfig = Record<ArmorSlot, ArmorPiecePreview>

export function createDefaultArmorPreviewConfig(): ArmorPreviewConfig {
	return Object.fromEntries(
		ARMOR_SLOTS.map((slot) => [
			slot,
			{
				material: null,
				trimPattern: null,
				trimMaterial: 'iron',
			},
		]),
	) as ArmorPreviewConfig
}

export function armorMaterialsForSlot(slot: ArmorSlot): readonly ArmorMaterial[] {
	return slot === 'helmet' ? ARMOR_MATERIALS : ARMOR_MATERIALS.filter((item) => item !== 'turtle')
}
