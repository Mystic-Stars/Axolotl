export type SkinEditorTheme = {
	dark: boolean
	colors: Record<string, string>
	metrics: Record<string, string>
}

const themeMetricMap = {
	'gap-xs': '--gap-xs',
	'gap-sm': '--gap-sm',
	'gap-md': '--gap-md',
	'gap-lg': '--gap-lg',
	'radius-xs': '--radius-xs',
	'radius-sm': '--radius-sm',
	'radius-md': '--radius-md',
} as const

const themeColorMap = {
	ui: '--surface-2',
	back: '--surface-1',
	dark: '--surface-1',
	border: '--surface-5',
	selected: '--surface-3',
	elevated: '--surface-3',
	button: '--surface-4',
	bright_ui: '--surface-4',
	accent: '--color-brand',
	accent_highlight: '--color-brand-highlight',
	focus_ring: '--color-focus-ring',
	hover: '--surface-3',
	frame: '--surface-1',
	text: '--color-text-default',
	light: '--color-text-primary',
	accent_text: '--color-accent-contrast',
	bright_ui_text: '--color-text-primary',
	subtle_text: '--color-text-tertiary',
	grid: '--surface-5',
	wireframe: '--color-text-tertiary',
	checkerboard: '--surface-1-5',
	menu_separator: '--surface-5',
	bright_border: '--surface-5',
} as const

function readVariables(styles: CSSStyleDeclaration, variables: Record<string, string>) {
	return Object.fromEntries(
		Object.entries(variables).map(([name, variable]) => [
			name,
			styles.getPropertyValue(variable).trim(),
		]),
	)
}

export function createSkinEditorTheme(): SkinEditorTheme {
	const root = document.documentElement
	const styles = getComputedStyle(root)
	return {
		dark: root.classList.contains('dark-mode') || root.classList.contains('oled-mode'),
		colors: readVariables(styles, themeColorMap),
		metrics: readVariables(styles, themeMetricMap),
	}
}
