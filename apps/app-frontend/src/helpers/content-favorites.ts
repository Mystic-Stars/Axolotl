import { invoke } from '@tauri-apps/api/core'

export type FavoriteProvider = 'modrinth' | 'curseforge' | 'mcarchive'
export type FavoriteContentType = 'mod' | 'resourcepack' | 'datapack' | 'shader'

export interface ContentFavorite {
	provider: FavoriteProvider
	project_id: string
	content_type: FavoriteContentType
	saved_at: number
}

export interface ContentFavoriteInput {
	provider: FavoriteProvider
	project_id: string
	content_type: FavoriteContentType
}

export const FAVORITE_CONTENT_TYPES: FavoriteContentType[] = [
	'mod',
	'resourcepack',
	'datapack',
	'shader',
]

export function isFavoriteContentType(value: string): value is FavoriteContentType {
	return FAVORITE_CONTENT_TYPES.includes(value as FavoriteContentType)
}

export function contentFavoriteKey(provider: FavoriteProvider, projectId: string) {
	return `${provider}:${projectId}`
}

export async function listContentFavorites(): Promise<ContentFavorite[]> {
	return await invoke('plugin:content-favorites|content_favorites_list')
}

export async function addContentFavorite(favorite: ContentFavoriteInput): Promise<ContentFavorite> {
	return await invoke('plugin:content-favorites|content_favorites_add', { favorite })
}

export async function removeContentFavorite(
	provider: FavoriteProvider,
	projectId: string,
): Promise<void> {
	await invoke('plugin:content-favorites|content_favorites_remove', {
		provider,
		projectId,
	})
}
