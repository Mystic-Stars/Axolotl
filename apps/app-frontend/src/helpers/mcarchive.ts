import { invoke } from '@tauri-apps/api/core'

export type McArchiveGameVersion = {
	id: number
	name: string
	versionType?: string | null
}

export type McArchiveFile = {
	uuid: string
	name: string
	sha256?: string | null
	archiveUrl?: string | null
	directUrl?: string | null
	redirectUrl?: string | null
	pageUrl?: string | null
}

export type McArchiveModVersion = {
	uuid: string
	name: string
	gameVersions: McArchiveGameVersion[]
	files: McArchiveFile[]
}

export type McArchiveMod = {
	uuid: string
	slug: string
	name: string
	summary?: string | null
	description?: string | null
	pageUrl?: string | null
	modVersions: McArchiveModVersion[]
}

export function getMcArchiveGameVersions() {
	return invoke<McArchiveGameVersion[]>('plugin:mcarchive|mcarchive_get_game_versions')
}

export async function searchMcArchiveMods(query: string, gameVersion?: string | null) {
	const keyword = query.trim()
	const search = (value: string, includeGameVersion = true) =>
		invoke<McArchiveMod[]>('plugin:mcarchive|mcarchive_search_mods', {
			keyword: value,
			...(includeGameVersion && gameVersion ? { gameVersion } : {}),
		})

	let results = await search(keyword)
	if (results.length === 0 && gameVersion) {
		results = await search(keyword, false)
	}
	if (results.length > 0) return results

	// MCArchive's keyword matching does not treat punctuation or whitespace as
	// interchangeable, while its project slugs commonly omit both.
	const slugLikeKeyword = keyword.replace(/[\s_-]+/g, '')
	if (slugLikeKeyword && slugLikeKeyword !== keyword) {
		const slugResults = await search(slugLikeKeyword, !!gameVersion)
		if (slugResults.length > 0) return slugResults
		if (gameVersion) {
			const unfilteredSlugResults = await search(slugLikeKeyword, false)
			if (unfilteredSlugResults.length > 0) return unfilteredSlugResults
		}
	}

	if (!keyword) return results

	// The archive API's keyword matching is intentionally conservative. Fetch the
	// small project index only when its direct query misses, then match locally.
	const catalog = await search('', !!gameVersion)
	const normalized = keyword.toLocaleLowerCase().replace(/[\s_-]+/g, '')
	return catalog.filter((project) =>
		[project.slug, project.name, project.description ?? ''].some((value) =>
			value
				.toLocaleLowerCase()
				.replace(/[\s_-]+/g, '')
				.includes(normalized),
		),
	)
}

export function getMcArchiveModBySlug(slug: string) {
	return invoke<McArchiveMod>('plugin:mcarchive|mcarchive_get_mod_by_slug', { slug })
}
