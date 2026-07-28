export type SeedMapDimension = 'overworld' | 'nether' | 'end'

export type SeedMapBiomeCategory =
	| 'beach'
	| 'cave'
	| 'desert'
	| 'forest'
	| 'ice'
	| 'jungle'
	| 'mesa'
	| 'mountains'
	| 'mushroom'
	| 'ocean'
	| 'plains'
	| 'river'
	| 'savanna'
	| 'swamp'
	| 'taiga'
	| 'nether'
	| 'end'

export type SeedMapBiome = {
	id: number
	dimensions: readonly SeedMapDimension[]
	category: SeedMapBiomeCategory
	color: string
}

export type SeedMapBiomeGroup = {
	category: SeedMapBiomeCategory
	dimension: SeedMapDimension
	biomes: SeedMapBiome[]
}

const OVERWORLD: readonly SeedMapDimension[] = ['overworld']
const NETHER: readonly SeedMapDimension[] = ['nether']
const END: readonly SeedMapDimension[] = ['end']

/**
 * Java biome ids as used by cubiomes, grouped for the picker. Colors follow
 * the classic community biome palette.
 */
export const SEED_MAP_BIOMES: readonly SeedMapBiome[] = [
	{ id: 16, dimensions: OVERWORLD, category: 'beach', color: '#FADE55' },
	{ id: 26, dimensions: OVERWORLD, category: 'beach', color: '#FAF0C0' },
	{ id: 25, dimensions: OVERWORLD, category: 'beach', color: '#A2A284' },
	{ id: 183, dimensions: OVERWORLD, category: 'cave', color: '#0E252A' },
	{ id: 174, dimensions: OVERWORLD, category: 'cave', color: '#D9A85C' },
	{ id: 175, dimensions: OVERWORLD, category: 'cave', color: '#7BC545' },
	{ id: 187, dimensions: OVERWORLD, category: 'cave', color: '#C8C828' },
	{ id: 2, dimensions: OVERWORLD, category: 'desert', color: '#FA9418' },
	{ id: 27, dimensions: OVERWORLD, category: 'forest', color: '#307444' },
	{ id: 185, dimensions: OVERWORLD, category: 'forest', color: '#E87B9F' },
	{ id: 29, dimensions: OVERWORLD, category: 'forest', color: '#40511A' },
	{ id: 132, dimensions: OVERWORLD, category: 'forest', color: '#2D8E49' },
	{ id: 4, dimensions: OVERWORLD, category: 'forest', color: '#056621' },
	{ id: 178, dimensions: OVERWORLD, category: 'forest', color: '#3E6B4E' },
	{ id: 155, dimensions: OVERWORLD, category: 'forest', color: '#589C6C' },
	{ id: 186, dimensions: OVERWORLD, category: 'forest', color: '#8E7FA3' },
	{ id: 34, dimensions: OVERWORLD, category: 'forest', color: '#4E824E' },
	{ id: 181, dimensions: OVERWORLD, category: 'ice', color: '#C4C4C4' },
	{ id: 11, dimensions: OVERWORLD, category: 'ice', color: '#3938C9' },
	{ id: 140, dimensions: OVERWORLD, category: 'ice', color: '#B4DCE0' },
	{ id: 168, dimensions: OVERWORLD, category: 'jungle', color: '#768E14' },
	{ id: 21, dimensions: OVERWORLD, category: 'jungle', color: '#537B09' },
	{ id: 23, dimensions: OVERWORLD, category: 'jungle', color: '#6D9930' },
	{ id: 37, dimensions: OVERWORLD, category: 'mesa', color: '#D94515' },
	{ id: 165, dimensions: OVERWORLD, category: 'mesa', color: '#FF6D3D' },
	{ id: 38, dimensions: OVERWORLD, category: 'mesa', color: '#B09765' },
	{ id: 180, dimensions: OVERWORLD, category: 'mountains', color: '#C0C0C0' },
	{ id: 177, dimensions: OVERWORLD, category: 'mountains', color: '#5EB93D' },
	{ id: 179, dimensions: OVERWORLD, category: 'mountains', color: '#D6DADB' },
	{ id: 182, dimensions: OVERWORLD, category: 'mountains', color: '#A0A0A0' },
	{ id: 131, dimensions: OVERWORLD, category: 'mountains', color: '#888888' },
	{ id: 3, dimensions: OVERWORLD, category: 'mountains', color: '#606060' },
	{ id: 14, dimensions: OVERWORLD, category: 'mushroom', color: '#FF00FF' },
	{ id: 46, dimensions: OVERWORLD, category: 'ocean', color: '#2070DD' },
	{ id: 49, dimensions: OVERWORLD, category: 'ocean', color: '#185DC0' },
	{ id: 50, dimensions: OVERWORLD, category: 'ocean', color: '#2040A0' },
	{ id: 48, dimensions: OVERWORLD, category: 'ocean', color: '#0060B0' },
	{ id: 24, dimensions: OVERWORLD, category: 'ocean', color: '#000030' },
	{ id: 10, dimensions: OVERWORLD, category: 'ocean', color: '#7090D6' },
	{ id: 45, dimensions: OVERWORLD, category: 'ocean', color: '#0090D0' },
	{ id: 0, dimensions: OVERWORLD, category: 'ocean', color: '#000070' },
	{ id: 44, dimensions: OVERWORLD, category: 'ocean', color: '#0098DB' },
	{ id: 1, dimensions: OVERWORLD, category: 'plains', color: '#8DB360' },
	{ id: 12, dimensions: OVERWORLD, category: 'plains', color: '#C6C6C6' },
	{ id: 129, dimensions: OVERWORLD, category: 'plains', color: '#B5DB88' },
	{ id: 7, dimensions: OVERWORLD, category: 'river', color: '#0000FF' },
	{ id: 35, dimensions: OVERWORLD, category: 'savanna', color: '#BDB25F' },
	{ id: 36, dimensions: OVERWORLD, category: 'savanna', color: '#A79D64' },
	{ id: 163, dimensions: OVERWORLD, category: 'savanna', color: '#E5DA87' },
	{ id: 184, dimensions: OVERWORLD, category: 'swamp', color: '#67B554' },
	{ id: 6, dimensions: OVERWORLD, category: 'swamp', color: '#07F9B2' },
	{ id: 32, dimensions: OVERWORLD, category: 'taiga', color: '#596651' },
	{ id: 160, dimensions: OVERWORLD, category: 'taiga', color: '#818E79' },
	{ id: 30, dimensions: OVERWORLD, category: 'taiga', color: '#31554A' },
	{ id: 5, dimensions: OVERWORLD, category: 'taiga', color: '#0B6659' },
	{ id: 8, dimensions: NETHER, category: 'nether', color: '#572526' },
	{ id: 170, dimensions: NETHER, category: 'nether', color: '#4D3A2E' },
	{ id: 171, dimensions: NETHER, category: 'nether', color: '#981A11' },
	{ id: 172, dimensions: NETHER, category: 'nether', color: '#49907B' },
	{ id: 173, dimensions: NETHER, category: 'nether', color: '#645F63' },
	{ id: 9, dimensions: END, category: 'end', color: '#8080FF' },
	{ id: 40, dimensions: END, category: 'end', color: '#4B4BAB' },
	{ id: 41, dimensions: END, category: 'end', color: '#C9C959' },
	{ id: 42, dimensions: END, category: 'end', color: '#B5B536' },
	{ id: 43, dimensions: END, category: 'end', color: '#7070CC' },
]

/** Display names for the Java biome ids shown in the picker. */
export const SEED_MAP_BIOME_NAMES: Readonly<Record<number, string>> = {
	0: 'Ocean',
	1: 'Plains',
	2: 'Desert',
	3: 'Windswept Hills',
	4: 'Forest',
	5: 'Taiga',
	6: 'Swamp',
	7: 'River',
	8: 'Nether Wastes',
	9: 'The End',
	10: 'Frozen Ocean',
	11: 'Frozen River',
	12: 'Snowy Plains',
	14: 'Mushroom Fields',
	16: 'Beach',
	21: 'Jungle',
	23: 'Sparse Jungle',
	24: 'Deep Ocean',
	25: 'Stony Shore',
	26: 'Snowy Beach',
	27: 'Birch Forest',
	29: 'Dark Forest',
	30: 'Snowy Taiga',
	32: 'Old Growth Pine Taiga',
	34: 'Windswept Forest',
	35: 'Savanna',
	36: 'Savanna Plateau',
	37: 'Badlands',
	38: 'Wooded Badlands',
	40: 'Small End Islands',
	41: 'End Midlands',
	42: 'End Highlands',
	43: 'End Barrens',
	44: 'Warm Ocean',
	45: 'Lukewarm Ocean',
	46: 'Cold Ocean',
	48: 'Deep Lukewarm Ocean',
	49: 'Deep Cold Ocean',
	50: 'Deep Frozen Ocean',
	129: 'Sunflower Plains',
	131: 'Windswept Gravelly Hills',
	132: 'Flower Forest',
	140: 'Ice Spikes',
	155: 'Old Growth Birch Forest',
	160: 'Old Growth Spruce Taiga',
	163: 'Windswept Savanna',
	165: 'Eroded Badlands',
	168: 'Bamboo Jungle',
	170: 'Soul Sand Valley',
	171: 'Crimson Forest',
	172: 'Warped Forest',
	173: 'Basalt Deltas',
	174: 'Dripstone Caves',
	175: 'Lush Caves',
	177: 'Meadow',
	178: 'Grove',
	179: 'Snowy Slopes',
	180: 'Jagged Peaks',
	181: 'Frozen Peaks',
	182: 'Stony Peaks',
	183: 'Deep Dark',
	184: 'Mangrove Swamp',
	185: 'Cherry Grove',
	186: 'Pale Garden',
	187: 'Sulfur Caves',
}

export function seedMapBiomeGroups(): SeedMapBiomeGroup[] {
	const groups = new Map<SeedMapBiomeCategory, SeedMapBiomeGroup>()
	for (const biome of SEED_MAP_BIOMES) {
		const dimension = biome.dimensions[0] ?? 'overworld'
		const group = groups.get(biome.category) ?? {
			category: biome.category,
			dimension,
			biomes: [],
		}
		group.biomes.push(biome)
		groups.set(biome.category, group)
	}
	return [...groups.values()]
}

export function seedMapBiomeSlug(name: string): string {
	return name.toLocaleLowerCase().replaceAll(' ', '-')
}
