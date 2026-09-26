import { expect, it } from 'vitest'
import { ref } from 'vue'

import MultiSelect from '../components/base/MultiSelect.vue'
import { I18N_INJECTION_KEY } from '../providers/i18n'
import { applyTheme, mountThemed, waitFor } from './visual-harness'

const options = Array.from({ length: 8 }, (_, index) => ({
	value: `option-${index}`,
	label: `Selected option ${index}`,
}))
const selected = options.map((option) => option.value)

function activate(element: HTMLElement) {
	element.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0 }))
	// `detail: 1` marks a real click; the trigger ignores keyboard-synthesised
	// clicks (`detail: 0`), which it would otherwise treat as a duplicate.
	element.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 1 }))
}

it('keeps the multiselect open while removing a tag from its overflow popover', async () => {
	const teleports = document.createElement('div')
	teleports.id = 'teleports'
	document.body.append(teleports)

	const host = document.createElement('div')
	host.style.width = '8rem'
	document.body.append(host)

	const wrapper = await mountThemed(
		MultiSelect,
		{
			modelValue: selected,
			options,
			maxTagRows: 1,
			showChevron: false,
		},
		'dark',
		{
			attachTo: host,
			global: {
				provide: {
					[I18N_INJECTION_KEY as symbol]: {
						locale: ref('en-US'),
						t: (key: string) => key,
						setLocale: () => undefined,
					},
				},
			},
		},
	)
	applyTheme('dark')
	await wrapper.vm.$nextTick()

	await waitFor(() => !!document.querySelector('[aria-haspopup="dialog"]'), {
		label: 'the selected-tag overflow trigger',
	})

	const listboxTrigger = document.querySelector('[aria-haspopup="listbox"]') as HTMLElement
	activate(listboxTrigger)
	await waitFor(() => !!document.querySelector('[aria-multiselectable="true"]'), {
		label: 'the multiselect listbox to open',
	})

	const overflowTrigger = document.querySelector('[aria-haspopup="dialog"]') as HTMLElement
	// The trigger must be keyboard-reachable, not just clickable.
	expect(overflowTrigger.tagName, 'the overflow trigger is a real button').toBe('BUTTON')
	expect(overflowTrigger.tabIndex, 'the overflow trigger can take focus').toBeGreaterThanOrEqual(0)
	overflowTrigger.focus()
	expect(document.activeElement, 'focus lands on the overflow trigger').toBe(overflowTrigger)

	activate(overflowTrigger)
	await waitFor(() => !!document.querySelector('.multiselect-overflow-popover'), {
		label: 'the selected-tag overflow popover to open',
	})

	const overflowTag = document.querySelector('.multiselect-overflow-popover span') as HTMLElement
	// A real pointer sequence, not `.click()`: the outside-click handler listens
	// for pointerdown, so a synthetic click alone would never reach it and the
	// ignore boundary below would go untested.
	overflowTag.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0 }))
	overflowTag.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
	overflowTag.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, button: 0 }))
	overflowTag.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 1 }))
	await waitFor(() => !!document.querySelector('[aria-multiselectable="true"]'), {
		label: 'the multiselect to remain open after overflow interaction',
	})

	wrapper.unmount()
	host.remove()
	teleports.remove()
})
