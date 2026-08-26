export type ServerTypeId = 'vanilla' | 'fabric' | 'paper' | 'forge' | 'neoforge' | 'quilt'

/**
 * How a server jar is obtained and booted for a given server type.
 * `direct`: the downloaded jar is the server itself.
 * `installer`: an installer must run in the server directory before launch.
 * `todo`: support is planned but not implemented yet.
 */
export type ServerInstallMode = 'direct' | 'installer' | 'todo'

export interface ServerTypeDefinition {
	id: ServerTypeId
	label: string
	installMode: ServerInstallMode
	needsLoaderVersion: boolean
}

export interface ServerJarDownload {
	url: string
	filename: string
	sha1?: string
	size?: number
}

export interface VanillaVersionInfoDownload {
	sha1: string
	size: number
	url: string
}

export interface VanillaVersionInfo {
	downloads: { server?: VanillaVersionInfoDownload }
}

export interface PaperBuildDownload {
	name: string
	url: string
	checksums?: { sha256?: string }
	size?: number
}

/** A build from the PaperMC Fill v3 downloads service. */
export interface PaperBuild {
	id: number
	channel: string
	downloads: { 'server:default'?: PaperBuildDownload }
}

export interface ResolveServerJarInput {
	gameVersion: string
	loaderVersion?: string
	installerVersion?: string
	vanillaVersionInfo?: VanillaVersionInfo
	paperBuild?: PaperBuild
}

export type ServerStatus = 'created' | 'eula_pending' | 'ready' | 'starting' | 'running' | 'crashed'

/** Persisted server manifest, stored as `axolotl-server.json` in the server directory. */
export interface ManagedServerManifest {
	id: string
	name: string
	serverType: ServerTypeId
	gameVersion: string
	loaderVersion?: string
	createdAt: string
	javaPath?: string
	memoryMb?: number
	jvmArgs?: string[]
	lastStartedAt?: string
}

export interface ManagedServer extends ManagedServerManifest {
	path: string
	status: ServerStatus
	port?: number
	eulaAccepted: boolean
}

export interface ServerStatusInput {
	manifest: Pick<ManagedServerManifest, 'id'>
	isRunning: boolean
	isStarting: boolean
	lastExitWasCrash: boolean
	eulaAccepted: boolean
	eulaFileExists: boolean
}

export interface ServerLaunchOptions {
	javaPath: string
	memoryMb: number
	jvmArgs?: string[]
}

export const DEFAULT_SERVER_PORT = 25565
export const DEFAULT_SERVER_MEMORY_MB = 2048
