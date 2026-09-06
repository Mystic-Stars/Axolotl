<script setup lang="ts">
import { WrenchIcon } from '@modrinth/assets'
import {
	defineMessages,
	injectNotificationManager,
	NewButton as Button,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { inject, ref, watch } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { isDev } from '@/helpers/utils'
import { handleSevereError } from '@/store/error.js'
import { useTheming } from '@/store/state'
import { DEFAULT_FEATURE_FLAGS, type FeatureFlag } from '@/store/theme.ts'

import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

const themeStore = useTheming()
const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const isDevEnvironment = await isDev()
const previewMinecraftCrashModal = inject<() => void>('previewMinecraftCrashModal')
const previewPrivacyConsentModal = inject<() => Promise<void>>('previewPrivacyConsentModal')
const previewRemoteAnnouncement = inject<(type: 'modal' | 'notification', withAction: boolean) => void>('previewRemoteAnnouncement')
const previewWithAction = ref(false)
const messages = defineMessages({
	announcementPreview: {
		id: 'app.settings.developer.announcement-preview',
		defaultMessage: 'Announcement preview',
	},
	announcementPreviewDescription: {
		id: 'app.settings.developer.announcement-preview-description',
		defaultMessage: 'Preview the real announcement components using local sample content, without fetching or marking real announcements as read.',
	},
	previewWithAction: {
		id: 'app.settings.developer.preview-with-action',
		defaultMessage: 'Include an optional external-link button',
	},
	previewModal: {
		id: 'app.settings.developer.preview-announcement-modal',
		defaultMessage: 'Preview startup modal',
	},
	previewPopup: {
		id: 'app.settings.developer.preview-announcement-popup',
		defaultMessage: 'Preview Popup notification',
	},
	resetToDefault: {
		id: 'app.settings.feature-flags.reset-to-default',
		defaultMessage: 'Reset to default',
	},
	developerTools: {
		id: 'app.settings.about.developer-tools',
		defaultMessage: 'Developer tools',
	},
	testError: {
		id: 'app.settings.about.test-error',
		defaultMessage: 'Trigger test error',
	},
	testErrorMessage: {
		id: 'app.settings.about.test-error-message',
		defaultMessage: 'Test error triggered from the development settings.',
	},
	testNotificationError: {
		id: 'app.settings.about.test-notification-error',
		defaultMessage: 'Trigger notification test error',
	},
	testNotificationErrorTitle: {
		id: 'app.settings.about.test-notification-error-title',
		defaultMessage: 'Test notification error',
	},
	previewMinecraftCrashModal: {
		id: 'app.settings.about.preview-minecraft-crash-modal',
		defaultMessage: 'Preview Minecraft crash window',
	},
	previewPrivacyConsentModal: {
		id: 'app.settings.about.preview-privacy-consent-modal',
		defaultMessage: 'Preview privacy & security modal',
	},
})

const settings = ref(await getSettings())
const options = ref<FeatureFlag[]>(Object.keys(DEFAULT_FEATURE_FLAGS))

function setFeatureFlag(key: string, value: boolean) {
	themeStore.featureFlags[key] = value
	settings.value.feature_flags[key] = value
}

function triggerTestError() {
	handleSevereError(new Error(formatMessage(messages.testErrorMessage)))
}

function triggerTestNotificationError() {
	addNotification({
		title: formatMessage(messages.testNotificationErrorTitle),
		text: formatMessage(messages.testErrorMessage),
		type: 'error',
	})
}

watch(
	settings,
	async () => {
		await setSettings(settings.value)
	},
	{ deep: true },
)
</script>
<template>
	<SettingsSection>
		<SettingsRow v-for="option in options" :key="option">
			<template #label>{{ option.replaceAll('_', ' ') }}</template>
			<template #control>
				<div class="flex items-center gap-2">
					<Button
						type="quiet"
						:disabled="themeStore.getFeatureFlag(option) === DEFAULT_FEATURE_FLAGS[option]"
						@click="setFeatureFlag(option, DEFAULT_FEATURE_FLAGS[option])"
					>
						{{ formatMessage(messages.resetToDefault) }}
					</Button>
					<Toggle
						:id="`feature-flag-${option}`"
						:model-value="themeStore.getFeatureFlag(option)"
						@update:model-value="() => setFeatureFlag(option, !themeStore.getFeatureFlag(option))"
					/>
				</div>
			</template>
		</SettingsRow>
	</SettingsSection>

	<SettingsSection v-if="(themeStore.devMode || isDevEnvironment) && previewRemoteAnnouncement">
		<template #header>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.announcementPreview) }}</h2>
		</template>
		<div class="flex flex-col gap-4 p-4">
			<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.announcementPreviewDescription) }}</p>
			<div class="flex items-center gap-2">
				<Toggle id="announcement-preview-action" v-model="previewWithAction" />
				<label for="announcement-preview-action">{{ formatMessage(messages.previewWithAction) }}</label>
			</div>
			<div class="flex flex-wrap gap-2">
				<Button type="base" @click="previewRemoteAnnouncement('modal', previewWithAction)">
					{{ formatMessage(messages.previewModal) }}
				</Button>
				<Button type="base" @click="previewRemoteAnnouncement('notification', previewWithAction)">
					{{ formatMessage(messages.previewPopup) }}
				</Button>
			</div>
		</div>
	</SettingsSection>

	<SettingsSection v-if="isDevEnvironment">
		<template #header>
			<h2 class="m-0 flex items-center gap-2 text-lg font-semibold text-contrast">
				<WrenchIcon class="size-5 text-secondary" />
				{{ formatMessage(messages.developerTools) }}
			</h2>
		</template>
		<div class="flex flex-wrap gap-2 p-4">
			<Button type="base" @click="triggerTestError">
				<WrenchIcon /> {{ formatMessage(messages.testError) }}
			</Button>
			<Button type="base" @click="triggerTestNotificationError">
				<WrenchIcon /> {{ formatMessage(messages.testNotificationError) }}
			</Button>
			<Button type="base" @click="previewMinecraftCrashModal?.()">
				<WrenchIcon /> {{ formatMessage(messages.previewMinecraftCrashModal) }}
			</Button>
			<Button type="base" @click="previewPrivacyConsentModal?.()">
				<WrenchIcon /> {{ formatMessage(messages.previewPrivacyConsentModal) }}
			</Button>
		</div>
	</SettingsSection>
</template>
