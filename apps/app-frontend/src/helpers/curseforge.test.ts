import assert from 'node:assert/strict'
import test from 'node:test'

import {
	type CurseForgeFile,
	getCurseForgeDownloadFailureDetails,
	hasCompatibleCurseForgeFile,
} from './curseforge.ts'

function curseForgeFile(id: number, isAvailable: boolean, gameVersions: string[]): CurseForgeFile {
	return {
		id,
		modId: 322385,
		isAvailable,
		displayName: '',
		fileName: '',
		releaseType: 1,
		fileDate: '',
		fileLength: 0,
		hashes: [],
		fileFingerprint: 0,
		downloadCount: 0,
		gameVersions,
		dependencies: [],
	}
}

test('recognizes CurseForge download diagnostics without exposing them in the notification', () => {
	const details = getCurseForgeDownloadFailureDetails(
		new Error(
			'Network download error: connection failed\nDownload failed after 4/4 attempts. Recent attempt history:\n- attempt=4; url=https://mediafilez.forgecdn.net/files/example.jar; proxy=System; category=connect',
		),
	)

	assert.match(details ?? '', /forgecdn\.net/)
})

test('does not classify non-CurseForge download failures', () => {
	assert.equal(
		getCurseForgeDownloadFailureDetails(
			new Error(
				'Download failed after 4/4 attempts. Recent attempt history:\n- url=https://cdn.modrinth.com/data/example.jar',
			),
		),
		null,
	)
})

test('requires an available exact CurseForge game version match', () => {
	const files = [
		curseForgeFile(1, true, ['1.19.2']),
		curseForgeFile(2, false, ['1.20.1']),
		curseForgeFile(3, true, ['1.20.1']),
	]

	assert.equal(hasCompatibleCurseForgeFile(files, '1.20.1'), true)
	assert.equal(hasCompatibleCurseForgeFile(files, '1.20.2'), false)
})
