import { expect, it } from 'vitest'

import ButtonFrame from '../components/base/buttons/ButtonFrame.vue'
import ButtonStyled from '../components/base/ButtonStyled.vue'
import { mountThemed } from './visual-harness'

/**
 * The pilot migration recipe, verified by measurement.
 *
 * `ButtonStyled circular type="transparent"` — the most common icon-button shape
 * in the codebase (152 `circular` call sites) — is reproduced exactly by
 * `ButtonFrame` with a `quiet` type, `circular` and `iconOnly`. Asserted rather
 * than described, so the recipe stays true as either component evolves.
 */

function measure(element: HTMLElement) {
	const style = getComputedStyle(element)
	const box = element.getBoundingClientRect()
	return {
		height: style.height,
		width: `${Math.round(box.width)}px`,
		radius: style.borderRadius.replace(/999\d+px/, 'round'),
		background: style.backgroundColor,
		color: style.color,
	}
}

it('reproduces the legacy circular icon button with ButtonFrame', async () => {
	const legacy = await mountThemed(ButtonStyled, { circular: true, type: 'transparent' }, 'dark', {
		slots: { default: '<button class="btn">X</button>' },
	})
	const replacement = await mountThemed(
		ButtonFrame,
		{
			as: 'button',
			type: 'quiet',
			size: 'md',
			circular: true,
			iconOnly: true,
			interaction: 'surface',
		},
		'dark',
		{ slots: { default: 'X' } },
	)

	expect(measure(replacement.element as HTMLElement)).toEqual(
		measure(legacy.element.querySelector('button') as HTMLElement),
	)
})
