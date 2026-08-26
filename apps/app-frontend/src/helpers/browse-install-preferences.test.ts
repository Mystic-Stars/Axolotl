import assert from 'node:assert/strict'
import test from 'node:test'

import type { Labrinth } from '@modrinth/api-client'
import {
	getLatestMatchingInstallVersion,
	getTargetInstallPreferences,
} from '@modrinth/ui/src/layouts/shared/browse-tab/composables/install-logic.ts'

function version(
	id: string,
	loaders: string[],
	options: {
		datePublished?: string
		versionType?: Labrinth.Versions.v2.Version['version_type']
	} = {},
): Labrinth.Versions.v2.Version {
	return {
		id,
		date_published: options.datePublished ?? '2026-08-18T00:00:00Z',
		version_type: options.versionType ?? 'release',
		game_versions: ['1.21.1'],
		loaders,
	} as Labrinth.Versions.v2.Version
}

test('resource packs ignore the target game version and match the Minecraft loader', () => {
	const preferences = getTargetInstallPreferences(
		{ gameVersion: '26.2', loader: 'neoforge' },
		'resourcepack',
	)

	assert.deepEqual(preferences, {
		gameVersions: [],
		loaders: ['minecraft'],
	})
	assert.equal(
		getLatestMatchingInstallVersion(
			[version('minecraft-resource-pack', ['minecraft']), version('neoforge-mod', ['neoforge'])],
			preferences,
		)?.id,
		'minecraft-resource-pack',
	)
})

test('automatic installs prefer a compatible release over a newer beta', () => {
	const preferences = getTargetInstallPreferences(
		{ gameVersion: '1.21.1', loader: 'neoforge' },
		'mod',
	)

	assert.equal(
		getLatestMatchingInstallVersion(
			[
				version('newer-beta', ['neoforge'], {
					datePublished: '2026-06-01T00:00:00Z',
					versionType: 'beta',
				}),
				version('stable-release', ['neoforge'], {
					datePublished: '2026-05-01T00:00:00Z',
				}),
			],
			preferences,
		)?.id,
		'stable-release',
	)
})

test('shader packs use their Iris compatibility tag instead of the target mod loader', () => {
	const preferences = getTargetInstallPreferences(
		{ gameVersion: '1.21.1', loader: 'neoforge' },
		'shader',
	)

	assert.deepEqual(preferences, {
		gameVersions: ['1.21.1'],
		loaders: ['iris'],
	})
	assert.equal(
		getLatestMatchingInstallVersion(
			[version('iris-shader-pack', ['iris']), version('neoforge-mod', ['neoforge'])],
			preferences,
		)?.id,
		'iris-shader-pack',
	)
})
