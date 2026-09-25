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
