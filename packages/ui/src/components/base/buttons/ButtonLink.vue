<script setup lang="ts">
import { computed, ref, useAttrs } from 'vue'

import ButtonFrame from './ButtonFrame.vue'
import type { ButtonLinkProps } from './types'

const props = withDefaults(defineProps<ButtonLinkProps>(), {
	as: 'a',
	type: 'base',
	size: 'md',
	iconOnly: false,
	circular: false,
	disabled: false,
})

const attrs = useAttrs()
const frame = ref<InstanceType<typeof ButtonFrame> | null>(null)
const element = computed(() => frame.value?.element ?? null)
const isNativeAnchor = computed(() => props.as === 'a')
const destination = computed(() => props.href ?? attrs.to)

if (!destination.value) {
	console.warn('ButtonLink requires an href or a to attribute.')
}

function preventDisabledNavigation(event: MouseEvent) {
	if (!props.disabled) return
	event.preventDefault()
	event.stopImmediatePropagation()
}

function preventDisabledKeyboard(event: KeyboardEvent) {
	if (!props.disabled || (event.key !== 'Enter' && event.key !== ' ')) return
	event.preventDefault()
	event.stopImmediatePropagation()
}

defineExpose({ element })
</script>

<template>
	<ButtonFrame
		ref="frame"
		:as="props.as"
		:type="props.type"
		:color="props.color"
		:size="props.size"
		:interaction="props.interaction"
		:icon-only="props.iconOnly"
		:circular="props.circular"
		:href="isNativeAnchor && !props.disabled ? props.href : undefined"
		:aria-label="props.iconOnly ? props.label : undefined"
		:aria-disabled="props.disabled || undefined"
		:tabindex="props.disabled ? -1 : undefined"
		@click.capture="preventDisabledNavigation"
		@keydown.capture="preventDisabledKeyboard"
	>
		<slot />
	</ButtonFrame>
</template>
