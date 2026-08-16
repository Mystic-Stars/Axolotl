export interface LoaderMetadataVersion {
	id: string
	stable: boolean
}

interface LoaderMetadataGameVersion {
	id: string
	versionGroup?: string
	loaders: LoaderMetadataVersion[]
}

interface LoaderMetadataVersionGroup {
	id: string
	loaders: LoaderMetadataVersion[]
}

export interface LoaderMetadataManifest {
	gameVersions: LoaderMetadataGameVersion[]
	versionGroups?: LoaderMetadataVersionGroup[]
}

export type LoaderMetadataStatus = 'unknown' | 'loading' | 'success' | 'error'
export type LoaderSupportState = 'unknown' | 'loading' | 'supported' | 'unsupported' | 'error'
export type GameVersionMetadataState = 'loading' | 'ready' | 'empty' | 'error'
export type LoaderVersionSummaryState = 'loading' | 'selected' | 'empty'

export function loaderMetadataCacheKey(loader: string, gameVersion?: string): string {
	return gameVersion ? `${loader}:${gameVersion}` : loader
}

export function loaderMetadataQueryKey(loader: string, gameVersion?: string) {
	return gameVersion
		? (['creation-flow', 'loader-versions', loader, gameVersion] as const)
		: (['creation-flow', 'loader-manifest', loader] as const)
}

export function loaderVersionsForGameVersion(
	manifest: LoaderMetadataManifest | undefined,
	gameVersion: string,
): LoaderMetadataVersion[] {
	if (!manifest) return []
	const entry = manifest.gameVersions.find((version) => version.id === gameVersion)
	if (!entry) return []
	if (!entry.versionGroup) return entry.loaders
	return manifest.versionGroups?.find((group) => group.id === entry.versionGroup)?.loaders ?? []
}

export function loaderSupportState(
	status: LoaderMetadataStatus,
	manifest: LoaderMetadataManifest | undefined,
	gameVersion: string,
): LoaderSupportState {
	if (status !== 'success') return status
	return loaderVersionsForGameVersion(manifest, gameVersion).length > 0
		? 'supported'
		: 'unsupported'
}

export function isLoaderSupportStateDisabled(state: LoaderSupportState): boolean {
	return state === 'unknown' || state === 'loading' || state === 'unsupported'
}

export function gameVersionSelectorText(
	state: GameVersionMetadataState,
	labels: {
		loading: string
		empty: string
		error: string
		placeholder: string
		searchPlaceholder: string
	},
) {
	if (state === 'loading') {
		return { placeholder: labels.loading, searchPlaceholder: labels.loading }
	}
	if (state === 'empty') {
		return { placeholder: labels.empty, searchPlaceholder: labels.empty }
	}
	if (state === 'error') {
		return { placeholder: labels.error, searchPlaceholder: labels.error }
	}
	return { placeholder: labels.placeholder, searchPlaceholder: labels.searchPlaceholder }
}

export function loaderVersionSelectorText(
	loading: boolean,
	empty: boolean,
	labels: {
		loading: string
		empty: string
		placeholder: string
		searchPlaceholder: string
	},
) {
	if (loading) {
		return { placeholder: labels.loading, searchPlaceholder: labels.loading }
	}
	if (empty) {
		return { placeholder: labels.empty, searchPlaceholder: labels.empty }
	}
	return { placeholder: labels.placeholder, searchPlaceholder: labels.searchPlaceholder }
}

export function loaderVersionSummaryState(
	loading: boolean,
	selectedVersion: string | null,
): LoaderVersionSummaryState {
	if (loading) return 'loading'
	return selectedVersion ? 'selected' : 'empty'
}

export function createLatestRequestGuard() {
	let current = 0
	return {
		begin: () => ++current,
		isCurrent: (request: number) => request === current,
	}
}
