<script setup lang="ts">
import { ExternalIcon, ShieldIcon, SpinnerIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NewModal,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ref } from 'vue'

import { type PrivacySettings, savePrivacySettings } from '@/helpers/settings'

const emit = defineEmits<{
	saved: [privacy: PrivacySettings]
}>()

const CONSENT_VERSION = 1
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const modal = ref<InstanceType<typeof NewModal>>()
const telemetry = ref(true)
const discordRpc = ref(true)
const saving = ref(false)

const messages = defineMessages({
	title: {
		id: 'app.privacy-consent.title',
		defaultMessage: 'Privacy & security',
	},
	intro: {
		id: 'app.privacy-consent.intro',
		defaultMessage:
			'Choose what Axolotl may send or display. Nothing is sent until you confirm these choices.',
	},
	telemetry: {
		id: 'app.privacy-consent.telemetry',
		defaultMessage: 'Allow anonymous telemetry',
	},
	telemetryDescription: {
		id: 'app.privacy-consent.telemetry-description',
		defaultMessage:
			'Helps count opted-in installations and daily active users. Full Minecraft logs and account credentials are never uploaded.',
	},
	discordRpc: {
		id: 'app.privacy-consent.discord-rpc',
		defaultMessage: 'Discord Rich Presence',
	},
	discordRpcDescription: {
		id: 'app.privacy-consent.discord-rpc-description',
		defaultMessage:
			'Shows your current launcher or game activity in Discord when Discord is running.',
	},
	privacyPolicy: {
		id: 'app.privacy-consent.privacy-policy',
		defaultMessage: 'Read the privacy policy',
	},
	continue: {
		id: 'app.privacy-consent.continue',
		defaultMessage: 'Save and continue',
	},
})

function show(current: PrivacySettings) {
	telemetry.value = true
	discordRpc.value = current.discord_rpc
	modal.value?.show()
}

async function save() {
	if (saving.value) return
	saving.value = true
	try {
		const privacy = await savePrivacySettings({
			telemetry: telemetry.value,
			discord_rpc: discordRpc.value,
			consent_version: CONSENT_VERSION,
		})
		modal.value?.hide()
		emit('saved', privacy)
	} catch (error) {
		handleError(error)
	} finally {
		saving.value = false
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)" :closable="false" max-width="600px">
		<div class="flex flex-col gap-6">
			<div class="flex items-start gap-3">
				<ShieldIcon class="mt-0.5 size-6 shrink-0 text-brand" />
				<p class="m-0 leading-relaxed text-primary">
					{{ formatMessage(messages.intro) }}
				</p>
			</div>

			<div class="flex items-center justify-between gap-5">
				<div class="min-w-0">
					<label for="consent-telemetry" class="font-semibold text-contrast">
						{{ formatMessage(messages.telemetry) }}
					</label>
					<p class="mb-0 mt-1 text-sm leading-relaxed text-secondary">
						{{ formatMessage(messages.telemetryDescription) }}
					</p>
				</div>
				<Toggle id="consent-telemetry" v-model="telemetry" :disabled="saving" />
			</div>

			<div class="flex items-center justify-between gap-5">
				<div class="min-w-0">
					<label for="consent-discord-rpc" class="font-semibold text-contrast">
						{{ formatMessage(messages.discordRpc) }}
					</label>
					<p class="mb-0 mt-1 text-sm leading-relaxed text-secondary">
						{{ formatMessage(messages.discordRpcDescription) }}
					</p>
				</div>
				<Toggle id="consent-discord-rpc" v-model="discordRpc" :disabled="saving" />
			</div>
		</div>

		<template #actions>
			<div class="flex items-center justify-between gap-4">
				<ButtonStyled type="transparent">
					<button type="button" :disabled="saving" @click="openUrl('https://axlmc.org/privacy')">
						<ExternalIcon />
						{{ formatMessage(messages.privacyPolicy) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button type="button" :disabled="saving" @click="save">
						<SpinnerIcon v-if="saving" class="animate-spin" />
						{{ formatMessage(messages.continue) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
