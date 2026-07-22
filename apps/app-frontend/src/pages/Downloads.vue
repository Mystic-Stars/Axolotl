<template>
	<div class="flex flex-col gap-3 p-6">
		<NavTabs
			:active-index="tab === 'active' ? 0 : 1"
			:links="downloadTabs"
			mode="local"
			@tab-click="selectTab"
		/>

		<div class="flex flex-wrap items-center gap-2">
			<StyledInput
				v-model="query"
				:icon="SearchIcon"
				:placeholder="formatMessage(messages.search)"
				clearable
				wrapper-class="flex-1 min-w-0"
			/>
			<DropdownSelect
				v-model="provider"
				class="!w-44"
				name="download-provider"
				:options="providerOptions"
				:display-name="providerFilterLabel"
			/>
			<DropdownSelect
				v-if="tab === 'history'"
				v-model="historyStatus"
				class="!w-44"
				name="download-status"
				:options="historyStatusOptions"
				:display-name="historyStatusLabel"
			/>
			<ButtonStyled v-if="tab === 'history' && historyJobs.length" class="ml-auto" type="outlined">
				<button @click="clearHistoryModal?.show()">
					<TrashIcon />
					{{ formatMessage(messages.clearHistory) }}
				</button>
			</ButtonStyled>
		</div>

		<div
			v-if="visibleJobs.length || (tab === 'active' && legacyDownloads.length)"
			class="flex flex-col gap-3"
		>
			<Card
				v-for="bar in tab === 'active' ? legacyDownloads : []"
				:key="String(bar.loading_bar_uuid ?? bar.id)"
				class="!p-4"
			>
				<div class="flex items-center gap-3">
					<div
						class="flex size-12 items-center justify-center rounded-xl bg-brand-highlight text-brand"
					>
						<DownloadIcon />
					</div>
					<div class="min-w-0 flex-grow">
						<div class="truncate font-semibold text-contrast">{{ bar.title || bar.message }}</div>
						<div class="truncate text-sm text-secondary">{{ bar.message }}</div>
					</div>
					<TagItem>
						<component :is="providerIcon(legacyProvider(bar))" />
						{{ providerLabel(legacyProvider(bar)) }}
					</TagItem>
					<Badge color="orange" :type="statusLabel('running')" />
				</div>
				<ProgressBar
					class="mt-4"
					full-width
					:progress="legacyPercent(bar)"
					:max="100"
					:label="formatMessage(messages.progress)"
					show-progress
				/>
			</Card>

			<Card v-for="job in visibleJobs" :key="job.job_id" class="!p-0">
				<div class="flex flex-wrap items-center gap-4 p-4">
					<img
						v-if="job.display?.icon"
						:src="displayIcon(job.display.icon)"
						alt=""
						class="size-12 rounded-xl object-cover"
					/>
					<div
						v-else
						class="flex size-12 items-center justify-center rounded-xl bg-brand-highlight text-brand"
					>
						<DownloadIcon />
					</div>
					<div class="min-w-48 flex-grow">
						<div class="flex flex-wrap items-center gap-2">
							<h2 class="m-0 truncate text-lg font-semibold text-contrast">
								{{ jobTitle(job) }}
							</h2>
							<TagItem>
								<component :is="providerIcon(job.provider)" />
								{{ providerLabel(job.provider) }}
							</TagItem>
							<Badge :color="statusColor(job.status)" :type="statusLabel(job.status)" />
							<Badge
								v-if="job.instance_deleted"
								color="gray"
								:type="formatMessage(messages.instanceDeleted)"
							/>
						</div>
						<div class="mt-1 flex flex-wrap items-center gap-2 text-sm text-secondary">
							<span>{{ phaseLabel(job.phase) }}</span>
							<BulletDivider />
							<span>{{ formatDate(job.finished ?? job.modified) }}</span>
							<template v-if="job.instance_id">
								<BulletDivider />
								<span>{{
									formatMessage(messages.instanceTarget, { instance: job.instance_id })
								}}</span>
							</template>
						</div>
						<div
							v-if="downloadTelemetry(job).length"
							class="mt-1 flex flex-wrap items-center gap-2 text-sm text-secondary"
						>
							<template
								v-for="(metric, index) in downloadTelemetry(job)"
								:key="`${index}-${metric}`"
							>
								<BulletDivider v-if="index > 0" />
								<span>{{ metric }}</span>
							</template>
						</div>
					</div>
					<div class="flex flex-wrap items-center gap-2">
						<ButtonStyled v-if="canCancel(job)" color="red" type="outlined" size="small">
							<button :disabled="busy.has(job.job_id)" @click="cancel(job)">
								<XIcon />{{ formatMessage(messages.cancel) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="canRetry(job)" color="brand" size="small">
							<button :disabled="busy.has(job.job_id)" @click="retry(job)">
								<RefreshCwIcon />{{ formatMessage(messages.retry) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="job.error" type="outlined" size="small">
							<button :disabled="busy.has(job.job_id)" @click="copyDiagnostics(job)">
								<ClipboardCopyIcon />{{ formatMessage(messages.copyDiagnostics) }}
							</button>
						</ButtonStyled>
						<ButtonStyled
							v-if="job.instance_id && !job.instance_deleted"
							type="outlined"
							size="small"
						>
							<button @click="router.push(`/instance/${encodeURIComponent(job.instance_id!)}`)">
								<ExternalIcon />{{ formatMessage(messages.openInstance) }}
							</button>
						</ButtonStyled>
						<ButtonStyled type="transparent" size="small">
							<button @click="toggleExpanded(job.job_id)">
								<ChevronDownIcon :class="expanded.has(job.job_id) ? 'rotate-180' : ''" />
								{{
									expanded.has(job.job_id)
										? formatMessage(messages.hideDetails)
										: formatMessage(messages.details)
								}}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="tab === 'history'" circular type="transparent" size="small">
							<button
								v-tooltip="formatMessage(messages.deleteRecord)"
								:aria-label="formatMessage(messages.deleteRecord)"
								:disabled="busy.has(job.job_id)"
								@click="remove(job)"
							>
								<TrashIcon />
							</button>
						</ButtonStyled>
					</div>
				</div>

				<div v-if="showProgress(job)" class="px-4 pb-4">
					<ProgressBar
						full-width
						:progress="jobPercent(job)"
						:max="100"
						:label="progressText(job)"
						:waiting="job.status === 'queued'"
						show-progress
					/>
				</div>

				<div
					v-if="expanded.has(job.job_id)"
					class="border-0 border-t border-solid border-divider p-4"
				>
					<Admonition
						v-if="job.error"
						class="mb-4"
						type="critical"
						:header="formatMessage(messages.errorDetails)"
					>
						{{ job.error.message }}
					</Admonition>
					<Table
						v-if="job.items.length"
						:columns="itemColumns"
						:data="job.items"
						row-key="id"
						table-min-width="42rem"
						virtualized
						class="max-h-80 overflow-y-auto"
					>
						<template #cell-name="{ row }">
							<div class="min-w-0 py-2">
								<div class="truncate font-medium text-contrast">{{ row.name }}</div>
								<div
									v-if="row.project_id && row.version_id"
									class="truncate text-xs text-secondary"
								>
									{{
										formatMessage(messages.projectFile, {
											projectId: row.project_id,
											fileId: row.version_id,
										})
									}}
								</div>
								<div v-if="row.error" class="truncate text-xs text-red">
									{{ itemError(row) }}
								</div>
								<ButtonStyled v-if="row.manual_url" type="transparent" size="small">
									<button class="!px-0" @click.stop="openManualDownload(row)">
										<ExternalIcon />{{ formatMessage(messages.manualDownload) }}
									</button>
								</ButtonStyled>
							</div>
						</template>
						<template #cell-status="{ row }">
							<Badge :color="itemStatusColor(row.status)" :type="statusLabel(row.status)" />
						</template>
						<template #cell-attempts="{ row }">
							<span>{{ itemAttempts(row) }}</span>
						</template>
						<template #cell-progress="{ row }">
							<span>{{ itemProgress(row) }}</span>
						</template>
					</Table>
					<EmptyState
						v-else
						type="no-documents"
						:heading="formatMessage(messages.noFileDetailsTitle)"
						:description="formatMessage(messages.noFileDetails)"
					/>
				</div>
			</Card>
		</div>

		<Card v-else>
			<EmptyState
				:type="query ? 'no-search-result' : 'no-tasks'"
				:heading="formatMessage(query ? messages.noResultsTitle : messages.emptyTitle)"
				:description="
					formatMessage(query ? messages.noResultsDescription : messages.emptyDescription)
				"
			/>
		</Card>
	</div>

	<ConfirmModal
		ref="clearHistoryModal"
		:danger="true"
		:markdown="false"
		:title="formatMessage(messages.clearHistoryTitle)"
		:description="formatMessage(messages.confirmClear)"
		:proceed-label="formatMessage(messages.clearHistory)"
		@proceed="clearHistory"
	/>
</template>

<script setup lang="ts">
import {
	ChevronDownIcon,
	ClipboardCopyIcon,
	ClockIcon,
	CurseForgeIcon,
	DownloadIcon,
	ExternalIcon,
	ModrinthIcon,
	RefreshCwIcon,
	SearchIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Badge,
	BulletDivider,
	ButtonStyled,
	Card,
	ConfirmModal,
	defineMessages,
	DropdownSelect,
	EmptyState,
	injectNotificationManager,
	NavTabs,
	ProgressBar,
	StyledInput,
	Table,
	type TableColumn,
	TagItem,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

import {
	download_job_support_details,
	type InstallJobSnapshot,
	type InstallJobStatus,
	type InstallPhaseId,
} from '@/helpers/install'
import type { LoadingBar } from '@/helpers/state'
import { injectDownloadManager } from '@/providers/download-manager'

type DownloadItem = InstallJobSnapshot['items'][number]

const manager = injectDownloadManager()
const router = useRouter()
const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const tab = ref<'active' | 'history'>('active')
const query = ref('')
const provider = ref('all')
const historyStatus = ref('all')
const expanded = ref(new Set<string>())
const busy = ref(new Set<string>())
const clearHistoryModal = ref<InstanceType<typeof ConfirmModal>>()

const messages = defineMessages({
	inProgress: { id: 'app.downloads.in-progress', defaultMessage: 'In progress' },
	history: { id: 'app.downloads.history', defaultMessage: 'History' },
	search: { id: 'app.downloads.search', defaultMessage: 'Search downloads' },
	allSources: { id: 'app.downloads.all-sources', defaultMessage: 'All sources' },
	allStatuses: { id: 'app.downloads.all-statuses', defaultMessage: 'All statuses' },
	application: { id: 'app.downloads.application', defaultMessage: 'Application' },
	local: { id: 'app.downloads.local', defaultMessage: 'Local' },
	clearHistory: { id: 'app.downloads.clear-history', defaultMessage: 'Clear history' },
	clearHistoryTitle: {
		id: 'app.downloads.clear-history-title',
		defaultMessage: 'Clear download history?',
	},
	cancel: { id: 'app.downloads.cancel', defaultMessage: 'Cancel' },
	retry: { id: 'app.downloads.retry', defaultMessage: 'Retry' },
	copyDiagnostics: { id: 'app.downloads.copy-diagnostics', defaultMessage: 'Copy diagnostics' },
	openInstance: { id: 'app.downloads.open-instance', defaultMessage: 'Open instance' },
	instanceDeleted: { id: 'app.downloads.instance-deleted', defaultMessage: 'Instance deleted' },
	details: { id: 'app.downloads.details', defaultMessage: 'Details' },
	hideDetails: { id: 'app.downloads.hide-details', defaultMessage: 'Hide details' },
	deleteRecord: { id: 'app.downloads.delete-record', defaultMessage: 'Delete record' },
	errorDetails: { id: 'app.downloads.error-details', defaultMessage: 'Download failed' },
	progress: { id: 'app.downloads.progress', defaultMessage: 'Progress' },
	noFileDetailsTitle: {
		id: 'app.downloads.no-file-details-title',
		defaultMessage: 'No file details',
	},
	noFileDetails: {
		id: 'app.downloads.no-file-details',
		defaultMessage: 'No file details were recorded for this download.',
	},
	emptyTitle: { id: 'app.downloads.empty-title', defaultMessage: 'No downloads yet' },
	emptyDescription: {
		id: 'app.downloads.empty-description',
		defaultMessage: 'New downloads and installation progress will appear here.',
	},
	noResultsTitle: { id: 'app.downloads.no-results-title', defaultMessage: 'No matching downloads' },
	noResultsDescription: {
		id: 'app.downloads.no-results-description',
		defaultMessage: 'Try changing your search or filters.',
	},
	confirmClear: {
		id: 'app.downloads.confirm-clear',
		defaultMessage:
			'Completed, failed, interrupted, and canceled records will be permanently deleted.',
	},
	instanceTarget: { id: 'app.downloads.instance-target', defaultMessage: 'Instance: {instance}' },
	notAvailable: { id: 'app.downloads.not-available', defaultMessage: '—' },
	itemName: { id: 'app.downloads.item-name', defaultMessage: 'File' },
	itemStatus: { id: 'app.downloads.item-status', defaultMessage: 'Status' },
	itemAttempts: { id: 'app.downloads.item-attempts', defaultMessage: 'Attempts' },
	attemptProgress: {
		id: 'app.downloads.item-attempt-progress',
		defaultMessage: '{attempt}/{maxAttempts}',
	},
	itemProgress: { id: 'app.downloads.item-progress', defaultMessage: 'Downloaded' },
	manualDownload: {
		id: 'app.curseforge.manual-downloads.open',
		defaultMessage: 'Open',
	},
	manualDownloadRequired: {
		id: 'app.downloads.manual-download-required',
		defaultMessage: 'CurseForge requires this file to be downloaded manually.',
	},
	projectFile: {
		id: 'app.curseforge.manual-downloads.project-file',
		defaultMessage: 'Project {projectId} · File {fileId}',
	},
	downloadSource: { id: 'app.downloads.download-source', defaultMessage: 'Source: {source}' },
	downloadSourceOfficial: { id: 'app.downloads.source.official', defaultMessage: 'Official' },
	downloadSourceBmclapi: { id: 'app.downloads.source.bmclapi', defaultMessage: 'OpenBMCLAPI' },
	downloadSourceMcim: { id: 'app.downloads.source.mcim', defaultMessage: 'MCIM' },
	downloadSourceAlternate: {
		id: 'app.downloads.source.alternate',
		defaultMessage: 'Alternate source',
	},
	downloadSpeed: { id: 'app.downloads.download-speed', defaultMessage: 'Speed: {speed}/s' },
	downloadEtaSeconds: {
		id: 'app.downloads.download-eta-seconds',
		defaultMessage: '{seconds}s remaining',
	},
	downloadEtaMinutes: {
		id: 'app.downloads.download-eta-minutes',
		defaultMessage: '{minutes}m remaining',
	},
	downloadEtaHours: {
		id: 'app.downloads.download-eta-hours',
		defaultMessage: '{hours}h {minutes}m remaining',
	},
	downloadFallbacks: {
		id: 'app.downloads.download-fallbacks',
		defaultMessage: '{count} fallbacks',
	},
})

const statusMessages = defineMessages({
	queued: { id: 'app.downloads.status.queued', defaultMessage: 'Queued' },
	running: { id: 'app.downloads.status.running', defaultMessage: 'Running' },
	canceling: { id: 'app.downloads.status.canceling', defaultMessage: 'Canceling' },
	waiting_for_user: {
		id: 'app.downloads.status.waiting-for-user',
		defaultMessage: 'Action needed',
	},
	succeeded: { id: 'app.downloads.status.succeeded', defaultMessage: 'Completed' },
	failed: { id: 'app.downloads.status.failed', defaultMessage: 'Failed' },
	interrupted: { id: 'app.downloads.status.interrupted', defaultMessage: 'Interrupted' },
	canceled: { id: 'app.downloads.status.canceled', defaultMessage: 'Canceled' },
	completed: { id: 'app.downloads.item-status.completed', defaultMessage: 'Completed' },
	skipped: { id: 'app.downloads.item-status.skipped', defaultMessage: 'Skipped' },
	downloading: { id: 'app.downloads.item-status.downloading', defaultMessage: 'Downloading' },
	verifying: { id: 'app.downloads.item-status.verifying', defaultMessage: 'Verifying' },
	writing: { id: 'app.downloads.item-status.writing', defaultMessage: 'Writing' },
})

const phaseMessages = defineMessages({
	preparing_instance: {
		id: 'app.downloads.phase.preparing-instance',
		defaultMessage: 'Preparing instance',
	},
	resolving_pack: { id: 'app.downloads.phase.resolving-pack', defaultMessage: 'Resolving modpack' },
	downloading_pack_file: {
		id: 'app.downloads.phase.downloading-pack-file',
		defaultMessage: 'Downloading modpack',
	},
	reading_pack_manifest: {
		id: 'app.downloads.phase.reading-pack-manifest',
		defaultMessage: 'Reading manifest',
	},
	downloading_content: {
		id: 'app.downloads.phase.downloading-content',
		defaultMessage: 'Downloading content',
	},
	extracting_overrides: {
		id: 'app.downloads.phase.extracting-overrides',
		defaultMessage: 'Extracting overrides',
	},
	resolving_minecraft: {
		id: 'app.downloads.phase.resolving-minecraft',
		defaultMessage: 'Resolving Minecraft',
	},
	resolving_loader: {
		id: 'app.downloads.phase.resolving-loader',
		defaultMessage: 'Resolving loader',
	},
	preparing_java: { id: 'app.downloads.phase.preparing-java', defaultMessage: 'Preparing Java' },
	downloading_minecraft: {
		id: 'app.downloads.phase.downloading-minecraft',
		defaultMessage: 'Downloading Minecraft',
	},
	running_loader_processors: {
		id: 'app.downloads.phase.running-loader-processors',
		defaultMessage: 'Installing loader',
	},
	finalizing: { id: 'app.downloads.phase.finalizing', defaultMessage: 'Finalizing' },
	rolling_back: { id: 'app.downloads.phase.rolling-back', defaultMessage: 'Rolling back changes' },
})

const legacyDownloads = manager.legacyDownloads
const historyJobs = manager.historyJobs
const providerOptions = [
	'all',
	'modrinth',
	'curse_forge',
	'minecraft',
	'java',
	'application',
	'local',
]
const historyStatusOptions = ['all', 'succeeded', 'failed', 'interrupted', 'canceled']
const downloadTabs = computed(() => [
	{
		href: 'active',
		label: formatMessage(messages.inProgress),
		icon: DownloadIcon,
	},
	{ href: 'history', label: formatMessage(messages.history), icon: ClockIcon },
])
const itemColumns = computed<TableColumn[]>(() => [
	{ key: 'name', label: formatMessage(messages.itemName), width: '48%' },
	{ key: 'status', label: formatMessage(messages.itemStatus), width: '18%' },
	{ key: 'attempts', label: formatMessage(messages.itemAttempts), width: '16%' },
	{ key: 'progress', label: formatMessage(messages.itemProgress), width: '18%', align: 'right' },
])
const sourceJobs = computed(() =>
	tab.value === 'active' ? manager.activeJobs.value : manager.historyJobs.value,
)
const visibleJobs = computed(() => {
	const normalized = query.value.trim().toLowerCase()
	return sourceJobs.value.filter((job) => {
		if (provider.value !== 'all' && job.provider !== provider.value) return false
		if (
			tab.value === 'history' &&
			historyStatus.value !== 'all' &&
			job.status !== historyStatus.value
		)
			return false
		return (
			!normalized ||
			jobTitle(job).toLowerCase().includes(normalized) ||
			job.job_id.includes(normalized)
		)
	})
})
function jobTitle(job: InstallJobSnapshot) {
	return (
		job.display?.title ||
		(job.details.type === 'instance' ? job.details.name : null) ||
		(job.details.type === 'modpack' ? job.details.title : null) ||
		job.job_id
	)
}

function displayIcon(icon: string) {
	return /^(https?:|data:|blob:|asset:|tauri:)/.test(icon) ? icon : convertFileSrc(icon)
}

function providerLabel(value: InstallJobSnapshot['provider']) {
	return {
		modrinth: 'Modrinth',
		curse_forge: 'CurseForge',
		minecraft: 'Minecraft',
		java: 'Java',
		application: formatMessage(messages.application),
		local: formatMessage(messages.local),
	}[value]
}

function providerFilterLabel(value: string) {
	return value === 'all'
		? formatMessage(messages.allSources)
		: providerLabel(value as InstallJobSnapshot['provider'])
}

function providerIcon(value: InstallJobSnapshot['provider']) {
	return value === 'curse_forge'
		? CurseForgeIcon
		: value === 'modrinth'
			? ModrinthIcon
			: DownloadIcon
}

function legacyProvider(bar: LoadingBar): InstallJobSnapshot['provider'] {
	if (bar.bar_type?.type === 'pack_download') return 'curse_forge'
	if (bar.bar_type?.type === 'minecraft_download') return 'minecraft'
	if (bar.bar_type?.type === 'java_download') return 'java'
	if (bar.bar_type?.type === 'launcher_update') return 'application'
	return 'local'
}

function historyStatusLabel(value: string) {
	return value === 'all' ? formatMessage(messages.allStatuses) : statusLabel(value)
}

function statusLabel(status: string) {
	return status in statusMessages
		? formatMessage(statusMessages[status as keyof typeof statusMessages])
		: status
}

function phaseLabel(phase: InstallPhaseId) {
	return formatMessage(phaseMessages[phase])
}

function statusColor(status: InstallJobStatus): 'green' | 'red' | 'orange' | 'blue' | 'gray' {
	if (status === 'succeeded') return 'green'
	if (status === 'failed' || status === 'interrupted' || status === 'canceled') return 'red'
	if (status === 'running' || status === 'waiting_for_user' || status === 'canceling')
		return 'orange'
	return 'blue'
}

function itemStatusColor(
	status: DownloadItem['status'],
): 'green' | 'red' | 'orange' | 'blue' | 'gray' {
	if (status === 'completed') return 'green'
	if (status === 'failed' || status === 'canceled') return 'red'
	if (status === 'waiting_for_user') return 'orange'
	if (status === 'skipped') return 'gray'
	return 'blue'
}

function canCancel(job: InstallJobSnapshot) {
	return job.status === 'queued' || job.status === 'running'
}

function canRetry(job: InstallJobSnapshot) {
	return job.status === 'failed' || job.status === 'interrupted' || job.status === 'canceled'
}

function showProgress(job: InstallJobSnapshot) {
	return ['queued', 'running', 'canceling'].includes(job.status)
}

function jobPercent(job: InstallJobSnapshot) {
	const progress = job.progress?.secondary ?? job.progress
	if (!progress?.total) return job.status === 'succeeded' ? 100 : 0
	return Math.min(100, Math.max(0, Math.round((progress.current / progress.total) * 100)))
}

function progressText(job: InstallJobSnapshot) {
	if (job.summary.bytes_total)
		return `${formatBytes(job.summary.bytes_downloaded)} / ${formatBytes(job.summary.bytes_total)}`
	if (job.summary.files_total) return `${job.summary.files_completed} / ${job.summary.files_total}`
	return phaseLabel(job.phase)
}

function downloadTelemetry(job: InstallJobSnapshot) {
	const summary = job.summary
	const metrics: string[] = []
	if (summary.source) {
		metrics.push(
			formatMessage(messages.downloadSource, {
				source: downloadSourceLabel(summary.source),
			}),
		)
	}
	if (summary.speed_bytes_per_second && summary.speed_bytes_per_second > 0) {
		metrics.push(
			formatMessage(messages.downloadSpeed, {
				speed: formatBytes(summary.speed_bytes_per_second),
			}),
		)
	}
	if (summary.eta_seconds != null) {
		metrics.push(formatDownloadEta(summary.eta_seconds))
	}
	if (summary.fallback_count > 0) {
		metrics.push(formatMessage(messages.downloadFallbacks, { count: summary.fallback_count }))
	}
	return metrics
}

function downloadSourceLabel(source: string) {
	switch (source) {
		case 'official':
			return formatMessage(messages.downloadSourceOfficial)
		case 'bmclapi':
			return formatMessage(messages.downloadSourceBmclapi)
		case 'mcim':
			return formatMessage(messages.downloadSourceMcim)
		default:
			return formatMessage(messages.downloadSourceAlternate)
	}
}

function formatDownloadEta(seconds: number) {
	const clamped = Math.max(0, Math.round(seconds))
	if (clamped < 60) {
		return formatMessage(messages.downloadEtaSeconds, { seconds: clamped })
	}
	const minutes = Math.floor(clamped / 60)
	if (minutes < 60) {
		return formatMessage(messages.downloadEtaMinutes, { minutes })
	}
	return formatMessage(messages.downloadEtaHours, {
		hours: Math.floor(minutes / 60),
		minutes: minutes % 60,
	})
}

function itemProgress(item: DownloadItem) {
	if (!item.bytes_total) return formatMessage(messages.notAvailable)
	return `${formatBytes(item.bytes_downloaded)} / ${formatBytes(item.bytes_total)}`
}

function itemAttempts(item: DownloadItem) {
	if (!item.attempt || !item.max_attempts) return formatMessage(messages.notAvailable)
	return formatMessage(messages.attemptProgress, {
		attempt: item.attempt,
		maxAttempts: item.max_attempts,
	})
}

function itemError(item: DownloadItem) {
	if (item.error?.includes('requires manual download')) {
		return formatMessage(messages.manualDownloadRequired)
	}
	return item.error ?? ''
}

async function openManualDownload(item: DownloadItem) {
	if (item.manual_url) await openUrl(item.manual_url)
}

function legacyPercent(bar: LoadingBar) {
	if (!bar.total) return 0
	return Math.min(100, Math.max(0, Math.round(((bar.current ?? 0) / bar.total) * 100)))
}

function formatDate(value: string) {
	return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
		new Date(value),
	)
}

function selectTab(index: number) {
	tab.value = index === 0 ? 'active' : 'history'
}

function toggleExpanded(jobId: string) {
	const next = new Set(expanded.value)
	if (next.has(jobId)) {
		next.delete(jobId)
	} else {
		next.add(jobId)
	}
	expanded.value = next
}

async function withBusy(jobId: string, action: () => Promise<void>) {
	busy.value = new Set([...busy.value, jobId])
	try {
		await action()
	} catch (error) {
		handleError(error)
	} finally {
		const next = new Set(busy.value)
		next.delete(jobId)
		busy.value = next
	}
}

async function cancel(job: InstallJobSnapshot) {
	await withBusy(job.job_id, () => manager.cancel(job.job_id))
}

async function retry(job: InstallJobSnapshot) {
	await withBusy(job.job_id, () => manager.retry(job.job_id))
}

async function remove(job: InstallJobSnapshot) {
	await withBusy(job.job_id, () => manager.remove(job.job_id))
}

async function copyDiagnostics(job: InstallJobSnapshot) {
	await withBusy(job.job_id, async () =>
		navigator.clipboard.writeText(await download_job_support_details(job.job_id)),
	)
}

async function clearHistory() {
	try {
		await manager.clearHistory()
	} catch (error) {
		handleError(error)
	}
}
</script>
