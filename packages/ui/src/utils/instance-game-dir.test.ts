import assert from 'node:assert/strict'
import test from 'node:test'

import {
	instanceVersionFolderName,
	isolatedGameDirOverride,
	joinIsolatedGameDir,
} from './instance-game-dir.ts'

test('strips the versions/ segment from a generic .minecraft scan key', () => {
	assert.equal(
		instanceVersionFolderName({
			name: 'versions/bmc4-v61',
			path: 'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61',
		}),
		'bmc4-v61',
	)
})

test('strips the launcher prefix and versions/ segment from a PCL2CE scan key', () => {
	assert.equal(
		instanceVersionFolderName({
			name: '.minecraft:versions/bmc4-v61',
			path: 'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61',
		}),
		'bmc4-v61',
	)
})

test('falls back to stripping the key when no path is available', () => {
	assert.equal(instanceVersionFolderName({ name: 'versions/bmc4-v61' }), 'bmc4-v61')
	assert.equal(instanceVersionFolderName({ name: '.minecraft:versions/bmc4-v61' }), 'bmc4-v61')
})

test('keeps plain instance names unchanged', () => {
	assert.equal(instanceVersionFolderName({ name: 'My Pack' }), 'My Pack')
	assert.equal(
		instanceVersionFolderName({ name: 'My Pack', path: 'C:\\MultiMC\\instances\\My Pack' }),
		'My Pack',
	)
})

test('handles forward-slash paths', () => {
	assert.equal(
		instanceVersionFolderName({
			name: 'versions/bmc4-v61',
			path: 'D:/Mincraft/PCL2CE/.minecraft/versions/bmc4-v61',
		}),
		'bmc4-v61',
	)
})

test('builds the isolated override under the .minecraft root', () => {
	assert.equal(
		isolatedGameDirOverride('D:\\Mincraft\\PCL2CE\\.minecraft', {
			name: 'versions/bmc4-v61',
			path: 'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61',
		}),
		'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61',
	)
})

test('builds the isolated override from a prefixed PCL2CE key', () => {
	assert.equal(
		isolatedGameDirOverride('D:\\Mincraft\\PCL2CE\\.minecraft', {
			name: '.minecraft:versions/bmc4-v61',
			path: 'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61',
		}),
		'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61',
	)
})

test('uses forward slashes consistently on POSIX-style roots', () => {
	assert.equal(
		isolatedGameDirOverride('/home/user/.minecraft', {
			name: 'versions/bmc4-v61',
			path: '/home/user/.minecraft/versions/bmc4-v61',
		}),
		'/home/user/.minecraft/versions/bmc4-v61',
	)
})

test('trims trailing separators from the root before joining', () => {
	assert.equal(
		isolatedGameDirOverride('D:\\Mincraft\\PCL2CE\\.minecraft\\', {
			name: 'versions/bmc4-v61',
			path: 'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61',
		}),
		'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61',
	)
})

test('returns the root itself when it is already the version folder', () => {
	const versionPath = 'D:\\Mincraft\\PCL2CE\\.minecraft\\versions\\bmc4-v61'
	assert.equal(
		isolatedGameDirOverride(versionPath, {
			name: 'versions/bmc4-v61',
			path: versionPath,
		}),
		versionPath,
	)
})

test('joinIsolatedGameDir matches the root separator style', () => {
	assert.equal(
		joinIsolatedGameDir('D:\\Games\\.minecraft', '1.20.1'),
		'D:\\Games\\.minecraft\\versions\\1.20.1',
	)
	assert.equal(
		joinIsolatedGameDir('/home/user/.minecraft', '1.20.1'),
		'/home/user/.minecraft/versions/1.20.1',
	)
	assert.equal(
		joinIsolatedGameDir('D:\\Games\\.minecraft\\', '1.20.1'),
		'D:\\Games\\.minecraft\\versions\\1.20.1',
	)
})
