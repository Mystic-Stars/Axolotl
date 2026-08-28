import type { SymlinkMethodInstance } from '../providers/instance-import'

/**
 * The version folder name for an import instance: the last path segment of the
 * resolved version folder, falling back to the scan key with the launcher
 * prefix and `versions/` segment stripped
 * (e.g. `.minecraft:versions/1.12.2` or `versions/1.12.2` → `1.12.2`).
 */
export function instanceVersionFolderName(instance: SymlinkMethodInstance): string {
	const pathName = instance.path?.split(/[\\/]/).filter(Boolean).pop()
	if (pathName) return pathName
	const name = instance.name ?? ''
	const colon = name.lastIndexOf(':')
	const stripped = colon >= 0 ? name.slice(colon + 1) : name
	return stripped.replace(/^versions[\\/]/, '')
}

/**
 * Builds the version-isolated game dir `<root>/versions/<folderName>`, using the
 * separator style of `root` so the path reads consistently on every platform
 * (Windows backslashes, POSIX forward slashes).
 */
export function joinIsolatedGameDir(root: string, folderName: string): string {
	const sep = root.includes('\\') ? '\\' : '/'
	const base = root.replace(/[\\/]+$/, '')
	return `${base}${sep}versions${sep}${folderName}`
}

/**
 * The game-dir override for a version-isolated import: `<root>/versions/<folder>`.
 * When `root` is already the version folder itself (e.g. the instance path when
 * no launcher root is known), it is returned as-is instead of appending a
 * second `versions/<folder>` segment.
 */
export function isolatedGameDirOverride(root: string, instance: SymlinkMethodInstance): string {
	const folder = instanceVersionFolderName(instance)
	const rootName = root.split(/[\\/]/).filter(Boolean).pop()
	if (rootName === folder) return root
	return joinIsolatedGameDir(root, folder)
}
