/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank instance object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/core'

import type { HomeDashboardConfig } from '@/components/home/home-dashboard'
import type { Hooks, MemorySettings, WindowSize } from '@/helpers/types'
import type { AccentColorSetting, ColorTheme, FeatureFlag, HomeLayout } from '@/store/theme.ts'
import { DEFAULT_FEATURE_FLAGS } from '@/store/theme.ts'

export type {
	BrowseContentDisplayMode,
	BrowseContentProjectType,
} from './browse-display-mode.ts'
export {
	getLastBrowseContentDisplayMode,
	getLastBrowseContentProjectType,
	isBrowseContentProjectType,
	setLastBrowseContentDisplayMode,
	setLastBrowseContentProjectType,
} from './browse-display-mode.ts'

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
export type DownloadSourceMode =
	| 'auto'
	| 'official_only'
	| 'mirror_preferred'
	| 'official_preferred'
export type DownloadEngine = 'legacy' | 'xmcl'

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

export type BrowseContentSource = 'all' | 'modrinth' | 'curseforge'

const BROWSE_CONTENT_SOURCE_STORAGE_KEY = 'axolotl-browse-content-source'

export function getLastBrowseContentSource(): BrowseContentSource | null {
	const value = localStorage.getItem(BROWSE_CONTENT_SOURCE_STORAGE_KEY)
	return value === 'all' || value === 'modrinth' || value === 'curseforge' ? value : null
}

export function setLastBrowseContentSource(source: BrowseContentSource) {
	localStorage.setItem(BROWSE_CONTENT_SOURCE_STORAGE_KEY, source)
}

export type AppSettings = {
	max_concurrent_downloads: number
	max_concurrent_writes: number
	download_engine: DownloadEngine
	auto_concurrent_downloads: boolean
	minecraft_metadata_source: DownloadSourceMode
	minecraft_file_source: DownloadSourceMode
	modrinth_source: DownloadSourceMode
	curseforge_source: DownloadSourceMode
	bypass_curseforge_download_restrictions: boolean
	mojang_auth_source: DownloadSourceMode

	theme: ColorTheme
	accent_color: AccentColorSetting
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
	transparent_background: boolean
	transparent_background_opacity: number
	transparent_background_blur: boolean
	sidebar_instance_count: number
	auto_hide_downloads_button: boolean
	home_layout: HomeLayout
	minimal_home_instance_id: string | null
	home_widgets: HomeDashboardConfig | null
	terracotta_public_nodes: string[]

	telemetry: boolean
	telemetry_consent_version: number
	discord_rpc: boolean
	onboarded: boolean
	onboarding_version: number
	onboarding_instance_tour_completed: boolean

	extra_launch_args: string[]
	custom_env_vars: [string, string][]
	memory: MemorySettings
	force_fullscreen: boolean
	game_resolution: WindowSize
	hide_on_process_start: boolean
	auto_set_java_high_performance_mode: boolean
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

export type PrivacySettings = {
	telemetry: boolean
	discord_rpc: boolean
	consent_version: number
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
	settings.download_engine ??= 'legacy'
	settings.auto_set_java_high_performance_mode ??= true
	settings.minecraft_metadata_source ??=
		usesLegacyDefaults || !hasLegacySettings ? 'auto' : legacySource(settings.use_minecraft_mirror)
	settings.minecraft_file_source ??=
		usesLegacyDefaults || !hasLegacySettings ? 'auto' : legacySource(settings.use_minecraft_mirror)
	settings.modrinth_source ??=
		usesLegacyDefaults || !hasLegacySettings ? 'auto' : legacySource(settings.use_modrinth_mirror)
	settings.curseforge_source ??=
		usesLegacyDefaults || !hasLegacySettings ? 'auto' : legacySource(settings.use_curseforge_mirror)
	settings.bypass_curseforge_download_restrictions ??= true
	settings.mojang_auth_source ??= 'auto'
	settings.terracotta_public_nodes ??= ['wss://center.node.1tmc.top']
	settings.feature_flags ??= {}
	for (const [key, value] of Object.entries(DEFAULT_FEATURE_FLAGS)) {
		settings.feature_flags[key as FeatureFlag] ??= value
	}

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
	return settings
}

// Set full settings object
export async function set(settings: AppSettings) {
	syncLegacyMirrorSettings(settings)
	const result = await invoke('plugin:settings|settings_set', { settings })
	return result
}

export async function cancel_directory_change(): Promise<void> {
	return await invoke('plugin:settings|cancel_directory_change')
}

export async function getPrivacySettings(): Promise<PrivacySettings> {
	return await invoke('plugin:settings|privacy_get')
}

export async function savePrivacySettings(privacy: PrivacySettings): Promise<PrivacySettings> {
	return await invoke('plugin:settings|privacy_set', { privacy })
}

export async function setTelemetryEnabled(enabled: boolean): Promise<PrivacySettings> {
	return await invoke('plugin:settings|telemetry_set', { enabled })
}

export async function setDiscordRpcEnabled(enabled: boolean): Promise<PrivacySettings> {
	return await invoke('plugin:settings|discord_rpc_set', { enabled })
}
