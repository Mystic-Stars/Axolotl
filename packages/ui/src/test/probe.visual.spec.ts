import { expect, it } from 'vitest'

import ProgressBar from '../components/base/ProgressBar.vue'
import { applyTheme, mountThemed, THEMES } from './visual-harness'

async function settle(): Promise<void> {
	await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
}

it('probes', async () => {
	const rows: unknown[] = []
	for (const theme of THEMES) {
		applyTheme(theme)
		for (const [progress, max] of [
			[0, 1],
			[0.25, 1],
			[0.5, 1],
			[0.75, 1],
			[1, 1],
			[50, 100],
			[0, 100],
			[100, 100],
		] as const) {
			const wrapper = await mountThemed(ProgressBar, { progress, max }, theme)
			await settle()
			const track = wrapper.element.querySelector('[role="progressbar"]') as HTMLElement
			const fill = track.firstElementChild as HTMLElement
			const trackBox = track.getBoundingClientRect()
			const fillBox = fill.getBoundingClientRect()
			rows.push({
				theme,
				progress,
				max,
				inlineWidth: fill.style.width,
				trackW: trackBox.width,
				fillW: fillBox.width,
				ratio: fillBox.width / trackBox.width,
				trackBg: getComputedStyle(track).backgroundColor,
				fillBg: getComputedStyle(fill).backgroundColor,
				trackH: trackBox.height,
				fillH: fillBox.height,
				fillTransition: getComputedStyle(fill).transition,
			})
			wrapper.unmount()
		}
	}
	throw new Error(`PROBE ${JSON.stringify(rows, null, 1)}`)
})
