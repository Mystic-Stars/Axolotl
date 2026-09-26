import { describe, expect, it, vi } from 'vitest'
import { defineComponent, h } from 'vue'

import Button from '../components/base/buttons/Button.vue'
import ButtonLink from '../components/base/buttons/ButtonLink.vue'
import { mountThemed } from './visual-harness'

const LinkRoot = defineComponent({
	name: 'LinkRoot',
	inheritAttrs: false,
	props: {
		to: { type: String, required: true },
	},
	setup(props, { attrs, slots }) {
		return () => h('a', { ...attrs, href: props.to }, slots.default?.())
	},
})

describe('current button public API', () => {
	it('explicitly forwards icon-only and circular visuals and naming', async () => {
		const wrapper = await mountThemed(
			Button,
			{ iconOnly: true, circular: true, label: 'Open menu', size: 'xl' },
			'dark',
			{ slots: { default: '<svg aria-hidden="true"></svg>' } },
		)
		const element = wrapper.element as HTMLButtonElement
		const style = getComputedStyle(element)

		expect(element.tagName).toBe('BUTTON')
		expect(element.getAttribute('aria-label')).toBe('Open menu')
		expect(style.width).toBe(style.height)
		expect(Number.parseFloat(style.borderRadius)).toBeGreaterThanOrEqual(
			Number.parseFloat(style.height) / 2,
		)
	})

	it('forwards native button type and attributes to one button root', async () => {
		const wrapper = await mountThemed(
			Button,
			{ nativeType: 'submit', name: 'intent', value: 'save', form: 'editor' },
			'dark',
			{ slots: { default: 'Save' } },
		)
		const element = wrapper.element as HTMLButtonElement

		expect(element.tagName).toBe('BUTTON')
		expect(element.type).toBe('submit')
		expect(element.name).toBe('intent')
		expect(element.value).toBe('save')
		expect(element.getAttribute('form')).toBe('editor')
		expect(element.querySelector('button, a')).toBeNull()
	})
})

describe('ButtonLink semantics', () => {
	it('renders a native anchor root without nesting an interactive element', async () => {
		const wrapper = await mountThemed(
			ButtonLink,
			{ href: '/projects', target: '_blank', rel: 'noreferrer' },
			'dark',
			{ slots: { default: 'Projects' } },
		)
		const element = wrapper.element as HTMLAnchorElement

		expect(element.tagName).toBe('A')
		expect(element.getAttribute('href')).toBe('/projects')
		expect(element.target).toBe('_blank')
		expect(element.rel).toBe('noreferrer')
		expect(element.querySelector('button, a')).toBeNull()
	})

	it('uses a supplied link component as the single interactive root', async () => {
		const wrapper = await mountThemed(ButtonLink, { as: LinkRoot, to: '/settings' }, 'dark', {
			slots: { default: 'Settings' },
		})
		const element = wrapper.element as HTMLAnchorElement

		expect(element.tagName).toBe('A')
		expect(element.getAttribute('href')).toBe('/settings')
		expect(element.hasAttribute('data-button')).toBe(true)
		expect(element.querySelector('button, a')).toBeNull()
	})

	it('removes native navigation and suppresses activation while disabled', async () => {
		const onClick = vi.fn()
		const wrapper = await mountThemed(
			ButtonLink,
			{ href: '/danger', disabled: true, onClick },
			'dark',
			{ slots: { default: 'Unavailable' } },
		)
		const element = wrapper.element as HTMLAnchorElement

		expect(element.hasAttribute('href')).toBe(false)
		expect(element.getAttribute('aria-disabled')).toBe('true')
		expect(element.tabIndex).toBe(-1)

		const allowed = element.dispatchEvent(
			new MouseEvent('click', { bubbles: true, cancelable: true }),
		)
		expect(allowed).toBe(false)
		expect(onClick).not.toHaveBeenCalled()
	})

	it('requires an accessible name for an icon-only link', async () => {
		const wrapper = await mountThemed(
			ButtonLink,
			{ href: '/help', iconOnly: true, label: 'Help' },
			'dark',
			{ slots: { default: '<svg aria-hidden="true"></svg>' } },
		)

		expect(wrapper.element.getAttribute('aria-label')).toBe('Help')
	})
})
