<template>
	<DropdownMenuRoot v-model:open="open" :modal="false">
		<DropdownMenuTrigger as-child>
			<slot name="trigger">
				<button ref="trigger" v-bind="$attrs" v-tooltip="tooltip">
					<slot></slot>
				</button>
			</slot>
		</DropdownMenuTrigger>

		<DropdownMenuPortal :to="portalTarget">
			<DropdownMenuContent
				ref="content"
				:side="side"
				:align="align"
				:side-offset="4"
				:class="[dropdownClass, 'menu-surface']"
				:aria-label="dropdownId || undefined"
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
function focusFirstContent(event: Event) {
	event.preventDefault()
	const root = content.value?.$el as HTMLElement | undefined
	root
		?.querySelector<HTMLElement>('button:not(:disabled), a[href], [tabindex]:not([tabindex="-1"])')
		?.focus()
}

/** The wrapper focuses the first available control, including non-menu buttons. */

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
