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
		class="gap-3"
		:class="{
			'flex flex-col': layout === 'list' || layout === 'compact',
			'grid grid-cols-[repeat(auto-fill,minmax(350px,1fr))]': layout === 'grid' || layout === 'gallery',
		}"
		role="list"
	>
		<slot />
	</div>
</template>
