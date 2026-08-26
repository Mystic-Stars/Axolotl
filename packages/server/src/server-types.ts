import type {
	PaperBuild,
	ResolveServerJarInput,
	ServerJarDownload,
	ServerTypeDefinition,
	ServerTypeId,
} from './types.ts'

const FABRIC_META_URL = 'https://meta.fabricmc.net/v2'
const PAPER_API_URL = 'https://fill.papermc.io/v3'
const PAPER_PROJECT = 'paper'

/**
 * Known server types. `forge`, `neoforge` and `quilt` require an installer run
 * step that is not implemented yet (TODO) but are registered so the UI and
 * future CLI share one source of truth.
 */
export const SERVER_TYPES: Record<ServerTypeId, ServerTypeDefinition> = {
	vanilla: {
		id: 'vanilla',
		label: 'Vanilla',
		installMode: 'direct',
		needsLoaderVersion: false,
	},
	fabric: {
		id: 'fabric',
		label: 'Fabric',
		installMode: 'direct',
		needsLoaderVersion: true,
	},
	paper: {
		id: 'paper',
		label: 'Paper',
		installMode: 'direct',
		needsLoaderVersion: false,
	},
	forge: {
		id: 'forge',
		label: 'Forge',
		installMode: 'installer',
		needsLoaderVersion: true,
	},
	neoforge: {
		id: 'neoforge',
		label: 'NeoForge',
		installMode: 'installer',
		needsLoaderVersion: true,
	},
	quilt: {
		id: 'quilt',
		label: 'Quilt',
		installMode: 'installer',
		needsLoaderVersion: true,
	},
}

export function listServerTypes(): ServerTypeDefinition[] {
	return Object.values(SERVER_TYPES)
}

export function isServerTypeSupported(type: ServerTypeId): boolean {
	return SERVER_TYPES[type].installMode !== 'installer'
}

/** URL of the Fabric server launcher jar for a specific game/loader/installer combination. */
export function fabricServerJarUrl(
	gameVersion: string,
	loaderVersion: string,
	installerVersion: string,
): string {
	return `${FABRIC_META_URL}/versions/loader/${gameVersion}/${loaderVersion}/${installerVersion}/server/jar`
}

export function fabricInstallerVersionsUrl(): string {
	return `${FABRIC_META_URL}/versions/installer`
}

export function fabricLoaderVersionsForGameUrl(gameVersion: string): string {
	return `${FABRIC_META_URL}/versions/loader/${gameVersion}`
}

export function paperBuildsUrl(gameVersion: string): string {
	return `${PAPER_API_URL}/projects/${PAPER_PROJECT}/versions/${gameVersion}/builds`
}

/**
 * Resolves the server jar download for a server type from metadata the caller
 * fetched. Returns null when the type needs an installer step or required
 * metadata is missing.
 */
export function resolveServerJar(
	type: ServerTypeId,
	input: ResolveServerJarInput,
): ServerJarDownload | null {
	switch (type) {
		case 'vanilla': {
			const server = input.vanillaVersionInfo?.downloads.server
			if (!server) return null
			return { url: server.url, filename: 'server.jar', sha1: server.sha1, size: server.size }
		}
		case 'fabric': {
			if (!input.loaderVersion || !input.installerVersion) return null
			return {
				url: fabricServerJarUrl(input.gameVersion, input.loaderVersion, input.installerVersion),
				filename: 'fabric-server.jar',
			}
		}
		case 'paper': {
			const download = input.paperBuild?.downloads['server:default']
			if (!download) return null
			return { url: download.url, filename: 'server.jar' }
		}
		default:
			return null
	}
}

export type PaperBuildsResponse = PaperBuild[]

/** The newest stable build from a Fill v3 builds response (builds are newest first). */
export function latestStablePaperBuild(response: PaperBuildsResponse): PaperBuild | null {
	return response?.find((build) => build.channel === 'STABLE') ?? null
}

export interface FabricInstallerVersionsResponse {
	version: string
	stable: boolean
}

/** The newest installer version from the `/v2/versions/installer` response (a top-level array). */
export function pickFabricInstallerVersion(
	response: FabricInstallerVersionsResponse[],
): string | null {
	return response?.[0]?.version ?? null
}

/** Minimum Java major version required to run a given game version. */
/**
 * Minimum Java major version required to run a given game version.
 * Handles both the legacy `1.x` scheme and the year-based scheme (`26.2`,
 * `26w14a`), which needs Java 25.
 */
export function requiredJavaMajorVersion(gameVersion: string): number {
	const yearSnapshot = /^(\d{2})w/.exec(gameVersion)
	if (yearSnapshot) {
		return Number(yearSnapshot[1]) >= 26 ? 25 : 21
	}

	const match = /^(\d+)(?:\.(\d+))?(?:\.(\d+))?/.exec(gameVersion)
	if (!match) return 25
	const major = Number(match[1])
	const minor = Number(match[2] ?? 0)
	const patch = Number(match[3] ?? 0)

	// Year-based releases (26.1+) require Java 25
	if (major >= 21) return 25
	// Legacy 1.x releases
	if (major === 1) {
		if (minor > 20 || (minor === 20 && patch >= 5)) return 21
		if (minor >= 17) return 17
		return 8
	}
	return 25
}
