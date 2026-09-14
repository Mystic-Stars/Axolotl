<script setup lang="ts">
withDefaults(
	defineProps<{
		layout?: 'list' | 'compact' | 'grid' | 'gallery'
	}>(),
	{
		layout: 'list',
	},
)
</script>

<template>
	<div
		v-if="layout === 'compact'"
		class="flex w-full items-center gap-3 rounded-xl border border-solid border-divider bg-bg p-3"
		aria-hidden="true"
	>
		<div class="size-12 shrink-0 animate-pulse rounded-full bg-surface-3"></div>
		<div class="flex min-w-0 flex-1 flex-col gap-2">
			<div class="h-4 w-2/5 animate-pulse rounded bg-surface-3"></div>
			<div class="h-3 w-3/5 animate-pulse rounded bg-surface-2"></div>
		</div>
		<div class="h-3 w-14 shrink-0 animate-pulse rounded bg-surface-2"></div>
	</div>

	<div
		v-else-if="layout === 'grid' || layout === 'gallery'"
		class="flex w-full flex-col overflow-hidden rounded-xl border border-solid border-divider bg-bg"
		aria-hidden="true"
	>
		<div class="aspect-[2/1] w-full animate-pulse bg-surface-2"></div>
		<div class="flex gap-3 p-4">
			<div class="size-14 shrink-0 animate-pulse rounded-2xl bg-surface-3"></div>
			<div class="flex min-w-0 flex-1 flex-col gap-2">
				<div class="h-4 w-3/5 animate-pulse rounded bg-surface-3"></div>
				<div class="h-3 w-full animate-pulse rounded bg-surface-2"></div>
				<div class="h-3 w-4/5 animate-pulse rounded bg-surface-2"></div>
				<div class="mt-1 flex gap-2">
					<div class="h-5 w-14 animate-pulse rounded-full bg-surface-2"></div>
					<div class="h-5 w-16 animate-pulse rounded-full bg-surface-2"></div>
				</div>
			</div>
		</div>
	</div>

	<!-- list (default): horizontal project row, not a full-page banner card -->
	<div
		v-else
		class="flex w-full items-start gap-3 rounded-xl border border-solid border-divider bg-bg p-4"
		aria-hidden="true"
	>
		<div class="size-[100px] shrink-0 animate-pulse rounded-2xl bg-surface-3"></div>
		<div class="flex min-w-0 flex-1 flex-col gap-2">
			<div class="flex items-center gap-2">
				<div class="h-4 w-1/3 animate-pulse rounded bg-surface-3"></div>
				<div class="h-5 w-12 animate-pulse rounded-full bg-surface-2"></div>
			</div>
			<div class="h-3 w-full animate-pulse rounded bg-surface-2"></div>
			<div class="h-3 w-2/3 animate-pulse rounded bg-surface-2"></div>
			<div class="mt-2 flex flex-wrap gap-2">
				<div class="h-5 w-14 animate-pulse rounded-full bg-surface-2"></div>
				<div class="h-5 w-16 animate-pulse rounded-full bg-surface-2"></div>
				<div class="h-5 w-12 animate-pulse rounded-full bg-surface-2"></div>
			</div>
		</div>
		<div class="hidden shrink-0 flex-col items-end gap-2 sm:flex">
			<div class="h-4 w-16 animate-pulse rounded bg-surface-2"></div>
			<div class="h-3 w-12 animate-pulse rounded bg-surface-2"></div>
		</div>
	</div>
</template>
