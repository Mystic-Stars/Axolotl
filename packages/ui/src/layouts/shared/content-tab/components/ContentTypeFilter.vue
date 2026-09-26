<script setup lang="ts">
import { useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

import type { ContentFilterOption } from '../composables'

const { formatMessage } = useVIntl()

const selected = defineModel<string[]>('selected', { required: true })

const props = defineProps<{
	options: ContentFilterOption[]
	totalCount: number
	filterCounts: Record<string, number>
}>()

const emit = defineEmits<{
	toggle: [id: string, event: MouseEvent]
}>()
</script>

<template>
	<div class="@container flex flex-col gap-2">
		<div class="flex flex-wrap items-center gap-1.5">
			<button
				class="cursor-pointer rounded-full px-3 py-1.5 text-base font-semibold leading-5 transition-all duration-100 active:scale-[0.97]"
				:class="
					selected.length === 0
						? 'bg-brand-highlight text-brand'
						: 'bg-surface-4 text-[var(--color-text-default)] hover:bg-surface-5'
				"
				:aria-pressed="selected.length === 0"
				@click="selected = []"
			>
				{{ formatMessage(commonMessages.allProjectType) }}
				<span class="ml-1 text-sm font-normal opacity-70">{{ props.totalCount }}</span>
			</button>
			<button
				v-for="option in props.options"
				:key="option.id"
				class="cursor-pointer rounded-full px-3 py-1.5 text-base font-semibold leading-5 transition-all duration-100 active:scale-[0.97]"
				:class="
					selected.includes(option.id)
						? 'bg-brand-highlight text-brand'
						: 'bg-surface-4 text-[var(--color-text-default)] hover:bg-surface-5'
				"
				:aria-pressed="selected.includes(option.id)"
				@click="(event) => emit('toggle', option.id, event)"
			>
				{{ option.label }}
				<span class="ml-1 text-sm font-normal opacity-70">{{
					props.filterCounts[option.id] ?? 0
				}}</span>
			</button>
		</div>
	</div>
</template>
