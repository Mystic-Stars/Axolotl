import { computeServerStatus, type ServerStatus } from '@modrinth/server'
import { injectNotificationManager } from '@modrinth/ui'
import { computed, reactive, ref } from 'vue'

import {
    base64ToBytes,
    serverEventListener,
    type ServerExitReason,
    type ServerInfoData,
    servers,
} from '@/helpers/servers'

import { ServerConsoleBuffer } from './server-console-buffer'

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
const consoleOutputListeners = new Map<string, Set<(data: Uint8Array) => void>>()
const consoleOutputBuffers = new Map<string, ServerConsoleBuffer>()
const CONSOLE_OUTPUT_CAPACITY = 64 * 1024

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
            } else if (payload.event === 'console_output') {
                const data = base64ToBytes(payload.data)
                const buffer =
                    consoleOutputBuffers.get(serverId) ??
                    new ServerConsoleBuffer(CONSOLE_OUTPUT_CAPACITY)
                buffer.push(data)
                consoleOutputBuffers.set(serverId, buffer)
                for (const listener of consoleOutputListeners.get(serverId) ?? []) {
                    listener(data)
                }
            } else if (payload.event === 'started') {
                void refresh()
            } else if (payload.event === 'stopped') {
                consoleOutputBuffers.delete(serverId)
                void refresh()
                if (payload.reason) exitReasonHandler?.(serverId, payload.reason)
            }
        })
    }
    return listenerPromise
}

export function subscribeServerConsoleOutput(
    serverId: string,
    listener: (data: Uint8Array) => void,
): () => void {
    const listeners = consoleOutputListeners.get(serverId) ?? new Set()
    listeners.add(listener)
    consoleOutputListeners.set(serverId, listeners)
    for (const data of consoleOutputBuffers.get(serverId)?.values() ?? []) listener(data)
    void ensureListener()
    return () => {
        listeners.delete(listener)
        if (listeners.size === 0) consoleOutputListeners.delete(serverId)
    }
}

export async function hydrateLog(serverId: string) {
    try {
        const buffer = await servers.getLogBuffer(serverId)
        // The backend log buffer is the authoritative, lossless source: the
        // per-line `server` events can be dropped in bursts (e.g. the server's
        // startup or a `help` dump), but every line is still persisted there.
        // Reconcile by appending only the lines we haven't displayed yet rather
        // than blindly replacing, so live events and this catch-up stay in sync.
        const current = (logLines[serverId] ??= [])
        if (buffer.length > current.length) {
            for (const line of buffer.slice(current.length)) current.push(line)
        }
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
