<script setup lang="ts">
defineProps<{
	title: string
	compact?: boolean
	dense?: boolean
}>()
</script>

<template>
	<!-- the weird padding and negative margin are to include the potential hover underline in bounding box which affects rendering on firefox -->
	<span
		class="project-card-title truncate pb-[2px] mb-[-2px] font-semibold text-[var(--color-text-primary)] m-0 leading-none smart-clickable:underline-on-hover"
		:class="dense ? 'text-base' : compact ? 'text-lg' : 'text-xl'"
	>
		{{ title }}
	</span>
</template>
<style scoped>
@container (width < 550px) {
	.project-card-title {
		@apply text-base;
	}
}
</style>
