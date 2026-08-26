import assert from 'node:assert/strict'
import test from 'node:test'

import { contentFavoriteKey, isFavoriteContentType } from './content-favorites.ts'

test('content favorites keep provider-qualified identities separate', () => {
	assert.notEqual(
		contentFavoriteKey('modrinth', 'same-id'),
		contentFavoriteKey('curseforge', 'same-id'),
	)
})

test('content favorites only accept installable content types', () => {
	assert.equal(isFavoriteContentType('mod'), true)
	assert.equal(isFavoriteContentType('resourcepack'), true)
	assert.equal(isFavoriteContentType('datapack'), true)
	assert.equal(isFavoriteContentType('shader'), true)
	assert.equal(isFavoriteContentType('modpack'), false)
	assert.equal(isFavoriteContentType('world'), false)
})
