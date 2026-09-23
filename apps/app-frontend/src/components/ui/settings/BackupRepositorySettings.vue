<script setup lang="ts">
import { FolderIcon } from '@modrinth/assets'
import {
	Admonition,
	Button,
	defineMessages,
	injectFilePicker,
	injectNotificationManager,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { onMounted, onUnmounted, ref } from 'vue'

import {
	type BackupRepositoryStatus,
	getBackupRepositoryStatus,
	listBackupOperations,
	listenBackupProgress,
	moveBackupRepository,
} from '@/helpers/instance-backup'

import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const filePicker = injectFilePicker()
const { handleError } = injectNotificationManager()
const status = ref<BackupRepositoryStatus | null>(null)
const loading = ref(true)
const moving = ref(false)
const movedBytes = ref(0)
const moveTotalBytes = ref(0)
const movingOperationId = ref<string | null>(null)
let unlisten: (() => void) | null = null

const messages = defineMessages({
	title: { id: 'settings.backups.repository.title', defaultMessage: 'Backup repository' },
	description: {
		id: 'settings.backups.repository.description',
		defaultMessage: 'Deduplicated instance backup data and its independent index database.',
	},
	location: { id: 'settings.backups.repository.location', defaultMessage: 'Location' },
	locationDescription: {
		id: 'settings.backups.repository.location-description',
		defaultMessage:
			'Changing the location moves the complete repository. Choose a new or empty folder.',
	},
	change: { id: 'settings.backups.repository.change', defaultMessage: 'Change folder' },
	moving: { id: 'settings.backups.repository.moving', defaultMessage: 'Moving repository...' },
	movingProgress: {
		id: 'settings.backups.repository.moving-progress',
		defaultMessage: 'Moving repository: {processed} of {total}',
	},
	usage: { id: 'settings.backups.repository.usage', defaultMessage: 'Storage usage' },
	usageDescription: {
		id: 'settings.backups.repository.usage-description',
		defaultMessage:
			'{stored} stored for {logical} of backup data across {count, plural, one {# snapshot} other {# snapshots}}.',
	},
	unavailable: {
		id: 'settings.backups.repository.unavailable',
		defaultMessage: 'Repository unavailable',
	},
})

async function refresh() {
	loading.value = true
	try {
		status.value = await getBackupRepositoryStatus()
	} catch (error) {
		handleError(error)
	} finally {
		loading.value = false
	}
}

async function changeLocation() {
	const selection = await filePicker.pickFolder?.()
	if (!selection) return
	moving.value = true
	movedBytes.value = 0
	moveTotalBytes.value = 0
	try {
		movingOperationId.value = await moveBackupRepository(selection.path)
	} catch (error) {
		handleError(error)
		moving.value = false
	}
}

onMounted(async () => {
	unlisten = await listenBackupProgress((event) => {
		if (event.operationType !== 'repository_move') return
		if (movingOperationId.value && event.operationId !== movingOperationId.value) return
		movingOperationId.value = event.operationId
		moving.value = !event.finalState
		movedBytes.value = event.processedBytes
		moveTotalBytes.value = event.totalBytes
		if (event.finalState) void refresh()
	})
	const activeMove = (await listBackupOperations(undefined, true)).find(
		(operation) => operation.operation_type === 'repository_move',
	)
	if (activeMove) {
		movingOperationId.value = activeMove.id
		moving.value = true
		movedBytes.value = activeMove.processed_bytes
		moveTotalBytes.value = activeMove.total_bytes
	}
	await refresh()
})

onUnmounted(() => unlisten?.())
</script>

<template>
	<SettingsSection
		:title="formatMessage(messages.title)"
		:description="formatMessage(messages.description)"
	>
		<SettingsRow>
			<template #label>{{ formatMessage(messages.location) }}</template>
			<template #description>
				<span class="break-all">{{ status?.path ?? '...' }}</span>
				<span class="mt-1 block">{{ formatMessage(messages.locationDescription) }}</span>
			</template>
			<template #control>
				<Button type="outlined" :disabled="loading || moving" @click="changeLocation"
					><FolderIcon />
					{{
						formatMessage(
							moving && moveTotalBytes > 0
								? messages.movingProgress
								: moving
									? messages.moving
									: messages.change,
							{
								processed: formatBytes(movedBytes),
								total: formatBytes(moveTotalBytes),
							},
						)
					}}
				</Button>
			</template>
		</SettingsRow>
		<SettingsRow v-if="status">
			<template #label>{{ formatMessage(messages.usage) }}</template>
			<template #description>
				{{
					formatMessage(messages.usageDescription, {
						stored: formatBytes(status.stored_size),
						logical: formatBytes(status.logical_size),
						count: status.snapshot_count,
					})
				}}
			</template>
		</SettingsRow>
		<Admonition
			v-if="status && !status.available"
			type="critical"
			:header="formatMessage(messages.unavailable)"
		>
			{{ status.error }}
		</Admonition>
	</SettingsSection>
</template>
