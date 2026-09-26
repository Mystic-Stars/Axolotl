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
	return {
		// Width is part of the contract, not a detail: the legacy button drew its
		// ring as a real 1px border, which participates in layout, so an
		// auto-width button is 2px wider than one using a box-shadow ring.
		// Asserting it here is what stops that from silently narrowing every
		// migrated text button.
		width: style.width,
		height: style.height,
		radius: style.borderRadius,
		weight: style.fontWeight,
		textColor: style.color,
	}
}

async function measureCurrent(props: Record<string, unknown>) {
	const wrapper = await mountThemed(Button, props, 'dark', { slots: { default: 'Label' } })
	const style = getComputedStyle(wrapper.element as HTMLElement)
	return {
		width: style.width,
		height: style.height,
		radius: style.borderRadius,
		weight: style.fontWeight,
		textColor: style.color,
	}
}

/** Geometry shared by a legacy button and its replacement, excluding size. */
async function measureShapeGeometry(
	legacyProps: Record<string, unknown>,
	currentProps: Record<string, unknown>,
) {
	const legacy = await measureLegacy(legacyProps)
	const current = await measureCurrent(currentProps)
	// `textColor` is asserted separately: the migration deliberately moves the
	// default label colour to `--color-text-primary`.
	const { textColor: _l, ...legacyShape } = legacy
	const { textColor: _c, ...currentShape } = current
	return { legacyShape, currentShape }
}

describe('button mapping', () => {
	it.each(Object.entries(SIZE_MAP))(
		'maps legacy %s onto current %s with identical geometry',
		async (legacySize, currentSize) => {
			const { legacyShape, currentShape } = await measureShapeGeometry(
				{ size: legacySize },
				{ size: currentSize },
			)

			expect(currentShape).toEqual(legacyShape)
		},
	)

	it('maps the legacy small size onto 2xs without changing its height', async () => {
		const { legacyShape, currentShape } = await measureShapeGeometry(
			{ size: 'small' },
			{ size: '2xs' },
		)

		// `small` is 24px and `2xs` was added to match it. Before `2xs` existed
		// the nearest size was `xs` (28px), so migrating `size="small"` grew
		// every one of those buttons; this assertion is what prevents that
		// regression from returning.
		expect(legacyShape.height).toBe('24px')
		expect(currentShape).toEqual(legacyShape)
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

	// Each entry is a legacy shape and the current props that replace it. Height,
	// width, radius and weight must match exactly; only the label colour is
	// expected to move.
	const SHAPE_MAP: [string, Record<string, unknown>, Record<string, unknown>][] = [
		['default', {}, {}],
		['outlined', { type: 'outlined' }, { type: 'outlined' }],
		['transparent', { type: 'transparent' }, { type: 'quiet' }],
		['small', { size: 'small' }, { size: '2xs' }],
		['large', { size: 'large' }, { size: 'xl' }],
		['brand', { color: 'brand' }, { type: 'colored', color: 'brand' }],
		['outlined red', { type: 'outlined', color: 'red' }, { type: 'outlined', color: 'red' }],
		['chip', { type: 'chip', color: 'brand' }, { type: 'chip', color: 'brand' }],
		[
			'highlight-colored-text',
			{ type: 'highlight-colored-text', color: 'brand' },
			{ type: 'chip-text', color: 'brand' },
		],
		['highlight', { type: 'highlight', color: 'brand' }, { type: 'highlight', color: 'brand' }],
	]

	it.each(SHAPE_MAP)(
		'keeps %s geometry identical across the migration',
		async (_n, legacyProps, currentProps) => {
			const { legacyShape, currentShape } = await measureShapeGeometry(legacyProps, currentProps)

			expect(currentShape).toEqual(legacyShape)
			// Guard against the assertion passing on empty values.
			expect(Number.parseFloat(legacyShape.width)).toBeGreaterThan(0)
		},
	)

	it('only moves the label colour, never the geometry', async () => {
		const legacy = await measureLegacy({})
		const current = await measureCurrent({})

		// The one deliberate change: legacy labelled with `--color-text-default` (body
		// text), the replacement with `--color-text-primary` (heading).
		expect(current.textColor).not.toBe(legacy.textColor)
		expect(current.width).toBe(legacy.width)
		expect(current.height).toBe(legacy.height)
	})

	it('maps the legacy circular icon button onto a 1:1 icon-only button', async () => {
		const icon = '<svg width="24" height="24"></svg>'

		const legacyWrapper = await mountThemed(
			ButtonStyled,
			{ circular: true, size: 'large' },
			'dark',
			{ slots: { default: `<button class="btn">${icon}</button>` } },
		)
		const legacyStyle = getComputedStyle(
			legacyWrapper.element.querySelector('button') as HTMLElement,
		)

		const currentWrapper = await mountThemed(
			Button,
			{ size: 'xl', circular: true, 'icon-only': true },
			'dark',
			{ slots: { default: icon } },
		)
		const currentStyle = getComputedStyle(currentWrapper.element as HTMLElement)

		// `Button` does not declare `iconOnly`/`circular`; they reach
		// `ButtonFrame` as fall-through attributes, which Vue matches against its
		// camelCase props. Render the same size without them so this test proves
		// they are what makes the button square instead of padded -- if the
		// fall-through ever stops working, the assertions below fail rather than
		// quietly measuring an icon-only button of the wrong shape.
		const paddedWrapper = await mountThemed(Button, { size: 'xl' }, 'dark', {
			slots: { default: icon },
		})
		const paddedStyle = getComputedStyle(paddedWrapper.element as HTMLElement)

		// The icon-only button must stay a square. A `width` that comes from the
		// surrounding layout rather than from the size ladder turns it into a
		// rectangle, and because the frame does not shrink, the buttons after it
		// are pushed out of the header entirely.
		expect(legacyStyle.width).toBe('48px')
		expect(currentStyle.width).toBe(legacyStyle.width)
		expect(currentStyle.height).toBe(legacyStyle.height)
		// `rounded-full` (9999px) and the legacy literal (99999px) are both fully
		// round on a 48px box, so compare the effect rather than the number.
		expect(Number.parseFloat(legacyStyle.borderRadius)).toBeGreaterThanOrEqual(24)
		expect(Number.parseFloat(currentStyle.borderRadius)).toBeGreaterThanOrEqual(24)
		expect(paddedStyle.width).toBe('54px')
		expect(Number.parseFloat(paddedStyle.borderRadius)).toBeLessThan(24)
	})
})
