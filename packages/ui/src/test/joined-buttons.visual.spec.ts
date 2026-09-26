import { describe, expect, it } from 'vitest'
import { type Component, defineComponent, h } from 'vue'

import Button from '../components/base/buttons/Button.vue'
import ButtonGroup from '../components/base/buttons/ButtonGroup.vue'
import ButtonStyled from '../components/base/ButtonStyled.vue'
import { mountThemed } from './visual-harness'

const CurrentGroup = defineComponent({
	render() {
		return h(ButtonGroup, null, {
			default: () => [
				h(
					Button as Component,
					{ type: 'colored', color: 'brand', size: 'xl' },
					{ default: () => 'Launch' },
				),
				h(Button as Component, { type: 'quiet', size: 'xl' }, { default: () => 'Options' }),
			],
		})
	},
})

const LegacyGroup = defineComponent({
	render() {
		return h('div', { class: 'joined-buttons' }, [
			h(
				ButtonStyled,
				{ type: 'standard', size: 'large' },
				{ default: () => h('button', { class: 'btn' }, ['Launch']) },
			),
			h(
				ButtonStyled,
				{ type: 'transparent', circular: true, size: 'large' },
				{ default: () => h('button', { class: 'btn' }, ['Options']) },
			),
		])
	},
})

function expectJoinedCorners(left: HTMLElement, right: HTMLElement) {
	const leftStyle = getComputedStyle(left)
	const rightStyle = getComputedStyle(right)

	expect(leftStyle.borderTopRightRadius).toBe('0px')
	expect(leftStyle.borderBottomRightRadius).toBe('0px')
	expect(Number.parseFloat(leftStyle.borderTopLeftRadius)).toBeGreaterThan(0)
	expect(rightStyle.borderTopLeftRadius).toBe('0px')
	expect(rightStyle.borderBottomLeftRadius).toBe('0px')
	expect(Number.parseFloat(rightStyle.borderTopRightRadius)).toBeGreaterThan(0)
}

describe('joined buttons', () => {
	it('joins current button roots through ButtonGroup and data-button', async () => {
		const root = (await mountThemed(CurrentGroup, {}, 'dark')).element as HTMLElement
		const buttons = root.querySelectorAll<HTMLElement>(':scope > [data-button]')

		expect(buttons).toHaveLength(2)
		expectJoinedCorners(buttons[0], buttons[1])
	})

	it('keeps the legacy global joined-buttons seam until its call sites migrate', async () => {
		const root = (await mountThemed(LegacyGroup, {}, 'dark')).element as HTMLElement
		const buttons = root.querySelectorAll<HTMLElement>(':scope > .btn-wrapper > .btn')

		expect(buttons).toHaveLength(2)
		expectJoinedCorners(buttons[0], buttons[1])
	})
})
