import { describe, expect, it } from 'vitest'

import Button from '../components/base/buttons/Button.vue'
import type { ButtonSize } from '../components/base/buttons/types'
import ButtonStyled from '../components/base/ButtonStyled.vue'
import { mountThemed } from './visual-harness'

/**
 * Pins the legacy → current button mapping, measured rather than assumed.
 *
 * The two systems are not pixel-identical, so this asserts the correspondences
 * that do hold exactly (`standard`→`md`, `large`→`xl`, `small`→`2xs`). `2xs` is
 * 24px, added specifically so the legacy `small` size keeps its height instead
 * of growing into `xs` (28px). Pinning these means a future change to either
 * system's geometry fails here instead of silently changing how migrated
 * buttons look.
 */

const SIZE_MAP: Record<string, ButtonSize> = {
	small: '2xs',
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

	it('maps the legacy small size onto 2xs without changing its height', async () => {
		const legacy = await measureLegacy({ size: 'small' })
		const current = await measureCurrent({ size: '2xs' })

		// `small` is 24px and `2xs` was added to match it. Before `2xs` existed
		// the nearest size was `xs` (28px), so migrating `size="small"` grew
		// every one of those buttons; this assertion is what prevents that
		// regression from returning.
		expect(legacy.height).toBe('24px')
		expect(current).toEqual(legacy)
	})

	it('keeps the current size ladder ordered', async () => {
		const sizes: ButtonSize[] = ['2xs', 'xs', 'sm', 'md', 'lg', 'xl']
		const heights = await Promise.all(
			sizes.map(async (size) => (await measureCurrent({ size })).height),
		)

		const numeric = heights.map((height) => Number.parseFloat(height))
		expect(numeric).toEqual([...numeric].sort((a, b) => a - b))
		expect(new Set(numeric).size).toBe(sizes.length)
	})
})
