import {
    type DirectLinkSyncReport,
    type ExternalMinecraftRoot,
    sync_direct_links,
} from './instance'

export const DIRECT_LINKS_SYNCED_EVENT = 'axolotl-direct-links-synced'

let requestedRoots: ExternalMinecraftRoot[] = []
let syncWorker: Promise<DirectLinkSyncReport> | undefined
let syncPending = false

/**
 * Serializes direct-link reconciliation across Settings, routing, and focus
 * events. Each request records a fresh snapshot; changes received during an
 * in-flight reconciliation always run immediately afterwards.
 */
export function syncConfiguredDirectLinks(
    roots: readonly ExternalMinecraftRoot[],
): Promise<DirectLinkSyncReport> {
    requestedRoots = roots.map((root) => ({ ...root }))
    syncPending = true
    if (!syncWorker) {
        syncWorker = drainSyncRequests().finally(() => {
            syncWorker = undefined
        })
    }
    return syncWorker
}

async function drainSyncRequests() {
    let latestReport: DirectLinkSyncReport = {
        imported: 0,
        updated: 0,
        removed: 0,
        missing: 0,
        errors: [],
    }
    while (syncPending) {
        syncPending = false
        const report = await sync_direct_links(requestedRoots)
        latestReport = report
        window.dispatchEvent(
            new CustomEvent<DirectLinkSyncReport>(DIRECT_LINKS_SYNCED_EVENT, { detail: report }),
        )
    }
    return latestReport
}
