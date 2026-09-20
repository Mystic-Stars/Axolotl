<script setup lang="ts">
import { FileArchiveIcon, SaveIcon, TrashIcon, UndoIcon, XIcon } from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useFormatBytes,
	useFormatDateTime,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import { get_full_path } from '@/helpers/instance'
import {
	type BackupExclusion,
	type BackupOperation,
	type BackupOperationState,
	type BackupProgressStage,
	type BackupRestorePreview,
	type BackupSnapshot,
	cancelBackup,
	deleteBackup,
	disableBackups,
	enableBackups,
	getBackupConfig,
	type InstanceBackupConfig,
	type InstanceBackupEligibility,
	listBackups,
	listBackupOperations,
	listenBackupProgress,
	restoreBackup,
	startBackupRestorePreview,
	startBackup,
	updateBackupExclusions,
} from '@/helpers/instance-backup'
import { injectInstanceSettings } from '@/providers/instance-settings'

import BackupExclusionSelector from './BackupExclusionSelector.vue'

const { instance } = injectInstanceSettings()
const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const formatDate = useFormatDateTime({ dateStyle: 'medium', timeStyle: 'short' })
const { handleError } = injectNotificationManager()

const messages = defineMessages({
	title: { id: 'instance.backups.title', defaultMessage: 'Instance backups' },
	description: {
		id: 'instance.backups.description',
		defaultMessage: 'Create deduplicated snapshots of the whole instance while it is closed.',
	},
	loading: { id: 'instance.backups.loading', defaultMessage: 'Loading backup settings...' },
	unavailable: { id: 'instance.backups.unavailable', defaultMessage: 'Backups are unavailable' },
	notInstalled: {
		id: 'instance.backups.unavailable.not-installed',
		defaultMessage: 'Finish installing this instance before enabling backups.',
	},
	symlinkInstance: {
		id: 'instance.backups.unavailable.symlink',
		defaultMessage: 'Imported symbolic-link instances cannot be backed up.',
	},
	directLinked: {
		id: 'instance.backups.unavailable.direct-linked',
		defaultMessage: 'Instances linked directly to another launcher cannot be backed up.',
	},
	externalDirectory: {
		id: 'instance.backups.unavailable.external-directory',
		defaultMessage: 'Instances that use an external game directory cannot be backed up.',
	},
	rootMissing: {
		id: 'instance.backups.unavailable.root-missing',
		defaultMessage: 'The instance folder is missing.',
	},
	rootLink: {
		id: 'instance.backups.unavailable.root-link',
		defaultMessage: 'Instances whose root folder is a link cannot be backed up.',
	},
	choose: {
		id: 'instance.backups.choose',
		defaultMessage:
			'Everything is backed up by default. Add files or folders that should be excluded.',
	},
	enable: { id: 'instance.backups.enable', defaultMessage: 'Enable backups' },
	editExclusions: { id: 'instance.backups.edit-exclusions', defaultMessage: 'Edit exclusions' },
	saveExclusions: {
		id: 'instance.backups.save-exclusions',
		defaultMessage: 'Save exclusions',
	},
	allIncluded: {
		id: 'instance.backups.all-included',
		defaultMessage: 'All instance files and folders are included.',
	},
	excludedPaths: {
		id: 'instance.backups.excluded-paths',
		defaultMessage: 'Excluded: {paths}',
	},
	backupNow: { id: 'instance.backups.backup-now', defaultMessage: 'Back up now' },
	cancelBackup: { id: 'instance.backups.cancel', defaultMessage: 'Cancel backup' },
	stageScanning: {
		id: 'instance.backups.stage.scanning',
		defaultMessage: 'Scanning files',
	},
	stageHashing: { id: 'instance.backups.stage.hashing', defaultMessage: 'Hashing files' },
	stageSaving: { id: 'instance.backups.stage.saving', defaultMessage: 'Saving snapshot index' },
	stageQueued: { id: 'instance.backups.stage.queued', defaultMessage: 'Waiting to start' },
	stageValidating: {
		id: 'instance.backups.stage.validating',
		defaultMessage: 'Validating restore',
	},
	stageMaterializing: {
		id: 'instance.backups.stage.materializing',
		defaultMessage: 'Preparing changed files',
	},
	stageApplying: {
		id: 'instance.backups.stage.applying',
		defaultMessage: 'Applying changed files',
	},
	stagePreview: {
		id: 'instance.backups.stage.preview',
		defaultMessage: 'Analyzing restore changes',
	},
	progressBytes: {
		id: 'instance.backups.progress-bytes',
		defaultMessage: '{processed} of {total}',
	},
	stageFailed: { id: 'instance.backups.stage.failed', defaultMessage: 'Backup failed' },
	stageInterrupted: {
		id: 'instance.backups.stage.interrupted',
		defaultMessage: 'Backup operation was interrupted',
	},
	snapshots: { id: 'instance.backups.snapshots', defaultMessage: 'Snapshots' },
	empty: { id: 'instance.backups.empty', defaultMessage: 'No backups have been created yet.' },
	snapshotMeta: {
		id: 'instance.backups.snapshot-meta',
		defaultMessage: '{files, plural, one {# file} other {# files}} · {size}',
	},
	addedSize: { id: 'instance.backups.added-size', defaultMessage: '{size} newly stored' },
	restore: { id: 'instance.backups.restore', defaultMessage: 'Restore backup' },
	delete: { id: 'instance.backups.delete', defaultMessage: 'Delete backup' },
	disable: { id: 'instance.backups.disable', defaultMessage: 'Disable backups' },
	disableBlocked: {
		id: 'instance.backups.disable-blocked',
		defaultMessage: 'Delete every snapshot before disabling backups.',
	},
	restoreTitle: { id: 'instance.backups.restore-title', defaultMessage: 'Restore this backup?' },
	restoreBody: {
		id: 'instance.backups.restore-body',
		defaultMessage:
			'Upcoming operations: add {added, plural, one {# file} other {# files}}, modify {modified, plural, one {# file} other {# files}}, and delete {deleted, plural, one {# file} other {# files}}. Continue?',
	},
	deleteTitle: { id: 'instance.backups.delete-title', defaultMessage: 'Delete this backup?' },
	deleteBody: {
		id: 'instance.backups.delete-body',
		defaultMessage:
			'This snapshot will be permanently deleted. Unused stored files will also be removed.',
	},
})

const config = ref<InstanceBackupConfig | null>(null)
const snapshots = ref<BackupSnapshot[]>([])
const instanceRoot = ref('')
const loading = ref(true)
const editingExclusions = ref(false)
const excludedPaths = ref<BackupExclusion[]>([])
const action = ref<string | null>(null)
const operation = ref<BackupOperation | null>(null)
const selectedSnapshot = ref<BackupSnapshot | null>(null)
const restorePreview = ref<BackupRestorePreview | null>(null)
const deleteModal = ref<InstanceType<typeof NewModal>>()
const restoreModal = ref<InstanceType<typeof NewModal>>()
let unlisten: (() => void) | null = null

const eligible = computed(() => config.value?.eligibility === 'eligible')
const terminalStates: BackupOperationState[] = ['completed', 'cancelled', 'failed', 'interrupted']
const isRunning = computed(
	() => operation.value !== null && !terminalStates.includes(operation.value.state),
)
const canCancel = computed(() => operation.value?.cancellable === true)
const operationLabel = computed(() => {
	if (operation.value?.operation_type === 'restore_preview') {
		return formatMessage(messages.stagePreview)
	}
	switch (operation.value?.state) {
		case 'queued':
			return formatMessage(messages.stageQueued)
		case 'hashing':
			return formatMessage(messages.stageHashing)
		case 'saving':
			return formatMessage(messages.stageSaving)
		case 'validating':
			return formatMessage(messages.stageValidating)
		case 'materializing':
			return formatMessage(messages.stageMaterializing)
		case 'applying':
			return formatMessage(messages.stageApplying)
		default:
			return formatMessage(messages.stageScanning)
	}
})

function operationFromEvent(event: {
	operationId: string
	operationType: BackupOperation['operation_type']
	instanceId?: string
	stage: BackupProgressStage
	processedBytes: number
	totalBytes: number
	snapshotId?: string
	message?: string
}): BackupOperation {
	const now = Date.now()
	const state: BackupOperationState =
		event.stage === 'restoring' || event.stage === 'copying'
			? 'materializing'
			: event.stage === 'deleting'
				? 'queued'
				: event.stage
	return {
		id: event.operationId,
		operation_type: event.operationType,
		instance_id: event.instanceId ?? instance.value.id,
		snapshot_id: event.snapshotId,
		state,
		processed_bytes: event.processedBytes,
		total_bytes: event.totalBytes,
		cancellable: event.stage === 'scanning' || event.stage === 'hashing',
		cancel_requested: false,
		error: event.message,
		created_at: operation.value?.id === event.operationId ? operation.value.created_at : now,
		updated_at: now,
		finished_at: ['completed', 'cancelled', 'failed'].includes(event.stage) ? now : undefined,
	}
}

async function refreshOperation() {
	const operations = await listBackupOperations(instance.value.id)
	const latest = operations[0] ?? null
	operation.value =
		latest && (isRunningState(latest.state) || ['failed', 'interrupted'].includes(latest.state))
			? latest
			: null
	if (operation.value?.operation_type === 'restore_preview' && operation.value.snapshot_id) {
		selectedSnapshot.value =
			snapshots.value.find((snapshot) => snapshot.id === operation.value?.snapshot_id) ?? null
	}
}

function isRunningState(state: BackupOperationState) {
	return !terminalStates.includes(state)
}

const eligibilityMessages: Record<
	Exclude<InstanceBackupEligibility, 'eligible'>,
	keyof typeof messages
> = {
	not_installed: 'notInstalled',
	symlink_instance: 'symlinkInstance',
	direct_linked_instance: 'directLinked',
	external_game_directory: 'externalDirectory',
	root_missing: 'rootMissing',
	root_is_link: 'rootLink',
}

const unavailableReason = computed(() => {
	const eligibility = config.value?.eligibility
	if (!eligibility || eligibility === 'eligible') return ''
	return formatMessage(messages[eligibilityMessages[eligibility]])
})

async function refresh() {
	loading.value = true
	try {
		const nextConfig = await getBackupConfig(instance.value.id)
		config.value = nextConfig
		excludedPaths.value = nextConfig.enabled ? [...nextConfig.excluded_paths] : []
		const [nextRoot, nextSnapshots] = await Promise.all([
			nextConfig.eligibility === 'eligible' ? get_full_path(instance.value.id) : '',
			listBackups(instance.value.id),
		])
		instanceRoot.value = nextRoot
		snapshots.value = nextSnapshots
		await refreshOperation()
	} catch (error) {
		handleError(error)
	} finally {
		loading.value = false
	}
}

async function runAction(name: string, task: () => Promise<void>) {
	action.value = name
	try {
		await task()
	} catch (error) {
		handleError(error)
	} finally {
		action.value = null
	}
}

function enable() {
	void runAction('enable', async () => {
		config.value = await enableBackups(instance.value.id, excludedPaths.value)
		editingExclusions.value = false
	})
}

function beginEditingExclusions() {
	excludedPaths.value = [...(config.value?.excluded_paths ?? [])]
	editingExclusions.value = true
}

function saveExclusions() {
	void runAction('exclusions', async () => {
		config.value = await updateBackupExclusions(instance.value.id, excludedPaths.value)
		editingExclusions.value = false
	})
}

function start() {
	void runAction('start', async () => {
		const id = await startBackup(instance.value.id)
		operation.value = {
			id,
			operation_type: 'create',
			instance_id: instance.value.id,
			state: 'queued',
			processed_bytes: 0,
			total_bytes: 0,
			cancellable: true,
			cancel_requested: false,
			created_at: Date.now(),
			updated_at: Date.now(),
		}
	})
}

function cancel() {
	if (!operation.value || !canCancel.value) return
	void runAction('cancel', async () => {
		if (await cancelBackup(operation.value!.id)) {
			operation.value = {
				...operation.value!,
				cancellable: false,
				cancel_requested: true,
			}
		}
	})
}

function confirmDelete(snapshot: BackupSnapshot) {
	selectedSnapshot.value = snapshot
	deleteModal.value?.show()
}

function confirmRestore(snapshot: BackupSnapshot) {
	void runAction(`preview:${snapshot.id}`, async () => {
		selectedSnapshot.value = snapshot
		const id = await startBackupRestorePreview(snapshot.id)
		operation.value = {
			id,
			operation_type: 'restore_preview',
			instance_id: instance.value.id,
			snapshot_id: snapshot.id,
			state: 'queued',
			processed_bytes: 0,
			total_bytes: snapshot.logical_size,
			cancellable: true,
			cancel_requested: false,
			created_at: Date.now(),
			updated_at: Date.now(),
		}
	})
}

function removeSnapshot() {
	if (!selectedSnapshot.value) return
	void runAction(`delete:${selectedSnapshot.value.id}`, async () => {
		await deleteBackup(selectedSnapshot.value!.id)
		await refresh()
		deleteModal.value?.hide()
		selectedSnapshot.value = null
	})
}

function restoreSnapshot() {
	if (!selectedSnapshot.value || !restorePreview.value) return
	void runAction(`restore:${selectedSnapshot.value.id}`, async () => {
		const id = await restoreBackup(selectedSnapshot.value!.id, restorePreview.value!.plan_token)
		operation.value = {
			id,
			operation_type: 'restore',
			instance_id: instance.value.id,
			snapshot_id: selectedSnapshot.value!.id,
			state: 'queued',
			processed_bytes: 0,
			total_bytes: selectedSnapshot.value!.logical_size,
			cancellable: true,
			cancel_requested: false,
			created_at: Date.now(),
			updated_at: Date.now(),
		}
		restoreModal.value?.hide()
		selectedSnapshot.value = null
		restorePreview.value = null
	})
}

function disable() {
	void runAction('disable', async () => {
		await disableBackups(instance.value.id)
		await refresh()
	})
}

onMounted(async () => {
	unlisten = await listenBackupProgress(async (event) => {
		if (
			!['create', 'restore', 'restore_preview'].includes(event.operationType) ||
			event.instanceId !== instance.value.id
		)
			return
		operation.value = operationFromEvent(event)
		if (event.operationType === 'restore_preview' && event.stage === 'completed') {
			const completed = (await listBackupOperations(instance.value.id)).find(
				(candidate) => candidate.id === event.operationId,
			)
			const snapshot = snapshots.value.find((candidate) => candidate.id === event.snapshotId)
			if (completed?.restore_preview && snapshot) {
				selectedSnapshot.value = snapshot
				restorePreview.value = completed.restore_preview
				operation.value = null
				restoreModal.value?.show()
				return
			}
		}
		if (['completed', 'cancelled', 'failed'].includes(event.stage)) {
			void refresh()
		}
	})
	await refresh()
})

onUnmounted(() => unlisten?.())
watch(
	() => instance.value.id,
	() => void refresh(),
)
</script>

<template>
	<div class="flex flex-col gap-5">
		<header>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
			<p class="m-0 mt-1 text-secondary">{{ formatMessage(messages.description) }}</p>
		</header>

		<p v-if="loading" class="m-0 text-secondary">{{ formatMessage(messages.loading) }}</p>
		<Admonition v-else-if="!eligible" type="warning" :header="formatMessage(messages.unavailable)">
			{{ unavailableReason }}
		</Admonition>
		<template v-else-if="config">
			<div v-if="!config.enabled || editingExclusions" class="flex flex-col gap-4">
				<p class="m-0 text-secondary">{{ formatMessage(messages.choose) }}</p>
				<BackupExclusionSelector
					v-model="excludedPaths"
					:instance-id="instance.id"
					:instance-root="instanceRoot"
					:disabled="action !== null"
				/>
				<div class="flex flex-wrap gap-2">
					<ButtonStyled>
						<button
							type="button"
							:disabled="action !== null"
							@click="config.enabled ? saveExclusions() : enable()"
						>
							<SaveIcon />
							{{ formatMessage(config.enabled ? messages.saveExclusions : messages.enable) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="config.enabled" type="outlined">
						<button type="button" :disabled="action !== null" @click="editingExclusions = false">
							<XIcon />
							{{ formatMessage(commonMessages.cancelButton) }}
						</button>
					</ButtonStyled>
				</div>
			</div>

			<template v-else>
				<div class="flex flex-col gap-2 rounded-lg border border-solid border-surface-4 p-3">
					<p class="m-0 text-sm text-secondary">
						{{
							config.excluded_paths.length === 0
								? formatMessage(messages.allIncluded)
								: formatMessage(messages.excludedPaths, {
										paths: config.excluded_paths.map((entry) => entry.path).join(', '),
									})
						}}
					</p>
					<div class="flex flex-wrap gap-2">
						<ButtonStyled>
							<button type="button" :disabled="isRunning || action !== null" @click="start">
								<FileArchiveIcon />
								{{ formatMessage(messages.backupNow) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="canCancel" type="outlined">
							<button type="button" :disabled="action !== null" @click="cancel">
								<XIcon />
								{{ formatMessage(messages.cancelBackup) }}
							</button>
						</ButtonStyled>
						<ButtonStyled type="outlined">
							<button
								type="button"
								:disabled="isRunning || action !== null"
								@click="beginEditingExclusions"
							>
								{{ formatMessage(messages.editExclusions) }}
							</button>
						</ButtonStyled>
					</div>
					<p v-if="isRunning" class="m-0 text-sm font-medium text-contrast">
						{{ operationLabel }}
						<span v-if="(operation?.total_bytes ?? 0) > 0" class="ml-2 text-secondary">
							{{
								formatMessage(messages.progressBytes, {
									processed: formatBytes(operation?.processed_bytes ?? 0),
									total: formatBytes(operation?.total_bytes ?? 0),
								})
							}}
						</span>
					</p>
					<p
						v-else-if="operation?.state === 'failed' || operation?.state === 'interrupted'"
						class="m-0 text-sm text-red"
					>
						{{
							formatMessage(
								operation.state === 'interrupted'
									? messages.stageInterrupted
									: messages.stageFailed,
							)
						}}<span v-if="operation.error">: {{ operation.error }}</span>
					</p>
				</div>

				<section class="flex flex-col gap-3">
					<h3 class="m-0 text-base font-semibold text-contrast">
						{{ formatMessage(messages.snapshots) }}
					</h3>
					<p v-if="snapshots.length === 0" class="m-0 text-secondary">
						{{ formatMessage(messages.empty) }}
					</p>
					<div
						v-else
						class="divide-y divide-solid divide-surface-4 rounded-lg border border-solid border-surface-4"
					>
						<div
							v-for="snapshot in snapshots"
							:key="snapshot.id"
							class="flex items-center gap-3 px-3 py-3"
						>
							<FileArchiveIcon class="size-5 shrink-0 text-secondary" />
							<div class="min-w-0 flex-1">
								<p class="m-0 font-semibold text-contrast">
									{{ formatDate(new Date(snapshot.created_at)) }}
								</p>
								<p class="m-0 text-sm text-secondary">
									{{
										formatMessage(messages.snapshotMeta, {
											files: snapshot.file_count,
											size: formatBytes(snapshot.logical_size),
										})
									}}
									·
									{{
										formatMessage(messages.addedSize, { size: formatBytes(snapshot.added_size) })
									}}
								</p>
							</div>
							<ButtonStyled circular size="small" type="transparent">
								<button
									type="button"
									:disabled="action !== null || isRunning"
									:aria-label="formatMessage(messages.restore)"
									@click="confirmRestore(snapshot)"
								>
									<UndoIcon />
								</button>
							</ButtonStyled>
							<ButtonStyled circular color="red" size="small" type="transparent">
								<button
									type="button"
									:disabled="action !== null || isRunning"
									:aria-label="formatMessage(messages.delete)"
									@click="confirmDelete(snapshot)"
								>
									<TrashIcon />
								</button>
							</ButtonStyled>
						</div>
					</div>
				</section>

				<div class="flex flex-col items-start gap-2">
					<ButtonStyled color="red" type="outlined">
						<button
							type="button"
							:disabled="snapshots.length > 0 || action !== null || isRunning"
							@click="disable"
						>
							{{ formatMessage(messages.disable) }}
						</button>
					</ButtonStyled>
					<p v-if="snapshots.length > 0" class="m-0 text-sm text-secondary">
						{{ formatMessage(messages.disableBlocked) }}
					</p>
				</div>
			</template>
		</template>

		<NewModal
			ref="deleteModal"
			:header="formatMessage(messages.deleteTitle)"
			fade="danger"
			max-width="500px"
		>
			<Admonition type="critical">{{ formatMessage(messages.deleteBody) }}</Admonition>
			<template #actions>
				<div class="flex items-center justify-end gap-2">
					<ButtonStyled type="outlined">
						<button :disabled="action?.startsWith('delete:')" @click="deleteModal?.hide()">
							{{ formatMessage(commonMessages.cancelButton) }}
						</button>
					</ButtonStyled>
					<ButtonStyled color="red">
						<button :disabled="action?.startsWith('delete:')" @click="removeSnapshot">
							<TrashIcon />{{ formatMessage(messages.delete) }}
						</button>
					</ButtonStyled>
				</div>
			</template>
		</NewModal>
		<NewModal ref="restoreModal" :header="formatMessage(messages.restoreTitle)" max-width="500px">
			<Admonition v-if="restorePreview" type="warning">
				{{
					formatMessage(messages.restoreBody, {
						added: restorePreview.added_files,
						modified: restorePreview.modified_files,
						deleted: restorePreview.deleted_files,
					})
				}}
			</Admonition>
			<template #actions>
				<div class="flex items-center justify-end gap-2">
					<ButtonStyled type="outlined">
						<button :disabled="action?.startsWith('restore:')" @click="restoreModal?.hide()">
							{{ formatMessage(commonMessages.cancelButton) }}
						</button>
					</ButtonStyled>
					<ButtonStyled>
						<button :disabled="action?.startsWith('restore:')" @click="restoreSnapshot">
							<UndoIcon />{{ formatMessage(messages.restore) }}
						</button>
					</ButtonStyled>
				</div>
			</template>
		</NewModal>
	</div>
</template>
