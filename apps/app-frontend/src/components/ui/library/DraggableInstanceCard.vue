<script setup lang="ts">
import { KeyboardSensor, PointerSensor, useDraggable } from '@dnd-kit/vue'
import { useEventListener } from '@vueuse/core'
import { computed, ref } from 'vue'

import type { GameInstance } from '@/helpers/types'

const props = defineProps<{
	instance: GameInstance
	instanceGroupId: string
	disabled: boolean
}>()

const emit = defineEmits<{
	(e: 'contextmenu' | 'card-click', event: MouseEvent): void
	(e: 'start-long-press' | 'cancel-long-press'): void
}>()

const cardElement = ref<HTMLElement>()

const instanceCardSensors = [
	PointerSensor.configure({
		preventActivation: () => false,
	}),
	KeyboardSensor,
]

const dragDisabled = computed(() => props.disabled)

const { isDragging } = useDraggable({
	id: computed(() => `instance:${props.instanceGroupId}:${props.instance.id}`),
	element: cardElement,
	disabled: dragDisabled,
	sensors: instanceCardSensors,
	data: computed(() => ({
		instanceId: props.instance.id,
		fromGroup: props.instanceGroupId,
	})),
})

let wantsClick = false

useEventListener(window, 'pointerdown', () => {
	wantsClick = true
})
useEventListener(window, 'pointerup', () => {
	if (isDragging.value) {
		wantsClick = false
	}
})
useEventListener(window, 'blur', () => {
	wantsClick = false
})

function onCardClick(event: MouseEvent) {
	if (!wantsClick || isDragging.value) return
	wantsClick = false
	emit('card-click', event)
}

function onContextMenu(event: MouseEvent) {
	emit('contextmenu', event)
}

function onMouseDown() {
	if (!dragDisabled.value) {
		emit('start-long-press')
	}
}

function onMouseUp() {
	emit('cancel-long-press')
}
</script>

<template>
	<div
		ref="cardElement"
		class="group relative min-w-0 w-full"
		@click="onCardClick"
		@mousedown="onMouseDown"
		@mouseup="onMouseUp"
		@mouseleave="onMouseUp"
		@touchstart="!dragDisabled && onMouseDown()"
		@touchend="onMouseUp"
		@touchcancel="onMouseUp"
		@contextmenu.prevent.stop="onContextMenu"
	>
		<slot :is-dragging="isDragging" />
		<slot v-if="!isDragging" name="overlay" />
	</div>
</template>
