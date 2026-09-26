<template>
	<button
		v-if="action"
		:class="[baseClass, 'transition-transform active:scale-[0.95] cursor-pointer hover:underline']"
		@click="action"
	>
		<slot />
	</button>
	<component :is="as" v-else :class="baseClass">
		<slot />
	</component>
</template>
<script setup lang="ts">
withDefaults(
	defineProps<{
		action?: (event: MouseEvent) => void
		/**
		 * The element used when there is no `action`. `span` exists for tags
		 * placed inside another interactive element, where a `div` would be
		 * invalid content and could trap a nested control.
		 */
		as?: 'div' | 'span'
	}>(),
	{
		action: undefined,
		as: 'div',
	},
)

const baseClass =
	'bg-[--_bg-color,var(--color-button-bg)] text-nowrap border-[--_bg-color,var(--surface-5)] border-[1px] border-solid px-2 py-1 leading-none rounded-full font-normal text-sm inline-flex items-center gap-1 text-[--_color,var(--color-text-tertiary)] [&>svg]:shrink-0 [&>svg]:h-4 [&>svg]:w-4'
</script>
