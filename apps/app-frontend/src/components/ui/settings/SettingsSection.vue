<script setup lang="ts">
import { Card } from '@modrinth/ui'

withDefaults(
	defineProps<{
		title?: string
		description?: string
	}>(),
	{
		title: undefined,
		description: undefined,
	},
)
</script>

<template>
	<section class="flex min-w-0 flex-col gap-3">
		<header
			v-if="title || description || $slots.header"
			class="settings-section-header flex items-start justify-between gap-4"
		>
			<div class="min-w-0">
				<h2 v-if="title" class="m-0 text-base font-semibold text-contrast">{{ title }}</h2>
				<p v-if="description" class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ description }}
				</p>
				<slot name="header" />
			</div>
			<slot name="extra" />
		</header>

		<Card class="settings-section-card">
			<slot />
		</Card>
	</section>
</template>

<style scoped>
.settings-section-card {
	margin: 0;
	padding: 0;
	background: var(--surface-2);
	border-color: var(--surface-4);
	border-radius: var(--radius-md);
}

@media (max-width: 700px) {
	.settings-section-header {
		flex-direction: column;
		gap: var(--gap-sm);
	}
}
</style>
