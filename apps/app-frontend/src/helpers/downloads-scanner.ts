export interface DownloadsScanLoopOptions<T> {
    scan: () => Promise<T>
    onResult: (result: T) => void
    onError?: (error: unknown) => void
    onScanningChange?: (scanning: boolean) => void
    intervalMs?: number
    schedule?: (callback: () => void, delay: number) => unknown
    cancelSchedule?: (timer: unknown) => void
}

export interface MissingContentScannerSettings {
    enabled: boolean
    directory: string | null
}

type ScannerSettingsStorage = Pick<Storage, 'getItem' | 'setItem'>

const MISSING_CONTENT_SCANNER_SETTINGS_KEY = 'axolotl-missing-content-scanner'

export function getMissingContentScannerSettings(
    storage: ScannerSettingsStorage = localStorage,
): MissingContentScannerSettings {
    try {
        const parsed = JSON.parse(storage.getItem(MISSING_CONTENT_SCANNER_SETTINGS_KEY) ?? '{}')
        return {
            enabled: parsed.enabled !== false,
            directory:
                typeof parsed.directory === 'string' && parsed.directory.trim()
                    ? parsed.directory
                    : null,
        }
    } catch {
        return { enabled: true, directory: null }
    }
}

export function setMissingContentScannerSettings(
    settings: MissingContentScannerSettings,
    storage: ScannerSettingsStorage = localStorage,
) {
    storage.setItem(
        MISSING_CONTENT_SCANNER_SETTINGS_KEY,
        JSON.stringify({
            enabled: settings.enabled,
            directory: settings.directory?.trim() || null,
        }),
    )
}

export type DownloadsScannerPresentationPhase =
    | 'idle'
    | 'monitoring'
    | 'importing'
    | 'verifying'
    | 'waiting_for_stability'
    | 'rejected'
    | 'imported'
    | 'error'
    | 'unavailable'

export interface DownloadsScannerPresentationState {
    phase: DownloadsScannerPresentationPhase
    downloadDirectory: string | null
    importedCount: number
    pendingCandidates: number
    importingItemIds: string[]
    rejectedItemIds: string[]
    verifyingItemIds: string[]
}

export type DownloadsScannerPresentationEvent =
    | { type: 'reset' }
    | { type: 'scan_failed' }
    | {
          type: 'items_updated'
          items: Array<{ id: string; status: string }>
      }
    | {
          type: 'scan_result'
          downloadDirectory: string | null
          importedItemIds: string[]
          rejectedItemIds: string[]
          pendingCandidates: number
          hasErrors: boolean
          items: Array<{ id: string; status: string }>
      }
    | { type: 'items_resolved'; itemIds: string[] }

export function createDownloadsScannerPresentationState(): DownloadsScannerPresentationState {
    return {
        phase: 'idle',
        downloadDirectory: null,
        importedCount: 0,
        pendingCandidates: 0,
        importingItemIds: [],
        rejectedItemIds: [],
        verifyingItemIds: [],
    }
}

function itemIdsWithStatus(items: Array<{ id: string; status: string }>, status: string) {
    return items.filter((item) => item.status === status).map((item) => item.id)
}

function withoutItemIds(current: string[], removed: string[]) {
    const removedSet = new Set(removed)
    return current.filter((itemId) => !removedSet.has(itemId))
}

function withPhase(
    state: Omit<DownloadsScannerPresentationState, 'phase'>,
    options: { failed?: boolean } = {},
): DownloadsScannerPresentationState {
    let phase: DownloadsScannerPresentationPhase
    if (state.importingItemIds.length > 0) phase = 'importing'
    else if (state.verifyingItemIds.length > 0) phase = 'verifying'
    else if (state.pendingCandidates > 0) phase = 'waiting_for_stability'
    else if (state.rejectedItemIds.length > 0) phase = 'rejected'
    else if (state.importedCount > 0) phase = 'imported'
    else if (options.failed) phase = 'error'
    else if (state.downloadDirectory) phase = 'monitoring'
    else phase = 'unavailable'
    return { ...state, phase }
}

export function reduceDownloadsScannerPresentation(
    state: DownloadsScannerPresentationState,
    event: DownloadsScannerPresentationEvent,
): DownloadsScannerPresentationState {
    if (event.type === 'reset') return createDownloadsScannerPresentationState()

    const current = {
        downloadDirectory: state.downloadDirectory,
        importedCount: state.importedCount,
        pendingCandidates: state.pendingCandidates,
        importingItemIds: [...state.importingItemIds],
        rejectedItemIds: [...state.rejectedItemIds],
        verifyingItemIds: [...state.verifyingItemIds],
    }

    if (event.type === 'scan_failed') {
        if (
            state.importingItemIds.length > 0 ||
            state.verifyingItemIds.length > 0 ||
            state.pendingCandidates > 0 ||
            state.rejectedItemIds.length > 0 ||
            state.importedCount > 0
        ) {
            return state
        }
        return withPhase(current, { failed: true })
    }

    if (event.type === 'items_updated') {
        const importingItemIds = itemIdsWithStatus(event.items, 'writing')
        const verifyingItemIds = itemIdsWithStatus(event.items, 'verifying')
        if (importingItemIds.length === 0 && verifyingItemIds.length === 0) return state
        current.importingItemIds = importingItemIds
        current.verifyingItemIds = verifyingItemIds
        current.rejectedItemIds = withoutItemIds(current.rejectedItemIds, [
            ...importingItemIds,
            ...verifyingItemIds,
        ])
        return withPhase(current)
    }

    if (event.type === 'items_resolved') {
        current.pendingCandidates = 0
        current.rejectedItemIds = withoutItemIds(current.rejectedItemIds, event.itemIds)
        current.importingItemIds = withoutItemIds(current.importingItemIds, event.itemIds)
        current.verifyingItemIds = withoutItemIds(current.verifyingItemIds, event.itemIds)
        return withPhase(current)
    }

    current.downloadDirectory = event.downloadDirectory
    current.pendingCandidates = event.pendingCandidates
    current.importingItemIds = itemIdsWithStatus(event.items, 'writing')
    current.verifyingItemIds = itemIdsWithStatus(event.items, 'verifying')
    current.rejectedItemIds = withoutItemIds(event.rejectedItemIds, [
        ...event.importedItemIds,
        ...current.importingItemIds,
        ...current.verifyingItemIds,
    ])
    current.importedCount += event.importedItemIds.length
    return withPhase(current, { failed: event.hasErrors })
}

export function createDownloadsScanLoop<T>(options: DownloadsScanLoopOptions<T>) {
    const schedule = options.schedule ?? ((callback, delay) => setTimeout(callback, delay))
    const cancelSchedule = options.cancelSchedule ?? ((timer) => clearTimeout(timer as number))
    const intervalMs = options.intervalMs ?? 3000
    let active = false
    let generation = 0
    let inFlight = false
    let timer: unknown

    function clearTimer() {
        if (timer != null) cancelSchedule(timer)
        timer = undefined
    }

    function scheduleNext(delay: number) {
        if (!active) return
        clearTimer()
        timer = schedule(() => {
            timer = undefined
            void runNow()
        }, delay)
    }

    function start() {
        stop()
        active = true
        generation += 1
        scheduleNext(0)
    }

    function stop() {
        active = false
        generation += 1
        clearTimer()
        options.onScanningChange?.(false)
    }

    async function runNow() {
        if (!active) return
        if (inFlight) {
            scheduleNext(100)
            return
        }
        const runGeneration = generation
        inFlight = true
        options.onScanningChange?.(true)
        try {
            const result = await options.scan()
            if (active && runGeneration === generation) options.onResult(result)
        } catch (error) {
            if (active && runGeneration === generation) options.onError?.(error)
        } finally {
            inFlight = false
            if (active && runGeneration === generation) {
                options.onScanningChange?.(false)
                scheduleNext(intervalMs)
            }
        }
    }

    return {
        start,
        stop,
        runNow,
        isActive: () => active,
    }
}
