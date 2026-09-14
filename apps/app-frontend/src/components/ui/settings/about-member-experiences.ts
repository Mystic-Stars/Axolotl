import type { Component } from 'vue'

import AboutMergeGame from '../AboutMergeGame.vue'

export type AboutMemberExperience = {
	component: Component
	longPressDuration: number
}

const memberExperiences: Record<string, AboutMemberExperience> = {
	'axolotl-merge': {
		component: AboutMergeGame,
		longPressDuration: 800,
	},
}

export function getAboutMemberExperience(experience: unknown): AboutMemberExperience | undefined {
	return typeof experience === 'string' ? memberExperiences[experience] : undefined
}
