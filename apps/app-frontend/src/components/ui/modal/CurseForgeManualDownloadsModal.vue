<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		:fade="remainingCount > 0 ? 'warning' : 'standard'"
		:on-hide="stopScanning"
		max-width="680px"
		scrollable
	>
		<div class="flex flex-col gap-4">
			<p class="m-0 text-secondary">
				{{
					installed == null
						? formatMessage(messages.existingBody, { manual: remainingCount })
						: formatMessage(messages.body, {
								installed,
								manual: remainingCount,
							})
				}}
			</p>

			<Admonition
				:type="remainingCount === 0 ? 'success' : 'info'"
				:header="
					formatMessage(remainingCount === 0 ? messages.allImported : messages.automaticImport)
				"
			>
				<template #icon>
					<CheckIcon v-if="remainingCount === 0" class="size-5 shrink-0" aria-hidden="true" />
					<SpinnerIcon
						v-else-if="scanning"
						class="size-5 shrink-0 animate-spin"
						aria-hidden="true"
					/>
					<FolderSearchIcon v-else class="size-5 shrink-0" aria-hidden="true" />
				</template>
				<span class="min-w-0 break-words text-secondary">{{ scannerStatus }}</span>
			</Admonition>

			<div class="max-h-72 overflow-y-auto rounded-lg border border-surface-5 bg-surface-2">
				<div
					v-for="item in items"
					:key="itemKey(item)"
					class="flex min-h-16 items-center justify-between gap-3 border-0 border-b border-solid border-surface-5 px-3 py-2 last:border-b-0"
				>
					<div class="flex min-w-0 items-center gap-3">
						<div class="flex size-8 shrink-0 items-center justify-center rounded-full bg-surface-4">
							<CheckIcon v-if="isImported(item)" class="size-5 text-green" aria-hidden="true" />
							<FolderSearchIcon v-else class="size-5 text-secondary" aria-hidden="true" />
						</div>
						<div class="min-w-0 flex flex-col gap-0.5">
							<span class="truncate font-medium text-contrast">{{ item.fileName }}</span>
							<span class="truncate text-sm text-secondary">
								{{
									formatMessage(messages.projectFile, {
										projectId: item.projectId,
										fileId: item.fileId,
									})
								}}
							</span>
							<span class="text-sm" :class="isImported(item) ? 'text-green' : 'text-secondary'">
								{{ itemStatus(item) }}
							</span>
						</div>
					</div>
					<div v-if="!isImported(item)" class="flex shrink-0 flex-wrap justify-end gap-2">
						<ButtonStyled type="outlined" size="small">
							<button :disabled="busyKeys.has(itemKey(item))" @click="openOne(item)">
								<ExternalIcon aria-hidden="true" />
								{{ formatMessage(messages.open) }}
							</button>
						</ButtonStyled>
						<ButtonStyled type="outlined" size="small">
							<button :disabled="busyKeys.has(itemKey(item))" @click="chooseLocalFile(item)">
								<SpinnerIcon v-if="busyKeys.has(itemKey(item))" class="animate-spin" />
								<UploadIcon v-else aria-hidden="true" />
								{{ formatMessage(messages.chooseFile) }}
							</button>
						</ButtonStyled>
					</div>
				</div>
			</div>
		</div>

		<template #actions>
			<div class="flex flex-wrap justify-end gap-2">
				<ButtonStyled type="outlined">
					<button @click="hide">
						{{ formatMessage(commonMessages.closeButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="instanceId" type="outlined">
					<button @click="goToInstance">
						{{ formatMessage(messages.viewInstance) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="remainingCount > 0" color="orange">
					<button @click="openAll">
						<ExternalIcon aria-hidden="true" />
						{{ formatMessage(messages.openAll) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import {
	CheckIcon,
	ExternalIcon,
	FolderSearchIcon,
	SpinnerIcon,
	UploadIcon,
} from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import {
	classifyCurseForgeManualDownloadImportError,
	type CurseForgeManualDownloadImport,
	type CurseForgeManualDownloadScanResult,
	importCurseForgeManualDownloads,
	importPendingCurseForgeManualDownloadFile,
	listPendingCurseForgeManualDownloads,
} from '@/helpers/curseforge'
import {
	type CurseForgeManualDownloadItem,
	getCurseForgeManualDownloadUrl,
} from '@/helpers/curseforge-manual'
import { getMissingContentScannerSettings } from '@/helpers/downloads-scanner'
import { instance_listener } from '@/helpers/events.js'
import { get_content_snapshot } from '@/helpers/instance'
import { get_instance_worlds } from '@/helpers/worlds'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const messages = defineMessages({
	header: {
		id: 'app.curseforge.manual-downloads.header',
		defaultMessage: 'Complete CurseForge downloads',
	},
	body: {
		id: 'app.curseforge.manual-downloads.body',
		defaultMessage:
			'Installed {installed, number} files automatically. {manual, number} files require browser download.',
	},
	existingBody: {
		id: 'app.curseforge.manual-downloads.existing-body',
		defaultMessage:
			'{manual, number} files still require browser download. Downloaded files will be verified and imported automatically.',
	},
	projectFile: {
		id: 'app.curseforge.manual-downloads.project-file',
		defaultMessage: 'Project {projectId} / File {fileId}',
	},
	open: {
		id: 'app.curseforge.manual-downloads.open',
		defaultMessage: 'Download',
	},
	openAll: {
		id: 'app.curseforge.manual-downloads.open-all',
		defaultMessage: 'Open missing',
	},
	chooseFile: {
		id: 'app.curseforge.manual-downloads.choose-file',
		defaultMessage: 'Choose local file',
	},
	viewInstance: {
		id: 'app.curseforge.manual-downloads.view-instance',
		defaultMessage: 'View instance',
	},
	automaticImport: {
		id: 'app.curseforge.manual-downloads.automatic-import',
		defaultMessage: 'Automatic import',
	},
	allImported: {
		id: 'app.curseforge.manual-downloads.all-imported',
		defaultMessage: 'All files imported',
	},
	checkingDownloads: {
		id: 'app.curseforge.manual-downloads.checking-downloads',
		defaultMessage: 'Checking the monitored import folder...',
	},
	watchingDownloads: {
		id: 'app.curseforge.manual-downloads.watching-downloads',
		defaultMessage: 'Watching {path}. Files are verified before import.',
	},
	downloadsUnavailable: {
		id: 'app.curseforge.manual-downloads.downloads-unavailable',
		defaultMessage: 'The monitored import folder is unavailable.',
	},
	automaticImportDisabled: {
		id: 'app.curseforge.manual-downloads.automatic-import-disabled',
		defaultMessage: 'Automatic import is disabled in Resource Management settings.',
	},
	scannerFailed: {
		id: 'app.curseforge.manual-downloads.scanner-failed',
		defaultMessage: 'Automatic import check failed; retrying.',
	},
	importComplete: {
		id: 'app.curseforge.manual-downloads.import-complete',
		defaultMessage: 'Imported into the instance',
	},
	waiting: {
		id: 'app.curseforge.manual-downloads.waiting',
		defaultMessage: 'Waiting for download',
	},
	retrying: {
		id: 'app.curseforge.manual-downloads.retrying',
		defaultMessage: 'File verification failed. Choose the required file.',
	},
	stateChanged: {
		id: 'app.curseforge.manual-downloads.state-changed',
		defaultMessage: 'Download state changed. Waiting for synchronization.',
	},
})

const emit = defineEmits<{
	(e: 'view-instance', instanceId: string): void
	(e: 'imported', instanceId: string, imports: CurseForgeManualDownloadImport[]): void
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const items = ref<CurseForgeManualDownloadItem[]>([])
const candidateItems = ref<CurseForgeManualDownloadItem[]>([])
const installed = ref<number | null>(null)
const instanceId = ref<string | null>(null)
const scanning = ref(false)
const scanError = ref(false)
const downloadDirectory = ref<string | null>(null)
const scannerEnabled = ref(true)
const scanDirectory = ref<string | null>(null)
const importedKeys = ref(new Set<string>())
const inconsistentKeys = ref(new Set<string>())
const errorKeys = ref(new Set<string>())
const busyKeys = ref(new Set<string>())
let scannerActive = false
let scanGeneration = 0
let reconciliationGeneration = 0
let scanInFlight: Promise<CurseForgeManualDownloadScanResult> | undefined
let scanInterval: ReturnType<typeof setInterval> | null = null
let unlistenInstances: (() => void) | null = null

const remainingCount = computed(
	() => items.value.filter((item) => !importedKeys.value.has(itemKey(item))).length,
)
const scannerStatus = computed(() => {
	if (remainingCount.value === 0) return formatMessage(messages.importComplete)
	if (!scannerEnabled.value) return formatMessage(messages.automaticImportDisabled)
	if (scanError.value) return formatMessage(messages.scannerFailed)
	if (downloadDirectory.value) {
		return formatMessage(messages.watchingDownloads, { path: downloadDirectory.value })
	}
	if (scanning.value) return formatMessage(messages.checkingDownloads)
	return formatMessage(messages.downloadsUnavailable)
})

function itemKey(item: Pick<CurseForgeManualDownloadItem, 'projectId' | 'fileId'>) {
	return `${item.projectId}:${item.fileId}`
}

function isImported(item: CurseForgeManualDownloadItem) {
	return importedKeys.value.has(itemKey(item))
}

function itemStatus(item: CurseForgeManualDownloadItem) {
	if (isImported(item)) return formatMessage(messages.importComplete)
	if (errorKeys.value.has(itemKey(item))) return formatMessage(messages.retrying)
	if (inconsistentKeys.value.has(itemKey(item))) return formatMessage(messages.stateChanged)
	return formatMessage(messages.waiting)
}

function show(payload: {
	items: CurseForgeManualDownloadItem[]
	installed?: number
	instanceId?: string | null
}) {
	stopScanning()
	scannerActive = true
	const seededItems = [...new Map(payload.items.map((item) => [itemKey(item), item])).values()]
	candidateItems.value = seededItems
	items.value = seededItems
	installed.value = payload.installed ?? null
	instanceId.value = payload.instanceId ?? null
	downloadDirectory.value = null
	scanError.value = false
	importedKeys.value = new Set()
	inconsistentKeys.value = new Set()
	errorKeys.value = new Set()
	busyKeys.value = new Set()
	const scannerSettings = getMissingContentScannerSettings()
	scannerEnabled.value = scannerSettings.enabled
	scanDirectory.value = scannerSettings.directory
	modal.value?.show()
	void reconcileManualDownloadState().catch(handleError)
	if (scannerEnabled.value) {
		void scanDownloads()
		scanInterval = setInterval(() => {
			if (scannerActive) void scanDownloads()
		}, 3000)
	}
}

async function reconcileManualDownloadState() {
	const currentInstanceId = instanceId.value
	if (!scannerActive || !currentInstanceId) return
	const generation = ++reconciliationGeneration
	const hasWorldCandidates = candidateItems.value.some((item) => item.projectType === 'world')
	const [pending, snapshot, worlds] = await Promise.all([
		listPendingCurseForgeManualDownloads(currentInstanceId),
		get_content_snapshot(currentInstanceId),
		hasWorldCandidates ? get_instance_worlds(currentInstanceId) : Promise.resolve([]),
	])
	if (
		!scannerActive ||
		currentInstanceId !== instanceId.value ||
		generation !== reconciliationGeneration
	) {
		return
	}

	const pendingByKey = new Map(pending.map((item) => [itemKey(item), item]))
	const candidateByKey = new Map(candidateItems.value.map((item) => [itemKey(item), item]))
	for (const [key, item] of pendingByKey) candidateByKey.set(key, item)
	const nextCandidates = [...candidateByKey.values()]
	const nextItems = nextCandidates.map((item) => pendingByKey.get(itemKey(item)) ?? item)
	const materializedByKey = new Map<string, string>()
	for (const item of snapshot.items) {
		if (
			item.provider === 'curseforge' &&
			item.providerProjectId != null &&
			item.providerReleaseId != null &&
			item.materializationState === 'present' &&
			item.content != null
		) {
			materializedByKey.set(
				`${item.providerProjectId}:${item.providerReleaseId}`,
				item.expectedRelativePath,
			)
		}
	}

	const nextImported = new Set<string>()
	const nextInconsistent = new Set<string>()
	const newlyImported: CurseForgeManualDownloadImport[] = []
	for (const item of nextItems) {
		const key = itemKey(item)
		if (pendingByKey.has(key)) continue
		if (item.projectType === 'world') {
			const worldName = item.fileName.replace(/\.zip$/i, '')
			const importedWorld = worlds.some(
				(world) => world.type === 'singleplayer' && world.path === worldName,
			)
			if (importedWorld) {
				nextImported.add(key)
				if (!importedKeys.value.has(key)) {
					newlyImported.push({
						projectId: item.projectId,
						fileId: item.fileId,
						relativePath: `saves/${worldName}`,
					})
				}
				continue
			}
		}
		const relativePath = materializedByKey.get(key)
		if (relativePath == null) {
			nextInconsistent.add(key)
			continue
		}
		nextImported.add(key)
		if (!importedKeys.value.has(key)) {
			newlyImported.push({ projectId: item.projectId, fileId: item.fileId, relativePath })
		}
	}

	candidateItems.value = nextCandidates
	items.value = nextItems
	importedKeys.value = nextImported
	inconsistentKeys.value = nextInconsistent
	errorKeys.value = new Set([...errorKeys.value].filter((key) => pendingByKey.has(key)))
	if (newlyImported.length > 0) emit('imported', currentInstanceId, newlyImported)
}

function hide() {
	modal.value?.hide()
}

function stopScanning() {
	scannerActive = false
	scanGeneration += 1
	reconciliationGeneration += 1
	if (scanInterval != null) {
		clearInterval(scanInterval)
		scanInterval = null
	}
}

async function scanDownloads(): Promise<void> {
	if (!scannerEnabled.value) return
	if (scanInFlight) {
		await scanInFlight.catch(() => undefined)
		return
	}

	const currentInstanceId = instanceId.value
	if (!currentInstanceId) return
	const generation = scanGeneration
	if (remainingCount.value === 0) return

	scanning.value = true
	const operation = importCurseForgeManualDownloads(currentInstanceId, scanDirectory.value)
	scanInFlight = operation
	try {
		const result = await operation
		if (generation !== scanGeneration) return
		scanError.value = false
		downloadDirectory.value = result.downloadDirectory ?? null
		errorKeys.value = new Set(result.errors.map((item) => `${item.projectId}:${item.fileId}`))
		await reconcileManualDownloadState().catch(handleError)
	} catch {
		if (generation !== scanGeneration) return
		scanError.value = true
		errorKeys.value = new Set(
			items.value
				.filter((item) => !isImported(item) && !inconsistentKeys.value.has(itemKey(item)))
				.map((item) => itemKey(item)),
		)
	} finally {
		if (scanInFlight === operation) scanInFlight = undefined
		if (generation === scanGeneration) {
			scanning.value = false
		} else if (!scanInFlight) {
			scanning.value = false
		}
	}
}

async function openOne(item: CurseForgeManualDownloadItem) {
	await openUrl(getCurseForgeManualDownloadUrl(item))
}

async function chooseLocalFile(item: CurseForgeManualDownloadItem) {
	const currentInstanceId = instanceId.value
	const key = itemKey(item)
	if (!currentInstanceId || busyKeys.value.has(key)) return
	const selected = await open({ multiple: false })
	const sourcePath = typeof selected === 'string' ? selected : null
	if (!sourcePath) return
	busyKeys.value = new Set(busyKeys.value).add(key)
	try {
		await importPendingCurseForgeManualDownloadFile(
			currentInstanceId,
			item.projectId,
			item.fileId,
			sourcePath,
		)
		await reconcileManualDownloadState().catch(handleError)
	} catch (error) {
		const errorKind = classifyCurseForgeManualDownloadImportError(error)
		if (errorKind === 'verification_failed') {
			const nextErrors = new Set(errorKeys.value)
			nextErrors.add(key)
			errorKeys.value = nextErrors
			const nextInconsistent = new Set(inconsistentKeys.value)
			nextInconsistent.delete(key)
			inconsistentKeys.value = nextInconsistent
		} else if (errorKind === 'not_pending') {
			const nextErrors = new Set(errorKeys.value)
			nextErrors.delete(key)
			errorKeys.value = nextErrors
			await reconcileManualDownloadState().catch(handleError)
		} else {
			handleError(error)
		}
	} finally {
		const nextBusy = new Set(busyKeys.value)
		nextBusy.delete(key)
		busyKeys.value = nextBusy
	}
}

async function openAll() {
	for (const item of items.value) {
		if (isImported(item)) continue
		await openOne(item)
	}
}

function goToInstance() {
	if (!instanceId.value) return
	hide()
	emit('view-instance', instanceId.value)
}

onMounted(() => {
	void instance_listener(async (event: { event: string; instance_id: string }) => {
		if (
			event.event === 'content_changed' &&
			event.instance_id === instanceId.value &&
			scannerActive
		) {
			await reconcileManualDownloadState().catch(() => {
				scanError.value = true
			})
		}
	})
		.then((unlisten) => {
			unlistenInstances = unlisten
		})
		.catch(() => undefined)
})

onUnmounted(() => {
	stopScanning()
	unlistenInstances?.()
})

defineExpose({
	show,
	hide,
})
</script>
