import { expect, it } from 'vitest'

import TagOverflowPopover from '../components/project/TagOverflowPopover.vue'
import { applyTheme, assertTokensLoaded, mountThemed, waitFor } from './visual-harness'

const content = () => document.querySelector('.menu-surface') as HTMLElement | null

function openPopover() {
	const trigger = document.querySelector('[aria-haspopup="dialog"]') as HTMLElement | null
	trigger?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0 }))
	trigger?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
}

async function mountOverflow() {
	const wrapper = await mountThemed(
		TagOverflowPopover,
		{ count: 2, wrapperClass: 'w-full' },
		'dark',
		{
			attrs: { 'data-no-row-click': '' },
			slots: {
				trigger: '<span id="overflow-trigger">+2</span>',
				default: '<button id="overflow-tag">1.21.1</button>',
			},
		},
	)

	return wrapper
}

it('loads the token layer', () => {
	assertTokensLoaded()
})

it('opens on trigger activation and forwards trigger attributes', async () => {
	const wrapper = await mountOverflow()
	applyTheme('dark')
	await wrapper.vm.$nextTick()

	const trigger = document.querySelector('[aria-haspopup="dialog"]')
	expect(trigger?.getAttribute('data-no-row-click')).toBe('')
	expect(content()).toBeNull()

	openPopover()
	await waitFor(() => !!content(), { label: 'the overflow popover to open' })
	expect(content()?.textContent).toContain('1.21.1')

	wrapper.unmount()
})

it('closes on Escape', async () => {
	const wrapper = await mountOverflow()
	applyTheme('dark')
	await wrapper.vm.$nextTick()

	openPopover()
	await waitFor(() => !!content(), { label: 'the overflow popover to open' })

	document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
	await waitFor(() => !content(), { label: 'the overflow popover to close on Escape' })

	wrapper.unmount()
})

it('exposes a focusable button trigger that keyboard users can activate', async () => {
	// The trigger was a plain div at one point, which is neither focusable nor
	// activatable. A diff cannot show that, and every pointer test still passes,
	// so the trigger element itself is asserted here.
	const wrapper = await mountOverflow()
	applyTheme('dark')
	await wrapper.vm.$nextTick()

	const trigger = document.querySelector('[aria-haspopup="dialog"]') as HTMLElement
	expect(trigger, 'the trigger exists').toBeTruthy()
	expect(trigger.tagName, 'the trigger is a real button').toBe('BUTTON')
	expect(trigger.getAttribute('type')).toBe('button')
	expect(trigger.tabIndex, 'the trigger can receive focus').toBeGreaterThanOrEqual(0)

	// Reaching it by keyboard is the point: focus it, then press Enter.
	trigger.focus()
	expect(document.activeElement, 'focus actually lands on the trigger').toBe(trigger)

	trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
	trigger.click()
	await waitFor(() => !!content(), { label: 'Enter to open the overflow popover' })

	wrapper.unmount()
})
