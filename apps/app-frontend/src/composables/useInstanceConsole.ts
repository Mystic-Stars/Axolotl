import { createConsoleState } from '@modrinth/ui'

import {
    clear_log_buffer,
    get_live_log_buffer,
    get_logs,
    get_minecraft_latest_log_cursor,
} from '@/helpers/logs'

import { mergeLiveLogHistory } from './console-history'

type ConsoleState = ReturnType<typeof createConsoleState>

interface LogEntry {
    filename: string
    name?: string
    log_type: string
    output?: string | null
    age?: number
    live?: boolean
}

interface InstanceConsoleEntry {
    liveConsole: ConsoleState
    historicalConsole: ConsoleState
    historicalCache: Map<string, string>
    logList: LogEntry[] | null
    liveHistoryHydration: Promise<void> | null
}

const instances = new Map<string, InstanceConsoleEntry>()

function getOrCreate(instanceId: string): InstanceConsoleEntry {
    let entry = instances.get(instanceId)
    if (entry) return entry

    entry = {
        liveConsole: createConsoleState(),
        historicalConsole: createConsoleState(),
        historicalCache: new Map(),
        logList: null,
        liveHistoryHydration: null,
    }
    instances.set(instanceId, entry)
    return entry
}

async function hydrate(instanceId: string): Promise<void> {
    const entry = getOrCreate(instanceId)
    if (entry.liveConsole.output.value.length > 0) return

    if (entry.liveHistoryHydration) {
        return entry.liveHistoryHydration
    }

    const hydration = (async () => {
        const [latestLog, buffer] = await Promise.all([
            get_minecraft_latest_log_cursor(instanceId, 0)
                .then((result) => result.output)
                .catch(() => ''),
            get_live_log_buffer(instanceId),
        ])

        if (entry.liveConsole.output.value.length > 0) return

        const history = mergeLiveLogHistory(latestLog, buffer)
        if (history) {
            await entry.liveConsole.addLegacyLog(history)
        }
    })()

    entry.liveHistoryHydration = hydration
    try {
        await hydration
    } finally {
        if (entry.liveHistoryHydration === hydration) {
            entry.liveHistoryHydration = null
        }
    }
}

async function getHistoricalLogs(instanceId: string): Promise<LogEntry[]> {
    const entry = getOrCreate(instanceId)
    if (entry.logList) return entry.logList

    const logs: LogEntry[] = await get_logs(instanceId, true)
    entry.logList = logs

    for (const log of logs) {
        if (log.output) {
            entry.historicalCache.set(log.filename, log.output)
        }
    }

    return logs
}

function getHistoricalContent(instanceId: string, filename: string): string | undefined {
    return instances.get(instanceId)?.historicalCache.get(filename)
}

function invalidate(instanceId: string): void {
    const entry = instances.get(instanceId)
    if (!entry) return
    entry.historicalCache.clear()
    entry.logList = null
}

async function clearLive(instanceId: string): Promise<void> {
    const entry = getOrCreate(instanceId)
    entry.liveConsole.clear()
    await clear_log_buffer(instanceId).catch(() => {})
}

async function destroy(instanceId: string): Promise<void> {
    instances.delete(instanceId)
    await clear_log_buffer(instanceId).catch(() => {})
}

export function useInstanceConsole(instanceId: string) {
    const entry = getOrCreate(instanceId)
    return {
        liveConsole: entry.liveConsole,
        historicalConsole: entry.historicalConsole,
        hydrate: () => hydrate(instanceId),
        getHistoricalLogs: () => getHistoricalLogs(instanceId),
        getHistoricalContent: (filename: string) => getHistoricalContent(instanceId, filename),
        invalidate: () => invalidate(instanceId),
        clearLive: () => clearLive(instanceId),
        destroy: () => destroy(instanceId),
    }
}
