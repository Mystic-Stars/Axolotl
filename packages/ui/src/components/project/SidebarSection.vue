<script setup lang="ts">
withDefaults(
	defineProps<{
		title?: string
		/** When false, the whole section is hidden (matches previous `v-if` wrappers). */
		visible?: boolean
		/** Optional `h2` extra classes, e.g. font-semibold. */
		titleClass?: string
	}>(),
	{
		title: undefined,
		visible: true,
		titleClass: '',
	},
)
</script>

<template>
	<section v-if="visible" class="flex flex-col gap-3">
		<h2 v-if="title" class="text-lg m-0" :class="titleClass">{{ title }}</h2>
		<slot name="header-extra" />
		<slot />
	</section>
</template>
