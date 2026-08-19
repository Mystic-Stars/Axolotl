<script setup lang="ts">
import { defineMessages, StyledInput, Toggle, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { get, set } from '@/helpers/settings.ts'

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
	<div>
		<div class="flex flex-col gap-6">
			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.fullscreen) }}
					</h3>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.fullscreenDescription) }}
					</p>
				</div>

				<Toggle id="fullscreen" v-model="settings.force_fullscreen" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.width) }}
					</h3>
					<p class="m-0 leading-tight">{{ formatMessage(messages.widthDescription) }}</p>
				</div>

				<StyledInput
					id="width"
					v-model="settings.game_resolution[0]"
					:disabled="settings.force_fullscreen"
					autocomplete="off"
					type="number"
					:placeholder="formatMessage(messages.widthPlaceholder)"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.height) }}
					</h3>
					<p class="m-0 leading-tight">{{ formatMessage(messages.heightDescription) }}</p>
				</div>

				<StyledInput
					id="height"
					v-model="settings.game_resolution[1]"
					:disabled="settings.force_fullscreen"
					autocomplete="off"
					type="number"
					:placeholder="formatMessage(messages.heightPlaceholder)"
				/>
			</div>
		</div>

		<hr class="my-6 bg-button-border border-none h-[1px]" />

		<div class="flex flex-col gap-6">
			<div class="flex flex-col gap-2.5">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.environmentVariables) }}
				</h2>
				<StyledInput
					id="env-vars"
					v-model="settings.envVars"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.environmentVariablesPlaceholder)"
					wrapper-class="w-full"
				/>
			</div>
		</div>

		<hr class="my-6 bg-button-border border-none h-[1px]" />

		<div class="flex flex-col gap-6">
			<div class="flex flex-col gap-2.5">
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.preLaunchHook) }}
				</h3>
				<StyledInput
					id="pre-launch"
					v-model="settings.hooks.pre_launch"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.preLaunchPlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">{{ formatMessage(messages.preLaunchDescription) }}</p>
			</div>

			<div class="flex flex-col gap-2.5">
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.wrapperHook) }}
				</h3>
				<StyledInput
					id="wrapper"
					v-model="settings.hooks.wrapper"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.wrapperPlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">{{ formatMessage(messages.wrapperDescription) }}</p>
			</div>

			<div class="flex flex-col gap-2.5">
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.postExitHook) }}
				</h3>
				<StyledInput
					id="post-exit"
					v-model="settings.hooks.post_exit"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.postExitPlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">{{ formatMessage(messages.postExitDescription) }}</p>
			</div>
		</div>
	</div>
</template>
