<template>
	<button
		class="group flex w-full hover:cursor-pointer gap-3 rounded-[var(--radius-xl)] p-3 text-left transition-all hover:brightness-110 active:scale-[0.98] border-none"
		:class="['items-center', selected ? 'bg-brand-highlight' : 'bg-surface-4']"
		@click="$emit('click')"
	>
		<div
			v-if="!noIconBox"
			class="flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl"
			:class="[
				noIconBorder ? '' : 'border border-solid',
				noIconBorder ? '' : selected ? 'border-brand' : 'border-surface-5',
			]"
		>
			<component
				:is="icon"
				class="size-8 text-secondary"
				:class="selected ? '!stroke-brand' : ''"
				stroke-width="1.5"
			/>
		</div>
		<div v-else class="flex size-12 shrink-0 items-center justify-center">
			<component
				:is="icon"
				class="size-7 text-secondary"
				:class="selected ? '!stroke-brand' : ''"
				stroke-width="1.5"
			/>
		</div>
		<div class="flex flex-1 flex-col gap-1">
			<span class="text-base font-semibold text-contrast">{{ title }}</span>
			<span class="text-left text-sm font-medium text-primary">{{ description }}</span>
			<span v-if="note" class="text-left text-xs text-tertiary">{{ note }}</span>
		</div>
		<ChevronRightIcon
			class="size-5 shrink-0 text-secondary opacity-0 transition-opacity duration-100 group-hover:opacity-100"
		/>
	</button>
</template>

<script setup lang="ts">
import { ChevronRightIcon } from '@modrinth/assets'
import type { Component } from 'vue'

defineProps<{
	icon: Component
	title: string
	description: string
	note?: string
	selected?: boolean
	noIconBox?: boolean
	noIconBorder?: boolean
}>()

defineEmits<{
	(e: 'click'): void
}>()
</script>
