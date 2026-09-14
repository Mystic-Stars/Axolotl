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
	NewButton as Button,
	Slider,
	ThemeSelector,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { appDataDir, join } from '@tauri-apps/api/path'
import type { DragDropEvent } from '@tauri-apps/api/webview'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import { exists, mkdir, readFile, remove, writeFile } from '@tauri-apps/plugin-fs'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import { getShowScrollTop, setShowScrollTop } from '@/helpers/scroll-top-state'
import { get, set } from '@/helpers/settings.ts'
import { getOS } from '@/helpers/utils'
import { useTheming } from '@/store/state'
import {
	type AccentColor,
	type CloseBehavior,
	type ColorTheme,
	DEFAULT_CUSTOM_ACCENT_COLOR,
	deriveAccentVariants,
	type FeatureFlag,
	hexToHsl,
	type HomeLayout,
	hslToHex,
	parseCustomAccentColor,
} from '@/store/theme.ts'

import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

const props = withDefaults(
	defineProps<{
		scope?: 'interface' | 'home-navigation' | 'content-downloads'
	}>(),
	{ scope: 'interface' },
)

const themeStore = useTheming()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

themeStore.showScrollTop = getShowScrollTop()

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
	accentColorSystem: {
		id: 'app.appearance-settings.accent-color.system',
		defaultMessage: 'Follow system',
	},
	accentColorSystemUnsupported: {
		id: 'app.appearance-settings.accent-color.system-unsupported',
		defaultMessage: 'System unsupported',
	},
	accentColorSystemUnsupportedLabel: {
		id: 'app.appearance-settings.accent-color.system-unsupported-label',
		defaultMessage: 'Follow system: System unsupported',
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
			'Choose or drop a custom image, then fine-tune how it blends with the launcher interface.',
	},
	customBackgroundEmpty: {
		id: 'app.appearance-settings.custom-background.empty',
		defaultMessage: 'No custom background selected',
	},
	customBackgroundChoose: {
		id: 'app.appearance-settings.custom-background.choose',
		defaultMessage: 'Choose image',
	},
	customBackgroundChooseOrDrop: {
		id: 'app.appearance-settings.custom-background.choose-or-drop',
		defaultMessage: 'Click to choose, or drop an image here',
	},
	customBackgroundDropHint: {
		id: 'app.appearance-settings.custom-background.drop-hint',
		defaultMessage: 'Drop image here',
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
	closeBehaviorTitle: {
		id: 'app.appearance-settings.close-behavior.title',
		defaultMessage: 'Choose how to close Axolotl Launcher',
	},
	closeBehaviorDescription: {
		id: 'app.appearance-settings.close-behavior.description',
		defaultMessage: 'Choose whether closing the window exits the launcher or hides it to the tray.',
	},
	closeBehaviorAsk: {
		id: 'app.appearance-settings.close-behavior.ask',
		defaultMessage: 'Ask every time',
	},
	closeBehaviorClose: {
		id: 'app.appearance-settings.close-behavior.close',
		defaultMessage: 'Close directly',
	},
	closeBehaviorLightweight: {
		id: 'app.appearance-settings.close-behavior.lightweight',
		defaultMessage: 'Hide to tray',
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
	showScrollTopTitle: {
		id: 'app.appearance-settings.show-scroll-top.title',
		defaultMessage: 'Show "back to top" button',
	},
	showScrollTopDescription: {
		id: 'app.appearance-settings.show-scroll-top.description',
		defaultMessage: 'Show a floating back-to-top button on scrollable pages.',
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

const isCustomAccent = computed(() => settings.value.accent_color.startsWith('custom:'))
const isSystemAccent = computed(() => settings.value.accent_color === 'system')
const customAccentHex = ref(
	parseCustomAccentColor(settings.value.accent_color) ?? DEFAULT_CUSTOM_ACCENT_COLOR,
)
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

const CUSTOM_BACKGROUND_EXTENSIONS = ['png', 'jpeg', 'jpg', 'webp', 'gif', 'avif', 'bmp']

function isCustomBackgroundImagePath(path: string) {
	const extension = path.split('.').pop()?.toLowerCase() ?? ''
	return CUSTOM_BACKGROUND_EXTENSIONS.includes(extension)
}

async function storeCustomBackgroundBytes(bytes: Uint8Array, extension: string) {
	const backgroundDirectory = await join(await appDataDir(), 'backgrounds')
	const storedPath = await join(
		backgroundDirectory,
		`launcher-background-${Date.now()}.${extension}`,
	)
	const previousPath = settings.value.custom_background_path

	await mkdir(backgroundDirectory, { recursive: true })
	await writeFile(storedPath, bytes)

	settings.value.custom_background_path = storedPath

	if (previousPath && previousPath !== storedPath && (await exists(previousPath))) {
		try {
			await remove(previousPath)
		} catch (error) {
			console.warn('Failed to remove previous custom background', error)
		}
	}
}

async function storeCustomBackgroundFromPath(sourcePath: string) {
	try {
		const extension = sourcePath.split('.').pop()?.toLowerCase() ?? 'png'
		await storeCustomBackgroundBytes(await readFile(sourcePath), extension)
	} catch (error) {
		handleError(error)
	}
}

async function storeCustomBackgroundFromDroppedPath(path: string) {
	try {
		const data = await invoke<ArrayBuffer>('plugin:files|file_read_dragged_file', { path })
		const extension = path.split('.').pop()?.toLowerCase() ?? 'png'
		await storeCustomBackgroundBytes(new Uint8Array(data), extension)
	} catch (error) {
		handleError(error)
	}
}

async function chooseCustomBackground() {
	const selectedPath = await open({
		multiple: false,
		filters: [
			{
				name: 'Image',
				extensions: [...CUSTOM_BACKGROUND_EXTENSIONS],
			},
		],
	})

	if (!selectedPath || Array.isArray(selectedPath)) return

	await storeCustomBackgroundFromPath(selectedPath)
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

const backgroundPreviewRef = ref<HTMLElement | null>(null)
const isBackgroundDragActive = ref(false)
let unlistenNativeBackgroundDrop: (() => void) | null = null

function pointInBackgroundPreview(position: { x: number; y: number }) {
	const rect = backgroundPreviewRef.value?.getBoundingClientRect()
	if (!rect) return false
	const scale = window.devicePixelRatio || 1
	const x = position.x / scale
	const y = position.y / scale
	return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}

async function setupNativeBackgroundDrop() {
	if (props.scope !== 'interface') return

	try {
		unlistenNativeBackgroundDrop = await getCurrentWebview().onDragDropEvent(
			(event: { payload: DragDropEvent }) => {
				const payload = event.payload
				if (payload.type === 'leave') {
					isBackgroundDragActive.value = false
					return
				}

				const path = payload.paths?.find((item) => isCustomBackgroundImagePath(item))
				const inside = pointInBackgroundPreview(payload.position)

				if (payload.type === 'enter' || payload.type === 'over') {
					isBackgroundDragActive.value = Boolean(path && inside)
					return
				}

				if (payload.type === 'drop') {
					isBackgroundDragActive.value = false
					if (path && inside) void storeCustomBackgroundFromDroppedPath(path)
				}
			},
		)
	} catch (error) {
		console.warn('Failed to set up native drop handler for launcher background', error)
	}
}

onMounted(() => {
	void setupNativeBackgroundDrop()
})

onUnmounted(() => {
	unlistenNativeBackgroundDrop?.()
	unlistenNativeBackgroundDrop = null
})

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
			settings.value.close_behavior,
		] as const,
	([
		path,
		blur,
		opacity,
		transparent,
		transparentOpacity,
		transparentBlur,
		sidebarInstanceCount,
		closeBehavior,
	]) => {
		themeStore.customBackgroundPath = path
		themeStore.customBackgroundBlur = blur
		themeStore.customBackgroundOpacity = opacity
		themeStore.transparentBackground = transparent
		themeStore.transparentBackgroundOpacity = transparentOpacity
		themeStore.transparentBackgroundBlur = transparentBlur
		themeStore.setTransparentBackgroundClass()
		themeStore.sidebarInstanceCount = sidebarInstanceCount
		themeStore.closeBehavior = closeBehavior as CloseBehavior
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
	<div class="flex flex-col gap-6">
		<SettingsSection v-if="props.scope === 'interface'">
			<template #header>
				<h2
					id="settings-target-appearance-color-theme"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.colorThemeTitle) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.colorThemeDescription) }}
				</p>
			</template>
			<div class="flex flex-col gap-4 p-4">
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
			</div>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'interface'">
			<template #header>
				<h2
					id="settings-target-appearance-accent-color"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.accentColorTitle) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.accentColorDescription) }}
				</p>
			</template>
			<div class="flex flex-col gap-4 p-4 @container">
				<!-- flex-wrap + per-chip basis: long i18n labels reflow instead of colliding -->
				<div
					class="flex flex-wrap gap-2"
					role="radiogroup"
					:aria-label="formatMessage(messages.accentColorTitle)"
				>
					<button
						v-for="accentColor in accentColorOptions"
						:key="accentColor.value"
						type="button"
						role="radio"
						:aria-checked="settings.accent_color === accentColor.value"
						class="relative flex min-w-0 flex-1 basis-[5.75rem] items-center justify-center gap-2 overflow-hidden rounded-lg border border-solid px-2 py-2.5 @xl:pe-5 @4xl:ps-3 font-semibold transition-all active:scale-[0.97]"
						:class="
							settings.accent_color === accentColor.value
								? 'border-brand bg-brand-highlight text-brand'
								: 'border-surface-4 bg-surface-3 text-secondary hover:border-surface-5 hover:text-contrast'
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
						<span class="hidden min-w-0 truncate @xl:block">{{
							formatMessage(accentColor.label)
						}}</span>
						<CheckIcon
							v-if="settings.accent_color === accentColor.value"
							class="absolute end-2 top-1/2 hidden size-3.5 shrink-0 -translate-y-1/2 @xl:block"
						/>
					</button>
					<button
						type="button"
						role="radio"
						:disabled="themeStore.systemAccentSupported !== true"
						:aria-checked="isSystemAccent"
						:aria-label="
							formatMessage(
								themeStore.systemAccentSupported === false
									? messages.accentColorSystemUnsupportedLabel
									: messages.accentColorSystem,
							)
						"
						class="relative flex min-w-0 flex-1 basis-[8.25rem] items-center justify-center gap-2 overflow-hidden rounded-lg border border-solid px-2 py-2.5 @xl:pe-5 @4xl:ps-3 font-semibold transition-all enabled:active:scale-[0.97]"
						:class="
							themeStore.systemAccentSupported !== true
								? 'cursor-not-allowed border-surface-4 bg-surface-2 text-secondary opacity-60'
								: isSystemAccent
									? 'border-brand bg-brand-highlight text-brand'
									: 'border-surface-4 bg-surface-3 text-secondary hover:border-surface-5 hover:text-contrast'
						"
						@click="
							() => {
								themeStore.setAccentColor('system')
								settings.accent_color = 'system'
							}
						"
					>
						<span
							class="size-4 shrink-0 rounded-full ring-2 ring-white/20"
							:style="{
								backgroundColor: themeStore.systemAccentColor ?? 'var(--color-pink)',
							}"
						/>
						<span class="hidden min-w-0 flex-col text-start leading-tight @xl:flex">
							<span class="truncate">{{ formatMessage(messages.accentColorSystem) }}</span>
							<span
								v-if="themeStore.systemAccentSupported === false"
								class="truncate text-xs font-normal"
							>
								{{ formatMessage(messages.accentColorSystemUnsupported) }}
							</span>
						</span>
						<CheckIcon
							v-if="isSystemAccent"
							class="absolute end-2 top-1/2 hidden size-3.5 shrink-0 -translate-y-1/2 @xl:block"
						/>
					</button>
					<button
						type="button"
						role="radio"
						:aria-checked="isCustomAccent"
						class="relative flex min-w-0 flex-1 basis-[6.75rem] items-center justify-center gap-2 overflow-hidden rounded-lg border border-solid px-2 py-2.5 @xl:pe-5 @4xl:ps-3 font-semibold transition-all active:scale-[0.97]"
						:class="
							isCustomAccent
								? 'border-brand bg-brand-highlight text-brand'
								: 'border-surface-4 bg-surface-3 text-secondary hover:border-surface-5 hover:text-contrast'
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
						<span class="hidden min-w-0 truncate @xl:block">{{
							formatMessage(messages.accentColorCustom)
						}}</span>
						<CheckIcon
							v-if="isCustomAccent"
							class="absolute end-2 top-1/2 hidden size-3.5 shrink-0 -translate-y-1/2 @xl:block"
						/>
					</button>
				</div>

				<div
					v-if="isCustomAccent"
					class="rounded-lg border border-solid border-surface-4 bg-surface-3 p-4"
				>
					<label class="block">
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
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'interface'">
			<template #header>
				<h2
					id="settings-target-appearance-launcher-background"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.customBackgroundTitle) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.customBackgroundDescription) }}
				</p>
			</template>
			<div class="flex flex-col gap-4 p-4 appearance-panel--divided">
				<div
					ref="backgroundPreviewRef"
					class="group relative h-44 cursor-pointer overflow-hidden rounded-lg border border-solid transition-colors"
					:class="
						isBackgroundDragActive ? 'border-brand bg-surface-2' : 'border-surface-4 bg-surface-1'
					"
					role="button"
					tabindex="0"
					:aria-label="formatMessage(messages.customBackgroundChoose)"
					@click="chooseCustomBackground"
					@keydown.enter.prevent="chooseCustomBackground"
					@keydown.space.prevent="chooseCustomBackground"
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
					<div class="absolute inset-0 bg-surface-1/35" />
					<div class="relative flex h-full items-center justify-center">
						<div
							v-if="!customBackgroundPreview"
							class="flex flex-col items-center gap-2 text-secondary"
						>
							<ImageIcon class="size-8" />
							<span class="font-semibold">{{
								isBackgroundDragActive
									? formatMessage(messages.customBackgroundDropHint)
									: formatMessage(messages.customBackgroundEmpty)
							}}</span>
							<span v-if="!isBackgroundDragActive" class="text-sm">
								{{ formatMessage(messages.customBackgroundChooseOrDrop) }}
							</span>
						</div>
						<div
							v-else-if="isBackgroundDragActive"
							class="absolute inset-0 flex items-center justify-center bg-surface-1/70"
						>
							<span class="font-semibold text-contrast">
								{{ formatMessage(messages.customBackgroundDropHint) }}
							</span>
						</div>
						<div
							v-if="customBackgroundPreview && !isBackgroundDragActive"
							class="absolute inset-x-0 bottom-0 flex items-center justify-center gap-2 bg-surface-1/80 p-3 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
						>
							<Button type="base" native-type="button" @click.stop="chooseCustomBackground">
								<UploadIcon />
								{{ formatMessage(messages.customBackgroundReplace) }}
							</Button>
							<Button
								type="outlined"
								color="red"
								native-type="button"
								@click.stop="removeCustomBackground"
							>
								<TrashIcon />
								{{ formatMessage(messages.customBackgroundRemove) }}
							</Button>
						</div>
					</div>
				</div>

				<div v-if="customBackgroundPreview" class="grid gap-5 lg:grid-cols-2">
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
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-transparent-background" tabindex="-1">
						{{ formatMessage(messages.transparentBackgroundTitle) }}
					</span>
				</template>
				<template #description>
					<span class="block">{{ formatMessage(messages.transparentBackgroundDescription) }}</span>
					<span v-if="customBackgroundPreview" class="mt-1 block text-orange">
						{{ formatMessage(messages.transparentBackgroundConflict) }}
					</span>
				</template>
				<template #control>
					<Toggle
						id="transparent-background"
						:model-value="settings.transparent_background"
						@update:model-value="(e) => (settings.transparent_background = !!e)"
					/>
				</template>
			</SettingsRow>
			<SettingsRow v-if="settings.transparent_background" stacked>
				<template #label>{{ formatMessage(messages.transparentBackgroundOpacity) }}</template>
				<template #description>
					{{ formatMessage(messages.transparentBackgroundOpacityDescription) }}
				</template>
				<template #control>
					<div class="w-full">
						<Slider
							id="transparent-background-opacity"
							v-model="settings.transparent_background_opacity"
							:min="0"
							:max="100"
							:step="5"
							unit="%"
						/>
					</div>
				</template>
			</SettingsRow>
			<SettingsRow v-if="settings.transparent_background && os !== 'Linux'">
				<template #label>{{ formatMessage(messages.transparentBackgroundBlurTitle) }}</template>
				<template #description>
					{{ formatMessage(messages.transparentBackgroundBlurDescription) }}
				</template>
				<template #control>
					<Toggle
						id="transparent-background-blur"
						:model-value="settings.transparent_background_blur"
						@update:model-value="(e) => (settings.transparent_background_blur = !!e)"
					/>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'home-navigation'">
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-home-layout" tabindex="-1">
						{{ formatMessage(messages.homeLayoutTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.homeLayoutDescription) }}</template>
				<template #control>
					<div
						class="inline-flex shrink-0 items-center gap-1 rounded-lg border border-solid border-surface-4 bg-surface-3 p-1"
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
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-default-landing-page" tabindex="-1">
						{{ formatMessage(messages.defaultLandingPageTitle) }}
					</span>
				</template>
				<template #description>{{
					formatMessage(messages.defaultLandingPageDescription)
				}}</template>
				<template #control>
					<div class="w-full">
						<Combobox
							id="opening-page"
							v-model="settings.default_page"
							:name="formatMessage(messages.defaultLandingPageTitle)"
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
				</template>
			</SettingsRow>
			<SettingsRow stacked>
				<template #label>
					<span id="settings-target-appearance-sidebar-instance-limit" tabindex="-1">
						{{ formatMessage(messages.sidebarInstanceCountTitle) }}
					</span>
				</template>
				<template #description>{{
					formatMessage(messages.sidebarInstanceCountDescription)
				}}</template>
				<template #control>
					<div class="w-full">
						<Slider
							id="sidebar-instance-count"
							v-model="settings.sidebar_instance_count"
							:min="0"
							:max="50"
							:step="1"
						/>
					</div>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-auto-hide-downloads" tabindex="-1">
						{{ formatMessage(messages.autoHideDownloadsButtonTitle) }}
					</span>
				</template>
				<template #description>
					{{ formatMessage(messages.autoHideDownloadsButtonDescription) }}
				</template>
				<template #control>
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
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-show-play-time" tabindex="-1">
						{{ formatMessage(messages.showPlayTimeTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.showPlayTimeDescription) }}</template>
				<template #control>
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
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-show-scroll-top" tabindex="-1">
						{{ formatMessage(messages.showScrollTopTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.showScrollTopDescription) }}</template>
				<template #control>
					<Toggle
						id="show-scroll-top"
						:model-value="themeStore.showScrollTop"
						@update:model-value="
							(value) => {
								themeStore.showScrollTop = !!value
								setShowScrollTop(themeStore.showScrollTop)
							}
						"
					/>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'interface'">
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-advanced-rendering" tabindex="-1">
						{{ formatMessage(messages.advancedRenderingTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.advancedRenderingDescription) }}</template>
				<template #control>
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
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-page-transitions" tabindex="-1">
						{{ formatMessage(messages.pageTransitionsTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.pageTransitionsDescription) }}</template>
				<template #control>
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
				</template>
			</SettingsRow>
			<SettingsRow v-if="os !== 'MacOS'">
				<template #label>
					<span id="settings-target-appearance-native-decorations" tabindex="-1">
						{{ formatMessage(messages.nativeDecorationsTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.nativeDecorationsDescription) }}</template>
				<template #control>
					<Toggle id="native-decorations" v-model="settings.native_decorations" />
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-close-behavior" tabindex="-1">
						{{ formatMessage(messages.closeBehaviorTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.closeBehaviorDescription) }}</template>
				<template #control>
					<Combobox
						id="close-behavior"
						v-model="settings.close_behavior"
						:name="formatMessage(messages.closeBehaviorTitle)"
						:options="[
							{ value: 'ask', label: formatMessage(messages.closeBehaviorAsk) },
							{ value: 'close', label: formatMessage(messages.closeBehaviorClose) },
							{ value: 'lightweight', label: formatMessage(messages.closeBehaviorLightweight) },
						]"
						@update:model-value="(value) => (themeStore.closeBehavior = value as CloseBehavior)"
					/>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-hide-nametag" tabindex="-1">
						{{ formatMessage(messages.hideNametagTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.hideNametagDescription) }}</template>
				<template #control>
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
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'content-downloads'">
			<SettingsRow>
				<template #label>
					<span id="settings-target-content-auto-install-dependencies" tabindex="-1">
						{{ formatMessage(messages.autoInstallDependenciesTitle) }}
					</span>
				</template>
				<template #description>
					{{ formatMessage(messages.autoInstallDependenciesDescription) }}
				</template>
				<template #control>
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
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-appearance-unknown-pack-warning" tabindex="-1">
						{{ formatMessage(messages.unknownPackWarningTitle) }}
					</span>
				</template>
				<template #description>{{
					formatMessage(messages.unknownPackWarningDescription)
				}}</template>
				<template #control>
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
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-content-skip-nonessential-warnings" tabindex="-1">
						{{ formatMessage(messages.skipNonEssentialWarningsTitle) }}
					</span>
				</template>
				<template #description>
					{{ formatMessage(messages.skipNonEssentialWarningsDescription) }}
				</template>
				<template #control>
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
				</template>
			</SettingsRow>
		</SettingsSection>
	</div>
</template>

<style scoped lang="scss">
.appearance-panel--divided {
	border-bottom: 1px solid var(--settings-divider, var(--surface-4));
}

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
		border: 0.1875rem solid var(--surface-4);
		box-shadow: var(--shadow-button);
	}

	&::-moz-range-thumb {
		width: 1.25rem;
		height: 1.25rem;
		border-radius: 50%;
		background: var(--color-brand);
		border: 0.1875rem solid var(--surface-4);
		box-shadow: var(--shadow-button);
	}
}
</style>
