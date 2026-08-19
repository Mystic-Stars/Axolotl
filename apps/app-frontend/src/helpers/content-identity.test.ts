import assert from 'node:assert/strict'
import test from 'node:test'

import {
	compareContentIdentities,
	contentIdentityInputsFromSnapshot,
	contentIdentityFromInput,
	normalizeContentIdentityText,
} from './content-identity.ts'

function identity(
	provider: 'modrinth' | 'curseforge',
	projectId: string,
	contentType = 'mod',
	values: Record<string, string> = {},
) {
	return contentIdentityFromInput({ provider, projectId, contentType, ...values })
}

test('normalizes platform suffixes and versions', () => {
	assert.equal(normalizeContentIdentityText('Sodium-Fabric-0.5.8.jar'), 'sodium')
	assert.equal(normalizeContentIdentityText('sodium_forge_1.20.1'), 'sodium')
})

test('same provider never creates a cross-platform conflict', () => {
	assert.equal(
		compareContentIdentities(
			identity('modrinth', 'a', 'mod', { title: 'Sodium' }),
			identity('modrinth', 'b', 'mod', { title: 'Sodium' }),
		),
		null,
	)
})

test('different content types never conflict', () => {
	assert.equal(
		compareContentIdentities(
			identity('modrinth', 'a', 'mod', { title: 'Sodium' }),
			identity('curseforge', 'b', 'resourcepack', { title: 'Sodium' }),
		),
		null,
	)
})

test('matching sha1 is an exact cross-platform conflict', () => {
	const result = compareContentIdentities(
		identity('modrinth', 'a', 'mod', { sha1: 'ABC123', title: 'First' }),
		identity('curseforge', 'b', 'mod', { sha1: 'abc123', title: 'Second' }),
	)
	assert.equal(result?.source, 'sha1')
	assert.equal(result?.confidence, 'exact')
})

test('matching names are heuristic conflicts', () => {
	const result = compareContentIdentities(
		identity('modrinth', 'a', 'mod', { title: 'Example Mod' }),
		identity('curseforge', 'b', 'mod', { title: 'example-mod' }),
	)
	assert.equal(result?.source, 'heuristic')
})

test('curated mapping conflicts are exact', () => {
	const left = { ...identity('modrinth', 'a'), key: 'mapping:1', ambiguous: false }
	const right = { ...identity('curseforge', 'b'), key: 'mapping:1', ambiguous: false }
	const result = compareContentIdentities(left, right)
	assert.equal(result?.source, 'curated_mapping')
	assert.equal(result?.confidence, 'exact')
})

test('pack-managed snapshot members participate in cross-platform conflicts', () => {
	const [installedInput] = contentIdentityInputsFromSnapshot(
		[
			{
				projectType: 'mod',
				provider: 'modrinth',
				providerProjectId: 'AANobbMI',
				expectedRelativePath: 'mods/sodium-fabric-0.6.13.jar',
				content: null,
			},
		],
		{
			modrinth: new Map([['AANobbMI', { slug: 'sodium', title: 'Sodium' }]]),
		},
	)
	const installed = { ...contentIdentityFromInput(installedInput), key: 'mapping:sodium' }
	const candidate = {
		...identity('curseforge', '394468', 'mod', { slug: 'sodium' }),
		key: 'mapping:sodium',
	}

	const result = compareContentIdentities(candidate, installed)
	assert.equal(result?.source, 'curated_mapping')
	assert.equal(result?.confidence, 'exact')
})
