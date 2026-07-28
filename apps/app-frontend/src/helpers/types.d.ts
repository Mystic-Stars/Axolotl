import type { ModrinthId } from '@modrinth/utils'

export type GameInstance = {
	id: string
	path: string
	install_stage: InstallStage
	launcher_feature_version: string

	name: string
	icon_path?: string
	symlink_target?: string | null

	game_version: string
	protocol_version?: number
	loader: InstanceLoader
	loader_version?: string

	groups: string[]

	link?: InstanceLink | null
	update_channel: ReleaseChannel

	created: Date
	modified: Date
	last_played?: Date
	pinned_at?: Date

	submitted_time_played: number
	recent_time_played: number

	java_path?: string
	extra_launch_args?: string[]
	custom_env_vars?: [string, string][]

	memory?: MemorySettings
	force_fullscreen?: boolean
	game_resolution?: [number, number]
	hooks: Hooks
}

type InstallStage =
	| 'installed'
	| 'minecraft_installing'
	| 'pack_installed'
	| 'pack_installing'
	| 'not_installed'

type InstanceLinkIdentity = {
	project_id?: ModrinthId | null
	version_id?: ModrinthId | null
	server_project_id?: ModrinthId | null
	content_project_id?: ModrinthId | null
	content_version_id?: ModrinthId | null
}

export type InstanceLink = InstanceLinkIdentity &
	(
		| {
				type: 'modrinth_modpack'
				project_id: ModrinthId
				version_id: ModrinthId
		  }
		| {
				type: 'curseforge_modpack'
				project_id: string
				version_id: string
		  }
		| {
				type: 'server_project'
				project_id: ModrinthId
		  }
		| {
				type: 'server_project_modpack'
				server_project_id: ModrinthId
				content_project_id?: ModrinthId | null
				content_version_id: ModrinthId
				project_id?: ModrinthId
				version_id?: ModrinthId
		  }
		| {
				type: 'imported_modpack'
				project_id?: ModrinthId | null
				version_id?: ModrinthId | null
				name?: string | null
				version_number?: string | null
				filename?: string | null
		  }
		| {
				type: 'modrinth_hosting'
				server_id: string
				instance_ids: string[]
				active_instance_id?: string | null
		  }
		| {
				type: 'shared_instance'
				shared_instance_id: string
		  }
	)

export type Instance = GameInstance

type ReleaseChannel = 'release' | 'beta' | 'alpha'

export type InstanceLoader = 'vanilla' | 'forge' | 'fabric' | 'quilt' | 'neoforge'
  | 'lite_loader'
  | 'labymod'
  | 'cleanroom'
  | 'legacy_fabric'

type ContentFile = {
	enabled: boolean
	modrinth?: {
		project_id: string
		version_id: string
	}
	provider_refs: Array<{
		provider: 'modrinth' | 'curseforge'
		project_id: string | number
		version_id?: string | null
		file_id?: number | null
	}>
	origin_provider: 'modrinth' | 'curseforge' | null
}

type ContentFileProjectType = 'mod' | 'datapack' | 'resourcepack' | 'shaderpack' | 'schematic'

type CacheBehaviour =
	// Serve expired data. If fetch fails / launcher is offline, errors are ignored
	| 'stale_while_revalidate_skip_offline'
	| 'cache_only'
	// Serve expired data, revalidate in background
	| 'stale_while_revalidate'
	// Must revalidate if data is expired
	| 'must_revalidate'
	// Ignore cache- always fetch updated data from origin
	| 'bypass'

type MemorySettings = {
	maximum: number
	automatic: boolean
}

type WindowSize = {
	width: number
	height: number
}

type Hooks = {
	pre_launch?: string
	wrapper?: string
	post_exit?: string
}

type Manifest = {
	gameVersions: ManifestGameVersion[]
	versionGroups?: ManifestVersionGroup[]
}

type ManifestGameVersion = {
	id: string
	stable: boolean
	versionGroup?: string
	loaders: ManifestLoaderVersion[]
}

type ManifestVersionGroup = {
	id: string
	loaders: ManifestLoaderVersion[]
}

type ManifestLoaderVersion = {
	id: string
	url: string
	stable: boolean
}

type AppSettings = {
	max_concurrent_downloads: number
	max_concurrent_writes: number

	theme: 'dark' | 'light' | 'oled' | 'system'
	accent_color: 'pink' | 'orange' | 'green' | 'blue' | 'purple' | `custom:#${string}`
	default_page: 'Home' | 'DiscoverContent' | 'Library'
	collapsed_navigation: boolean
	advanced_rendering: boolean
	native_decorations: boolean
	custom_background_path: string | null
	custom_background_blur: number
	custom_background_opacity: number
	transparent_background: boolean
	transparent_background_opacity: number
	transparent_background_blur: boolean
	auto_hide_downloads_button: boolean
	worlds_in_home: boolean
	home_layout: 'standard' | 'minimal'
	minimal_home_instance_id: string | null

	telemetry: boolean
	discord_rpc: boolean
	developer_mode: boolean

	onboarded: boolean
	onboarding_version: number
	onboarding_instance_tour_completed: boolean

	extra_launch_args: string[]
	custom_env_vars: [string, string][]
	memory: MemorySettings
	force_fullscreen: boolean
	game_resolution: [number, number]
	hide_on_process_start: boolean
	auto_set_java_high_performance_mode: boolean
	hooks: Hooks

	custom_dir?: string
	prev_custom_dir?: string
	migrated: boolean
}
