import { describe, expect, it } from 'vitest'

import ProgressBar from '../components/base/ProgressBar.vue'
import {
	assertTokensLoaded,
	computedToken,
	mountThemed,
	type Theme,
	THEMES,
} from './visual-harness'

/**
 * Pins the shared `ProgressBar`'s rendered geometry and colour before the
 * app-local `apps/app-frontend/src/components/ui/ProgressBar.vue` is migrated
 * onto it.
 *
 * The two components are not interchangeable by signature, so the migration
 * cannot be mechanical and a silent visual change is the real risk:
 *
 * - shared: `progress` is a **fraction** of a separate `max` (default `1`), so
 *   half-filled is `:progress="0.5"` or `:progress="50" :max="100"`.
 * - app-local: `progress` is a **0-100 percentage** with no `max`, so the same
 *   half-filled bar is `:progress="50"`.
 *
 * The group below spells the app-local number out as a shared prop pair
 * (`{ progress: 50, max: 100 }`) and asserts it renders identically to the
 * fraction (`{ progress: 0.5, max: 1 }`) — that equivalence *is* the migration
 * recipe, verified rather than described. The app-local component is not
 * imported here: it lives in the desktop app, which this platform-agnostic
 * package must not depend on.
 *
 * Everything asserted is measured off a real browser layout (the track and its
 * filled child are positioned by `getBoundingClientRect`), not read back from
 * the props. Each state mounts fresh because the fill animates its `width`
 * over 0.2s — updating a mounted bar would measure a frame mid-transition.
 */

const COLORS = ['brand', 'green', 'red', 'orange', 'blue', 'purple', 'gray'] as const

/**
 * The same visual state, written in each API. `{ progress: 50, max: 100 }` is
 * the shape an app-local `progress: 50` call site becomes.
 */
const SAME_VISUAL_STATE = [
	{
		ratio: 0,
		spellings: [
			{ progress: 0, max: 1 },
			{ progress: 0, max: 100 },
		],
	},
	{
		ratio: 0.25,
		spellings: [
			{ progress: 0.25, max: 1 },
			{ progress: 25, max: 100 },
			{ progress: 1, max: 4 },
		],
	},
	{
		ratio: 0.5,
		spellings: [
			{ progress: 0.5, max: 1 },
			{ progress: 50, max: 100 },
		],
	},
	{
		ratio: 0.75,
		spellings: [
			{ progress: 0.75, max: 1 },
			{ progress: 75, max: 100 },
			{ progress: 3, max: 4 },
		],
	},
	{
		ratio: 1,
		spellings: [
			{ progress: 1, max: 1 },
			{ progress: 100, max: 100 },
		],
	},
]

/** The app-local component is also `h-2`, so the bar height is part of the contract. */
const TRACK_HEIGHT_PX = 8

async function settle(): Promise<void> {
	await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
}

async function measure(props: Record<string, unknown>, theme: Theme = 'dark') {
	const wrapper = await mountThemed(ProgressBar, props, theme)
	await settle()

	const track = wrapper.element.querySelector('[role="progressbar"]') as HTMLElement
	const fill = track.firstElementChild as HTMLElement
	const trackBox = track.getBoundingClientRect()
	const fillBox = fill.getBoundingClientRect()

	const measured = {
		trackWidth: trackBox.width,
		trackHeight: trackBox.height,
		fillWidth: fillBox.width,
		// The filled proportion is what a migration has to preserve; reading it
		// off the boxes means a component that ignores `progress` fails here.
		ratio: trackBox.width === 0 ? 0 : fillBox.width / trackBox.width,
		fillColor: getComputedStyle(fill).backgroundColor,
		ariaValueNow: track.getAttribute('aria-valuenow'),
	}

	wrapper.unmount()
	return measured
}

describe('progress bar mapping', () => {
	it('loads the token layer', () => {
		assertTokensLoaded()
	})

	describe('rendered geometry', () => {
		it('renders an empty bar at zero progress', async () => {
			const bar = await measure({ progress: 0, max: 1 })

			expect(bar.trackWidth).toBeGreaterThan(0)
			expect(bar.fillWidth).toBe(0)
			expect(bar.ratio).toBe(0)
		})

		it('renders a full bar when progress equals max', async () => {
			const bar = await measure({ progress: 1, max: 1 })

			expect(bar.trackWidth).toBeGreaterThan(0)
			expect(bar.fillWidth).toBe(bar.trackWidth)
			expect(bar.ratio).toBe(1)
		})

		it('renders the track at the height both components share', async () => {
			const bar = await measure({ progress: 0.5, max: 1 })

			expect(bar.trackHeight).toBe(TRACK_HEIGHT_PX)
		})

		it('fills the track in proportion to progress over max', async () => {
			for (const { progress, max } of [
				{ progress: 0.25, max: 1 },
				{ progress: 0.5, max: 1 },
				{ progress: 0.75, max: 1 },
				{ progress: 50, max: 100 },
				{ progress: 1, max: 3 },
			]) {
				const bar = await measure({ progress, max })

				expect(bar.ratio, `${progress}/${max} should fill ${progress / max}`).toBeCloseTo(
					progress / max,
					3,
				)
			}
		})

		it('increases monotonically as progress rises', async () => {
			const values = [0, 0.1, 0.25, 0.5, 0.75, 0.9, 1]
			const ratios: number[] = []

			for (const progress of values) {
				ratios.push((await measure({ progress, max: 1 })).ratio)
			}

			expect(ratios[0]).toBe(0)
			expect(ratios.at(-1)).toBe(1)
			for (let index = 1; index < ratios.length; index += 1) {
				expect(
					ratios[index],
					`${values[index]} should fill more than ${values[index - 1]}`,
				).toBeGreaterThan(ratios[index - 1])
			}
		})

		it('renders every spelling of the same fraction identically', async () => {
			for (const { ratio, spellings } of SAME_VISUAL_STATE) {
				const rendered = []
				for (const spelling of spellings) {
					rendered.push(await measure(spelling))
				}

				for (const [index, bar] of rendered.entries()) {
					const { progress, max } = spellings[index]
					const label = `${progress}/${max}`

					expect(bar.ratio, `${label} should render as ${ratio} of the track`).toBeCloseTo(ratio, 3)
					expect(bar.fillColor, `${label} should use the same fill colour`).toBe(
						rendered[0].fillColor,
					)
				}
			}
		})

		it('reports the proportion as the 0-100 percentage the app-local API takes', async () => {
			// The app-local `progress` is a whole percentage, so the number the
			// shared component publishes must be that same 0-100 value, not the
			// fraction — otherwise every migrated call site is off by 100x.
			for (const { progress, max } of [
				{ progress: 0.5, max: 1 },
				{ progress: 50, max: 100 },
				{ progress: 0.4, max: 1 },
				{ progress: 1, max: 3 },
			]) {
				const bar = await measure({ progress, max })

				expect(bar.ariaValueNow).toBe(String(Math.round((progress / max) * 100)))
			}
		})

		it('shows the same percentage in the optional label', async () => {
			const wrapper = await mountThemed(
				ProgressBar,
				{ progress: 0.4, max: 1, showProgress: true },
				'dark',
			)
			await settle()

			expect((wrapper.element as HTMLElement).textContent?.trim()).toBe('40%')
			wrapper.unmount()
		})
	})

	describe('colour', () => {
		it('resolves each fill from its own theme token', async () => {
			for (const theme of THEMES) {
				for (const color of COLORS) {
					const wrapper = await mountThemed(ProgressBar, { progress: 0.5, max: 1, color }, theme)
					await settle()

					const track = wrapper.element.querySelector('[role="progressbar"]') as HTMLElement
					const fill = track.firstElementChild as HTMLElement
					const expected = resolveToken(`--color-${color}`)

					expect(
						getComputedStyle(fill).backgroundColor,
						`${color} should fill from --color-${color} in ${theme}`,
					).toBe(expected)

					wrapper.unmount()
				}
			}
		})

		it('renders a distinct fill per hue', async () => {
			const fills = new Map<string, string>()

			for (const color of COLORS) {
				fills.set(color, (await measure({ progress: 0.5, max: 1, color })).fillColor)
			}

			// `--color-brand` aliases `--color-green`, so the two are deliberately
			// identical; every other hue must be its own colour rather than
			// collapsing to the default.
			expect(fills.get('brand')).toBe(fills.get('green'))
			const distinct = new Set(
				COLORS.filter((color) => color !== 'brand').map((color) => fills.get(color)),
			)
			expect(distinct.size).toBe(COLORS.length - 1)
		})

		it('follows the theme rather than a literal colour', async () => {
			const perTheme = new Map<string, string>()

			for (const theme of THEMES) {
				perTheme.set(theme, (await measure({ progress: 0.5, max: 1 }, theme)).fillColor)
			}

			// A genuinely token-driven fill changes between the lightest and
			// darkest theme; a hard-coded brand colour would not.
			expect(perTheme.get('light')).not.toBe(perTheme.get('oled'))
		})
	})
})

/** Resolves a token to the concrete colour the browser paints, for comparison. */
function resolveToken(token: string): string {
	const probe = document.createElement('span')
	probe.style.color = computedToken(document.documentElement, token)
	document.body.append(probe)
	const resolved = getComputedStyle(probe).color
	probe.remove()
	return resolved
}
