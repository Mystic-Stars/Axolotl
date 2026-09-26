import assert from 'node:assert/strict'
import test from 'node:test'

import {
	effectiveInstallProgress,
	effectiveParallelProgress,
	hasDeterminateInstallProgress,
	installProgressFraction,
	installProgressTextSource,
	preserveMonotonicProgress,
	type ProgressTextSnapshot,
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

test('content snapshots preserve displayed bytes without losing new item state', () => {
	const before = {
		...progressJob(),
		kind: 'change_content',
		phase: 'downloading_content',
		summary: { files_completed: 0, bytes_downloaded: 80, bytes_total: 100 },
		items: [{ status: 'downloading' }],
	}
	const next = {
		...before,
		summary: { files_completed: 1, bytes_downloaded: 60, bytes_total: 100 },
		items: [{ status: 'completed' }],
	}
	const merged = preserveMonotonicProgress(before, next)
	assert.equal(installProgressFraction(merged), 0.8)
	assert.equal(merged.summary.files_completed, 1)
	assert.equal(merged.items[0].status, 'completed')
	assert.equal(next.summary.bytes_downloaded, 60)
})

test('new phases, retries and terminal results retain authoritative progress', () => {
	const before = { ...progressJob(), progress: { current: 80, total: 100 } }
	for (const next of [
		{ ...before, phase: 'applying_content', progress: { current: 0, total: 100 } },
		{ ...before, status: 'queued', progress: { current: 0, total: 100 } },
		{ ...before, status: 'failed', progress: { current: 60, total: 100 } },
	])
		assert.equal(preserveMonotonicProgress(before, next), next)
})

test('modpack secondary byte progress remains monotonic', () => {
	const before = {
		...progressJob(),
		phase: 'downloading_content',
		progress: { current: 1, total: 3, secondary: { current: 80, total: 100 } },
	}
	const next = {
		...before,
		progress: { current: 2, total: 3, secondary: { current: 60, total: 100 } },
	}
	const merged = preserveMonotonicProgress(before, next)
	assert.equal(installProgressFraction(merged), 0.8)
	assert.equal(merged.progress.current, 2)
})

test('CurseForge modpacks use settled file progress for completion', () => {
	const job = {
		...progressJob(),
		provider: 'curse_forge',
		phase: 'downloading_content',
		progress: { current: 3, total: 3, secondary: { current: 900, total: 1000 } },
	}

	assert.equal(installProgressFraction(job), 1)
	assert.deepEqual(installProgressTextSource(job), { type: 'items', current: 3, total: 3 })
})

test('CurseForge file progress remains monotonic while byte samples change', () => {
	const before = {
		...progressJob(),
		provider: 'curse_forge',
		phase: 'downloading_content',
		progress: { current: 8, total: 10, secondary: { current: 1000, total: 1000 } },
	}
	const next = {
		...before,
		progress: { current: 7, total: 10, secondary: { current: 800, total: 1000 } },
	}
	const merged = preserveMonotonicProgress(before, next)

	assert.equal(merged.progress?.current, 8)
	assert.equal(installProgressFraction(merged), 0.8)
})

test('CurseForge completion switches from byte progress to settled files', () => {
	const before = {
		...progressJob(),
		provider: 'curse_forge',
		phase: 'downloading_content',
		progress: { current: 9, total: 10, secondary: { current: 999, total: 1000 } },
	}
	const completed = {
		...before,
		progress: { current: 10, total: 10, secondary: { current: 999, total: 1000 } },
	}

	const merged = preserveMonotonicProgress(before, completed)
	assert.equal(merged.progress?.current, 10)
	assert.equal(installProgressFraction(merged), 1)
})

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

test('CurseForge extraction does not inherit completed download bytes through the monotonic guard', () => {
	const downloaded: ProgressTextSnapshot = {
		...progressJob(),
		provider: 'curse_forge',
		phase: 'downloading_content',
		progress: { current: 10, total: 10, secondary: { current: 1000, total: 1000 } },
		summary: { files_completed: 10, files_total: 10, bytes_downloaded: 1000, bytes_total: 1000 },
	}
	for (const phase of ['extracting_overrides', 'finalizing']) {
		const next = { ...downloaded, phase, progress: null }
		const merged = preserveMonotonicProgress(downloaded, next)

		assert.equal(merged, next)
		assert.equal(installProgressFraction(merged), null)
		assert.equal(hasDeterminateInstallProgress(effectiveInstallProgress(merged)), false)
		assert.deepEqual(installProgressTextSource(merged), { type: 'phase' })
	}
})

test('CurseForge phase changes can restart progress even with the same total', () => {
	const before = {
		...progressJob(),
		provider: 'curse_forge',
		phase: 'downloading_content',
		progress: { current: 99, total: 100 },
	}
	for (const phase of ['extracting_overrides', 'downloading_minecraft']) {
		const next = { ...before, phase, progress: { current: 1, total: 100 } }
		assert.equal(installProgressFraction(preserveMonotonicProgress(before, next)), 0.01)
	}
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

test('content change uses aggregate live download bytes without losing its queued counter', () => {
	const preparing = progressJob({
		kind: 'change_content',
		phase: 'downloading_content',
		progress: { current: 0, total: 3 },
	})
	assert.deepEqual(effectiveInstallProgress(preparing), { current: 0, total: 3 })

	const downloading = progressJob({
		kind: 'change_content',
		phase: 'downloading_content',
		progress: { current: 0, total: 3 },
		summary: {
			files_completed: 0,
			files_total: 3,
			bytes_downloaded: 200,
			bytes_total: 800,
		},
	})
	assert.deepEqual(effectiveInstallProgress(downloading), { current: 200, total: 800 })
	assert.deepEqual(installProgressTextSource(downloading), {
		type: 'bytes',
		current: 200,
		total: 800,
	})
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
