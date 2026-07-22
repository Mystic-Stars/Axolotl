import type { ContentItem } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'

export interface ChineseSearchTranslation {
	chineseName: string
	curseforgeSlug?: string
	modrinthSlug?: string
	matchScore: number
	exact: boolean
}

export interface ChineseSearchResolution {
	isChinese: boolean
	normalizedQuery: string
	curseforgeQuery?: string
	modrinthQuery?: string
	modrinthSlugs: string[]
	translations: ChineseSearchTranslation[]
}

export interface ChineseNameLookup {
	modrinth: Record<string, string>
	curseforge: Record<string, string>
}

export function containsChineseSearchText(query: string): boolean {
	return /[\u3400-\u4dbf\u4e00-\u9fff]/u.test(query)
}

export function resolveChineseContentSearch(query: string) {
	return invoke<ChineseSearchResolution>('plugin:content-search|resolve_chinese_content_search', {
		query,
	})
}

export function lookupChineseContentNames(modrinthSlugs: string[], curseforgeSlugs: string[]) {
	return invoke<ChineseNameLookup>('plugin:content-search|lookup_chinese_content_names', {
		modrinthSlugs,
		curseforgeSlugs,
	})
}

export function bilingualTitle(chineseName: string, originalTitle: string) {
	const chineseTitle = chineseName.replace(/\s+\([^()]*[A-Za-z][^()]*\)$/u, '').trim()
	if (!chineseTitle || chineseTitle.toLocaleLowerCase() === originalTitle.toLocaleLowerCase()) {
		return originalTitle
	}
	return `${chineseTitle} (${originalTitle})`
}

/**
 * Rewrites content item titles to the bilingual `中文名 (English)` format used
 * by the Browse page, resolving names from the bundled wiki dictionary by
 * project slug. Items are returned unchanged unless the locale is zh-CN.
 */
export async function translateContentItemTitles<T extends ContentItem>(
	items: T[],
	locale: string,
): Promise<T[]> {
	if (locale !== 'zh-CN' || items.length === 0) return items

	const modrinthSlugs: string[] = []
	const curseforgeSlugs: string[] = []
	for (const item of items) {
		const slug = item.project?.slug
		if (!slug) continue
		if (item.primary_provider === 'curseforge') curseforgeSlugs.push(slug)
		else modrinthSlugs.push(slug)
	}
	if (modrinthSlugs.length === 0 && curseforgeSlugs.length === 0) return items

	const lookup = await lookupChineseContentNames(modrinthSlugs, curseforgeSlugs).catch(() => null)
	if (!lookup) return items

	return items.map((item) => {
		const project = item.project
		if (!project?.slug) return item
		const chineseName =
			item.primary_provider === 'curseforge'
				? lookup.curseforge[project.slug]
				: lookup.modrinth[project.slug]
		if (!chineseName) return item
		return { ...item, project: { ...project, title: bilingualTitle(chineseName, project.title) } }
	})
}
