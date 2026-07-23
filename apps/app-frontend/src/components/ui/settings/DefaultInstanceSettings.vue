<script setup lang="ts">
import {
	defineMessages,
	Checkbox,
	injectNotificationManager,
	Slider,
	StyledInput,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { ref, watch } from 'vue'

import MemoryAllocationDisplay from '@/components/ui/MemoryAllocationDisplay.vue'
import useMemorySlider from '@/composables/useMemorySlider'
import { get, set } from '@/helpers/settings.ts'

const { handleError } = injectNotificationManager()
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
	memory: { id: 'app.settings.defaults.memory', defaultMessage: 'Memory allocated' },
	memoryDescription: {
		id: 'app.settings.defaults.memory-description',
		defaultMessage: 'The memory allocated to each instance when it is run.',
	},
	automaticMemory: {
		id: 'app.settings.defaults.automatic-memory',
		defaultMessage: 'Automatically allocate memory at launch',
	},
	automaticMemoryDescription: {
		id: 'app.settings.defaults.automatic-memory-description',
		defaultMessage: 'Adjusts memory for each launch based on available RAM and installed mods.',
	},
	javaArguments: {
		id: 'app.settings.defaults.java-arguments',
		defaultMessage: 'Java arguments',
	},
	javaArgumentsPlaceholder: {
		id: 'app.settings.defaults.java-arguments-placeholder',
		defaultMessage: 'Enter Java arguments...',
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
fetchSettings.launchArgs = fetchSettings.extra_launch_args.join(' ')
fetchSettings.envVars = fetchSettings.custom_env_vars.map((x) => x.join('=')).join(' ')

const settings = ref(fetchSettings)

const { maxMemory, snapPoints } = (await useMemorySlider().catch(handleError)) as unknown as {
	maxMemory: number
	snapPoints: number[]
}

watch(
	settings,
	async () => {
		const setSettings = JSON.parse(JSON.stringify(settings.value))

		setSettings.extra_launch_args = setSettings.launchArgs.trim().split(/\s+/).filter(Boolean)
		setSettings.custom_env_vars = setSettings.envVars
			.trim()
			.split(/\s+/)
			.filter(Boolean)
			.map((x) => x.split('=').filter(Boolean))

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
					{{ formatMessage(messages.memory) }}
				</h2>
				<Checkbox
					v-model="settings.memory.automatic"
					:label="formatMessage(messages.automaticMemory)"
				/>
				<Slider
					id="max-memory"
					v-model="settings.memory.maximum"
					:disabled="settings.memory.automatic"
					:min="512"
					:max="maxMemory"
					:step="64"
					:snap-points="snapPoints"
					:snap-range="512"
					unit="MB"
				/>
				<p class="m-0 mt-1 leading-tight">
					{{
						formatMessage(
							settings.memory.automatic
								? messages.automaticMemoryDescription
								: messages.memoryDescription,
						)
					}}
				</p>
				<MemoryAllocationDisplay :memory="settings.memory" show-optimize-button />
			</div>

			<div class="flex flex-col gap-2.5">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.javaArguments) }}
				</h2>
				<StyledInput
					id="java-args"
					v-model="settings.launchArgs"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.javaArgumentsPlaceholder)"
					wrapper-class="w-full"
				/>
			</div>

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
