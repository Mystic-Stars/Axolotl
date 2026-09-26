<template>
	<span class="inline-flex flex-wrap items-center gap-1">
		<kbd
			v-for="(part, index) in parts"
			:key="`${index}-${part}`"
			aria-hidden="true"
			class="rounded-md border border-solid border-surface-4 bg-surface-3 px-2 py-0.5 font-mono text-sm text-[var(--color-text-primary)]"
		>
			{{ part }}
		</kbd>
		<span class="sr-only">{{ text }}</span>
	</span>
</template>

<script setup lang="ts">
import { useBindingParts, useBindingText } from '../../composables/use-keybinding-labels'
import type { KeyBinding } from '../../utils/keybinding'

const props = defineProps<{ binding: KeyBinding }>()

const parts = useBindingParts(() => props.binding)
const text = useBindingText(() => props.binding)
</script>
