import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type {
	StorageNode,
	StoragePath,
	StorageTree,
} from '@/components/ui/settings/storage/storageData'

export type StorageScanEvent =
	| { kind: 'started' }
	| { kind: 'category'; payload: { category: StorageNode } }
	| { kind: 'complete'; payload: { tree: StorageTree } }
	| { kind: 'error'; payload: { message: string } }

export interface StorageOpenResult {
	opened: string[]
	failed: { path: string; reason: string }[]
}

export function startStorageScan(force: boolean): Promise<void> {
	return invoke('plugin:storage|storage_scan_start', { force })
}

export function openStoragePaths(paths: StoragePath[]): Promise<StorageOpenResult> {
	return invoke('plugin:storage|storage_open_paths', { paths })
}

export function listenStorageScan(handler: (event: StorageScanEvent) => void): Promise<UnlistenFn> {
	return listen('storage-scan', (event) => handler(event.payload as StorageScanEvent))
}
