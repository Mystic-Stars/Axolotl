import { invoke } from '@tauri-apps/api/core'

export type MinecraftNewsItem = {
	title: string
	category?: string | null
	tag?: string | null
	date?: string | null
	image_url?: string | null
	read_more_url: string
}

const NEWS_TTL_MS = 30 * 60 * 1000
let cachedAt = 0
let cached: Promise<MinecraftNewsItem[]> | null = null

export function get_minecraft_news(limit = 12): Promise<MinecraftNewsItem[]> {
	if (!cached || Date.now() - cachedAt > NEWS_TTL_MS) {
		cachedAt = Date.now()
		cached = invoke<MinecraftNewsItem[]>('plugin:utils|get_minecraft_news', { limit }).catch(
			(error) => {
				cached = null
				throw error
			},
		)
	}
	return cached
}
