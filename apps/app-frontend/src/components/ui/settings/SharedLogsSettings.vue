<script setup lang="ts">
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { onMounted, ref } from 'vue'

import { delete_shared_log, list_shared_logs } from '@/helpers/logs'

import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

type SharedLog = {
	id: string
	url: string
	raw: string
	token: string
	provider: string
	instance_id: string | null
	instance_name: string | null
	truncated: boolean
	created_at: number
}

const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()
const logs = ref<SharedLog[]>([])
const loading = ref(true)
const deletingId = ref<string | null>(null)

const messages = defineMessages({
	title: { id: 'app.log-share.shared-logs.title', defaultMessage: 'Shared logs' },
	description: {
		id: 'app.log-share.shared-logs.description',
		defaultMessage:
			'Every diagnostic you have shared is listed here. Delete an entry to remove it from your history and request its removal from the sharing service.',
	},
	empty: { id: 'app.log-share.shared-logs.empty', defaultMessage: 'No shared logs yet.' },
	delete: { id: 'app.log-share.shared-logs.delete', defaultMessage: 'Delete' },
	deleting: { id: 'app.log-share.shared-logs.deleting', defaultMessage: 'Deleting…' },
	deleted: {
		id: 'app.log-share.shared-logs.deleted',
		defaultMessage: 'Shared log removed.',
	},
	deleteFailed: {
		id: 'app.log-share.shared-logs.delete-failed',
		defaultMessage: 'Failed to delete the shared log.',
	},
	truncated: { id: 'app.log-share.shared-logs.truncated', defaultMessage: 'Truncated' },
})

async function refresh(): Promise<void> {
	try {
		logs.value = await list_shared_logs()
	} catch (error) {
		handleError(error)
	} finally {
		loading.value = false
	}
}

async function remove(log: SharedLog): Promise<void> {
	if (deletingId.value) return
	deletingId.value = log.id
	try {
		await delete_shared_log(log.id, log.token)
		logs.value = logs.value.filter((item) => item.id !== log.id)
		addNotification({ title: formatMessage(messages.deleted), type: 'success' })
	} catch (error) {
		handleError(error)
		addNotification({ title: formatMessage(messages.deleteFailed), type: 'error' })
	} finally {
		deletingId.value = null
	}
}

function formatDate(timestamp: number): string {
	return new Date(timestamp * 1000).toLocaleString()
}

onMounted(refresh)
</script>

<template>
	<SettingsSection
		id="settings-target-shared-logs"
		:title="formatMessage(messages.title)"
		:description="formatMessage(messages.description)"
	>
		<SettingsRow stacked>
			<template #control>
				<div class="flex min-w-0 flex-col gap-2">
					<p v-if="loading" class="m-0 p-2 text-sm text-secondary">…</p>
					<p
						v-else-if="!logs.length"
						class="m-0 p-2 text-sm text-secondary"
					>
						{{ formatMessage(messages.empty) }}
					</p>
					<ul
						v-else
						class="m-0 flex max-h-72 list-none flex-col gap-1 overflow-y-auto p-2"
					>
						<li
							v-for="log in logs"
							:key="log.id"
							class="flex items-center gap-3 rounded-lg bg-surface-2 px-3 py-2"
						>
							<a
								:href="log.url || `https://logshare.cn/${log.id}`"
								target="_blank"
								rel="noopener noreferrer"
								class="min-w-0 flex-1 truncate text-primary underline"
							>
								{{ log.url || log.id }}
							</a>
							<span class="shrink-0 text-xs text-secondary">
								{{ log.provider }}
								<template v-if="log.truncated">
									· {{ formatMessage(messages.truncated) }}
								</template>
								<template v-if="log.instance_name">
									· {{ log.instance_name }}
								</template>
								· {{ formatDate(log.created_at) }}
							</span>
							<ButtonStyled type="outlined">
								<button
									:disabled="deletingId === log.id"
									@click="remove(log)"
								>
									{{
										deletingId === log.id
											? formatMessage(messages.deleting)
											: formatMessage(messages.delete)
									}}
								</button>
							</ButtonStyled>
						</li>
					</ul>
				</div>
			</template>
		</SettingsRow>
	</SettingsSection>
</template>