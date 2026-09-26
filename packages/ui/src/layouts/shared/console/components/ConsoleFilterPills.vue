<template>
	<div class="flex flex-wrap items-center gap-1.5">
		<button
			v-for="option in filterOptions"
			:key="option.id"
			class="cursor-pointer rounded-full px-3 py-1.5 text-base font-semibold leading-5 transition-all duration-100 active:scale-[0.97]"
			:class="
				modelValue.has(option.id)
					? 'bg-brand-highlight text-brand'
					: 'bg-surface-4 text-[var(--color-text-default)] hover:bg-surface-5'
			"
			:aria-pressed="modelValue.has(option.id)"
			@click="handleToggle(option.id)"
		>
			{{ option.label }}
		</button>
	</div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import { defineMessages, useVIntl } from '#ui/composables/i18n'

import type { LogLevel } from '../types'

type FilterValue = LogLevel

const modelValue = defineModel<Set<FilterValue>>({ required: true })

const emit = defineEmits<{
	toggle: [value: FilterValue]
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	error: { id: 'console.filter.error', defaultMessage: 'Error' },
	warn: { id: 'console.filter.warn', defaultMessage: 'Warn' },
	info: { id: 'console.filter.info', defaultMessage: 'Info' },
})

const FILTER_OPTIONS = [
	{ id: 'error' as const, message: messages.error },
	{ id: 'warn' as const, message: messages.warn },
	{ id: 'info' as const, message: messages.info },
]

const filterOptions = computed(() =>
	FILTER_OPTIONS.map((option) => ({ id: option.id, label: formatMessage(option.message) })),
)

function handleToggle(id: FilterValue) {
	emit('toggle', id)
}
</script>
