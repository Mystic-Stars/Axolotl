<script setup lang="ts">
import { CheckIcon, ImageIcon, TrashIcon, UploadIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	defineMessages,
	injectNotificationManager,
	type MessageDescriptor,
	Slider,
	ThemeSelector,
	Toggle,
	useVIntl,
	StyledInput,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { appDataDir, join } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { exists, mkdir, readFile, remove, writeFile } from '@tauri-apps/plugin-fs'
import { computed, ref, watch } from 'vue'

import { get, set } from '@/helpers/settings.ts'
import { getOS } from '@/helpers/utils'
import { useTheming } from '@/store/state'
import type { AccentColor, ColorTheme, FeatureFlag } from '@/store/theme.ts'

const themeStore = useTheming()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const worldsInHomeFlag: FeatureFlag = 'worlds_in_home'
const skipNonEssentialWarningsFlag: FeatureFlag = 'skip_non_essential_warnings'
const skipUnknownPackWarningFlag: FeatureFlag = 'skip_unknown_pack_warning'
const showPlayTimeFlag: FeatureFlag = 'show_instance_play_time'

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
	accentColorCustomTitle: {
		id: 'app.appearance-settings.accent-color.custom.title',
		defaultMessage: 'Custom Accent Color',
	},
	accentColorCustomDescription: {
		id: 'app.appearance-settings.accent-color.custom.description',
		defaultMessage: 'Select an accent color',
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
	advancedRenderingTitle: {
		id: 'app.appearance-settings.advanced-rendering.title',
		defaultMessage: 'Advanced rendering',
	},
	advancedRenderingDescription: {
		id: 'app.appearance-settings.advanced-rendering.description',
		defaultMessage:
			'Enables advanced rendering such as blur effects that may cause performance issues without hardware-accelerated rendering.',
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
	selectOption: {
		id: 'app.appearance-settings.select-option',
		defaultMessage: 'Select an option',
	},
	jumpBackIntoWorldsTitle: {
		id: 'app.appearance-settings.jump-back-into-worlds.title',
		defaultMessage: 'Jump back into worlds',
	},
	jumpBackIntoWorldsDescription: {
		id: 'app.appearance-settings.jump-back-into-worlds.description',
		defaultMessage: 'Includes recent worlds in the "Jump back in" section on the Home page.',
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
	{ value: 'custom', color: 'var(--color-custom)', label: messages.accentColorCustom },
]

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
		] as const,
	([path, blur, opacity]) => {
		themeStore.customBackgroundPath = path
		themeStore.customBackgroundBlur = blur
		themeStore.customBackgroundOpacity = opacity
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
			class="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-5"
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
		</div>
		<div v-if="settings.accent_color === 'custom'" class="mt-6 flex items-center justify-between">
			<!-- TODO: -->
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.accentColorCustomTitle) }}
				</h2>
				<p class="m-0 mt-1">
					{{ formatMessage(messages.accentColorCustomDescription) }}
				</p>
			</div>

			<StyledInput
				id="customAccentColor"
				v-model="settings.custom_accent_color"
				autocomplete="off"
				type="color"
				:placeholder="'#ffffff'"
			/>
			<!-- <Toggle
				id="advanced-rendering"
				:model-value="themeStore.advancedRendering"
				@update:model-value="
					(e) => {
						themeStore.advancedRendering = !!e
						settings.advanced_rendering = themeStore.advancedRendering
					}
				"
			/> -->
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
			<ButtonStyled>
				<button type="button" @click="chooseCustomBackground">
					<UploadIcon />
					{{
						formatMessage(
							customBackgroundPreview
								? messages.customBackgroundReplace
								: messages.customBackgroundChoose,
						)
					}}
				</button>
			</ButtonStyled>
			<ButtonStyled v-if="customBackgroundPreview" color="red" type="outlined">
				<button type="button" @click="removeCustomBackground">
					<TrashIcon />
					{{ formatMessage(messages.customBackgroundRemove) }}
				</button>
			</ButtonStyled>
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

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.jumpBackIntoWorldsTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.jumpBackIntoWorldsDescription) }}</p>
		</div>
		<Toggle
			:model-value="themeStore.getFeatureFlag(worldsInHomeFlag)"
			@update:model-value="
				() => {
					const newValue = !themeStore.getFeatureFlag(worldsInHomeFlag)
					themeStore.featureFlags[worldsInHomeFlag] = newValue
					settings.feature_flags[worldsInHomeFlag] = newValue
				}
			"
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

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.toggleSidebarTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.toggleSidebarDescription) }}</p>
		</div>
		<Toggle
			id="toggle-sidebar"
			:model-value="settings.toggle_sidebar"
			@update:model-value="
				(e) => {
					settings.toggle_sidebar = !!e
					themeStore.toggleSidebar = settings.toggle_sidebar
				}
			"
		/>
	</div>
</template>
