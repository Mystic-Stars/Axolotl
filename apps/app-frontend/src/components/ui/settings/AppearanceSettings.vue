<script setup lang="ts">
import {
	CheckIcon,
	ImageIcon,
	LayoutTemplateIcon,
	MinimizeIcon,
	TrashIcon,
	UploadIcon,
} from '@modrinth/assets'
import {
	Combobox,
	defineMessages,
	injectNotificationManager,
	type MessageDescriptor,
	Slider,
	ThemeSelector,
	Toggle,
	useVIntl,
	NewButton as Button,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { appDataDir, join } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { exists, mkdir, readFile, remove, writeFile } from '@tauri-apps/plugin-fs'
import { computed, ref, watch } from 'vue'

import { get, set } from '@/helpers/settings.ts'
import { getOS } from '@/helpers/utils'
import { useTheming } from '@/store/state'
import {
	type AccentColor,
	type ColorTheme,
	deriveAccentVariants,
	type FeatureFlag,
	hexToHsl,
	type HomeLayout,
	hslToHex,
	parseCustomAccentColor,
} from '@/store/theme.ts'

const themeStore = useTheming()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const skipNonEssentialWarningsFlag: FeatureFlag = 'skip_non_essential_warnings'
const skipUnknownPackWarningFlag: FeatureFlag = 'skip_unknown_pack_warning'
const showPlayTimeFlag: FeatureFlag = 'show_instance_play_time'
const pageTransitionsFlag: FeatureFlag = 'page_transitions'
const autoInstallDependenciesFlag: FeatureFlag = 'auto_install_dependencies'

const messages = defineMessages({
	colorThemeTitle: {
		id: 'app.appearance-settings.color-theme.title',
		defaultMessage: 'Color theme',
	},
	colorThemeDescription: {
		id: 'app.appearance-settings.color-theme.description',
		defaultMessage: 'Select your preferred color theme for Axolotl Launcher.',
	},
	accentColorTitle: {
		id: 'app.appearance-settings.accent-color.title',
		defaultMessage: 'Accent color',
	},
	accentColorDescription: {
		id: 'app.appearance-settings.accent-color.description',
		defaultMessage: 'Choose the color used for buttons, selections, and highlights.',
	},
	accentColorPink: {
		id: 'app.appearance-settings.accent-color.pink',
		defaultMessage: 'Pink',
	},
	accentColorOrange: {
		id: 'app.appearance-settings.accent-color.orange',
		defaultMessage: 'Orange',
	},
	accentColorGreen: {
		id: 'app.appearance-settings.accent-color.green',
		defaultMessage: 'Green',
	},
	accentColorBlue: {
		id: 'app.appearance-settings.accent-color.blue',
		defaultMessage: 'Blue',
	},
	accentColorPurple: {
		id: 'app.appearance-settings.accent-color.purple',
		defaultMessage: 'Purple',
	},
	accentColorCustom: {
		id: 'app.appearance-settings.accent-color.custom',
		defaultMessage: 'Custom',
	},
	accentColorCustomPalette: {
		id: 'app.appearance-settings.accent-color.custom-palette',
		defaultMessage: 'Preset palette',
	},
	accentColorCustomHue: {
		id: 'app.appearance-settings.accent-color.custom-hue',
		defaultMessage: 'Hue',
	},
	accentColorCustomHex: {
		id: 'app.appearance-settings.accent-color.custom-hex',
		defaultMessage: 'Hex color',
	},
	accentColorCustomPreviewLight: {
		id: 'app.appearance-settings.accent-color.custom-preview-light',
		defaultMessage: 'Light theme',
	},
	accentColorCustomPreviewDark: {
		id: 'app.appearance-settings.accent-color.custom-preview-dark',
		defaultMessage: 'Dark theme',
	},
	customBackgroundTitle: {
		id: 'app.appearance-settings.custom-background.title',
		defaultMessage: 'Launcher background',
	},
	customBackgroundDescription: {
		id: 'app.appearance-settings.custom-background.description',
		defaultMessage:
			'Choose a custom image and fine-tune how it blends with the launcher interface.',
	},
	customBackgroundEmpty: {
		id: 'app.appearance-settings.custom-background.empty',
		defaultMessage: 'No custom background selected',
	},
	customBackgroundChoose: {
		id: 'app.appearance-settings.custom-background.choose',
		defaultMessage: 'Choose image',
	},
	customBackgroundReplace: {
		id: 'app.appearance-settings.custom-background.replace',
		defaultMessage: 'Replace image',
	},
	customBackgroundRemove: {
		id: 'app.appearance-settings.custom-background.remove',
		defaultMessage: 'Remove',
	},
	customBackgroundBlur: {
		id: 'app.appearance-settings.custom-background.blur',
		defaultMessage: 'Background blur',
	},
	customBackgroundBlurDescription: {
		id: 'app.appearance-settings.custom-background.blur-description',
		defaultMessage: 'Soften image details to keep launcher content easy to read.',
	},
	customBackgroundOpacity: {
		id: 'app.appearance-settings.custom-background.opacity',
		defaultMessage: 'Background visibility',
	},
	customBackgroundOpacityDescription: {
		id: 'app.appearance-settings.custom-background.opacity-description',
		defaultMessage: 'Control how strongly the image shows through the interface.',
	},
	transparentBackgroundTitle: {
		id: 'app.appearance-settings.transparent-background.title',
		defaultMessage: 'Transparent background',
	},
	transparentBackgroundDescription: {
		id: 'app.appearance-settings.transparent-background.description',
		defaultMessage: 'Let your desktop show through the launcher window.',
	},
	transparentBackgroundOpacity: {
		id: 'app.appearance-settings.transparent-background.opacity',
		defaultMessage: 'Interface opacity',
	},
	transparentBackgroundOpacityDescription: {
		id: 'app.appearance-settings.transparent-background.opacity-description',
		defaultMessage:
			'Lower values show more of your desktop. Panels stay more solid than the background to keep text readable.',
	},
	transparentBackgroundBlurTitle: {
		id: 'app.appearance-settings.transparent-background.blur-title',
		defaultMessage: 'Background blur',
	},
	transparentBackgroundBlurDescription: {
		id: 'app.appearance-settings.transparent-background.blur-description',
		defaultMessage:
			'Frost what shows through the window. Dragging or resizing may feel less smooth while this is on.',
	},
	transparentBackgroundConflict: {
		id: 'app.appearance-settings.transparent-background.conflict',
		defaultMessage: 'Your custom background image is hidden while this is enabled.',
	},
	advancedRenderingTitle: {
		id: 'app.appearance-settings.advanced-rendering.title',
		defaultMessage: 'Advanced rendering',
	},
	advancedRenderingDescription: {
		id: 'app.appearance-settings.advanced-rendering.description',
		defaultMessage:
			'Enables advanced rendering such as blur effects that may cause performance issues without hardware-accelerated rendering.',
	},
	pageTransitionsTitle: {
		id: 'app.appearance-settings.page-transitions.title',
		defaultMessage: 'Page transition animations',
	},
	pageTransitionsDescription: {
		id: 'app.appearance-settings.page-transitions.description',
		defaultMessage: 'Animate content when switching between launcher pages.',
	},
	autoInstallDependenciesTitle: {
		id: 'app.appearance-settings.auto-install-dependencies.title',
		defaultMessage: 'Automatically install dependencies',
	},
	autoInstallDependenciesDescription: {
		id: 'app.appearance-settings.auto-install-dependencies.description',
		defaultMessage:
			'Download required dependencies when installing content. You can adjust the selection in the confirmation dialog before each install.',
	},
	hideNametagTitle: {
		id: 'app.appearance-settings.hide-nametag.title',
		defaultMessage: 'Hide nametag',
	},
	hideNametagDescription: {
		id: 'app.appearance-settings.hide-nametag.description',
		defaultMessage: 'Disables the nametag above your player on the skins page.',
	},
	nativeDecorationsTitle: {
		id: 'app.appearance-settings.native-decorations.title',
		defaultMessage: 'Native decorations',
	},
	nativeDecorationsDescription: {
		id: 'app.appearance-settings.native-decorations.description',
		defaultMessage: 'Use system window frame (app restart required).',
	},
	minimizeLauncherTitle: {
		id: 'app.appearance-settings.minimize-launcher.title',
		defaultMessage: 'Minimize launcher',
	},
	minimizeLauncherDescription: {
		id: 'app.appearance-settings.minimize-launcher.description',
		defaultMessage: 'Minimize the launcher when a Minecraft process starts.',
	},
	defaultLandingPageTitle: {
		id: 'app.appearance-settings.default-landing-page.title',
		defaultMessage: 'Default landing page',
	},
	defaultLandingPageDescription: {
		id: 'app.appearance-settings.default-landing-page.description',
		defaultMessage: 'Change the page to which the launcher opens on.',
	},
	defaultLandingPageHome: {
		id: 'app.appearance-settings.default-landing-page.home',
		defaultMessage: 'Home',
	},
	defaultLandingPageLibrary: {
		id: 'app.appearance-settings.default-landing-page.library',
		defaultMessage: 'Library',
	},
	defaultLandingPageDiscoverContent: {
		id: 'app.appearance-settings.default-landing-page.discover-content',
		defaultMessage: 'Discover content',
	},
	homeLayoutTitle: {
		id: 'app.appearance-settings.home-layout.title',
		defaultMessage: 'Home layout',
	},
	homeLayoutDescription: {
		id: 'app.appearance-settings.home-layout.description',
		defaultMessage: 'Choose between Information Home and a focused instance launcher.',
	},
	homeLayoutStandard: {
		id: 'app.appearance-settings.home-layout.standard',
		defaultMessage: 'Information',
	},
	homeLayoutMinimal: {
		id: 'app.appearance-settings.home-layout.minimal',
		defaultMessage: 'Minimal',
	},
	selectOption: {
		id: 'app.appearance-settings.select-option',
		defaultMessage: 'Select an option',
	},
	toggleSidebarTitle: {
		id: 'app.appearance-settings.toggle-sidebar.title',
		defaultMessage: 'Toggle sidebar',
	},
	toggleSidebarDescription: {
		id: 'app.appearance-settings.toggle-sidebar.description',
		defaultMessage: 'Enables the ability to toggle the sidebar.',
	},
	unknownPackWarningTitle: {
		id: 'app.appearance-settings.unknown-pack-warning.title',
		defaultMessage: 'Warn me before installing unknown modpacks',
	},
	unknownPackWarningDescription: {
		id: 'app.appearance-settings.unknown-pack-warning.description',
		defaultMessage:
			"If you attempt to install a Modrinth Pack file (.mrpack) that isn't hosted on Modrinth, we'll make sure you understand the risks before installing it.",
	},
	skipNonEssentialWarningsTitle: {
		id: 'app.appearance-settings.skip-non-essential-warnings.title',
		defaultMessage: 'Skip non-essential warnings',
	},
	skipNonEssentialWarningsDescription: {
		id: 'app.appearance-settings.skip-non-essential-warnings.description',
		defaultMessage:
			'Automatically skips low-risk confirmations like duplicate modpack installs, normal content deletion, bulk updates, unlinking modpacks, and repair prompts. Dangerous warnings will still be shown.',
	},
	showPlayTimeTitle: {
		id: 'app.appearance-settings.show-play-time.title',
		defaultMessage: 'Show play time',
	},
	showPlayTimeDescription: {
		id: 'app.appearance-settings.show-play-time.description',
		defaultMessage: `Displays how much time you've spent playing an instance.`,
	},
	sidebarInstanceCountTitle: {
		id: 'app.appearance-settings.sidebar-instance-count.title',
		defaultMessage: 'Sidebar instance limit',
	},
	sidebarInstanceCountDescription: {
		id: 'app.appearance-settings.sidebar-instance-count.description',
		defaultMessage: 'Maximum number of instances to show in the sidebar. Set to 0 to show all.',
	},
	autoHideDownloadsButtonTitle: {
		id: 'app.appearance-settings.auto-hide-downloads-button.title',
		defaultMessage: 'Auto-hide downloads button',
	},
	autoHideDownloadsButtonDescription: {
		id: 'app.appearance-settings.auto-hide-downloads-button.description',
		defaultMessage:
			'Hide the downloads button in the sidebar when there are no active download tasks.',
	},
})

const os = ref(await getOS())
const settings = ref(await get())
const customBackgroundPreview = computed(() =>
	settings.value.custom_background_path
		? convertFileSrc(settings.value.custom_background_path)
		: null,
)

const accentColorOptions: Array<{
	value: AccentColor
	color: string
	label: MessageDescriptor
}> = [
	{ value: 'pink', color: 'var(--color-pink)', label: messages.accentColorPink },
	{ value: 'orange', color: 'var(--color-orange)', label: messages.accentColorOrange },
	{ value: 'green', color: 'var(--color-green)', label: messages.accentColorGreen },
	{ value: 'blue', color: 'var(--color-blue)', label: messages.accentColorBlue },
	{ value: 'purple', color: 'var(--color-purple)', label: messages.accentColorPurple },
]

const CUSTOM_ACCENT_PALETTE = [
	'#ef4444',
	'#f97316',
	'#f59e0b',
	'#84cc16',
	'#22c55e',
	'#14b8a6',
	'#06b6d4',
	'#3b82f6',
	'#6366f1',
	'#a855f7',
	'#ec4899',
	'#f43f5e',
]

const isCustomAccent = computed(() => settings.value.accent_color.startsWith('custom:'))
const customAccentHex = ref(parseCustomAccentColor(settings.value.accent_color) ?? '#db2777')
const customAccentHexInput = ref(customAccentHex.value)
const customAccentHue = computed(() => Math.round(hexToHsl(customAccentHex.value).h))
const customAccentPreview = computed(() => deriveAccentVariants(customAccentHex.value))

function applyCustomAccent(hex: string) {
	const normalized = hex.toLowerCase()
	customAccentHex.value = normalized
	customAccentHexInput.value = normalized
	const value = `custom:${normalized}` as `custom:#${string}`
	themeStore.setAccentColor(value)
	settings.value.accent_color = value
}

function onCustomHueInput(value: string) {
	const { s, l } = hexToHsl(customAccentHex.value)
	applyCustomAccent(hslToHex(Number(value), Math.max(s, 40), l))
}

function onCustomHexInput(value: string) {
	customAccentHexInput.value = value
	const normalized = value.startsWith('#') ? value : `#${value}`
	if (/^#[0-9a-fA-F]{6}$/.test(normalized)) applyCustomAccent(normalized)
}

function setHomeLayout(value: string | number) {
	if (value !== 'standard' && value !== 'minimal') return
	settings.value.home_layout = value as HomeLayout
	themeStore.homeLayout = value as HomeLayout
}

async function chooseCustomBackground() {
	const selectedPath = await open({
		multiple: false,
		filters: [
			{
				name: 'Image',
				extensions: ['png', 'jpeg', 'jpg', 'webp', 'gif', 'avif', 'bmp'],
			},
		],
	})

	if (!selectedPath || Array.isArray(selectedPath)) return

	try {
		const extension = selectedPath.split('.').pop()?.toLowerCase() ?? 'png'
		const backgroundDirectory = await join(await appDataDir(), 'backgrounds')
		const storedPath = await join(backgroundDirectory, `launcher-background.${extension}`)
		const previousPath = settings.value.custom_background_path

		await mkdir(backgroundDirectory, { recursive: true })
		await writeFile(storedPath, await readFile(selectedPath))

		if (previousPath && previousPath !== storedPath && (await exists(previousPath))) {
			await remove(previousPath)
		}

		settings.value.custom_background_path = storedPath
	} catch (error) {
		handleError(error)
	}
}

async function removeCustomBackground() {
	const backgroundPath = settings.value.custom_background_path
	settings.value.custom_background_path = null

	if (!backgroundPath) return

	try {
		if (await exists(backgroundPath)) await remove(backgroundPath)
	} catch (error) {
		handleError(error)
	}
}

watch(
	() =>
		[
			settings.value.custom_background_path,
			settings.value.custom_background_blur,
			settings.value.custom_background_opacity,
			settings.value.transparent_background,
			settings.value.transparent_background_opacity,
			settings.value.transparent_background_blur,
			settings.value.sidebar_instance_count,
		] as const,
	([
		path,
		blur,
		opacity,
		transparent,
		transparentOpacity,
		transparentBlur,
		sidebarInstanceCount,
	]) => {
		themeStore.customBackgroundPath = path
		themeStore.customBackgroundBlur = blur
		themeStore.customBackgroundOpacity = opacity
		themeStore.transparentBackground = transparent
		themeStore.transparentBackgroundOpacity = transparentOpacity
		themeStore.transparentBackgroundBlur = transparentBlur
		themeStore.setTransparentBackgroundClass()
		themeStore.sidebarInstanceCount = sidebarInstanceCount
	},
	{ immediate: true },
)

watch(
	settings,
	async () => {
		await set(settings.value)
	},
	{ deep: true },
)
</script>
<template>
	<h2 class="m-0 text-lg font-semibold text-contrast">
		{{ formatMessage(messages.colorThemeTitle) }}
	</h2>
	<p class="m-0 mt-1">{{ formatMessage(messages.colorThemeDescription) }}</p>

	<ThemeSelector
		:update-color-theme="
			(theme: ColorTheme) => {
				themeStore.setThemeState(theme)
				settings.theme = theme
			}
		"
		:current-theme="settings.theme"
		:theme-options="themeStore.getThemeOptions()"
		system-theme-color="system"
	/>

	<div class="mt-6">
		<h2 class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.accentColorTitle) }}
		</h2>
		<p class="m-0 mt-1">{{ formatMessage(messages.accentColorDescription) }}</p>

		<div
			class="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6"
			role="radiogroup"
			:aria-label="formatMessage(messages.accentColorTitle)"
		>
			<button
				v-for="accentColor in accentColorOptions"
				:key="accentColor.value"
				type="button"
				role="radio"
				:aria-checked="settings.accent_color === accentColor.value"
				class="flex min-w-0 items-center gap-2 rounded-xl border border-solid px-3 py-2.5 font-semibold transition-all active:scale-[0.97]"
				:class="
					settings.accent_color === accentColor.value
						? 'border-brand bg-brand-highlight text-brand'
						: 'border-divider bg-button-bg text-secondary hover:border-surface-5 hover:text-contrast'
				"
				@click="
					() => {
						themeStore.setAccentColor(accentColor.value)
						settings.accent_color = accentColor.value
					}
				"
			>
				<span
					class="size-4 shrink-0 rounded-full ring-2 ring-white/20"
					:style="{ backgroundColor: accentColor.color }"
				/>
				<span class="truncate">{{ formatMessage(accentColor.label) }}</span>
				<CheckIcon
					v-if="settings.accent_color === accentColor.value"
					class="ml-auto size-4 shrink-0"
				/>
			</button>
			<button
				type="button"
				role="radio"
				:aria-checked="isCustomAccent"
				class="flex min-w-0 items-center gap-2 rounded-xl border border-solid px-3 py-2.5 font-semibold transition-all active:scale-[0.97]"
				:class="
					isCustomAccent
						? 'border-brand bg-brand-highlight text-brand'
						: 'border-divider bg-button-bg text-secondary hover:border-surface-5 hover:text-contrast'
				"
				@click="applyCustomAccent(customAccentHex)"
			>
				<span
					class="size-4 shrink-0 rounded-full ring-2 ring-white/20"
					:style="{
						background: isCustomAccent
							? customAccentHex
							: 'conic-gradient(#ef4444, #f59e0b, #22c55e, #06b6d4, #6366f1, #ec4899, #ef4444)',
					}"
				/>
				<span class="truncate">{{ formatMessage(messages.accentColorCustom) }}</span>
				<CheckIcon v-if="isCustomAccent" class="ml-auto size-4 shrink-0" />
			</button>
		</div>

		<div
			v-if="isCustomAccent"
			class="mt-3 rounded-xl border border-solid border-divider bg-button-bg p-4"
		>
			<div
				class="flex flex-wrap gap-2"
				role="group"
				:aria-label="formatMessage(messages.accentColorCustomPalette)"
			>
				<button
					v-for="presetColor in CUSTOM_ACCENT_PALETTE"
					:key="presetColor"
					type="button"
					class="size-7 shrink-0 cursor-pointer rounded-full border-none ring-2 transition-transform hover:scale-110 active:scale-95"
					:class="customAccentHex === presetColor ? 'ring-brand' : 'ring-white/20'"
					:style="{ backgroundColor: presetColor }"
					:aria-label="presetColor"
					@click="applyCustomAccent(presetColor)"
				/>
			</div>

			<label class="mt-4 block">
				<span class="text-sm font-semibold text-contrast">
					{{ formatMessage(messages.accentColorCustomHue) }}
				</span>
				<input
					type="range"
					min="0"
					max="360"
					step="1"
					:value="customAccentHue"
					class="hue-slider mt-2"
					:aria-label="formatMessage(messages.accentColorCustomHue)"
					@input="onCustomHueInput(($event.target as HTMLInputElement).value)"
				/>
			</label>

			<div class="mt-4 flex flex-wrap items-center gap-x-6 gap-y-3">
				<label class="flex items-center gap-2">
					<span class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.accentColorCustomHex) }}
					</span>
					<input
						type="text"
						maxlength="7"
						spellcheck="false"
						:value="customAccentHexInput"
						class="w-28"
						@input="onCustomHexInput(($event.target as HTMLInputElement).value)"
						@blur="customAccentHexInput = customAccentHex"
					/>
				</label>
				<div class="flex items-center gap-2">
					<span
						class="size-6 shrink-0 rounded-full ring-2 ring-white/20"
						:style="{ backgroundColor: customAccentPreview.light }"
					/>
					<span class="text-sm text-secondary">
						{{ formatMessage(messages.accentColorCustomPreviewLight) }}
					</span>
				</div>
				<div class="flex items-center gap-2">
					<span
						class="size-6 shrink-0 rounded-full ring-2 ring-white/20"
						:style="{ backgroundColor: customAccentPreview.dark }"
					/>
					<span class="text-sm text-secondary">
						{{ formatMessage(messages.accentColorCustomPreviewDark) }}
					</span>
				</div>
			</div>
		</div>
	</div>

	<div class="mt-6">
		<h2 class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.customBackgroundTitle) }}
		</h2>
		<p class="m-0 mt-1">{{ formatMessage(messages.customBackgroundDescription) }}</p>

		<div
			class="relative mt-3 h-44 overflow-hidden rounded-2xl border border-solid border-divider bg-bg"
		>
			<div
				v-if="customBackgroundPreview"
				class="absolute -inset-10 bg-cover bg-center"
				:style="{
					backgroundImage: `url(&quot;${customBackgroundPreview}&quot;)`,
					filter: `blur(${settings.custom_background_blur}px)`,
					opacity: settings.custom_background_opacity / 100,
				}"
			/>
			<div class="absolute inset-0 bg-bg/35" />
			<div class="relative flex h-full items-center justify-center">
				<div
					v-if="!customBackgroundPreview"
					class="flex flex-col items-center gap-2 text-secondary"
				>
					<ImageIcon class="size-8" />
					<span class="font-semibold">{{ formatMessage(messages.customBackgroundEmpty) }}</span>
				</div>
			</div>
		</div>

		<div class="mt-3 flex flex-wrap gap-2">
			<Button type="base" @click="chooseCustomBackground" native-type="button">
				<UploadIcon />
				{{
					formatMessage(
						customBackgroundPreview
							? messages.customBackgroundReplace
							: messages.customBackgroundChoose,
					)
				}}
			</Button>
			<Button
				v-if="customBackgroundPreview"
				type="outlined"
				color="red"
				@click="removeCustomBackground"
				native-type="button"
			>
				<TrashIcon />
				{{ formatMessage(messages.customBackgroundRemove) }}
			</Button>
		</div>

		<div v-if="customBackgroundPreview" class="mt-5 grid gap-5 lg:grid-cols-2">
			<div class="flex flex-col gap-2">
				<h3 class="m-0 font-semibold text-contrast">
					{{ formatMessage(messages.customBackgroundBlur) }}
				</h3>
				<Slider
					id="custom-background-blur"
					v-model="settings.custom_background_blur"
					:min="0"
					:max="40"
					:step="1"
					unit="px"
				/>
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.customBackgroundBlurDescription) }}
				</p>
			</div>
			<div class="flex flex-col gap-2">
				<h3 class="m-0 font-semibold text-contrast">
					{{ formatMessage(messages.customBackgroundOpacity) }}
				</h3>
				<Slider
					id="custom-background-opacity"
					v-model="settings.custom_background_opacity"
					:min="10"
					:max="100"
					:step="5"
					unit="%"
				/>
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.customBackgroundOpacityDescription) }}
				</p>
			</div>
		</div>
	</div>

	<div class="mt-6">
		<div class="flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.transparentBackgroundTitle) }}
				</h2>
				<p class="m-0 mt-1">{{ formatMessage(messages.transparentBackgroundDescription) }}</p>
			</div>

			<Toggle
				id="transparent-background"
				:model-value="settings.transparent_background"
				@update:model-value="(e) => (settings.transparent_background = !!e)"
			/>
		</div>

		<div v-if="settings.transparent_background" class="mt-4 flex flex-col gap-2">
			<h3 class="m-0 font-semibold text-contrast">
				{{ formatMessage(messages.transparentBackgroundOpacity) }}
			</h3>
			<Slider
				id="transparent-background-opacity"
				v-model="settings.transparent_background_opacity"
				:min="0"
				:max="100"
				:step="5"
				unit="%"
			/>
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.transparentBackgroundOpacityDescription) }}
			</p>
			<p v-if="customBackgroundPreview" class="m-0 text-sm text-orange">
				{{ formatMessage(messages.transparentBackgroundConflict) }}
			</p>
		</div>

		<div
			v-if="settings.transparent_background && os !== 'Linux'"
			class="mt-4 flex items-center justify-between gap-4"
		>
			<div>
				<h3 class="m-0 font-semibold text-contrast">
					{{ formatMessage(messages.transparentBackgroundBlurTitle) }}
				</h3>
				<p class="m-0 mt-1 text-sm text-secondary">
					{{ formatMessage(messages.transparentBackgroundBlurDescription) }}
				</p>
			</div>

			<Toggle
				id="transparent-background-blur"
				:model-value="settings.transparent_background_blur"
				@update:model-value="(e) => (settings.transparent_background_blur = !!e)"
			/>
		</div>
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.advancedRenderingTitle) }}
			</h2>
			<p class="m-0 mt-1">
				{{ formatMessage(messages.advancedRenderingDescription) }}
			</p>
		</div>

		<Toggle
			id="advanced-rendering"
			:model-value="themeStore.advancedRendering"
			@update:model-value="
				(e) => {
					themeStore.advancedRendering = !!e
					settings.advanced_rendering = themeStore.advancedRendering
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.pageTransitionsTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.pageTransitionsDescription) }}</p>
		</div>

		<Toggle
			id="page-transitions"
			:model-value="themeStore.getFeatureFlag(pageTransitionsFlag)"
			@update:model-value="
				(value) => {
					const enabled = !!value
					themeStore.featureFlags[pageTransitionsFlag] = enabled
					settings.feature_flags[pageTransitionsFlag] = enabled
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.autoInstallDependenciesTitle) }}
			</h2>
			<p class="m-0 mt-1">
				{{ formatMessage(messages.autoInstallDependenciesDescription) }}
			</p>
		</div>
		<Toggle
			id="auto-install-dependencies"
			:model-value="themeStore.getFeatureFlag(autoInstallDependenciesFlag)"
			@update:model-value="
				(value) => {
					const enabled = !!value
					themeStore.featureFlags[autoInstallDependenciesFlag] = enabled
					settings.feature_flags[autoInstallDependenciesFlag] = enabled
				}
			"
		/>
	</div>

	<div v-if="os !== 'MacOS'" class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.nativeDecorationsTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.nativeDecorationsDescription) }}</p>
		</div>
		<Toggle id="native-decorations" v-model="settings.native_decorations" />
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.minimizeLauncherTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.minimizeLauncherDescription) }}</p>
		</div>
		<Toggle id="minimize-launcher" v-model="settings.hide_on_process_start" />
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.showPlayTimeTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.showPlayTimeDescription) }}</p>
		</div>
		<Toggle
			:model-value="themeStore.getFeatureFlag(showPlayTimeFlag)"
			@update:model-value="
				() => {
					const newValue = !themeStore.getFeatureFlag(showPlayTimeFlag)
					themeStore.featureFlags[showPlayTimeFlag] = newValue
					settings.feature_flags[showPlayTimeFlag] = newValue
				}
			"
		/>
	</div>

	<div class="mt-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.homeLayoutTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.homeLayoutDescription) }}</p>
		</div>
		<div
			class="inline-flex shrink-0 items-center gap-1 rounded-xl border border-solid border-divider bg-bg-raised p-1"
			role="radiogroup"
			:aria-label="formatMessage(messages.homeLayoutTitle)"
		>
			<Button
				:type="settings.home_layout === 'standard' ? 'colored-text' : 'quiet'"
				:color="settings.home_layout === 'standard' ? 'brand' : undefined"
				native-type="button"
				role="radio"
				:aria-checked="settings.home_layout === 'standard'"
				@click="setHomeLayout('standard')"
			>
				<LayoutTemplateIcon aria-hidden="true" />
				{{ formatMessage(messages.homeLayoutStandard) }}
			</Button>
			<Button
				:type="settings.home_layout === 'minimal' ? 'colored-text' : 'quiet'"
				:color="settings.home_layout === 'minimal' ? 'brand' : undefined"
				native-type="button"
				role="radio"
				:aria-checked="settings.home_layout === 'minimal'"
				@click="setHomeLayout('minimal')"
			>
				<MinimizeIcon aria-hidden="true" />
				{{ formatMessage(messages.homeLayoutMinimal) }}
			</Button>
		</div>
	</div>

	<div class="mt-6 flex flex-col gap-2">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.sidebarInstanceCountTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.sidebarInstanceCountDescription) }}</p>
		</div>
		<Slider
			id="sidebar-instance-count"
			v-model="settings.sidebar_instance_count"
			:min="0"
			:max="50"
			:step="1"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.autoHideDownloadsButtonTitle) }}
			</h2>
			<p class="m-0 mt-1">
				{{ formatMessage(messages.autoHideDownloadsButtonDescription) }}
			</p>
		</div>
		<Toggle
			id="auto-hide-downloads-button"
			:model-value="themeStore.autoHideDownloadsButton"
			@update:model-value="
				(value) => {
					themeStore.autoHideDownloadsButton = !!value
					settings.auto_hide_downloads_button = themeStore.autoHideDownloadsButton
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.hideNametagTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.hideNametagDescription) }}</p>
		</div>
		<Toggle
			id="hide-nametag-skins-page"
			:model-value="themeStore.hideNametagSkinsPage"
			@update:model-value="
				(e) => {
					themeStore.hideNametagSkinsPage = !!e
					settings.hide_nametag_skins_page = themeStore.hideNametagSkinsPage
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.defaultLandingPageTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.defaultLandingPageDescription) }}</p>
		</div>
		<Combobox
			id="opening-page"
			v-model="settings.default_page"
			name="Opening page dropdown"
			class="max-w-40"
			:placeholder="formatMessage(messages.selectOption)"
			:options="[
				{
					value: 'Home',
					label: formatMessage(messages.defaultLandingPageHome),
				},
				{
					value: 'DiscoverContent',
					label: formatMessage(messages.defaultLandingPageDiscoverContent),
				},
				{
					value: 'Library',
					label: formatMessage(messages.defaultLandingPageLibrary),
				},
			]"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.unknownPackWarningTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.unknownPackWarningDescription) }}</p>
		</div>
		<Toggle
			:model-value="!themeStore.getFeatureFlag(skipUnknownPackWarningFlag)"
			@update:model-value="
				(e) => {
					const warnBeforeUnknownPackInstall = !!e
					const skipUnknownPackWarning = !warnBeforeUnknownPackInstall
					themeStore.featureFlags[skipUnknownPackWarningFlag] = skipUnknownPackWarning
					settings.feature_flags[skipUnknownPackWarningFlag] = skipUnknownPackWarning
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.skipNonEssentialWarningsTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.skipNonEssentialWarningsDescription) }}</p>
		</div>
		<Toggle
			:model-value="themeStore.getFeatureFlag(skipNonEssentialWarningsFlag)"
			@update:model-value="
				() => {
					const newValue = !themeStore.getFeatureFlag(skipNonEssentialWarningsFlag)
					themeStore.featureFlags[skipNonEssentialWarningsFlag] = newValue
					settings.feature_flags[skipNonEssentialWarningsFlag] = newValue
				}
			"
		/>
	</div>
</template>

<style scoped lang="scss">
.hue-slider {
	appearance: none;
	display: block;
	width: 100%;
	height: 0.75rem;
	min-height: 0;
	padding: 0;
	border: none;
	border-radius: var(--radius-max);
	background: linear-gradient(
		to right,
		hsl(0, 80%, 55%),
		hsl(60, 80%, 55%),
		hsl(120, 80%, 55%),
		hsl(180, 80%, 55%),
		hsl(240, 80%, 55%),
		hsl(300, 80%, 55%),
		hsl(360, 80%, 55%)
	);
	cursor: pointer;

	&:focus-visible {
		outline: 2px solid var(--color-focus-ring);
		outline-offset: 2px;
	}

	&::-webkit-slider-thumb {
		appearance: none;
		width: 1.25rem;
		height: 1.25rem;
		border-radius: 50%;
		background: var(--color-brand);
		border: 0.1875rem solid #ffffff;
		box-shadow: var(--shadow-button);
	}

	&::-moz-range-thumb {
		width: 1.25rem;
		height: 1.25rem;
		border-radius: 50%;
		background: var(--color-brand);
		border: 0.1875rem solid #ffffff;
		box-shadow: var(--shadow-button);
	}
}
</style>
