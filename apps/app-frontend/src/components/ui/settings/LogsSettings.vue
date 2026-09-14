<script setup lang="ts">
import { FolderOpenIcon, UploadIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { get, getUpdateChannel, set } from '@/helpers/settings.ts'
import { showLauncherLogsFolder } from '@/helpers/utils'

import LogExportModal from './LogExportModal.vue'
import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const [initialSettings, updateChannel] = await Promise.all([get(), getUpdateChannel()])
const settings = ref(initialSettings)
const isBeta = updateChannel === 'beta'
const exportModal = ref<InstanceType<typeof LogExportModal>>()

const messages = defineMessages({
	levelTitle: { id: 'app.settings.logs.level.title', defaultMessage: 'Log level' },
	levelDescription: {
		id: 'app.settings.logs.level.description',
		defaultMessage: 'Choose the lowest level written to the launcher log files.',
	},
	levelHint: {
		id: 'app.settings.logs.level.hint',
		defaultMessage:
			'Info is the default. Debug and Trace record more detail and create larger log files.',
	},
	levelHintBeta: {
		id: 'app.settings.logs.level.hint-beta',
		defaultMessage:
			'Beta uses Debug by default and only allows Debug or Trace so diagnostics retain enough detail.',
	},
	levelError: { id: 'app.settings.logs.level.error', defaultMessage: 'Error' },
	levelWarn: { id: 'app.settings.logs.level.warn', defaultMessage: 'Warning' },
	levelInfo: { id: 'app.settings.logs.level.info', defaultMessage: 'Info' },
	levelDebug: { id: 'app.settings.logs.level.debug', defaultMessage: 'Debug' },
	levelTrace: { id: 'app.settings.logs.level.trace', defaultMessage: 'Trace (everything)' },
	retentionTitle: {
		id: 'app.settings.logs.retention.title',
		defaultMessage: 'Automatic log retention',
	},
	retentionDescription: {
		id: 'app.settings.logs.retention.description',
		defaultMessage:
			'The last 30 minutes keep every level. Trace entries are dropped after 30 minutes, and debug entries after 2 hours. Warnings and errors stay for up to 3 days.',
	},
	exportTitle: { id: 'app.settings.logs.export.title', defaultMessage: 'Export logs' },
	exportDescription: {
		id: 'app.settings.logs.export.description',
		defaultMessage:
			'Package launcher logs with an optional environment summary, instance log, and crash analysis.',
	},
	exportButton: { id: 'app.settings.logs.export.button', defaultMessage: 'Export logs…' },
	openFolder: { id: 'app.settings.logs.open-folder', defaultMessage: 'Open logs folder' },
})

const logLevelOptions = computed(() => {
	const verboseOptions = [
		{ value: 'debug', label: formatMessage(messages.levelDebug) },
		{ value: 'trace', label: formatMessage(messages.levelTrace) },
	]
	if (isBeta) return verboseOptions

	return [
		{ value: 'error', label: formatMessage(messages.levelError) },
		{ value: 'warn', label: formatMessage(messages.levelWarn) },
		{ value: 'info', label: formatMessage(messages.levelInfo) },
		...verboseOptions,
	]
})

const levelHint = computed(() =>
	formatMessage(isBeta ? messages.levelHintBeta : messages.levelHint),
)

watch(
	settings,
	async () => {
		await set(settings.value)
	},
	{ deep: true },
)

async function openLogsFolder() {
	try {
		await showLauncherLogsFolder()
	} catch (error) {
		handleError(error)
	}
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<SettingsSection>
			<template #header>
				<h2
					id="settings-target-logs-level"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.levelTitle) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.levelDescription) }}
				</p>
			</template>
			<SettingsRow stacked>
				<template #label>
					<span>{{ formatMessage(messages.levelTitle) }}</span>
				</template>
				<template #description>{{ levelHint }}</template>
				<template #control>
					<div class="w-full">
						<Combobox
							id="log-level"
							v-model="settings.log_level"
							:name="formatMessage(messages.levelTitle)"
							:options="logLevelOptions"
						/>
					</div>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection>
			<template #header>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.retentionTitle) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.retentionDescription) }}
				</p>
			</template>
		</SettingsSection>

		<SettingsSection>
			<template #header>
				<h2
					id="settings-target-logs-export"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.exportTitle) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.exportDescription) }}
				</p>
			</template>
			<SettingsRow stacked>
				<template #label>
					<span>{{ formatMessage(messages.exportTitle) }}</span>
				</template>
				<template #description>{{ formatMessage(messages.exportDescription) }}</template>
				<template #control>
					<div class="flex flex-wrap gap-2">
						<ButtonStyled color="brand">
							<button type="button" @click="exportModal?.show()">
								<UploadIcon />
								{{ formatMessage(messages.exportButton) }}
							</button>
						</ButtonStyled>
						<ButtonStyled>
							<button type="button" @click="openLogsFolder">
								<FolderOpenIcon />
								{{ formatMessage(messages.openFolder) }}
							</button>
						</ButtonStyled>
					</div>
				</template>
			</SettingsRow>
		</SettingsSection>

		<LogExportModal ref="exportModal" />
	</div>
</template>
