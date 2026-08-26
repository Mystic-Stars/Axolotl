import assert from 'node:assert/strict'
import test from 'node:test'

import {
	compactSearchText,
	curseForgeQueryVariants,
	expandSearchQuery,
	modrinthQueryVariants,
	normalizeSearchText,
	slugifySearchText,
	splitCamelCaseSearchText,
} from './search-query.ts'

test('normalizes whitespace, diacritics and case', () => {
	assert.equal(normalizeSearchText('  Example   Mod  '), 'example mod')
	assert.equal(normalizeSearchText('Bélicraft'), 'belicraft')
	assert.equal(normalizeSearchText('Sodium Extra'), 'sodium extra')
})

test('compacts queries to alphanumerics only', () => {
	assert.equal(compactSearchText('Example Mod!'), 'examplemod')
	assert.equal(compactSearchText('sodium_extra'), 'sodiumextra')
	assert.equal(compactSearchText('铁 锭'), '铁锭')
})

test('slugifies queries for CurseForge', () => {
	assert.equal(slugifySearchText('Example Mod!'), 'example-mod')
	assert.equal(slugifySearchText('Sodium Extra'), 'sodium-extra')
	assert.equal(slugifySearchText('--sodium--'), 'sodium')
})

test('splits camelCase and PascalCase words', () => {
	assert.equal(splitCamelCaseSearchText('SodiumExtra'), 'Sodium Extra')
	assert.equal(splitCamelCaseSearchText('AE2Stuff'), 'AE2 Stuff')
	assert.equal(splitCamelCaseSearchText('sodiumextra'), 'sodiumextra')
})

test('modrinth variants preserve the typed form first', () => {
	assert.deepEqual(modrinthQueryVariants('example mod'), ['example mod', 'examplemod'])
	assert.deepEqual(modrinthQueryVariants('sodium-extra'), [
		'sodium-extra',
		'sodium extra',
		'sodiumextra',
	])
	assert.deepEqual(modrinthQueryVariants('SodiumExtra'), ['sodiumextra', 'sodium extra'])
	assert.deepEqual(modrinthQueryVariants(''), [])
})

test('curseforge variants put the slug form first', () => {
	assert.deepEqual(curseForgeQueryVariants('example mod'), [
		'example-mod',
		'example mod',
		'examplemod',
	])
	assert.deepEqual(curseForgeQueryVariants('sodium-extra'), ['sodium-extra', 'sodiumextra'])
	assert.deepEqual(curseForgeQueryVariants('SodiumExtra'), ['sodiumextra', 'sodium-extra'])
	assert.deepEqual(curseForgeQueryVariants('sodium extra'), [
		'sodium-extra',
		'sodium extra',
		'sodiumextra',
	])
	assert.deepEqual(curseForgeQueryVariants(''), [])
})

test('expandSearchQuery returns null for empty and whitespace-only queries', () => {
	assert.equal(expandSearchQuery(''), null)
	assert.equal(expandSearchQuery('   '), null)
})

test('expandSearchQuery flags compact queries', () => {
	const expansion = expandSearchQuery('sodiumextra')
	assert.ok(expansion)
	assert.equal(expansion.compact, true)
	assert.equal(expansion.normalized, 'sodiumextra')
	assert.deepEqual(expansion.modrinthVariants, ['sodiumextra'])
	assert.deepEqual(expansion.curseforgeVariants, ['sodiumextra'])
})

test('expandSearchQuery never treats spaced queries as compact', () => {
	const expansion = expandSearchQuery('sodium extra')
	assert.ok(expansion)
	assert.equal(expansion.compact, false)
})

test('expansion of hyphenated queries covers both providers', () => {
	const expansion = expandSearchQuery('sodium-extra')
	assert.ok(expansion)
	assert.deepEqual(expansion.modrinthVariants, ['sodium-extra', 'sodium extra', 'sodiumextra'])
	assert.deepEqual(expansion.curseforgeVariants, ['sodium-extra', 'sodiumextra'])
})

test('expansion dedupes and caps variants', () => {
	const expansion = expandSearchQuery('a b c')
	assert.ok(expansion)
	assert.ok(expansion.modrinthVariants.length <= 3)
	assert.ok(expansion.curseforgeVariants.length <= 3)
	assert.equal(new Set(expansion.modrinthVariants).size, expansion.modrinthVariants.length)
})