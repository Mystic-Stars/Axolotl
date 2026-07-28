/**
 * Bridge helpers for the drop classification Tauri commands.
 *
 * These wrap the Tauri `invoke()` calls so the frontend can classify
 * dropped files, scan launcher instances, and detect file locks.
 */
import type { ClassificationResult } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'

/**
 * Information about a process that has a file handle open.
 */
export interface LockingProcess {
	pid: number
	name: string
	path: string
	start_time: string | null
}

/**
 * Classify a dropped file or folder by its path on disk.
 *
 * @param path Absolute filesystem path to the dropped item
 * @returns Classification result indicating what kind of content it is
 */
export function classifyDroppedItem(path: string): Promise<ClassificationResult> {
	return invoke('plugin:drop|drop_classify', { path })
}

/**
 * Classify a dropped ZIP file by extracting it to a temporary directory first.
 *
 * This is a potentially **long-running** operation — the UI MUST prompt the
 * user before calling this, since extraction can take significant time for
 * large archives.
 *
 * Only call this when [`classifyDroppedItem`] returned `Unknown` with a reason
 * containing "extraction".
 *
 * @param path Absolute filesystem path to the ZIP file
 * @returns Classification result after extraction and analysis
 */
export function classifyDroppedItemWithExtraction(path: string): Promise<ClassificationResult> {
	return invoke('plugin:drop|drop_classify_extract', { path })
}

/**
 * Metadata about a single importable instance within a launcher.
 */
export interface ScanInstance {
	name: string
	/** Resolved filesystem path (informational — the backend resolves it) */
	path: string
	/** Minecraft version, if known (empty string otherwise) */
	version: string
	/** Mod loader, if known (e.g. "fabric", "forge"; empty string otherwise) */
	loader: string
}

/**
 * Result of scanning a single launcher type for importable instances.
 */
export interface ScanResult {
	launcherName: string
	launcherType: string
	instances: ScanInstance[]
}

/**
 * Scan a launcher's data directory for importable Minecraft instances.
 *
 * @param launcherType Launcher type name (e.g. "MultiMC", "PrismLauncher", "HMCL")
 * @param basePath    Root directory of the launcher's data
 * @returns List of scan results (one entry per launcher type)
 */
export async function scanLauncherInstances(
	launcherType: string,
	basePath: string,
): Promise<ScanResult[]> {
	const names: string[] = await invoke('plugin:drop|drop_scan_launcher_instances', {
		launcherType,
		basePath,
	})

	return [
		{
			launcherName: launcherType,
			launcherType,
			instances: names.map((name) => ({
				name,
				path: `${basePath}/${name}`,
				version: '',
				loader: '',
			})),
		},
	]
}

/**
 * Detect processes holding a file lock on the given path.
 *
 * @param path Absolute path to the file to check
 * @returns List of locking processes (empty if unavailable or none found)
 */
export function detectFileLock(path: string): Promise<LockingProcess[]> {
	return invoke('plugin:drop|drop_detect_file_lock', { path })
}

/**
 * Extract mod metadata from a JAR file without installing it.
 *
 * @param path Absolute path to the JAR file
 * @returns JSON string of LocalModMetadata, or null if no metadata found
 */
export function extractModMetadata(path: string): Promise<string | null> {
	return invoke('plugin:drop|drop_extract_mod_metadata', { path })
}

/**
 * Metadata extracted from a mod JAR file.
 */
export interface LocalModMetadata {
	mod_id: string
	name?: string
	version?: string
	authors?: string[]
	description?: string
	url?: string
	icon_path?: string
	minecraft_version?: string
	loader_version?: string
	loader?: string
}

export interface ModrinthLookupResult {
	hash: string
	project_id: string
	version_id: string
	project_name?: string
	project_slug?: string
	version_number?: string
	game_versions: string[]
	loaders: string[]
}

export async function lookupModHash(path: string): Promise<ModrinthLookupResult | null> {
	return invoke('plugin:drop|drop_lookup_mod_hash', { path })
}
