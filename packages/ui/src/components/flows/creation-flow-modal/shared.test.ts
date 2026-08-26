import assert from 'node:assert/strict'
import test from 'node:test'

import { isVersionTypeMatch } from './version-types.ts'

test('categorizes legacy alpha and beta versions as ancient', () => {
	assert.equal(isVersionTypeMatch('alpha', 'a1.2.3', 'ancient'), true)
	assert.equal(isVersionTypeMatch('beta', 'b1.7.3', 'ancient'), true)
	assert.equal(isVersionTypeMatch('old_alpha', 'inf-20100618', 'ancient'), true)
	assert.equal(isVersionTypeMatch('old_beta', 'b1.8.1', 'ancient'), true)
	assert.equal(isVersionTypeMatch('old_snapshot', 'c0.30_01c', 'ancient'), true)
})

test('keeps April Fools versions separate from snapshots and ancient versions', () => {
	assert.equal(isVersionTypeMatch('snapshot', '25w14craftmine', 'alpha'), true)
	assert.equal(isVersionTypeMatch('snapshot', '25w14craftmine', 'snapshot'), false)
	assert.equal(isVersionTypeMatch('alpha', '25w14craftmine', 'ancient'), false)
})

test('does not classify release and regular snapshot versions as ancient', () => {
	assert.equal(isVersionTypeMatch('release', '1.21.11', 'ancient'), false)
	assert.equal(isVersionTypeMatch('snapshot', '26w34a', 'ancient'), false)
	assert.equal(isVersionTypeMatch('snapshot', '26w34a', 'snapshot'), true)
})
