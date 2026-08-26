import assert from 'node:assert/strict'
import test from 'node:test'

import type { ContentFavorite, ContentFavoriteInput } from '../helpers/content-favorites.ts'
import { createContentFavoritesStore } from './useContentFavorites.ts'

function favorite(
	provider: ContentFavorite['provider'],
	projectId: string,
	contentType: ContentFavorite['content_type'],
	savedAt: number,
): ContentFavorite {
	return {
		provider,
		project_id: projectId,
		content_type: contentType,
		saved_at: savedAt,
	}
}

test('content favorites load, add, remove, and keep provider-qualified identities', async () => {
	const records = [
		favorite('modrinth', 'same-id', 'mod', 1),
		favorite('curseforge', 'same-id', 'resourcepack', 2),
	]
	const store = createContentFavoritesStore({
		async list() {
			return records
		},
		async add(input: ContentFavoriteInput) {
			const saved = favorite(input.provider, input.project_id, input.content_type, 3)
			records.push(saved)
			return saved
		},
		async remove(provider, projectId) {
			const index = records.findIndex(
				(record) => record.provider === provider && record.project_id === projectId,
			)
			if (index >= 0) records.splice(index, 1)
		},
	})

	await store.load()
	assert.deepEqual(
		store.favorites.value.map((record) => `${record.provider}:${record.project_id}`),
		['curseforge:same-id', 'modrinth:same-id'],
	)
	assert.equal(store.isFavorite('modrinth', 'same-id'), true)
	assert.equal(store.isFavorite('curseforge', 'same-id'), true)

	await store.add({ provider: 'modrinth', project_id: 'iris', content_type: 'shader' })
	assert.equal(store.isFavorite('modrinth', 'iris'), true)
	await store.remove('modrinth', 'same-id')
	assert.equal(store.isFavorite('modrinth', 'same-id'), false)
	assert.equal(store.isFavorite('curseforge', 'same-id'), true)
})

test('content favorite writes roll back busy state when persistence fails', async () => {
	const store = createContentFavoritesStore({
		async list() {
			return []
		},
		async add() {
			throw new Error('database unavailable')
		},
		async remove() {
			throw new Error('database unavailable')
		},
	})

	await assert.rejects(
		store.add({ provider: 'modrinth', project_id: 'sodium', content_type: 'mod' }),
	)
	assert.equal(store.isFavorite('modrinth', 'sodium'), false)
	assert.equal(store.isPending('modrinth', 'sodium'), false)

	await store.load()
	store.favorites.value = [favorite('curseforge', 'embeddium', 'mod', 4)]
	await assert.rejects(store.remove('curseforge', 'embeddium'))
	assert.equal(store.isFavorite('curseforge', 'embeddium'), true)
	assert.equal(store.isPending('curseforge', 'embeddium'), false)
})
