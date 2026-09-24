import { autoUpdate, computePosition, flip, offset, type Placement, shift } from '@floating-ui/vue'
import type { ObjectDirective } from 'vue'

/**
 * The project's tooltip directive, replacing `floating-vue`'s.
 *
 * It deliberately keeps that directive's call signature: several hundred call
 * sites use `v-tooltip`, and the shapes below are the ones they actually rely
 * on. The value is a string, an options object, or something falsy — and falsy
 * means "no tooltip", which callers use to suppress one conditionally,
 * including the shared `truncatedTooltip()` helper. That helper is why the
 * suppression contract matters rather than being a nicety.
 *
 * `html` writes raw markup for the two call sites that pass an
 * already-sanitised fragment; everything else is written as text, so untrusted
 * strings can never become markup.
 */

type TooltipOptions = {
	content?: string | null
	html?: boolean
	placement?: Placement
	popperClass?: string
	/** Only `['hover']` appears in this repo; focus is always honoured. */
	triggers?: string[]
}

type TooltipValue = string | TooltipOptions | null | undefined

const SHOW_DELAY_MS = 300
const MODIFIER_PLACEMENTS = ['top', 'right', 'bottom', 'left'] as const

let tooltipCounter = 0

function resolveContent(value: TooltipValue): { text: string; options: TooltipOptions } | null {
	if (value === null || value === undefined || value === '') return null
	if (typeof value === 'string') return { text: value, options: {} }

	const text = value.content
	if (text === null || text === undefined || text === '') return null

	return { text, options: value }
}

interface TooltipHandle {
	refresh: () => void
	destroy: () => void
}

function createTooltip(
	trigger: HTMLElement,
	bindingValue: () => TooltipValue,
	modifier: Placement | null,
): TooltipHandle {
	const popper = document.createElement('div')
	popper.className = 'tooltip-popper'
	popper.setAttribute('role', 'tooltip')
	// `aria-describedby` on the trigger has to resolve to a real element, so the
	// id is assigned once and reused for every show of this tooltip.
	popper.id = `tooltip-${++tooltipCounter}`

	let stopAutoUpdate: (() => void) | null = null
	let showTimer: ReturnType<typeof setTimeout> | null = null
	let placement: Placement = modifier ?? 'top'

	function mount() {
		const resolved = resolveContent(bindingValue())
		if (!resolved) return false

		placement = modifier ?? resolved.options.placement ?? 'top'
		popper.className = resolved.options.popperClass
			? `tooltip-popper ${resolved.options.popperClass}`
			: 'tooltip-popper'

		if (resolved.options.html) {
			popper.innerHTML = resolved.text
		} else {
			// Text, never markup: an untrusted string cannot become HTML here.
			popper.textContent = resolved.text
		}

		;(document.getElementById('teleports') ?? document.body).appendChild(popper)
		void updatePosition()
		stopAutoUpdate = autoUpdate(trigger, popper, () => void updatePosition())

		// The description is announced only while it is actually shown.
		trigger.setAttribute('aria-describedby', popper.id)
		return true
	}

	async function updatePosition() {
		const { x, y } = await computePosition(trigger, popper, {
			placement,
			middleware: [offset(8), flip(), shift({ padding: 8 })],
		})
		popper.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`
	}

	function unmount() {
		stopAutoUpdate?.()
		stopAutoUpdate = null
		popper.remove()
		trigger.removeAttribute('aria-describedby')
	}

	function cancelPendingShow() {
		if (showTimer === null) return
		clearTimeout(showTimer)
		showTimer = null
	}

	function show() {
		cancelPendingShow()
		if (!popper.isConnected) mount()
	}

	function hide() {
		cancelPendingShow()
		unmount()
	}

	function onEnter() {
		cancelPendingShow()
		showTimer = setTimeout(() => {
			showTimer = null
			show()
		}, SHOW_DELAY_MS)
	}

	const onLeave = () => hide()

	trigger.addEventListener('mouseenter', onEnter)
	trigger.addEventListener('mouseleave', onLeave)
	trigger.addEventListener('focusin', show)
	trigger.addEventListener('focusout', onLeave)

	return {
		refresh() {
			if (!popper.isConnected) return
			unmount()
			mount()
		},
		destroy() {
			cancelPendingShow()
			unmount()
			trigger.removeEventListener('mouseenter', onEnter)
			trigger.removeEventListener('mouseleave', onLeave)
			trigger.removeEventListener('focusin', show)
			trigger.removeEventListener('focusout', onLeave)
		},
	}
}

const handles = new WeakMap<HTMLElement, TooltipHandle>()

function placementModifierOf(modifiers: Record<string, boolean>): Placement | null {
	return (MODIFIER_PLACEMENTS.find((name) => modifiers[name]) as Placement | undefined) ?? null
}

export const tooltipDirective: ObjectDirective<HTMLElement, TooltipValue> = {
	mounted(el, binding) {
		if (!resolveContent(binding.value)) return
		handles.set(
			el,
			createTooltip(el, () => binding.value, placementModifierOf(binding.modifiers)),
		)
	},

	updated(el, binding) {
		const handle = handles.get(el)
		if (!handle) {
			// The value only became truthy after mount, so start now.
			if (resolveContent(binding.value)) tooltipDirective.mounted?.(el, binding)
			return
		}
		// An open tooltip has to pick up new content, and react to being
		// suppressed by a change to falsy.
		handle.refresh()
	},

	unmounted(el) {
		handles.get(el)?.destroy()
		handles.delete(el)
	},
}
