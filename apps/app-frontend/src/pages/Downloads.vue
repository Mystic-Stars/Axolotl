<template>
	<div class="box-border flex min-h-full w-full flex-col gap-3 p-6">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div data-onboarding-id="downloads-tabs">
				<NavTabs
					:active-index="tab === 'active' ? 0 : 1"
					:links="downloadTabs"
					mode="local"
					@tab-click="selectTab"
				/>
			</div>
			<ButtonStyled color="brand">
				<button @click="router.push('/create')">
					<PlusIcon />
					{{ formatMessage(messages.newDownload) }}
				</button>
			</ButtonStyled>
		</div>

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
					<img
						v-if="bar.bar_type?.icon"
						:src="displayIcon(bar.bar_type.icon)"
						alt=""
						class="size-12 rounded-xl object-cover"
					/>
					<div
						v-else
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

			<Card
				v-for="job in visibleJobs"
				:key="job.job_id"
				:data-install-job-id="job.job_id"
				class="!p-0"
			>
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
								<component :is="jobTypeIcon(job)" />
								{{ jobTypeLabel(job) }}
							</TagItem>
							<Badge :color="statusColor(job.status)" :type="statusLabel(job.status)" />
							<Badge
								v-if="job.instance_deleted"
								color="gray"
								:type="formatMessage(messages.instanceDeleted)"
							/>
						</div>
						<div class="mt-1 flex flex-wrap items-center gap-2 text-sm text-secondary">
							<span>{{ jobPhaseLabel(job) }}</span>
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
						<div v-if="activeRequestItems(job).length" class="mt-2 flex flex-col gap-1">
							<div
								v-for="item in activeRequestItems(job).slice(0, 3)"
								:key="`${item.id}-${item.request_url}`"
								class="flex min-w-0 items-center gap-2 text-xs text-secondary"
							>
								<GlobeIcon class="size-3.5 shrink-0" />
								<span v-if="item.source" class="shrink-0">
									{{ downloadSourceLabel(item.source) }}
								</span>
								<code v-tooltip="item.request_url" class="min-w-0 flex-1 truncate">{{
									item.request_url
								}}</code>
							</div>
							<div v-if="activeRequestItems(job).length > 3" class="text-xs text-secondary">
								{{
									formatMessage(messages.moreActiveRequests, {
										count: activeRequestItems(job).length - 3,
									})
								}}
							</div>
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
						<ButtonStyled
							v-if="canSkipMissingContent(job)"
							color="orange"
							type="outlined"
							size="small"
						>
							<button :disabled="busy.has(job.job_id)" @click="skipMissingContent(job)">
								<ArrowBigRightDashIcon />{{ formatMessage(messages.skipMissingFiles) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="job.status === 'waiting_for_user'" color="brand" size="small">
							<button :disabled="busy.has(job.job_id)" @click="resolveMissing(job)">
								<DownloadIcon />{{ formatMessage(messages.completeMissingFiles) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="job.error" type="outlined" size="small">
							<button :disabled="busy.has(job.job_id)" @click="copyDiagnostics(job)">
								<ClipboardCopyIcon />{{ formatMessage(messages.copyDiagnostics) }}
							</button>
						</ButtonStyled>
						<ButtonStyled
							v-if="tab === 'history' && isSuccessfulUpgradeJob(job)"
							color="brand"
							size="small"
						>
							<button @click="router.push(upgradeResultLocation(job))">
								<CheckCircleIcon />{{ formatMessage(messages.viewUpgradeResult) }}
							</button>
						</ButtonStyled>
						<ButtonStyled
							v-if="job.instance_id && !job.instance_deleted && job.status !== 'waiting_for_user'"
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
						:waiting="job.status === 'queued' || !hasDeterminateProgress(job)"
						show-progress
					/>
					<div v-if="job.parallel" class="mt-2">
						<ProgressBar
							full-width
							:progress="parallelPercent(job)"
							:max="100"
							:label="parallelProgressText(job)"
							:waiting="job.status === 'queued' || !hasDeterminateParallelProgress(job)"
							show-progress
						/>
					</div>
				</div>

				<div v-if="job.status === 'waiting_for_user'" class="px-4 pb-4">
					<Admonition type="warning" :header="formatMessage(messages.actionNeeded)">
						{{
							formatMessage(messages.missingRequiredContent, {
								completed: completedRequiredFiles(job),
								total: totalRequiredFiles(job),
								missing: missingRequiredFiles(job),
							})
						}}
					</Admonition>
				</div>

				<div
					v-if="expanded.has(job.job_id)"
					class="border-0 border-t border-solid border-divider p-4"
				>
					<Admonition
						v-if="job.rollback_error || job.error"
						class="mb-4"
						type="critical"
						:header="
							formatMessage(job.rollback_error ? messages.cleanupIncomplete : messages.errorDetails)
						"
					>
						{{ job.rollback_error?.message ?? job.error?.message }}
					</Admonition>
					<Table
						v-if="job.items.length"
						:columns="itemColumns"
						:data="reorderJobItems(job)"
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
								<div
									v-if="row.request_url"
									class="flex min-w-0 items-center gap-1.5 text-xs text-secondary"
								>
									<GlobeIcon class="size-3 shrink-0" />
									<span v-if="row.source" class="shrink-0">
										{{ downloadSourceLabel(row.source) }}
									</span>
									<code v-tooltip="row.request_url" class="min-w-0 flex-1 truncate">{{
										row.request_url
									}}</code>
								</div>
								<ButtonStyled v-if="row.manual_url" type="transparent" size="small">
									<button class="!px-0" @click.stop="openManualDownload(row)">
										<ExternalIcon />{{ formatMessage(messages.manualDownload) }}
									</button>
								</ButtonStyled>
							</div>
						</template>
						<template #cell-status="{ row }">
							<Badge
								:color="itemStatusColor(row.status)"
								:type="itemStatusLabel(job, row.status)"
							/>
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

		<Card v-else class="flex flex-1">
			<EmptyState
				class="my-auto"
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
	<MissingModpackContentModal ref="missingContentModal" />
</template>

<script setup lang="ts">
import {
	ArrowBigRightDashIcon,
	CheckCircleIcon,
	ChevronDownIcon,
	ClipboardCopyIcon,
	ClockIcon,
	CurseForgeIcon,
	DownloadIcon,
	ExternalIcon,
	GlobeIcon,
	ModrinthIcon,
	PlusIcon,
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
	type MessageDescriptor,
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
import { computed, nextTick, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import MissingModpackContentModal from '@/components/ui/modal/MissingModpackContentModal.vue'
import { listPendingCurseForgeManualDownloads } from '@/helpers/curseforge'
import type { CurseForgeManualDownloadItem } from '@/helpers/curseforge-manual'
import {
	download_job_support_details,
	type InstallJobSnapshot,
	type InstallJobStatus,
	type InstallPhaseId,
} from '@/helpers/install'
import {
	effectiveInstallProgress,
	effectiveParallelProgress,
	hasDeterminateInstallProgress,
	installProgressTextSource,
} from '@/helpers/install-progress'
import type { LoadingBar } from '@/helpers/state'
import { injectContentInstall } from '@/providers/content-install'
import { injectDownloadManager } from '@/providers/download-manager'

import {
	createDownloadFocusState,
	focusedDownloadJobId,
	reconcileDownloadFocus,
	stopDownloadFocusAutoFollow,
} from './download-focus'
import { isSuccessfulUpgradeJob, upgradeResultLocation } from './instance/upgrade/result'

type DownloadItem = InstallJobSnapshot['items'][number]

const manager = injectDownloadManager()
const { showCurseForgeManualDownloads } = injectContentInstall()
const route = useRoute()
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
const missingContentModal = ref<InstanceType<typeof MissingModpackContentModal>>()
const focusedJobId = computed(() => focusedDownloadJobId(route.query.job))
const focusState = ref(createDownloadFocusState(focusedJobId.value))

const messages = defineMessages({
	newDownload: { id: 'app.downloads.new-download', defaultMessage: 'New download' },
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
	skipMissingFiles: {
		id: 'app.downloads.skip-missing-files',
		defaultMessage: 'Skip',
	},
	completeMissingFiles: {
		id: 'app.downloads.complete-missing-files',
		defaultMessage: 'Complete missing files',
	},
	actionNeeded: { id: 'app.downloads.action-needed', defaultMessage: 'Action needed' },
	missingRequiredContent: {
		id: 'app.downloads.missing-required-content',
		defaultMessage:
			'{completed} / {total} required files are ready. {missing, plural, one {# file still needs to be downloaded.} other {# files still need to be downloaded.}}',
	},
	copyDiagnostics: { id: 'app.downloads.copy-diagnostics', defaultMessage: 'Copy diagnostics' },
	upgrade: { id: 'app.downloads.operation.upgrade', defaultMessage: 'Upgrade' },
	viewUpgradeResult: {
		id: 'app.downloads.view-upgrade-result',
		defaultMessage: 'View upgrade result',
	},
	openInstance: { id: 'app.downloads.open-instance', defaultMessage: 'Open instance' },
	instanceDeleted: { id: 'app.downloads.instance-deleted', defaultMessage: 'Instance deleted' },
	details: { id: 'app.downloads.details', defaultMessage: 'Details' },
	hideDetails: { id: 'app.downloads.hide-details', defaultMessage: 'Hide details' },
	deleteRecord: { id: 'app.downloads.delete-record', defaultMessage: 'Delete record' },
	errorDetails: { id: 'app.downloads.error-details', defaultMessage: 'Download failed' },
	cleanupIncomplete: {
		id: 'app.action-bar.install.summary.cleanup-incomplete',
		defaultMessage: "Cleanup didn't finish",
	},
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
	verifyingDownloadedFiles: {
		id: 'app.downloads.phase.verifying-downloaded-files',
		defaultMessage: 'Verifying files and continuing installation',
	},
	pendingVerification: {
		id: 'app.downloads.item-status.pending-verification',
		defaultMessage: 'Pending verification',
	},
	downloadSourceOfficial: { id: 'app.downloads.source.official', defaultMessage: 'Official' },
	downloadSourceBmclapi: { id: 'app.downloads.source.bmclapi', defaultMessage: 'OpenBMCLAPI' },
	downloadSourceMcim: { id: 'app.downloads.source.mcim', defaultMessage: 'MCIM' },
	downloadSourceTianpao: { id: 'app.downloads.source.tianpao', defaultMessage: 'Tianpao' },
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
	moreActiveRequests: {
		id: 'app.downloads.more-active-requests',
		defaultMessage: '+{count} active requests',
	},
	unknownPhase: { id: 'app.downloads.phase.unknown', defaultMessage: 'Working' },
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
	creating_backup: {
		id: 'app.downloads.phase.creating-backup',
		defaultMessage: 'Creating backup',
	},
	staging_content: {
		id: 'app.downloads.phase.staging-content',
		defaultMessage: 'Staging content',
	},
	applying_content: {
		id: 'app.downloads.phase.applying-content',
		defaultMessage: 'Applying content',
	},
	updating_loader: {
		id: 'app.downloads.phase.updating-loader',
		defaultMessage: 'Updating loader',
	},
	verifying: { id: 'app.downloads.phase.verifying', defaultMessage: 'Verifying' },
	finalizing: { id: 'app.downloads.phase.finalizing', defaultMessage: 'Finalizing' },
	completed: { id: 'app.downloads.phase.completed', defaultMessage: 'Completed' },
	rolling_back: { id: 'app.downloads.phase.rolling-back', defaultMessage: 'Rolling back changes' },
} satisfies Record<InstallPhaseId, MessageDescriptor>)

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

function jobTypeLabel(job: InstallJobSnapshot) {
	return job.kind === 'upgrade_unmanaged_instance'
		? formatMessage(messages.upgrade)
		: providerLabel(job.provider)
}

function jobTypeIcon(job: InstallJobSnapshot) {
	return job.kind === 'upgrade_unmanaged_instance' ? RefreshCwIcon : providerIcon(job.provider)
}

function legacyProvider(bar: LoadingBar): InstallJobSnapshot['provider'] {
	if (bar.bar_type?.type === 'pack_download' || bar.bar_type?.type === 'pack_file_download')
		return 'curse_forge'
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
	const message = (phaseMessages as Partial<Record<string, MessageDescriptor>>)[phase]
	return formatMessage(message ?? messages.unknownPhase)
}

function isRecoveryValidation(job: InstallJobSnapshot) {
	return job.execution_mode === 'recovery_validation'
}

function isLocalRecoveryValidation(job: InstallJobSnapshot) {
	return isRecoveryValidation(job) && activeRequestItems(job).length === 0
}

function jobPhaseLabel(job: InstallJobSnapshot) {
	if (job.rollback_error) return formatMessage(messages.cleanupIncomplete)
	return isLocalRecoveryValidation(job)
		? formatMessage(messages.verifyingDownloadedFiles)
		: phaseLabel(job.phase)
}

function itemStatusLabel(job: InstallJobSnapshot, status: DownloadItem['status']) {
	if (isRecoveryValidation(job) && status === 'queued') {
		return formatMessage(messages.pendingVerification)
	}
	return statusLabel(status)
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
	return ['queued', 'running', 'waiting_for_user'].includes(job.status)
}

function canRetry(job: InstallJobSnapshot) {
	return job.status === 'failed' || job.status === 'interrupted' || job.status === 'canceled'
}

function canSkipMissingContent(job: InstallJobSnapshot) {
	if (job.provider === 'curse_forge' && job.kind === 'install_pack_to_existing_instance') {
		return false
	}
	return (
		job.status === 'waiting_for_user' &&
		job.pause_reason?.type === 'missing_required_content' &&
		job.pause_reason.failed_files > 0
	)
}

function showProgress(job: InstallJobSnapshot) {
	return ['queued', 'running', 'canceling', 'waiting_for_user'].includes(job.status)
}

function jobPercent(job: InstallJobSnapshot) {
	if (job.status === 'succeeded') return 100
	if (job.status === 'waiting_for_user') {
		const total = totalRequiredFiles(job)
		if (!total) return 0
		return Math.min(99, (completedRequiredFiles(job) / total) * 100)
	}
	const progress = effectiveInstallProgress(job)
	if (!hasDeterminateInstallProgress(progress)) return 0
	return Math.min(99, Math.max(0, (progress.current / progress.total) * 100))
}

function hasDeterminateProgress(job: InstallJobSnapshot) {
	return hasDeterminateInstallProgress(effectiveInstallProgress(job))
}

function parallelPercent(job: InstallJobSnapshot) {
	const progress = effectiveParallelProgress(job)
	if (!hasDeterminateInstallProgress(progress)) return 0
	return Math.min(100, Math.max(0, (progress.current / progress.total) * 100))
}

function hasDeterminateParallelProgress(job: InstallJobSnapshot) {
	return hasDeterminateInstallProgress(effectiveParallelProgress(job))
}

function parallelProgressText(job: InstallJobSnapshot) {
	const progress = effectiveParallelProgress(job)
	if (!hasDeterminateInstallProgress(progress)) {
		return job.parallel ? phaseLabel(job.parallel.phase) : ''
	}
	return `${phaseLabel(job.parallel!.phase)}: ${formatBytes(progress.current)} / ${formatBytes(progress.total)}`
}

function progressText(job: InstallJobSnapshot) {
	const textSource = installProgressTextSource(job)
	if (textSource.type === 'required_files') {
		return `${completedRequiredFiles(job)} / ${totalRequiredFiles(job)}`
	}
	if (isLocalRecoveryValidation(job)) {
		const progress = effectiveInstallProgress(job)
		if (hasDeterminateInstallProgress(progress)) {
			const checked = job.progress?.secondary
				? `${formatBytes(progress.current)} / ${formatBytes(progress.total)}`
				: `${progress.current} / ${progress.total}`
			return `${formatMessage(messages.verifyingDownloadedFiles)}: ${checked}`
		}
		return formatMessage(messages.verifyingDownloadedFiles)
	}
	const finalStage = job.items.find(
		(item) => item.status === 'writing' || item.status === 'verifying',
	)
	if (finalStage) return statusLabel(finalStage.status)
	const progress = effectiveInstallProgress(job)
	if (
		job.status === 'running' &&
		hasDeterminateInstallProgress(progress) &&
		progress.current >= progress.total &&
		activeRequestItems(job).length === 0
	)
		return statusLabel('verifying')
	if (textSource.type === 'bytes')
		return `${formatBytes(textSource.current)} / ${formatBytes(textSource.total)}`
	if (textSource.type === 'items') return `${textSource.current} / ${textSource.total}`
	return phaseLabel(job.phase)
}

function isResolvedRequiredFile(item: DownloadItem) {
	return item.status === 'completed' || (item.status === 'skipped' && item.manual_url == null)
}

function isUnresolvedRequiredFile(item: DownloadItem) {
	return item.status === 'failed' || (item.status === 'skipped' && item.manual_url != null)
}

function completedRequiredFiles(job: InstallJobSnapshot) {
	return job.items.filter(isResolvedRequiredFile).length
}

function hasCurrentRequiredFileResolutionState(job: InstallJobSnapshot) {
	return (
		job.items.some(isUnresolvedRequiredFile) ||
		(job.items.length > 0 && job.items.every(isResolvedRequiredFile))
	)
}

function missingRequiredFiles(job: InstallJobSnapshot) {
	if (hasCurrentRequiredFileResolutionState(job)) {
		return job.items.filter(isUnresolvedRequiredFile).length
	}
	if (job.pause_reason?.type === 'missing_required_content') {
		return job.pause_reason.failed_files
	}
	return 0
}

function totalRequiredFiles(job: InstallJobSnapshot) {
	const completed = completedRequiredFiles(job)
	const missing = missingRequiredFiles(job)
	return Math.max(job.summary.files_total ?? 0, job.items.length, completed + missing)
}

function downloadTelemetry(job: InstallJobSnapshot) {
	const summary = job.summary
	const metrics: string[] = []
	if (summary.source && !isRecoveryValidation(job)) {
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
	if (summary.fallback_count > 0 && !isRecoveryValidation(job)) {
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
		case 'tianpao':
			return formatMessage(messages.downloadSourceTianpao)
		default:
			return formatMessage(messages.downloadSourceAlternate)
	}
}

const STATUS_ORDER: Record<string, number> = {
	failed: 0,
	skipped: 1,
	waiting_for_user: 2,
	downloading: 3,
	verifying: 3,
	writing: 3,
	queued: 4,
	canceled: 5,
	completed: 6,
}
function compareItemStatus(a: DownloadItem, b: DownloadItem) {
	return (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9)
}
const itemsCache = new Map<string, { items: InstallJobSnapshot['items']; result: DownloadItem[] }>()
function reorderJobItems(job: InstallJobSnapshot): DownloadItem[] {
	const seen = itemsCache.get(job.job_id)
	if (seen && seen.items === job.items) return seen.result
	const result = [...job.items].sort(compareItemStatus)
	itemsCache.set(job.job_id, { items: job.items, result })
	return result
}

function activeRequestItems(job: InstallJobSnapshot) {
	return job.items.filter((item) => item.status === 'downloading' && item.request_url)
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
	if (item.attempt == null || item.max_attempts == null) return formatMessage(messages.notAvailable)
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
	return Math.min(100, Math.max(0, ((bar.current ?? 0) / bar.total) * 100))
}

function formatDate(value: string) {
	return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
		new Date(value),
	)
}

function selectTab(index: number) {
	focusState.value = stopDownloadFocusAutoFollow(focusState.value)
	tab.value = index === 0 ? 'active' : 'history'
}

function focusedJobElement(jobId: string): HTMLElement | null {
	return (
		[...document.querySelectorAll<HTMLElement>('[data-install-job-id]')].find(
			(element) => element.dataset.installJobId === jobId,
		) ?? null
	)
}

watch(
	[focusedJobId, manager.jobs],
	async ([jobId, jobs]) => {
		if (focusState.value.jobId !== jobId) {
			focusState.value = createDownloadFocusState(jobId)
		}
		const effect = reconcileDownloadFocus(
			focusState.value,
			jobId ? (jobs.find((job) => job.job_id === jobId) ?? null) : null,
		)
		focusState.value = effect.state
		if (effect.tab) tab.value = effect.tab
		if (effect.expand && jobId) {
			expanded.value = new Set([...expanded.value, jobId])
		}
		if (effect.scroll && jobId) {
			await nextTick()
			focusedJobElement(jobId)?.scrollIntoView({ behavior: 'smooth', block: 'center' })
		}
	},
	{ immediate: true },
)

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

async function skipMissingContent(job: InstallJobSnapshot) {
	await withBusy(job.job_id, () => manager.skipMissingContent(job.job_id))
}

async function resolveMissing(job: InstallJobSnapshot) {
	await withBusy(job.job_id, async () => {
		const fallbackCurseForgeItems = job.items
			.filter(
				(item) =>
					item.status === 'skipped' && item.manual_url && item.project_id && item.version_id,
			)
			.map(
				(item): CurseForgeManualDownloadItem => ({
					projectId: Number(item.project_id),
					fileId: Number(item.version_id),
					fileName: item.name,
					websiteUrl: item.manual_url ?? undefined,
				}),
			)
		const instanceId = job.instance_id
		const hasGeneralMissingItems = job.items.some((item) => item.status === 'failed')
		if (instanceId && (job.provider === 'curse_forge' || fallbackCurseForgeItems.length > 0)) {
			try {
				const pending = await listPendingCurseForgeManualDownloads(instanceId)
				if (pending.length > 0) {
					showCurseForgeManualDownloads(instanceId, pending)
					return
				}
				await manager.refresh()
				if (!hasGeneralMissingItems) return
			} catch (error) {
				handleError(error)
				if (fallbackCurseForgeItems.length > 0) {
					showCurseForgeManualDownloads(instanceId, fallbackCurseForgeItems)
					return
				}
				if (!hasGeneralMissingItems) return
			}
		}
		await missingContentModal.value?.show(job)
	})
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
