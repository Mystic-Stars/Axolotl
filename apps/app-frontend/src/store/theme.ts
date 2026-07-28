import { defineStore } from 'pinia'

let systemThemeMq: MediaQueryList | null = null

export const DEFAULT_FEATURE_FLAGS = {
	project_background: false,
	page_path: false,
	worlds_tab: false,
	worlds_in_home: true,
	server_project_qa: false,
	show_version_environment_column: false,
	server_ram_as_bytes_always_on: false,
	always_show_app_controls: false,
	skip_non_essential_warnings: false,
	skip_unknown_pack_warning: false,
	i18n_debug: false,
	show_instance_play_time: true,
	advanced_filters_collapsed: true,
}

export const THEME_OPTIONS = ['dark', 'light', 'oled', 'system'] as const
export const ACCENT_COLOR_OPTIONS = ['pink', 'orange', 'green', 'blue', 'purple'] as const

export type FeatureFlag = keyof typeof DEFAULT_FEATURE_FLAGS
export type FeatureFlags = Record<FeatureFlag, boolean>
export type ColorTheme = (typeof THEME_OPTIONS)[number]
export type AccentColor = (typeof ACCENT_COLOR_OPTIONS)[number]
export type CustomAccentColor = `custom:#${string}`
export type AccentColorSetting = AccentColor | CustomAccentColor
export type HomeLayout = 'standard' | 'minimal'

/**
 * Extracts the `#rrggbb` hex from a `custom:#rrggbb` accent setting,
 * or returns null if the value is not a valid custom accent.
 */
export function parseCustomAccentColor(value: string): string | null {
	const match = /^custom:(#[0-9a-fA-F]{6})$/.exec(value)
	return match ? match[1].toLowerCase() : null
}

export function hexToHsl(hex: string): { h: number; s: number; l: number } {
	const n = parseInt(hex.slice(1), 16)
	const r = ((n >> 16) & 255) / 255
	const g = ((n >> 8) & 255) / 255
	const b = (n & 255) / 255
	const max = Math.max(r, g, b)
	const min = Math.min(r, g, b)
	const l = (max + min) / 2
	const d = max - min
	const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1))
	let h = 0
	if (d !== 0) {
		if (max === r) h = ((g - b) / d) % 6
		else if (max === g) h = (b - r) / d + 2
		else h = (r - g) / d + 4
		h *= 60
		if (h < 0) h += 360
	}
	return { h, s: s * 100, l: l * 100 }
}

export function hslToHex(h: number, s: number, l: number): string {
	const sn = s / 100
	const ln = l / 100
	const c = (1 - Math.abs(2 * ln - 1)) * sn
	const hp = (((h % 360) + 360) % 360) / 60
	const x = c * (1 - Math.abs((hp % 2) - 1))
	let [r, g, b] = [0, 0, 0]
	if (hp < 1) [r, g, b] = [c, x, 0]
	else if (hp < 2) [r, g, b] = [x, c, 0]
	else if (hp < 3) [r, g, b] = [0, c, x]
	else if (hp < 4) [r, g, b] = [0, x, c]
	else if (hp < 5) [r, g, b] = [x, 0, c]
	else [r, g, b] = [c, 0, x]
	const m = ln - c / 2
	const toHex = (channel: number) =>
		Math.round((channel + m) * 255)
			.toString(16)
			.padStart(2, '0')
	return `#${toHex(r)}${toHex(g)}${toHex(b)}`
}

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max)

/**
 * Derives per-theme accent variants from a single base color, mirroring the
 * relationship between the preset accents' light and dark values: hue is kept,
 * the light variant is clamped dark enough for light surfaces, and the dark
 * variant is lifted ~18 lightness points with a saturation boost.
 */
export function deriveAccentVariants(hex: string): { light: string; dark: string } {
	const { h, s, l } = hexToHsl(hex)
	const lightL = clamp(l, 34, 57)
	return {
		light: hslToHex(h, s, lightL),
		dark: hslToHex(h, Math.min(s * 1.15, 100), Math.min(lightL + 18, 80)),
	}
}

export type ThemeStore = {
	selectedTheme: ColorTheme
	selectedAccentColor: AccentColorSetting
	advancedRendering: boolean
	hideNametagSkinsPage: boolean
	toggleSidebar: boolean
	customBackgroundPath: string | null
	customBackgroundBlur: number
	customBackgroundOpacity: number
	transparentBackground: boolean
	transparentBackgroundOpacity: number
	transparentBackgroundBlur: boolean
	sidebarInstanceCount: number
	autoHideDownloadsButton: boolean
	homeLayout: HomeLayout
	minimalHomeInstanceId: string | null

	devMode: boolean
	featureFlags: FeatureFlags
}

export const DEFAULT_THEME_STORE: ThemeStore = {
	selectedTheme: 'dark',
	selectedAccentColor: 'pink',
	advancedRendering: true,
	hideNametagSkinsPage: false,
	toggleSidebar: false,
	customBackgroundPath: null,
	customBackgroundBlur: 12,
	customBackgroundOpacity: 65,
	transparentBackground: false,
	transparentBackgroundOpacity: 55,
	transparentBackgroundBlur: false,
	sidebarInstanceCount: 0,
	autoHideDownloadsButton: false,
	homeLayout: 'standard',
	minimalHomeInstanceId: null,

	devMode: false,
	featureFlags: DEFAULT_FEATURE_FLAGS,
}

export const useTheming = defineStore('themeStore', {
	state: () => DEFAULT_THEME_STORE,
	actions: {
		setThemeState(newTheme: ColorTheme) {
			if (THEME_OPTIONS.includes(newTheme)) {
				this.selectedTheme = newTheme
			} else {
				console.warn('Selected theme is not present. Check themeOptions.')
			}

			this.setThemeClass()
		},
		setAccentColor(newAccentColor: AccentColorSetting) {
			if (
				parseCustomAccentColor(newAccentColor) !== null ||
				ACCENT_COLOR_OPTIONS.includes(newAccentColor as AccentColor)
			) {
				this.selectedAccentColor = newAccentColor
			} else {
				console.warn('Selected accent color is not available.')
			}

			const html = document.documentElement
			for (const accentColor of ACCENT_COLOR_OPTIONS) {
				html.classList.remove(`accent-${accentColor}`)
			}
			html.classList.remove('accent-custom')

			const customHex = parseCustomAccentColor(this.selectedAccentColor)
			if (customHex) {
				const variants = deriveAccentVariants(customHex)
				html.style.setProperty('--custom-accent-light', variants.light)
				html.style.setProperty('--custom-accent-dark', variants.dark)
				html.classList.add('accent-custom')
			} else {
				html.style.removeProperty('--custom-accent-light')
				html.style.removeProperty('--custom-accent-dark')
				html.classList.add(`accent-${this.selectedAccentColor}`)
			}
		},
		setThemeClass() {
			const html = document.getElementsByTagName('html')[0]
			for (const theme of THEME_OPTIONS) {
				html.classList.remove(`${theme}-mode`)
			}

			systemThemeMq?.removeEventListener('change', this.setThemeClass)
			systemThemeMq = null

			let theme = this.selectedTheme
			if (this.selectedTheme === 'system') {
				systemThemeMq = window.matchMedia('(prefers-color-scheme: dark)')
				systemThemeMq.addEventListener('change', this.setThemeClass)
				theme = systemThemeMq.matches ? 'dark' : 'light'
			}

			html.classList.add(`${theme}-mode`)
		},
		/**
		 * Applies the transparent window mode to the root element. The palette
		 * lives on `body` in `global.scss`; this only supplies the toggle and the
		 * alpha every translucent surface is derived from.
		 */
		setTransparentBackgroundClass() {
			const html = document.documentElement
			html.classList.toggle('transparent-window', this.transparentBackground)
			html.style.setProperty(
				'--transparent-window-alpha',
				`${Math.min(Math.max(this.transparentBackgroundOpacity, 0), 100)}%`,
			)
		},
		getFeatureFlag(key: FeatureFlag) {
			return this.featureFlags[key] ?? DEFAULT_FEATURE_FLAGS[key]
		},
		getThemeOptions() {
			return THEME_OPTIONS
		},
	},
})
