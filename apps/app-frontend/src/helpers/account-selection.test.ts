import assert from 'node:assert/strict'
import test from 'node:test'

import { preferredOnlineAccountId } from './account-selection.ts'

const accounts = [
	{ account_id: 'offline', account_type: 'offline' },
	{ account_id: 'microsoft', account_type: 'microsoft' },
] as const

test('switches from an offline account to the first online account', () => {
	assert.equal(preferredOnlineAccountId('offline', accounts), 'microsoft')
})

test('keeps an online selection and handles an unknown selection', () => {
	assert.equal(preferredOnlineAccountId('microsoft', accounts), 'microsoft')
	assert.equal(preferredOnlineAccountId('missing', accounts), 'missing')
})
