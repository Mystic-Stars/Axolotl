import { describe, expect, it } from 'vitest'

import Button from '../components/base/buttons/Button.vue'
import type { ButtonSize } from '../components/base/buttons/types'
import ButtonStyled from '../components/base/ButtonStyled.vue'
import { mountThemed } from './visual-harness'

/**
 * Pins the legacy → current button mapping, measured rather than assumed.
 *
 * The two systems are not pixel-identical, so this asserts the correspondences
 * that do hold exactly (`standard`→`md`, `large`→`xl`) and records the one that
 * does not (`small`, 24px, has no current equivalent — `xs` is 28px). Pinning
 * it means a future change to either system's geometry fails here instead of
 * silently changing how migrated buttons look.
 */

const SIZE_MAP: Record<string, ButtonSize> = {
	standard: 'md',
	large: 'xl',
}

async function measureLegacy(props: Record<string, unknown>) {
	const wrapper = await mountThemed(ButtonStyled, props, 'dark', {
		slots: { default: '<button class="btn">Label</button>' },
	})
	const element = wrapper.element.querySelector('button') as HTMLElement
	const style = getComputedStyle(element)
	return { height: style.height, radius: style.borderRadius, weight: style.fontWeight }
}

async function measureCurrent(props: Record<string, unknown>) {
	const wrapper = await mountThemed(Button, props, 'dark', { slots: { default: 'Label' } })
	const style = getComputedStyle(wrapper.element as HTMLElement)
	return { height: style.height, radius: style.borderRadius, weight: style.fontWeight }
}

describe('button mapping', () => {
	it.each(Object.entries(SIZE_MAP))(
		'maps legacy %s onto current %s with identical geometry',
		async (legacySize, currentSize) => {
			const legacy = await measureLegacy({ size: legacySize })
			const current = await measureCurrent({ size: currentSize })

			expect(current).toEqual(legacy)
		},
	)

	it('has no current equivalent for the legacy small size', async () => {
		const legacy = await measureLegacy({ size: 'small' })
		const smallest = await measureCurrent({ size: 'xs' })

		// Recorded deliberately: `small` is 24px and `xs` is 28px, so a migrated
		// `size="small"` grows. Anyone changing this should change the assertion
		// consciously rather than discover it in the UI.
		expect(legacy.height).toBe('24px')
		expect(smallest.height).toBe('28px')
	})

	it('keeps the current size ladder ordered', async () => {
		const sizes: ButtonSize[] = ['xs', 'sm', 'md', 'lg', 'xl']
		const heights = await Promise.all(
			sizes.map(async (size) => (await measureCurrent({ size })).height),
		)

		const numeric = heights.map((height) => Number.parseFloat(height))
		expect(numeric).toEqual([...numeric].sort((a, b) => a - b))
		expect(new Set(numeric).size).toBe(sizes.length)
	})
})
