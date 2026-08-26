<script setup lang="ts">
import { defineMessages, StyledInput, Toggle, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { get, set } from '@/helpers/settings.ts'

import CrashAnalysisAISettings from './CrashAnalysisAISettings.vue'
import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	fullscreen: { id: 'app.settings.defaults.fullscreen', defaultMessage: 'Fullscreen' },
	fullscreenDescription: {
		id: 'app.settings.defaults.fullscreen-description',
		defaultMessage: 'Overwrites the options.txt file to start in full screen when launched.',
	},
	width: { id: 'app.settings.defaults.width', defaultMessage: 'Width' },
	widthDescription: {
		id: 'app.settings.defaults.width-description',
		defaultMessage: 'The width of the game window when launched.',
	},
	widthPlaceholder: {
		id: 'app.settings.defaults.width-placeholder',
		defaultMessage: 'Enter width...',
	},
	height: { id: 'app.settings.defaults.height', defaultMessage: 'Height' },
	heightDescription: {
		id: 'app.settings.defaults.height-description',
		defaultMessage: 'The height of the game window when launched.',
	},
	heightPlaceholder: {
		id: 'app.settings.defaults.height-placeholder',
		defaultMessage: 'Enter height...',
	},
	environmentVariables: {
		id: 'app.settings.defaults.environment-variables',
		defaultMessage: 'Environment variables',
	},
	environmentVariablesPlaceholder: {
		id: 'app.settings.defaults.environment-variables-placeholder',
		defaultMessage: 'Enter environment variables...',
	},
	preLaunchHook: {
		id: 'app.settings.defaults.pre-launch-hook',
		defaultMessage: 'Pre-launch hook',
	},
	preLaunchPlaceholder: {
		id: 'app.settings.defaults.pre-launch-placeholder',
		defaultMessage: 'Enter pre-launch command...',
	},
	preLaunchDescription: {
		id: 'app.settings.defaults.pre-launch-description',
		defaultMessage: 'Run before the instance is launched.',
	},
	wrapperHook: { id: 'app.settings.defaults.wrapper-hook', defaultMessage: 'Wrapper hook' },
	wrapperPlaceholder: {
		id: 'app.settings.defaults.wrapper-placeholder',
		defaultMessage: 'Enter wrapper command...',
	},
	wrapperDescription: {
		id: 'app.settings.defaults.wrapper-description',
		defaultMessage: 'Wrapper command for launching Minecraft.',
	},
	postExitHook: { id: 'app.settings.defaults.post-exit-hook', defaultMessage: 'Post-exit hook' },
	postExitPlaceholder: {
		id: 'app.settings.defaults.post-exit-placeholder',
		defaultMessage: 'Enter post-exit command...',
	},
	postExitDescription: {
		id: 'app.settings.defaults.post-exit-description',
		defaultMessage: 'Run after the game closes.',
	},
	lightweightMode: {
		id: 'app.appearance-settings.lightweight-mode.title',
		defaultMessage: 'Enter lightweight mode after launching a game',
	},
	lightweightModeDescription: {
		id: 'app.appearance-settings.lightweight-mode.description',
		defaultMessage:
			'Closes the launcher webview after Minecraft starts to reduce memory use. Restore it from the system tray.',
	},
	minimizeLauncher: {
		id: 'app.appearance-settings.minimize-launcher.title',
		defaultMessage: 'Minimize launcher',
	},
	minimizeLauncherDescription: {
		id: 'app.appearance-settings.minimize-launcher.description',
		defaultMessage: 'Minimize the launcher when a Minecraft process starts.',
	},
})

const fetchSettings = await get()
const settings = ref({
	...fetchSettings,
	envVars: fetchSettings.custom_env_vars.map((x) => x.join('=')).join(' '),
})

watch(
	settings,
	async () => {
		const setSettings = JSON.parse(JSON.stringify(settings.value))

		setSettings.custom_env_vars = setSettings.envVars
			.trim()
			.split(/\s+/)
			.filter(Boolean)
			.map((x: string) => x.split('=').filter(Boolean))

		if (!setSettings.hooks.pre_launch) {
			setSettings.hooks.pre_launch = null
		}
		if (!setSettings.hooks.wrapper) {
			setSettings.hooks.wrapper = null
		}
		if (!setSettings.hooks.post_exit) {
			setSettings.hooks.post_exit = null
		}

		if (!setSettings.custom_dir) {
			setSettings.custom_dir = null
		}

		await set(setSettings)
	},
	{ deep: true },
)
</script>

<template>
	<div class="flex flex-col gap-6">
		<SettingsSection>
			<SettingsRow>
				<template #label>
					<span id="settings-target-defaults-window" tabindex="-1">
						{{ formatMessage(messages.fullscreen) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.fullscreenDescription) }}</template>
				<template #control><Toggle id="fullscreen" v-model="settings.force_fullscreen" /></template>
			</SettingsRow>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.width) }}</template>
				<template #description>{{ formatMessage(messages.widthDescription) }}</template>
				<template #control>
					<StyledInput
						id="width"
						v-model="settings.game_resolution[0]"
						:disabled="settings.force_fullscreen"
						autocomplete="off"
						type="number"
						:placeholder="formatMessage(messages.widthPlaceholder)"
					/>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.height) }}</template>
				<template #description>{{ formatMessage(messages.heightDescription) }}</template>
				<template #control>
					<StyledInput
						id="height"
						v-model="settings.game_resolution[1]"
						:disabled="settings.force_fullscreen"
						autocomplete="off"
						type="number"
						:placeholder="formatMessage(messages.heightPlaceholder)"
					/>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection>
			<SettingsRow stacked>
				<template #label>
					<span id="settings-target-defaults-environment" tabindex="-1">
						{{ formatMessage(messages.environmentVariables) }}
					</span>
				</template>
				<template #control>
					<StyledInput
						id="env-vars"
						v-model="settings.envVars"
						autocomplete="off"
						type="text"
						:placeholder="formatMessage(messages.environmentVariablesPlaceholder)"
						wrapper-class="w-full"
					/>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection>
			<SettingsRow stacked>
				<template #label>
					<span id="settings-target-defaults-launch-hooks" tabindex="-1">
						{{ formatMessage(messages.preLaunchHook) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.preLaunchDescription) }}</template>
				<template #control>
					<StyledInput
						id="pre-launch"
						v-model="settings.hooks.pre_launch"
						autocomplete="off"
						type="text"
						:placeholder="formatMessage(messages.preLaunchPlaceholder)"
						wrapper-class="w-full"
					/>
				</template>
			</SettingsRow>
			<SettingsRow stacked>
				<template #label>{{ formatMessage(messages.wrapperHook) }}</template>
				<template #description>{{ formatMessage(messages.wrapperDescription) }}</template>
				<template #control>
					<StyledInput
						id="wrapper"
						v-model="settings.hooks.wrapper"
						autocomplete="off"
						type="text"
						:placeholder="formatMessage(messages.wrapperPlaceholder)"
						wrapper-class="w-full"
					/>
				</template>
			</SettingsRow>
			<SettingsRow stacked>
				<template #label>{{ formatMessage(messages.postExitHook) }}</template>
				<template #description>{{ formatMessage(messages.postExitDescription) }}</template>
				<template #control>
					<StyledInput
						id="post-exit"
						v-model="settings.hooks.post_exit"
						autocomplete="off"
						type="text"
						:placeholder="formatMessage(messages.postExitPlaceholder)"
						wrapper-class="w-full"
					/>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection>
			<SettingsRow>
				<template #label>
					<span id="settings-target-launch-lightweight-mode" tabindex="-1">
						{{ formatMessage(messages.lightweightMode) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.lightweightModeDescription) }}</template>
				<template #control>
					<Toggle
						id="enter-lightweight-mode-on-game-launch"
						:model-value="settings.enter_lightweight_mode_on_game_launch"
						@update:model-value="
							(value) => {
								settings.enter_lightweight_mode_on_game_launch = !!value
								if (value) settings.hide_on_process_start = false
							}
						"
					/>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-launch-minimize" tabindex="-1">
						{{ formatMessage(messages.minimizeLauncher) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.minimizeLauncherDescription) }}</template>
				<template #control>
					<Toggle
						id="minimize-launcher"
						:model-value="settings.hide_on_process_start"
						:disabled="settings.enter_lightweight_mode_on_game_launch"
						@update:model-value="(value) => (settings.hide_on_process_start = !!value)"
					/>
				</template>
			</SettingsRow>
		</SettingsSection>

		<CrashAnalysisAISettings />
	</div>
</template>
