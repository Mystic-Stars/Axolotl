/**
 * Shared setup for the app's browser-mode tests.
 *
 * Mirrors `packages/ui/src/test/visual-harness.ts`, for the same reason: the
 * app loads its design tokens through an entry stylesheet that a component-only
 * mount bypasses, so without importing it here every computed style would be
 * unstyled and an assertion about positioning or colour would pass regardless
 * of whether the component works.
 *
 * The SCSS entry is imported directly because Vite compiles it on import;
 * routing through an intermediate stylesheet would not run the Sass compiler
 * and would silently yield no tokens.
 */
import './visual-styles.scss'

import { mount, type MountingOptions } from '@vue/test-utils'
import type { Component } from 'vue'

export const THEMES = ['light', 'dark', 'oled'] as const
export type Theme = (typeof THEMES)[number]

/** The classes `store/theme.ts` toggles on `<html>` in the app. */
const THEME_CLASSES = THEMES.map((theme) => `${theme}-mode`)

export function applyTheme(theme: Theme): void {
	const root = document.documentElement
	root.classList.remove(...THEME_CLASSES)
	root.classList.add(`${theme}-mode`)
}

/**
 * Confirms the harness itself before any assertion trusts it. If the token
 * layer failed to load, every downstream measurement would be meaningless --
 * and, worse, would still pass.
 */
export function assertTokensLoaded(theme: Theme = 'dark'): void {
	applyTheme(theme)
	const surface = computedToken(document.documentElement, '--surface-2')

	if (!surface) {
		throw new Error(
			'Design tokens did not load. Check that src/test/visual-styles.scss imports @modrinth/assets/omorphia.scss.',
		)
	}
}

/**
 * Mounts a component into the document with a theme applied, so layout and
 * computed styles can be read.
 *
 * The mount is attached to `document.body` on purpose: an overlay that
 * positions itself against the viewport has no meaningful geometry while
 * detached, and the assertion would measure zeroes.
 */
export async function mountThemed<P extends Record<string, unknown>>(
	component: Component,
	props: P,
	theme: Theme,
	options: MountingOptions<P> = {},
) {
	applyTheme(theme)

	const wrapper = mount(component, {
		props,
		attachTo: document.body,
		...options,
	})

	await wrapper.vm.$nextTick()

	return wrapper
}

/** Reads a resolved custom property off an element, for token assertions. */
export function computedToken(element: Element, token: string): string {
	return getComputedStyle(element).getPropertyValue(token).trim()
}

/** Waits for a condition to hold, polling frames, for effects that settle late. */
export async function waitFor(
	predicate: () => boolean,
	{ timeoutMs = 2000, label = 'condition' } = {},
): Promise<void> {
	const deadline = performance.now() + timeoutMs
	while (performance.now() < deadline) {
		if (predicate()) return
		await new Promise((resolve) => requestAnimationFrame(resolve))
	}
	throw new Error(`Timed out after ${timeoutMs}ms waiting for ${label}`)
}
