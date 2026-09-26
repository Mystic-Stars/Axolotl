import { describe, expect, it } from 'vitest'
import { defineComponent, h } from 'vue'

import Button from '../components/base/buttons/Button.vue'
import ButtonStyled from '../components/base/ButtonStyled.vue'
import { mountThemed } from './visual-harness'

/**
 * A joined group squares the corner two buttons share so the seam reads as one
 * control. The two button generations find that corner differently: the legacy
 * one through `.btn` and the `ButtonStyled` wrapper, the current one through the
 * `data-button` marker on its own root. Missing either leaves a rounded edge
 * against a square one, which is what happened when the current button replaced
 * a `ButtonStyled` inside `Index.vue`'s server-instance launch row.
 */
const Group = defineComponent({
	render() {
		return h('div', { class: 'joined-buttons' }, [
			h(Button, { type: 'colored', color: 'brand', size: 'xl' }, { default: () => 'Launch' }),
			h(
				ButtonStyled,
				{ type: 'transparent', circular: true, size: 'large' },
				{
					default: () => h('button', null, ['v']),
				},
			),
		])
	},
})

describe('joined buttons', () => {
	it('squares the shared corner across both button generations', async () => {
		const root = (await mountThemed(Group, {}, 'dark')).element as HTMLElement

		const current = getComputedStyle(root.querySelector('[data-button]') as HTMLElement)
		const legacy = getComputedStyle(
			(root.querySelector('.btn-wrapper') as HTMLElement).querySelector('button') as HTMLElement,
		)

		// The current button is the left half, so it loses its right corners.
		expect(current.borderTopRightRadius).toBe('0px')
		expect(current.borderBottomRightRadius).toBe('0px')
		// Its outer corners keep the frame's radius, so this is about the join and
		// not about a button that lost its rounding entirely.
		expect(Number.parseFloat(current.borderTopLeftRadius)).toBeGreaterThan(0)

		// The legacy button is the right half, so it loses its left corners.
		expect(legacy.borderTopLeftRadius).toBe('0px')
		expect(legacy.borderBottomLeftRadius).toBe('0px')
		expect(Number.parseFloat(legacy.borderTopRightRadius)).toBeGreaterThan(0)
	})
})
