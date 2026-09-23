import type {
	ArmorMaterial,
	ArmorSlot,
	ArmorTrimMaterial,
	ArmorTrimPattern,
} from './armor-preview-types'

const assetModules = import.meta.glob<string>('../../assets/minecraft-armor/**/*.png', {
	eager: true,
	query: '?url',
	import: 'default',
})

function asset(path: string): string {
	const url = assetModules[`../../assets/minecraft-armor/${path}.png`]
	if (!url) throw new Error(`Missing Minecraft armor preview asset: ${path}`)
	return url
}

const armorTextureName: Record<ArmorMaterial, string> = {
	leather: 'leather',
	chainmail: 'chainmail',
	copper: 'copper',
	gold: 'gold',
	iron: 'iron',
	diamond: 'diamond',
	netherite: 'netherite',
	turtle: 'turtle_scute',
}

const armorItemName: Record<ArmorMaterial, string> = {
	leather: 'leather_default',
	chainmail: 'chainmail',
	copper: 'copper',
	gold: 'golden',
	iron: 'iron',
	diamond: 'diamond',
	netherite: 'netherite',
	turtle: 'turtle',
}

const trimIngredientName: Record<ArmorTrimMaterial, string> = {
	amethyst: 'amethyst_shard',
	copper: 'copper_ingot',
	diamond: 'diamond',
	emerald: 'emerald',
	gold: 'gold_ingot',
	iron: 'iron_ingot',
	lapis: 'lapis_lazuli',
	netherite: 'netherite_ingot',
	quartz: 'quartz',
	redstone: 'redstone',
	resin: 'resin_brick',
}

const darkerTrimMaterials = new Set<ArmorTrimMaterial>([
	'copper',
	'diamond',
	'gold',
	'iron',
	'netherite',
])

export type ArmorTextureLayer = 'humanoid' | 'leggings'

export function getArmorTextureUrl(material: ArmorMaterial, layer: ArmorTextureLayer): string {
	return asset(`entity/${layer}/${armorTextureName[material]}`)
}

export function getLeatherOverlayUrl(layer: ArmorTextureLayer): string {
	return asset(`entity/${layer}/leather_overlay`)
}

export function getArmorItemIcon(material: ArmorMaterial, slot: ArmorSlot): string {
	if (material === 'turtle') return asset('item/turtle_helmet')
	return asset(`item/${armorItemName[material]}_${slot}`)
}

export function getArmorSlotIcon(slot: ArmorSlot): string {
	return asset(`slot/${slot}`)
}

export function getTrimPatternIcon(pattern: ArmorTrimPattern): string {
	return asset(`item/${pattern}_armor_trim_smithing_template`)
}

export function getTrimMaterialIcon(material: ArmorTrimMaterial): string {
	return asset(`item/${trimIngredientName[material]}`)
}

export function getTrimPatternTextureUrl(
	pattern: ArmorTrimPattern,
	layer: ArmorTextureLayer,
): string {
	return asset(`trim/${layer}/${pattern}`)
}

export function getTrimPaletteUrl(material: ArmorTrimMaterial, darker: boolean): string {
	const suffix = darker && darkerTrimMaterials.has(material) ? '_darker' : ''
	return asset(`trim/palettes/${material}${suffix}`)
}

export function getTrimSourcePaletteUrl(): string {
	return asset('trim/palettes/trim_palette')
}
