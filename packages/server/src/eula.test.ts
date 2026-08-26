import assert from 'node:assert/strict'
import { test } from 'node:test'

import { parseEula, setEulaAccepted } from './eula.ts'

const SAMPLE_EULA = [
	'#By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).',
	'#Wed Jan 01 00:00:00 UTC 2025',
	'eula=false',
].join('\n')

test('parses eula state', () => {
	assert.equal(parseEula(SAMPLE_EULA).accepted, false)
	assert.equal(parseEula(SAMPLE_EULA.replace('eula=false', 'eula=true')).accepted, true)
	assert.equal(parseEula('').accepted, false)
})

test('accepting the eula preserves the surrounding text', () => {
	const accepted = setEulaAccepted(SAMPLE_EULA, true)
	assert.equal(parseEula(accepted).accepted, true)
	assert.equal(accepted.split('\n')[0], SAMPLE_EULA.split('\n')[0])
	assert.equal(accepted.split('\n')[1], SAMPLE_EULA.split('\n')[1])
})

test('declining rewrites eula back to false', () => {
	const accepted = setEulaAccepted(SAMPLE_EULA, true)
	const declined = setEulaAccepted(accepted, false)
	assert.equal(parseEula(declined).accepted, false)
})
