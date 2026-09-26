<script setup lang="ts">
defineProps<{
	name: string
	version?: string | null
	loader?: string | null
}>()

defineEmits<{
	select: []
}>()
</script>

<template>
	<button
		class="flex w-full items-center gap-3 rounded-xl border border-surface-4 bg-surface-2 p-3 text-left transition-all duration-200 hover:cursor-pointer hover:border-brand hover:bg-brand-highlight active:scale-[0.98]"
		@click="$emit('select')"
	>
		<slot name="prepend" />
		<div class="flex min-w-0 flex-1 flex-col">
			<span class="truncate text-sm font-semibold text-[var(--color-text-primary)]">{{
				name
			}}</span>
			<span v-if="version || loader" class="truncate text-xs text-[var(--color-text-tertiary)]">
				{{ version ?? '—' }}<template v-if="loader"> · {{ loader }}</template>
			</span>
		</div>
		<slot name="append" />
	</button>
</template>
