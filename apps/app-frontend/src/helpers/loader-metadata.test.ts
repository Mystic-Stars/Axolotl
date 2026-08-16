import assert from 'node:assert/strict'
import test from 'node:test'

import {
	createLatestRequestGuard,
	gameVersionSelectorText,
	isLoaderSupportStateDisabled,
	loaderMetadataCacheKey,
	loaderMetadataQueryKey,
	loaderSupportState,
	loaderVersionSelectorText,
	loaderVersionsForGameVersion,
	loaderVersionSummaryState,
} from '../../../../packages/ui/src/components/flows/creation-flow-modal/loader-metadata.ts'

const manifest = (gameVersion: string, versions: string[]) => ({
	gameVersions: [
		{
			id: gameVersion,
			loaders: versions.map((id) => ({ id, stable: true })),
		},
	],
})

test('isolates loader metadata by loader and Minecraft version', () => {
	const forge262Key = loaderMetadataCacheKey('forge', '26.2')
	const forge1201Key = loaderMetadataCacheKey('forge', '1.20.1')
	const cache = {
		[forge262Key]: manifest('26.2', []),
		[forge1201Key]: manifest('1.20.1', ['47.4.22', '47.4.21']),
	}

	assert.notEqual(forge262Key, forge1201Key)
	assert.deepEqual(loaderMetadataQueryKey('forge', '26.2'), [
		'creation-flow',
		'loader-versions',
		'forge',
		'26.2',
	])
	assert.deepEqual(loaderMetadataQueryKey('forge', '1.20.1'), [
		'creation-flow',
		'loader-versions',
		'forge',
		'1.20.1',
	])

	assert.deepEqual(loaderVersionsForGameVersion(cache[forge262Key], '26.2'), [])
	assert.deepEqual(
		loaderVersionsForGameVersion(cache[forge1201Key], '1.20.1').map((version) => version.id),
		['47.4.22', '47.4.21'],
	)
	assert.deepEqual(loaderVersionsForGameVersion(cache[forge262Key], '26.2'), [])
})

test('rejects stale loader metadata requests after rapid selection changes', () => {
	const guard = createLatestRequestGuard()
	const minecraftARequest = guard.begin()
	const minecraftBRequest = guard.begin()
	const minecraftAReturnRequest = guard.begin()

	assert.equal(guard.isCurrent(minecraftARequest), false)
	assert.equal(guard.isCurrent(minecraftBRequest), false)
	assert.equal(guard.isCurrent(minecraftAReturnRequest), true)

	const fabricRequest = guard.begin()
	const forgeRequest = guard.begin()
	const neoForgeRequest = guard.begin()
	assert.equal(guard.isCurrent(fabricRequest), false)
	assert.equal(guard.isCurrent(forgeRequest), false)
	assert.equal(guard.isCurrent(neoForgeRequest), true)
})

test('does not treat missing, loading, or errored metadata as unsupported', () => {
	assert.equal(loaderSupportState('unknown', undefined, '1.20.1'), 'unknown')
	assert.equal(loaderSupportState('loading', undefined, '1.20.1'), 'loading')
	assert.equal(loaderSupportState('error', undefined, '1.20.1'), 'error')
})

test('marks only a successfully resolved empty loader set as unsupported', () => {
	assert.equal(
		loaderSupportState('success', manifest('1.20.1', ['47.4.22']), '1.20.1'),
		'supported',
	)
	assert.equal(loaderSupportState('success', manifest('26.2', []), '26.2'), 'unsupported')
})

test('restores loader support state across Minecraft version changes', () => {
	const cache = {
		[loaderMetadataCacheKey('forge', '1.20.1')]: manifest('1.20.1', ['47.4.22']),
		[loaderMetadataCacheKey('forge', '26.2')]: manifest('26.2', []),
	}

	const support = (gameVersion: string) =>
		loaderSupportState(
			'success',
			cache[loaderMetadataCacheKey('forge', gameVersion) as keyof typeof cache],
			gameVersion,
		)

	assert.equal(support('1.20.1'), 'supported')
	assert.equal(support('26.2'), 'unsupported')
	assert.equal(support('1.20.1'), 'supported')
})

test('disables unresolved and unsupported loader chips without disabling request errors', () => {
	assert.equal(isLoaderSupportStateDisabled('unknown'), true)
	assert.equal(isLoaderSupportStateDisabled('loading'), true)
	assert.equal(isLoaderSupportStateDisabled('unsupported'), true)
	assert.equal(isLoaderSupportStateDisabled('supported'), false)
	assert.equal(isLoaderSupportStateDisabled('error'), false)
})

test('tracks loader chips through pending, supported, and unsupported responses', () => {
	const pendingB = loaderSupportState('loading', undefined, '26.2')
	assert.equal(isLoaderSupportStateDisabled(pendingB), true)

	const unsupportedB = loaderSupportState('success', manifest('26.2', []), '26.2')
	assert.equal(unsupportedB, 'unsupported')
	assert.equal(isLoaderSupportStateDisabled(unsupportedB), true)

	const pendingC = loaderSupportState('loading', undefined, '1.20.1')
	assert.equal(isLoaderSupportStateDisabled(pendingC), true)

	const supportedC = loaderSupportState('success', manifest('1.20.1', ['47.4.22']), '1.20.1')
	assert.equal(supportedC, 'supported')
	assert.equal(isLoaderSupportStateDisabled(supportedC), false)
})

test('uses the loading label for both loader version selector placeholders while pending', () => {
	assert.deepEqual(
		loaderVersionSelectorText(true, false, {
			loading: 'Loading',
			empty: 'No versions available',
			placeholder: 'Select loader version',
			searchPlaceholder: 'Search loader version...',
		}),
		{ placeholder: 'Loading', searchPlaceholder: 'Loading' },
	)
})

test('hides an old loader version behind the loading summary until the new version resolves', () => {
	assert.equal(loaderVersionSummaryState(true, '47.4.22'), 'loading')
	assert.equal(loaderVersionSummaryState(false, '65.1.0'), 'selected')
	assert.equal(loaderVersionSummaryState(false, null), 'empty')
})

test('shows a loader-specific message after game version metadata resolves empty', () => {
	const labels = {
		loading: 'Loading',
		empty: 'No game versions support this loader',
		error: 'Failed to load game versions',
		placeholder: 'Select game version',
		searchPlaceholder: 'Search game version...',
	}

	assert.deepEqual(gameVersionSelectorText('loading', labels), {
		placeholder: 'Loading',
		searchPlaceholder: 'Loading',
	})
	assert.deepEqual(gameVersionSelectorText('empty', labels), {
		placeholder: 'No game versions support this loader',
		searchPlaceholder: 'No game versions support this loader',
	})
	assert.deepEqual(gameVersionSelectorText('error', labels), {
		placeholder: 'Failed to load game versions',
		searchPlaceholder: 'Failed to load game versions',
	})
})
