import assert from 'node:assert/strict'
import { test } from 'node:test'

import { serverPropertiesDefinition } from './server-properties.ts'
import { configFieldLabel, resolveConfigField } from './types.ts'

test('declared keys keep their configured kind', () => {
	const difficulty = resolveConfigField(serverPropertiesDefinition, 'difficulty', 'normal')
	assert.equal(difficulty.kind, 'enum')
	assert.deepEqual(difficulty.options, ['peaceful', 'easy', 'normal', 'hard'])
	assert.equal(difficulty.inferred, false)

	const port = resolveConfigField(serverPropertiesDefinition, 'server-port', '25565')
	assert.equal(port.kind, 'integer')
	assert.equal(port.min, 1)
	assert.equal(port.max, 65535)

	const online = resolveConfigField(serverPropertiesDefinition, 'online-mode', 'true')
	assert.equal(online.kind, 'boolean')
})

test('unknown keys are inferred from their value', () => {
	const booleanField = resolveConfigField(serverPropertiesDefinition, 'some-flag', 'true')
	assert.equal(booleanField.kind, 'boolean')
	assert.equal(booleanField.inferred, true)

	const numericField = resolveConfigField(serverPropertiesDefinition, 'some-count', '12')
	assert.equal(numericField.kind, 'integer')

	const textField = resolveConfigField(serverPropertiesDefinition, 'some-name', 'hello')
	assert.equal(textField.kind, 'string')
})

test('labels are humanized from the key', () => {
	assert.equal(configFieldLabel('server-port'), 'Server port')
	assert.equal(configFieldLabel('rcon.password'), 'Rcon password')
})
