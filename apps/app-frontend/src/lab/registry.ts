import { PaletteIcon, WorldIcon } from '@modrinth/assets'
import type { Component } from 'vue'

import GradientTextLab from '@/pages/LabGradientText.vue'
import SeedMapLab from '@/pages/LabSeedMap.vue'

export type LabToolDefinition = {
	id: string
	category: 'creation' | 'maintenance' | 'world'
	route: string
	icon: Component
	component: Component
	title: string
	description: string
}

export const labTools: readonly LabToolDefinition[] = [
	{
		id: 'gradient-text',
		category: 'creation',
		route: '/lab/gradient-text',
		icon: PaletteIcon,
		component: GradientTextLab,
		title: 'Gradient text generator',
		description: 'Create Minecraft-ready gradient text without a browser.',
	},
	{
		id: 'seed-map',
		category: 'world',
		route: '/lab/seed-map',
		icon: WorldIcon,
		component: SeedMapLab,
		title: 'Seed map',
		description: 'Explore a Minecraft seed locally with biomes, structures, and saved markers.',
	},
]

export function getLabTool(id: string): LabToolDefinition | undefined {
	return labTools.find((tool) => tool.id === id)
}
