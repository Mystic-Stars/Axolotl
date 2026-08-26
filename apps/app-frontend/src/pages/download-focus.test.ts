import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

import type { InstallJobSnapshot, InstallJobStatus } from '@/helpers/install'

import {
	createDownloadFocusState,
	focusedDownloadJobId,
	reconcileDownloadFocus,
	stopDownloadFocusAutoFollow,
} from './download-focus.ts'

function job(status: InstallJobStatus): InstallJobSnapshot {
	return { job_id: 'job-a', status } as InstallJobSnapshot
}

test('focused active job selects Active and expands once', () => {
	const effect = reconcileDownloadFocus(createDownloadFocusState('job-a'), job('running'))
	assert.equal(effect.tab, 'active')
	assert.equal(effect.expand, true)
	assert.equal(effect.scroll, true)
	assert.equal(effect.state.autoFollow, true)
})

test('focused terminal job selects History and expands once', () => {
	const effect = reconcileDownloadFocus(createDownloadFocusState('job-a'), job('succeeded'))
	assert.equal(effect.tab, 'history')
	assert.equal(effect.expand, true)
	assert.equal(effect.state.autoFollow, false)
})

test('active focused job follows completion to History once', () => {
	const active = reconcileDownloadFocus(createDownloadFocusState('job-a'), job('running'))
	const completed = reconcileDownloadFocus(active.state, job('succeeded'))
	assert.equal(completed.tab, 'history')
	assert.equal(completed.expand, false)
	assert.equal(completed.scroll, false)
	assert.equal(completed.state.autoFollow, false)
	assert.equal(reconcileDownloadFocus(completed.state, job('succeeded')).tab, null)
})

test('manual tab selection stops completion auto-follow', () => {
	const active = reconcileDownloadFocus(createDownloadFocusState('job-a'), job('running'))
	const manual = stopDownloadFocusAutoFollow(active.state)
	assert.equal(reconcileDownloadFocus(manual, job('succeeded')).tab, null)
})

test('malformed or nonexistent focus leaves Downloads unchanged', () => {
	assert.equal(focusedDownloadJobId(undefined), null)
	assert.equal(focusedDownloadJobId(['job-a']), null)
	assert.equal(focusedDownloadJobId(''), null)
	const state = createDownloadFocusState('missing')
	assert.deepEqual(reconcileDownloadFocus(state, null), {
		state,
		tab: null,
		expand: false,
		scroll: false,
	})
})

test('Downloads focus reuses global manager without another install job listener', () => {
	const source = readFileSync(new URL('./Downloads.vue', import.meta.url), 'utf8')
	assert.doesNotMatch(source, /install_job_listener/)
	assert.doesNotMatch(source, /focusedJobId === job\.job_id/)
	assert.doesNotMatch(source, /ring-brand/)
})

test('Downloads covers upgrade phases and safely falls back for unknown phases', () => {
	const source = readFileSync(new URL('./Downloads.vue', import.meta.url), 'utf8')
	const phaseMessages = source.slice(
		source.indexOf('const phaseMessages ='),
		source.indexOf('const legacyDownloads'),
	)

	for (const phase of ['applying_content', 'verifying', 'completed']) {
		assert.match(phaseMessages, new RegExp(`\\b${phase}:`))
	}
	assert.match(phaseMessages, /satisfies Record<InstallPhaseId, MessageDescriptor>/)
	assert.match(source, /message \?\? messages\.unknownPhase/)
})

test('Downloads history links persisted upgrade result to standalone page', () => {
	const source = readFileSync(new URL('./Downloads.vue', import.meta.url), 'utf8')
	assert.match(source, /tab === 'history' && isSuccessfulUpgradeJob\(job\)/)
	assert.match(source, /router\.push\(upgradeResultLocation\(job\)\)/)
	assert.match(source, /messages\.viewUpgradeResult/)
	assert.doesNotMatch(source, /UpgradeResultDetails/)
})
