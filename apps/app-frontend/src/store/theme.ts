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
	pride_fundraiser: true,
	i18n_debug: false,
	show_instance_play_time: true,
	advanced_filters_collapsed: true,
}

export const THEME_OPTIONS = ['dark', 'light', 'oled', 'system'] as const
export const ACCENT_COLOR_OPTIONS = ['pink', 'orange', 'green', 'blue', 'purple', 'custom'] as const

export type FeatureFlag = keyof typeof DEFAULT_FEATURE_FLAGS
export type FeatureFlags = Record<FeatureFlag, boolean>
export type ColorTheme = (typeof THEME_OPTIONS)[number]
export type AccentColor = (typeof ACCENT_COLOR_OPTIONS)[number]

export type ThemeStore = {
	selectedTheme: ColorTheme
	selectedAccentColor: AccentColor
	advancedRendering: boolean
	hideNametagSkinsPage: boolean
	toggleSidebar: boolean
	customBackgroundPath: string | null
	customBackgroundBlur: number
	customBackgroundOpacity: number

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
		setAccentColor(newAccentColor: AccentColor) {
			if (ACCENT_COLOR_OPTIONS.includes(newAccentColor)) {
				this.selectedAccentColor = newAccentColor
			} else {
				console.warn('Selected accent color is not available.')
			}

			const html = document.documentElement
			for (const accentColor of ACCENT_COLOR_OPTIONS) {
				html.classList.remove(`accent-${accentColor}`)
			}
			html.classList.add(`accent-${this.selectedAccentColor}`)
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
		getFeatureFlag(key: FeatureFlag) {
			return this.featureFlags[key] ?? DEFAULT_FEATURE_FLAGS[key]
		},
		getThemeOptions() {
			return THEME_OPTIONS
		},
	},
})
