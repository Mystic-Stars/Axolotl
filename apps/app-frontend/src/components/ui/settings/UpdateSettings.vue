<script setup lang="ts">
import { RefreshCwIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { ref, watch } from 'vue'

import { getUpdateSource, setUpdateSource, type UpdateSource } from '@/helpers/settings.ts'
import { type AppUpdateCheckResult, checkForAppUpdate } from '@/providers/app-update.ts'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const selectedSource = ref<UpdateSource>(getUpdateSource())
const checking = ref(false)
const checkResult = ref<AppUpdateCheckResult | 'failed' | null>(null)

const messages = defineMessages({
	title: {
		id: 'app.settings.updates.title',
		defaultMessage: 'Update source',
	},
	description: {
		id: 'app.settings.updates.description',
		defaultMessage: 'Choose where Axolotl checks for launcher updates.',
	},
	cnb: {
		id: 'app.settings.updates.cnb',
		defaultMessage: 'CNB',
	},
	github: {
		id: 'app.settings.updates.github',
		defaultMessage: 'GitHub',
	},
	check: {
		id: 'app.settings.updates.check',
		defaultMessage: 'Check for updates',
	},
	checking: {
		id: 'app.settings.updates.checking',
		defaultMessage: 'Checking for updates…',
	},
	available: {
		id: 'app.settings.updates.available',
		defaultMessage: 'An update is available.',
	},
	upToDate: {
		id: 'app.settings.updates.up-to-date',
		defaultMessage: 'Axolotl is up to date.',
	},
	disabled: {
		id: 'app.settings.updates.disabled',
		defaultMessage: 'Updates are disabled in this build.',
	},
	offline: {
		id: 'app.settings.updates.offline',
		defaultMessage: 'Connect to the internet to check for updates.',
	},
	failed: {
		id: 'app.settings.updates.failed',
		defaultMessage: 'Could not check for updates.',
	},
	security: {
		id: 'app.settings.updates.security',
		defaultMessage: 'Updates are installed only when their cryptographic signature is valid.',
	},
})

const options: Array<{ value: UpdateSource; label: string }> = [
	{ value: 'cnb', label: formatMessage(messages.cnb) },
	{ value: 'github', label: formatMessage(messages.github) },
]

const resultMessages: Record<AppUpdateCheckResult | 'failed', keyof typeof messages> = {
	available: 'available',
	'up-to-date': 'upToDate',
	disabled: 'disabled',
	offline: 'offline',
	failed: 'failed',
}

watch(selectedSource, (source) => {
	setUpdateSource(source)
	checkResult.value = null
})

async function checkForUpdates() {
	checking.value = true
	checkResult.value = null

	try {
		checkResult.value = await checkForAppUpdate()
	} catch (error) {
		checkResult.value = 'failed'
		handleError(error)
	} finally {
		checking.value = false
	}
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<div class="grid grid-cols-[minmax(0,1fr)_11rem] items-center gap-6">
			<div class="flex min-w-0 flex-col gap-1">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.title) }}
				</h2>
				<p class="m-0 leading-relaxed text-secondary">
					{{ formatMessage(messages.description) }}
				</p>
			</div>
			<div class="w-44">
				<Combobox
					id="update-source"
					v-model="selectedSource"
					name="Update source"
					:options="options"
				/>
			</div>
		</div>

		<div class="flex flex-col items-start gap-3">
			<ButtonStyled color="brand">
				<button :disabled="checking" @click="checkForUpdates">
					<RefreshCwIcon :class="{ 'animate-spin': checking }" />
					{{ formatMessage(checking ? messages.checking : messages.check) }}
				</button>
			</ButtonStyled>
			<p v-if="checkResult" class="m-0 text-sm text-secondary" role="status">
				{{ formatMessage(messages[resultMessages[checkResult]]) }}
			</p>
		</div>

		<p class="m-0 rounded-xl bg-surface-4 p-4 text-sm leading-tight text-secondary">
			{{ formatMessage(messages.security) }}
		</p>
	</div>
</template>
