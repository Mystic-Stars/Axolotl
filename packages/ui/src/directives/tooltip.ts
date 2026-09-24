import '../styles/tooltip.css'

import { autoUpdate, computePosition, flip, offset, type Placement, shift } from '@floating-ui/vue'
import type { ObjectDirective } from 'vue'

import { resolveTooltipContent, type TooltipValue } from './tooltip-value'

/**
 * The project's tooltip directive, replacing `floating-vue`'s.
 *
 * It deliberately keeps that directive's call signature: several hundred call
 * sites use `v-tooltip`, and the shapes it accepts are the ones they rely on.
 * Falsy means "no tooltip" -- see `tooltip-value.ts`, which holds that
 * coercion on its own so it can be tested without loading this file's CSS.
 *
 * `html` writes raw markup for the two call sites that pass an
 * already-sanitised fragment; everything else is written as text, so untrusted
 * strings can never become markup.
 *
 * It lives in `packages/ui` rather than the app because the website compiles
 * these components too and needs the same directive; the stylesheet travels
 * with it for the same reason.
 */

/** Matches floating-vue's tooltip theme, so the feel is unchanged. */
const SHOW_DELAY_MS = 200
const DISTANCE_PX = 5

const PLACEMENTS = [
	'top',
	'top-start',
	'top-end',
	'right',
	'right-start',
	'right-end',
	'bottom',
	'bottom-start',
	'bottom-end',
	'left',
	'left-start',
	'left-end',
] as const

interface TooltipHandle {
	/** Called from `updated` with the fresh value; the stored one goes stale. */
	setValue: (value: TooltipValue) => void
	destroy: () => void
}

function createTooltip(trigger: HTMLElement, modifier: Placement | null): TooltipHandle {
	const popper = document.createElement('div')
	popper.className = 'tooltip-popper'
	popper.setAttribute('role', 'tooltip')
	// `aria-describedby` has to resolve to a real element, so the id is assigned
	// once and reused for every show.
	popper.id = `tooltip-${++tooltipCounter}`

	// The value is held here rather than captured from the binding passed to
	// `mounted`: Vue swaps that binding object on every re-render, so a closure
	// over it would keep serving the text from the first render.
	let value: TooltipValue
	let stopAutoUpdate: (() => void) | null = null
	let showTimer: ReturnType<typeof setTimeout> | null = null

	function mount() {
		const resolved = resolveTooltipContent(value)
		if (!resolved) return

		popper.className = resolved.options.popperClass
			? `tooltip-popper ${resolved.options.popperClass}`
			: 'tooltip-popper'

		if (resolved.options.html) {
			// Both call sites pass markup that has already been parsed and
			// sanitised upstream; do not introduce a caller that passes raw input.
			popper.innerHTML = resolved.text
		} else {
			// Text, never markup, so an untrusted string cannot become HTML.
			popper.textContent = resolved.text
		}

		;(document.getElementById('teleports') ?? document.body).appendChild(popper)

		const placement = modifier ?? resolved.options.placement ?? 'top'
		void updatePosition(placement)
		stopAutoUpdate = autoUpdate(trigger, popper, () => void updatePosition(placement))

		// Announced only while it is actually displayed.
		trigger.setAttribute('aria-describedby', popper.id)
	}

	async function updatePosition(placement: Placement) {
		const { x, y } = await computePosition(trigger, popper, {
			placement,
			// `fixed` so the returned coordinates are viewport-relative, matching
			// the popper's `position: fixed`.
			strategy: 'fixed',
			middleware: [offset(DISTANCE_PX), flip(), shift({ padding: 8 })],
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
		if (popper.isConnected) unmount()
		if (resolveTooltipContent(value)) mount()
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
		setValue(next) {
			value = next
			// Only re-render when something is actually on screen, so an update to
			// a closed tooltip costs nothing.
			if (popper.isConnected) show()
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

let tooltipCounter = 0

const handles = new WeakMap<HTMLElement, TooltipHandle>()

function placementModifierOf(modifiers: Record<string, boolean>): Placement | null {
	return (PLACEMENTS.find((name) => modifiers[name]) as Placement | undefined) ?? null
}

export const tooltipDirective: ObjectDirective<HTMLElement, TooltipValue> = {
	mounted(el, binding) {
		const handle = createTooltip(el, placementModifierOf(binding.modifiers))
		handles.set(el, handle)
		handle.setValue(binding.value)
	},

	updated(el, binding) {
		const handle = handles.get(el)
		if (!handle) return
		if (binding.value === binding.oldValue) return

		handle.setValue(binding.value)
	},

	unmounted(el) {
		handles.get(el)?.destroy()
		handles.delete(el)
	},
}
