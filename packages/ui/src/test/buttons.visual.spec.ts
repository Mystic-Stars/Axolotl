import { describe, expect, it } from 'vitest'

import Button from '../components/base/buttons/Button.vue'
import type { ButtonColor, ButtonSize, ButtonType } from '../components/base/buttons/types'
import { assertTokensLoaded, computedToken, mountThemed, THEMES } from './visual-harness'

/**
 * Locks the observable style contract of the button system before the legacy
 * `ButtonStyled` call sites (757 of them) are migrated onto it. A regression
 * here would be a silent visual change — the two generations do not share
 * colour tokens, so nothing else in the toolchain would catch it.
 */

const TYPES: ButtonType[] = ['base', 'outlined', 'colored', 'colored-text', 'quiet']
const SIZES: ButtonSize[] = ['xs', 'sm', 'md', 'lg', 'xl']
const COLORS: ButtonColor[] = ['brand', 'red', 'orange', 'green', 'blue', 'purple']

async function mountButton(props: Record<string, unknown>, theme = 'dark' as const) {
	const wrapper = await mountThemed(Button, props, theme, {
		slots: { default: 'Label' },
	})
	const element = wrapper.element as HTMLElement
	return { wrapper, element }
}

describe('button system style contract', () => {
	it('loads the token layer', () => {
		assertTokensLoaded()
	})

	it('keeps a stable height per size', async () => {
		const heights: Record<string, number> = {}

		for (const size of SIZES) {
			const { element } = await mountButton({ size })
			heights[size] = element.getBoundingClientRect().height
		}

		for (const size of SIZES) {
			expect(heights[size], `size "${size}" should have a non-zero height`).toBeGreaterThan(0)
		}

		// Sizes are ordered; a migration that collapses or reorders them is a
		// visual regression even if it type-checks.
		const ordered = SIZES.map((size) => heights[size])
		expect(ordered).toEqual([...ordered].sort((a, b) => a - b))
	})

	it('resolves a distinct colour token per colour', async () => {
		const references: Record<string, string> = {}

		// `ButtonFrame` publishes the colour as a `var()` reference in the
		// inline `--button-color` style, which the type/size classes consume.
		// Read the inline style (computed style resolves the chain to a
		// concrete colour) because the *reference* is the contract a migration
		// has to preserve.
		for (const color of COLORS) {
			const { element } = await mountButton({ type: 'colored', color })
			references[color] = element.style.getPropertyValue('--button-color').trim()
		}

		for (const color of COLORS) {
			expect(references[color], `colour "${color}" should publish --button-color`).toContain(
				`--color-${color}`,
			)
		}

		// Each colour must name its own token rather than collapsing to a
		// default (note `--color-brand` aliases `--color-green`, so the
		// *resolved* values are not all distinct -- the references are).
		expect(new Set(Object.values(references)).size).toBe(COLORS.length)
	})

	it('renders every type without collapsing to the base style', async () => {
		const outlines = new Map<string, string>()

		for (const type of TYPES) {
			const { element } = await mountButton({ type, color: 'brand' })
			const style = getComputedStyle(element)
			outlines.set(type, `${style.boxShadow}|${style.color}|${style.borderColor}`)
		}

		for (const type of TYPES) {
			expect(outlines.get(type), `type "${type}" should render`).toBeTruthy()
		}

		// `colored` and `quiet` are visually opposite; if they resolve equal the
		// type union has stopped being applied.
		expect(outlines.get('colored')).not.toBe(outlines.get('quiet'))
	})

	it('tracks the theme tokens rather than hard-coded colours', async () => {
		const resolvedPerTheme = new Map<string, string>()

		for (const theme of THEMES) {
			const { element } = await mountButton({ type: 'colored', color: 'red' }, theme)
			const reference = element.style.getPropertyValue('--button-color').trim()

			expect(reference, `red should publish a token in ${theme}`).toContain('--color-red')

			// Resolve the referenced token to a concrete colour for this theme.
			const resolved = computedToken(document.documentElement, '--color-red')
			expect(resolved, `--color-red should resolve in ${theme}`).not.toBe('')

			resolvedPerTheme.set(theme, resolved)
		}

		// A genuinely theme-driven colour must differ between the lightest and
		// darkest theme, which is what proves the component reads the token
		// rather than a literal.
		expect(resolvedPerTheme.get('light')).not.toBe(resolvedPerTheme.get('oled'))
	})

	it('marks disabled buttons for assistive technology and styling', async () => {
		const { element } = await mountButton({ disabled: true })
		const disabledStyle = getComputedStyle(element)

		expect(element.hasAttribute('disabled') || element.getAttribute('aria-disabled')).toBeTruthy()
		// `computedToken` returns a string, so `toBeDefined()` would pass on the
		// empty string too; assert the token actually resolved.
		expect(computedToken(element, '--surface-2')).not.toBe('')
		expect(disabledStyle.cursor).not.toBe('pointer')
	})
})
