import { computeServerStatus, type ServerStatus } from '@modrinth/server'
import { injectNotificationManager } from '@modrinth/ui'
import { computed, reactive, ref } from 'vue'

import { serverEventListener, type ServerExitReason, type ServerInfoData, servers } from '@/helpers/servers'

const LOG_CAPACITY = 5000

/** Reacts to a classified server self-exit, e.g. opening the EULA dialog. */
type ServerExitReasonHandler = (serverId: string, reason: ServerExitReason) => void
let exitReasonHandler: ServerExitReasonHandler | null = null

/**
 * Registers the single handler invoked when a server exits with a classified
 * reason. Returns a disposer that only clears the handler while it is still
 * the registered one.
 */
export function setServerExitReasonHandler(handler: ServerExitReasonHandler | null) {
	const registered = handler
	exitReasonHandler = handler
	return () => {
		if (exitReasonHandler === registered) exitReasonHandler = null
	}
}

const serverList = ref<ServerInfoData[]>([])
const logLines = reactive<Record<string, string[]>>({})
const isRefreshing = ref(false)
let listenerPromise: Promise<() => void> | null = null

export interface ServerView extends ServerInfoData {
	status: ServerStatus
}

export function serverStatus(server: ServerInfoData): ServerStatus {
	return computeServerStatus({
		manifest: { id: server.id },
		isRunning: server.running,
		isStarting: false,
		lastExitWasCrash: server.lastExitCrashed,
		eulaAccepted: server.eulaAccepted,
		eulaFileExists: server.eulaExists,
	})
}

async function appendLog(serverId: string, line: string) {
	const lines = (logLines[serverId] ??= [])
	lines.push(line)
	if (lines.length > LOG_CAPACITY) lines.splice(0, lines.length - LOG_CAPACITY)
}

async function ensureListener() {
	if (!listenerPromise) {
		listenerPromise = serverEventListener((serverId, payload) => {
			if (payload.event === 'log') {
				void appendLog(serverId, payload.line)
			} else if (payload.event === 'started') {
				void refresh()
			} else if (payload.event === 'stopped') {
				void refresh()
				if (payload.reason) exitReasonHandler?.(serverId, payload.reason)
			}
		})
	}
	return listenerPromise
}

export async function hydrateLog(serverId: string) {
	if (logLines[serverId]?.length) return
	try {
		const buffer = await servers.getLogBuffer(serverId)
		if (buffer.length > 0) logLines[serverId] = [...buffer]
	} catch {
		// Server may not have logs yet
	}
}

export async function refresh(): Promise<void> {
	if (isRefreshing.value) return
	isRefreshing.value = true
	try {
		await ensureListener()
		serverList.value = await servers.list()
	} finally {
		isRefreshing.value = false
	}
}

export function useServers() {
	const { handleError } = injectNotificationManager()

	const serverViews = computed<ServerView[]>(() =>
		serverList.value.map((server) => ({ ...server, status: serverStatus(server) })),
	)

	async function run(action: () => Promise<unknown>): Promise<boolean> {
		try {
			await action()
			return true
		} catch (error) {
			handleError(error)
			return false
		}
	}

	async function refreshList() {
		await run(refresh)
	}

	async function startServer(serverId: string) {
		await ensureListener()
		logLines[serverId] = []
		const ok = await run(() => servers.start(serverId))
		if (ok) await refresh()
		return ok
	}

	async function stopServer(serverId: string) {
		return run(() => servers.stop(serverId))
	}

	async function killServer(serverId: string) {
		return run(() => servers.kill(serverId))
	}

	async function deleteServer(serverId: string) {
		const ok = await run(() => servers.delete(serverId))
		if (ok) {
			logLines[serverId] = []
			await refresh()
		}
		return ok
	}

	async function sendCommand(serverId: string, command: string) {
		return run(() => servers.sendCommand(serverId, command))
	}

	return {
		servers: serverViews,
		rawServers: serverList,
		logLines,
		isRefreshing,
		refresh: refreshList,
		startServer,
		stopServer,
		killServer,
		deleteServer,
		sendCommand,
	}
}
