import { createContext } from '@modrinth/ui'
import { computed, type ComputedRef, type Ref, ref } from 'vue'

import { download_request_listener, install_job_listener, loading_listener } from '@/helpers/events'
import {
	download_history_clear,
	download_job_cancel,
	download_job_delete,
	download_job_list,
	download_job_retry,
	type DownloadRequestUpdate,
	type InstallJobSnapshot,
} from '@/helpers/install'
import type { LoadingBar } from '@/helpers/state'
import { progress_bars_list } from '@/helpers/state'

const activeStatuses = new Set(['queued', 'running', 'canceling', 'waiting_for_user'])
const downloadBarTypes = new Set([
	'java_download',
	'pack_file_download',
	'pack_download',
	'minecraft_download',
	'instance_update',
	'launcher_update',
])

export interface DownloadManager {
	jobs: Ref<InstallJobSnapshot[]>
	legacyDownloads: Ref<LoadingBar[]>
	activeJobs: ComputedRef<InstallJobSnapshot[]>
	historyJobs: ComputedRef<InstallJobSnapshot[]>
	activeCount: ComputedRef<number>
	queuedCount: ComputedRef<number>
	start: () => Promise<void>
	refresh: () => Promise<void>
	cancel: (jobId: string) => Promise<void>
	retry: (jobId: string) => Promise<void>
	remove: (jobId: string) => Promise<void>
	clearHistory: () => Promise<void>
	dispose: () => void
}

export function createDownloadManager(handleError: (error: unknown) => void): DownloadManager {
	const jobs = ref<InstallJobSnapshot[]>([])
	const legacyDownloads = ref<LoadingBar[]>([])
	let started = false
	let disposed = false
	let unlistenJobs: (() => void) | null = null
	let unlistenRequests: (() => void) | null = null
	let unlistenLoading: (() => void) | null = null
	let initializing = false
	const pendingInitialUpdates: Array<
		| { kind: 'job'; job: InstallJobSnapshot }
		| { kind: 'request'; update: DownloadRequestUpdate }
	> = []
	const pendingRequestUpdates = new Map<string, DownloadRequestUpdate[]>()

	function setJob(job: InstallJobSnapshot) {
		if (initializing) {
			pendingInitialUpdates.push({ kind: 'job', job })
			return
		}
		const current = jobs.value.find((candidate) => candidate.job_id === job.job_id)
		if (current && current.modified.localeCompare(job.modified) > 0) return
		jobs.value = [job, ...jobs.value.filter((candidate) => candidate.job_id !== job.job_id)].sort(
			(a, b) => b.created.localeCompare(a.created),
		)
		const pending = pendingRequestUpdates.get(job.job_id)
		if (pending) {
			pendingRequestUpdates.delete(job.job_id)
			for (const update of pending) updateRequest(update)
		}
	}

	function updateRequest(update: DownloadRequestUpdate) {
		if (initializing) {
			pendingInitialUpdates.push({ kind: 'request', update })
			return
		}
		const jobIndex = jobs.value.findIndex((job) => job.job_id === update.job_id)
		if (jobIndex === -1) {
			const pending = pendingRequestUpdates.get(update.job_id) ?? []
			pending.push(update)
			pendingRequestUpdates.set(update.job_id, pending)
			return
		}

		const job = jobs.value[jobIndex]
		const itemIndex = job.items.findIndex((item) => item.id === update.id)
		const current = itemIndex === -1 ? null : job.items[itemIndex]
		let item: InstallJobSnapshot['items'][number]

		switch (update.type) {
			case 'started':
				item = {
					...(current ?? {
						id: update.id,
						name: update.name,
						bytes_downloaded: 0,
					}),
					status: 'downloading',
					bytes_total: current?.bytes_total ?? update.bytes_total,
					attempt: update.attempt,
					max_attempts: update.max_attempts,
					error: null,
					request_url: update.url,
					source: update.source,
				}
				break
			case 'finished':
				if (!current) return
				item = {
					...current,
					status: 'completed',
					bytes_downloaded: update.bytes,
					bytes_total: current.bytes_total ?? update.bytes,
				}
				break
			case 'failed':
				if (!current) return
				item = { ...current, status: 'failed' }
				break
		}

		const items = [...job.items]
		if (itemIndex === -1) items.push(item)
		else items[itemIndex] = item
		const nextJobs = [...jobs.value]
		nextJobs[jobIndex] = { ...job, items }
		jobs.value = nextJobs
	}

	async function refresh() {
		const page = await download_job_list({ limit: 250 }).catch((error) => {
			handleError(error)
			return null
		})
		if (page && !disposed) jobs.value = page.jobs
	}

	async function refreshLegacyDownloads() {
		const bars = await progress_bars_list().catch((error) => {
			handleError(error)
			return {}
		})
		legacyDownloads.value = Object.values(bars)
			.filter((bar) => downloadBarTypes.has(bar.bar_type?.type ?? ''))
			.map((bar) => ({
				...bar,
				title: bar.title ?? bar.bar_type?.pack_name ?? bar.bar_type?.instance_name ?? bar.message,
			}))
	}

	async function start() {
		if (started || disposed) return
		started = true
		initializing = true
		unlistenRequests = await download_request_listener((update: DownloadRequestUpdate) =>
			updateRequest(update),
		)
		unlistenJobs = await install_job_listener((job: InstallJobSnapshot) => setJob(job))
		unlistenLoading = await loading_listener(() => void refreshLegacyDownloads())
		await Promise.all([refresh(), refreshLegacyDownloads()])
		initializing = false
		for (const update of pendingInitialUpdates.splice(0)) {
			if (update.kind === 'job') setJob(update.job)
			else updateRequest(update.update)
		}
	}

	async function cancel(jobId: string) {
		setJob(await download_job_cancel(jobId))
	}

	async function retry(jobId: string) {
		setJob(await download_job_retry(jobId))
	}

	async function remove(jobId: string) {
		await download_job_delete(jobId)
		jobs.value = jobs.value.filter((job) => job.job_id !== jobId)
	}

	async function clearHistory() {
		await download_history_clear()
		jobs.value = jobs.value.filter((job) => activeStatuses.has(job.status))
	}

	const activeJobs = computed(() => jobs.value.filter((job) => activeStatuses.has(job.status)))
	const historyJobs = computed(() => jobs.value.filter((job) => !activeStatuses.has(job.status)))

	return {
		jobs,
		legacyDownloads,
		activeJobs,
		historyJobs,
		activeCount: computed(() => activeJobs.value.length + legacyDownloads.value.length),
		queuedCount: computed(() => jobs.value.filter((job) => job.status === 'queued').length),
		start,
		refresh,
		cancel,
		retry,
		remove,
		clearHistory,
		dispose() {
			disposed = true
			initializing = false
			pendingInitialUpdates.length = 0
			pendingRequestUpdates.clear()
			unlistenJobs?.()
			unlistenRequests?.()
			unlistenLoading?.()
		},
	}
}

export const [injectDownloadManager, provideDownloadManager] = createContext<DownloadManager>(
	'root',
	'downloadManager',
)
