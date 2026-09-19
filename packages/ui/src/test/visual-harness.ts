/**
 * The app and website load the design tokens through their own entry
 * stylesheets (`omorphia.scss` / `global.scss`), which a component-only mount
 * bypasses. Without these imports a visual assertion would measure unstyled
 * markup and pass regardless of the token values.
 */
import '@modrinth/assets/styles/variables.scss'
import '@modrinth/assets/styles/defaults.scss'

import { mount, type MountingOptions } from '@vue/test-utils'
import type { Component } from 'vue'

export const THEMES = ['light', 'dark', 'oled'] as const
export type Theme = (typeof THEMES)[number]

/** The classes `store/theme.ts` toggles on `<html>` in the desktop app. */
const THEME_CLASSES = THEMES.map((theme) => `${theme}-mode`)

export function applyTheme(theme: Theme): void {
	const root = document.documentElement
	root.classList.remove(...THEME_CLASSES, 'accent-pink')
	root.classList.add(`${theme}-mode`)
}

/**
 * Verifies the harness itself before any assertion trusts it: if the token
 * layer failed to load, every computed style downstream would be meaningless.
 */
export function assertTokensLoaded(): void {
	const surface = getComputedStyle(document.documentElement).getPropertyValue('--surface-2').trim()

	if (!surface) {
		throw new Error(
			'Design tokens did not load. Check that packages/assets/styles/variables.scss is importable from this package.',
		)
	}
}

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

	// Vue applies classes asynchronously; wait for the render to settle before
	// anyone reads layout or computed styles.
	await wrapper.vm.$nextTick()

	return wrapper
}

/** Reads a resolved custom property off an element, for token assertions. */
export function computedToken(element: Element, token: string): string {
	return getComputedStyle(element).getPropertyValue(token).trim()
}
