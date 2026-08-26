import assert from 'node:assert/strict'
import test from 'node:test'

import {
	effectiveInstallProgress,
	effectiveParallelProgress,
	hasDeterminateInstallProgress,
	installProgressFraction,
	installProgressTextSource,
} from './install-progress.ts'

function progressJob(overrides: Record<string, unknown> = {}) {
	return {
		status: 'running',
		phase: 'resolving_loader',
		progress: null,
		details: { type: 'empty' },
		summary: {
			files_completed: 0,
			files_total: null,
			bytes_downloaded: 0,
			bytes_total: null,
		},
		...overrides,
	}
}

test('clears completed content progress when the next phase has no progress', () => {
	const completed = {
		phase: 'downloading_content',
		progress: { current: 10, total: 10 },
	}
	assert.equal(installProgressFraction(completed), 1)

	const nextPhase = {
		phase: 'downloading_minecraft',
		progress: null,
	}
	assert.equal(effectiveInstallProgress(nextPhase), null)
	assert.equal(installProgressFraction(nextPhase), null)
})

test('parallel track exposes its own progress', () => {
	const job = {
		phase: 'downloading_content',
		progress: { current: 2, total: 3, secondary: { current: 220, total: 300 } },
		parallel: {
			phase: 'downloading_minecraft',
			current: 120,
			total: 300,
		},
	}
	assert.deepEqual(effectiveInstallProgress(job), { current: 220, total: 300 })
	assert.deepEqual(effectiveParallelProgress(job), { current: 120, total: 300 })
	assert.equal(installProgressFraction(job), 220 / 300)

	assert.equal(effectiveParallelProgress({ phase: 'downloading_minecraft', progress: null }), null)
})

test('treats zero and non-finite totals as indeterminate', () => {
	for (const total of [0, Number.NaN, Number.POSITIVE_INFINITY]) {
		const progress = { current: 1, total }
		assert.equal(hasDeterminateInstallProgress(progress), false)
		assert.equal(installProgressFraction({ phase: 'downloading_minecraft', progress }), null)
	}
})

test('non-download phase ignores historical byte and file summary', () => {
	const source = installProgressTextSource(
		progressJob({
			phase: 'resolving_loader',
			summary: {
				files_completed: 186,
				files_total: 187,
				bytes_downloaded: 268,
				bytes_total: 18,
			},
		}),
	)

	assert.deepEqual(source, { type: 'phase' })
	assert.doesNotMatch(JSON.stringify(source), /268|18 MiB/)
})

test('non-download phase ignores historical file counter', () => {
	assert.deepEqual(
		installProgressTextSource(
			progressJob({
				phase: 'resolving_loader',
				summary: {
					files_completed: 186,
					files_total: 187,
					bytes_downloaded: 0,
					bytes_total: null,
				},
			}),
		),
		{ type: 'phase' },
	)
})

test('pack download uses current progress instead of stale summary', () => {
	assert.deepEqual(
		installProgressTextSource(
			progressJob({
				phase: 'downloading_pack_file',
				progress: { current: 0, total: 51 },
				summary: {
					files_completed: 0,
					files_total: null,
					bytes_downloaded: 268,
					bytes_total: 300,
				},
			}),
		),
		{ type: 'bytes', current: 0, total: 51 },
	)
})

test('minecraft download uses current progress instead of content summary', () => {
	assert.deepEqual(
		installProgressTextSource(
			progressJob({
				phase: 'downloading_minecraft',
				progress: { current: 0, total: 18 },
				summary: {
					files_completed: 187,
					files_total: 187,
					bytes_downloaded: 268,
					bytes_total: 300,
				},
			}),
		),
		{ type: 'bytes', current: 0, total: 18 },
	)
})

test('content download uses current secondary bytes', () => {
	assert.deepEqual(
		installProgressTextSource(
			progressJob({
				phase: 'downloading_content',
				progress: {
					current: 2,
					total: 3,
					secondary: { current: 220, total: 300 },
				},
			}),
		),
		{ type: 'bytes', current: 220, total: 300 },
	)
})

test('content download without secondary uses current file counter', () => {
	assert.deepEqual(
		installProgressTextSource(
			progressJob({
				phase: 'downloading_content',
				progress: { current: 2, total: 3 },
			}),
		),
		{ type: 'items', current: 2, total: 3 },
	)
})

test('Java downloading uses current byte progress', () => {
	assert.deepEqual(
		installProgressTextSource(
			progressJob({
				phase: 'preparing_java',
				progress: { current: 4, total: 12 },
				details: { type: 'java', major_version: 21, step: 'downloading' },
			}),
		),
		{ type: 'bytes', current: 4, total: 12 },
	)
})

test('waiting job preserves required-file progress policy', () => {
	assert.deepEqual(
		installProgressTextSource(
			progressJob({
				status: 'waiting_for_user',
				phase: 'downloading_content',
				progress: { current: 2, total: 3 },
			}),
		),
		{ type: 'required_files' },
	)
})
