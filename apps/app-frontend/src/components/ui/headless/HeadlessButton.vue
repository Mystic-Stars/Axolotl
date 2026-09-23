<template>
	<button
		:type="type"
		:disabled="disabled"
		:class="[
			color === 'brand' ? headlessTokenClasses.buttonBrand : headlessTokenClasses.buttonStandard,
			$attrs.class as string | undefined,
		]"
		v-bind="$attrs"
	>
		<slot />
	</button>
</template>

<script setup lang="ts">
import { headlessTokenClasses } from './token-classes'

withDefaults(
	defineProps<{
		color?: 'standard' | 'brand'
		type?: 'button' | 'submit' | 'reset'
		disabled?: boolean
	}>(),
	{
		color: 'standard',
		type: 'button',
		disabled: false,
	},
)

defineOptions({ inheritAttrs: false })
</script>
