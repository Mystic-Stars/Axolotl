<script setup lang="ts">
import { SearchIcon } from '@modrinth/assets'
import {
	Card,
	defineMessages,
	DropdownSelect,
	EmptyState,
	StyledInput,
	TagItem,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'
import { RouterLink } from 'vue-router'

import gradientTextToolCover from '@/assets/lab/gradient-text-tool-cover.png'
import seedMapToolCover from '@/assets/lab/seed-map-tool-cover.png'
import { type LabToolDefinition,labTools } from '@/lab/registry'

type LabCategory = 'all' | LabToolDefinition['category']

const { formatMessage } = useVIntl()
const search = ref('')
const category = ref<LabCategory>('all')
const toolCoverImages: Record<string, string> = {
	'gradient-text': gradientTextToolCover,
	'seed-map': seedMapToolCover,
}

const messages = defineMessages({
	title: { id: 'app.lab.title', defaultMessage: 'Lab' },
	toolCount: { id: 'app.lab.tool-count', defaultMessage: '{count} tool' },
	toolCountPlural: { id: 'app.lab.tool-count-plural', defaultMessage: '{count} tools' },
	search: { id: 'app.lab.search', defaultMessage: 'Search tools' },
	allTools: { id: 'app.lab.category.all', defaultMessage: 'All tools' },
	creation: { id: 'app.lab.category.creation', defaultMessage: 'Creation' },
	maintenance: { id: 'app.lab.category.maintenance', defaultMessage: 'Maintenance' },
	world: { id: 'app.lab.category.world', defaultMessage: 'World' },
	noResults: { id: 'app.lab.no-results', defaultMessage: 'No tools match your search.' },
	gradientTextTitle: {
		id: 'app.lab.gradient-text.title',
		defaultMessage: 'Gradient text generator',
	},
	gradientTextDescription: {
		id: 'app.lab.gradient-text.description',
		defaultMessage: 'Create Minecraft-ready gradient text without a browser.',
	},
	seedMapTitle: { id: 'app.lab.seed-map.title', defaultMessage: 'Seed map' },
	seedMapDescription: {
		id: 'app.lab.seed-map.description',
		defaultMessage: 'Explore a Minecraft seed locally with biomes, structures, and saved markers.',
	},
})

const categoryOptions: LabCategory[] = ['all', 'creation', 'maintenance', 'world']
const visibleTools = computed(() => {
	const normalizedSearch = search.value.trim().toLocaleLowerCase()

	return labTools.filter((tool) => {
		const matchingCategory = category.value === 'all' || tool.category === category.value
		const matchingSearch =
			!normalizedSearch ||
			[toolTitle(tool.id, tool.title), toolDescription(tool.id, tool.description)]
				.join(' ')
				.toLocaleLowerCase()
				.includes(normalizedSearch)

		return matchingCategory && matchingSearch
	})
})

function toolTitle(toolId: string, fallback: string) {
	if (toolId === 'gradient-text') return formatMessage(messages.gradientTextTitle)
	if (toolId === 'seed-map') return formatMessage(messages.seedMapTitle)
	return fallback
}

function toolDescription(toolId: string, fallback: string) {
	if (toolId === 'gradient-text') return formatMessage(messages.gradientTextDescription)
	if (toolId === 'seed-map') return formatMessage(messages.seedMapDescription)
	return fallback
}

function toolOnboardingId(toolId: string) {
	return toolId === 'gradient-text' || toolId === 'seed-map' ? `lab-${toolId}-card` : undefined
}

function toolIconClasses(toolId: string) {
	if (toolId === 'seed-map') return 'bg-highlight-green text-brand'
	return 'bg-brand-highlight text-brand'
}

function categoryLabel(value: LabCategory) {
	if (value === 'all') return formatMessage(messages.allTools)
	return formatMessage(messages[value])
}
</script>

<template>
	<main class="flex w-full flex-col gap-6 p-6">
		<header class="flex min-w-0 items-start justify-between gap-4">
			<div class="min-w-0">
				<h1 class="m-0 text-2xl font-bold text-contrast">{{ formatMessage(messages.title) }}</h1>
				<p class="m-0 mt-1 text-sm text-secondary">
					{{
						formatMessage(labTools.length === 1 ? messages.toolCount : messages.toolCountPlural, {
							count: labTools.length,
						})
					}}
				</p>
			</div>
		</header>

		<div class="flex flex-wrap gap-2" aria-label="Lab tool filters">
			<StyledInput
				v-model="search"
				:icon="SearchIcon"
				:placeholder="formatMessage(messages.search)"
				clearable
				wrapper-class="min-w-[14rem] flex-1"
			/>
			<DropdownSelect
				v-model="category"
				:options="categoryOptions"
				:display-name="categoryLabel"
				name="Lab category"
				class="w-48 max-[576px]:w-full"
			/>
		</div>

		<section
			v-if="visibleTools.length"
			aria-label="Lab tools"
			data-onboarding-id="lab-tools"
			class="grid grid-cols-[repeat(auto-fit,minmax(min(18rem,100%),24rem))] gap-3"
		>
			<RouterLink
				v-for="tool in visibleTools"
				:key="tool.id"
				:data-onboarding-id="toolOnboardingId(tool.id)"
				:to="tool.route"
				class="group min-w-0 text-inherit no-underline focus-visible:outline-none"
			>
				<Card
					class="!m-0 flex h-full flex-col overflow-hidden !p-0 transition-[filter,border-color] duration-200 group-hover:brightness-[0.85] group-focus-visible:brightness-[0.85] group-focus-visible:!border-brand"
				>
					<div
						class="relative flex aspect-[2/1] w-full shrink-0 items-center justify-center overflow-hidden border-0 border-b border-solid border-divider bg-surface-2"
					>
						<TagItem class="absolute right-3 top-3 z-10">
							{{ categoryLabel(tool.category) }}
						</TagItem>
						<img
							v-if="toolCoverImages[tool.id]"
							:src="toolCoverImages[tool.id]"
							alt=""
							class="h-full w-full object-cover"
						/>
						<div
							v-else
							class="flex size-14 items-center justify-center rounded-xl"
							:class="toolIconClasses(tool.id)"
						>
							<component :is="tool.icon" class="size-7" />
						</div>
					</div>
					<div class="flex min-w-0 flex-1 flex-col p-4">
						<h2 class="m-0 line-clamp-1 text-lg font-bold leading-tight text-contrast">
							{{ toolTitle(tool.id, tool.title) }}
						</h2>
						<p class="m-0 mt-1 line-clamp-2 text-sm leading-5 text-secondary">
							{{ toolDescription(tool.id, tool.description) }}
						</p>
					</div>
				</Card>
			</RouterLink>
		</section>

		<EmptyState
			v-else
			type="no-search-result"
			:heading="formatMessage(messages.noResults)"
			aria-live="polite"
		/>
	</main>
</template>
