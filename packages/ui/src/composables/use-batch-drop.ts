import type { ClassificationChoice, ClassificationResult } from './use-global-drop'

export type BatchDropPhase =
	| 'idle'
	| 'scanning'
	| 'picking-instance'
	| 'picking-world'
	| 'confirming'
	| 'installing'
	| 'done'
	| 'cancelled'

export type BatchDropScanState = 'pending' | 'scanning' | 'done' | 'skipped' | 'error'
export type BatchDropInstallState = 'queued' | 'processing' | 'success' | 'failed' | 'cancelled'
export type BatchDropResultStatus = 'success' | 'failed' | 'skipped' | 'cancelled'

export interface BatchDropItem {
	id: string
	sourcePath: string
	name: string
	/** Optional source qualifier used to disambiguate identical names (e.g. launcher folder). */
	sourceLabel?: string
	scanState: BatchDropScanState
	classification?: ClassificationResult
	/** Resolved type after confirmation: mod, resource_pack, shader_pack, world_save, litematic, schematic, datapack, modpack, instance. */
	itemType?: string
	innerBase?: string
	launcherType?: string
	basePath?: string
	instanceFolder?: string
	instancePath?: string
	fromZip?: boolean
	tempDir?: string
	reason?: string
	candidates?: string[]
	choices?: ClassificationChoice[]
	confirmedType?: string
	selected?: boolean
	importName?: string
	installState?: BatchDropInstallState
	installError?: string
	symlink?: boolean
	gameVersion?: string | null
	loader?: string | null
	loaderVersion?: string | null
}

export interface BatchDropGroup {
	id: string
	/** Stable type key used by the UI to choose labels/options. */
	type: string
	items: BatchDropItem[]
}

export interface BatchDropResult {
	id: string
	name: string
	status: BatchDropResultStatus
	message?: string
}
