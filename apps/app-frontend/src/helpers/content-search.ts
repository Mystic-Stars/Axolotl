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

export function containsChineseSearchText(query: string): boolean {
	return /[\u3400-\u4dbf\u4e00-\u9fff]/u.test(query)
}

export function resolveChineseContentSearch(query: string) {
	return invoke<ChineseSearchResolution>(
		'plugin:content-search|resolve_chinese_content_search',
		{ query },
	)
}
