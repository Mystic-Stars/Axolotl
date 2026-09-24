<template>
	<DropdownMenuRoot v-model:open="open" :modal="false">
		<DropdownMenuTrigger as-child>
			<button ref="trigger" v-bind="$attrs" v-tooltip="tooltip">
				<slot></slot>
			</button>
		</DropdownMenuTrigger>

		<DropdownMenuPortal :to="portalTarget">
			<DropdownMenuContent
				:side="side"
				:align="align"
				:side-offset="4"
				:class="[dropdownClass, 'menu-surface']"
				:aria-label="dropdownId || undefined"
				@close-auto-focus.prevent
				@open-auto-focus.prevent
			>
				<slot name="menu" :hide="hide"></slot>
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
	DropdownMenuContent,
	DropdownMenuPortal,
	DropdownMenuRoot,
	DropdownMenuTrigger,
} from 'reka-ui'
import { computed, ref } from 'vue'

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

const open = ref(false)
const trigger = ref<HTMLElement>()

/**
 * The menu is not modal and must not take focus when it opens: several callers
 * trigger it on hover, where moving focus would be hostile. Escape and
 * outside-click still close it, and focus returns to the trigger on close
 * (`@close-auto-focus` is left at its default rather than prevented).
 */

/**
 * Where the menu is portalled to.
 *
 * The app and the website both mount a `#teleports` host; a component mounted
 * on its own (a test, or a future embed) may not, so this falls back to `body`
 * rather than to a selector that matches nothing -- a menu teleported to a
 * missing target renders nowhere at all, which is silent and confusing.
 */
const portalTarget = computed(() => {
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
