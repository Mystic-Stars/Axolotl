/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank instance object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/core'

import { setModrinthSourceMode } from '@/config'
import type { Hooks, MemorySettings, WindowSize } from '@/helpers/types'
import type { AccentColor, ColorTheme, FeatureFlag } from '@/store/theme.ts'

// Settings object
/*

Settings {
    "memory": MemorySettings,
    "game_resolution": [int int],
    "custom_java_args": [String ...],
    "custom_env_args" : [(string, string) ... ]>,
    "java_globals": Hash of (string, Path),
    "default_user": Uuid string (can be null),
    "hooks": Hooks,
    "max_concurrent_downloads": uint,
    "version": u32,
    "collapsed_navigation": bool,
}

Memorysettings {
    "min": u32, can be null,
    "max": u32,
}

*/

export type UpdateSource = 'cnb' | 'github'
export type DownloadSourceMode = 'auto' | 'official_only' | 'mirror_preferred'

const UPDATE_SOURCE_STORAGE_KEY = 'axolotl-update-source'

export function getUpdateSource(): UpdateSource {
	const value = localStorage.getItem(UPDATE_SOURCE_STORAGE_KEY)
	const source = value === 'github' || value === 'official' ? 'github' : 'cnb'
	if (value !== source) {
		localStorage.setItem(UPDATE_SOURCE_STORAGE_KEY, source)
	}
	return source
}

export function setUpdateSource(source: UpdateSource) {
	localStorage.setItem(UPDATE_SOURCE_STORAGE_KEY, source)
}

export type AppSettings = {
	max_concurrent_downloads: number
	max_concurrent_writes: number
	auto_concurrent_downloads: boolean
	minecraft_metadata_source: DownloadSourceMode
	minecraft_file_source: DownloadSourceMode
	modrinth_source: DownloadSourceMode
	curseforge_source: DownloadSourceMode

	theme: ColorTheme
	accent_color: AccentColor
	locale: string
	default_page: 'Home' | 'DiscoverContent' | 'Library'
	collapsed_navigation: boolean
	hide_nametag_skins_page: boolean
	advanced_rendering: boolean
	native_decorations: boolean
	toggle_sidebar: boolean
	custom_background_path: string | null
	custom_background_blur: number
	custom_background_opacity: number

	telemetry: boolean
	discord_rpc: boolean
	onboarded: boolean

	extra_launch_args: string[]
	custom_env_vars: [string, string][]
	memory: MemorySettings
	force_fullscreen: boolean
	game_resolution: WindowSize
	hide_on_process_start: boolean
	hooks: Hooks

	custom_dir?: string | null
	prev_custom_dir?: string | null
	migrated: boolean

	developer_mode: boolean
	feature_flags: Record<FeatureFlag, boolean>

	skipped_update: string | null
	pending_update_toast_for_version: string | null
	auto_download_updates: boolean | null

	version: number
}

type LegacyMirrorSettings = {
	use_minecraft_mirror?: boolean
	use_modrinth_mirror?: boolean
	use_curseforge_mirror?: boolean
}

function normalizeDownloadSettings(settings: AppSettings & LegacyMirrorSettings): AppSettings {
	const hasLegacySettings =
		typeof settings.use_minecraft_mirror === 'boolean' &&
		typeof settings.use_modrinth_mirror === 'boolean' &&
		typeof settings.use_curseforge_mirror === 'boolean'
	const usesLegacyDefaults =
		hasLegacySettings &&
		!settings.use_minecraft_mirror &&
		!settings.use_modrinth_mirror &&
		settings.use_curseforge_mirror
	const legacySource = (enabled: boolean | undefined): DownloadSourceMode =>
		enabled ? 'mirror_preferred' : 'official_only'

	settings.auto_concurrent_downloads ??= true
	settings.minecraft_metadata_source ??=
		usesLegacyDefaults || !hasLegacySettings ? 'auto' : legacySource(settings.use_minecraft_mirror)
	settings.minecraft_file_source ??=
		usesLegacyDefaults || !hasLegacySettings ? 'auto' : legacySource(settings.use_minecraft_mirror)
	settings.modrinth_source ??=
		usesLegacyDefaults || !hasLegacySettings ? 'auto' : legacySource(settings.use_modrinth_mirror)
	settings.curseforge_source ??=
		usesLegacyDefaults || !hasLegacySettings ? 'auto' : legacySource(settings.use_curseforge_mirror)

	return settings
}

function syncLegacyMirrorSettings(settings: AppSettings & LegacyMirrorSettings) {
	const legacyValue = (source: DownloadSourceMode, current: boolean | undefined) => {
		if (source === 'mirror_preferred') return true
		if (source === 'official_only') return false
		return current ?? false
	}

	if (typeof settings.use_minecraft_mirror === 'boolean') {
		settings.use_minecraft_mirror = legacyValue(
			settings.minecraft_file_source,
			settings.use_minecraft_mirror,
		)
	}
	if (typeof settings.use_modrinth_mirror === 'boolean') {
		settings.use_modrinth_mirror = legacyValue(
			settings.modrinth_source,
			settings.use_modrinth_mirror,
		)
	}
	if (typeof settings.use_curseforge_mirror === 'boolean') {
		settings.use_curseforge_mirror = legacyValue(
			settings.curseforge_source,
			settings.use_curseforge_mirror,
		)
	}
}

// Get full settings object
export async function get() {
	const settings = normalizeDownloadSettings(
		(await invoke('plugin:settings|settings_get')) as AppSettings & LegacyMirrorSettings,
	)
	setModrinthSourceMode(settings.modrinth_source)
	return settings
}

// Set full settings object
export async function set(settings: AppSettings) {
	syncLegacyMirrorSettings(settings)
	const result = await invoke('plugin:settings|settings_set', { settings })
	setModrinthSourceMode(settings.modrinth_source)
	return result
}

export async function cancel_directory_change(): Promise<void> {
	return await invoke('plugin:settings|cancel_directory_change')
}
