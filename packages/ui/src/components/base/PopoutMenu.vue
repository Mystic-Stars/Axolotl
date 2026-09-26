<template>
	<DropdownMenuRoot v-model:open="open" :modal="false">
		<DropdownMenuTrigger
			as-child
			@keydown="noteTriggerKeydown"
			@pointerdown="noteTriggerPointerdown"
		>
			<slot name="trigger">
				<button ref="trigger" v-bind="$attrs" v-tooltip="tooltip">
					<slot></slot>
				</button>
			</slot>
		</DropdownMenuTrigger>

		<DropdownMenuPortal :to="portalTarget">
			<DropdownMenuContent
				:id="dropdownId || undefined"
				ref="content"
				:side="side"
				:align="align"
				:side-offset="4"
				:class="[dropdownClass, 'menu-surface']"
				@open-auto-focus="focusFirstContent"
			>
				<slot name="menu" :hide="hide"></slot>
				<DropdownMenuArrow class="menu-arrow" :width="14" :height="7" />
			</DropdownMenuContent>
		</DropdownMenuPortal>
	</DropdownMenuRoot>
</template>

<script setup lang="ts">
// Imported here, not only by the tooltip directive: `.menu-surface` styles this
// component's content element, so relying on the directive's import would leave
// the menu unstyled whenever no tooltip had rendered first.
import '../../styles/overlays.css'

import {
	DropdownMenuArrow,
	DropdownMenuContent,
	DropdownMenuPortal,
	DropdownMenuRoot,
	DropdownMenuTrigger,
} from 'reka-ui'
import { type ComponentPublicInstance, computed, ref } from 'vue'

const props = withDefaults(
	defineProps<{
		dropdownId?: string
		dropdownClass?: string
		tooltip?: string
		placement?: string
		container?: string | HTMLElement | boolean
	}>(),
	{
		dropdownId: undefined,
		dropdownClass: undefined,
		tooltip: undefined,
		placement: 'bottom-end',
		container: undefined,
	},
)

defineOptions({
	inheritAttrs: false,
})

const open = defineModel<boolean>('open', { default: false })
const trigger = ref<HTMLElement>()
const content = ref<ComponentPublicInstance>()

// A pointer open must not move focus: clicking a menu would otherwise pull the
// caret out of whatever the user was editing and drop a focus ring on the first
// item. Only a keyboard open needs focus moved into the menu, which is what
// makes the items reachable with the arrow keys.
let openedFromKeyboard = false

function noteTriggerKeydown(event: KeyboardEvent) {
	openedFromKeyboard = ['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)
}

function noteTriggerPointerdown() {
	openedFromKeyboard = false
}

function focusFirstContent(event: Event) {
	if (!openedFromKeyboard) {
		// Leave focus where the user left it; reka would otherwise focus the
		// content itself on a pointer open.
		event.preventDefault()
		return
	}
	event.preventDefault()
	const root = content.value?.$el as HTMLElement | undefined
	root
		?.querySelector<HTMLElement>('button:not(:disabled), a[href], [tabindex]:not([tabindex="-1"])')
		?.focus()
}

/**
 * Where the menu is portalled to.
 *
 * The app and the website both mount a `#teleports` host; a component mounted
 * on its own (a test, or a future embed) may not, so this falls back to `body`
 * rather than to a selector that matches nothing -- a menu teleported to a
 * missing target renders nowhere at all, which is silent and confusing.
 */
const portalTarget = computed(() => {
	if (typeof document === 'undefined') return 'body'
	const container = props.container
	if (container instanceof HTMLElement) return container
	if (typeof container === 'string' && container !== 'body') {
		return document.querySelector(container) ? container : 'body'
	}
	return document.getElementById('teleports') ? '#teleports' : 'body'
})

/** `bottom-end` and friends are floating-ui placement names. */
const side = computed(() => {
	const [side] = props.placement.split('-')
	return side as 'top' | 'right' | 'bottom' | 'left'
})

const align = computed(() => {
	const [, align] = props.placement.split('-')
	if (align === 'start') return 'start'
	if (align === 'end') return 'end'
	return 'center'
})

function show() {
	open.value = true
}

function hide() {
	open.value = false
	trigger.value?.focus()
}

defineExpose({ show, hide })
</script>
