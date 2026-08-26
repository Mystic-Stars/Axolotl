import type { InstancePostUpgradeWarning } from './instance'

export function normalizePostUpgradePath(path: string): string {
	return path.replaceAll('\\', '/').replace(/^(?:\.\/)+/, '')
}

export function postUpgradeWarningForContent(
	warnings: InstancePostUpgradeWarning[],
	contentId: string | null | undefined,
	relativePath: string | null | undefined,
): InstancePostUpgradeWarning | null {
	const normalizedPath = relativePath ? normalizePostUpgradePath(relativePath) : null
	const contentMatch = contentId
		? warnings.find((warning) => warning.contentId === contentId)
		: undefined
	if (contentMatch) return contentMatch
	return normalizedPath
		? (warnings.find(
				(warning) =>
					!!warning.relativePath &&
					normalizePostUpgradePath(warning.relativePath) === normalizedPath,
			) ?? null)
		: null
}

export function shouldExpandUpgradeWarningsByDefault(count: number): boolean {
	return count > 0 && count <= 10
}
