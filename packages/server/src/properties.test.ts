import assert from 'node:assert/strict'
import { test } from 'node:test'

import { getProperty, parseProperties, serializeProperties, setProperty } from './properties.ts'

test('parses simple pairs with = separator', () => {
	const entries = parseProperties('server-port=25565\nonline-mode=true')
	assert.deepEqual(
		entries.map((e) => (e.type === 'pair' ? [e.key, e.value] : e.type)),
		[
			['server-port', '25565'],
			['online-mode', 'true'],
		],
	)
})

test('preserves comments, blank lines and key order on round-trip', () => {
	const text =
		'#Minecraft server properties\n#Wed Jan 01 00:00:00 UTC 2025\nserver-port=25565\n\nmotd=A Minecraft Server\nwhite-list=false'
	const entries = parseProperties(text)
	const roundTripped = serializeProperties(entries)
	assert.equal(roundTripped.split('\n')[0], '#Minecraft server properties')
	assert.equal(roundTripped.split('\n')[3], '')
	const reparsed = parseProperties(roundTripped)
	assert.equal(getProperty(reparsed, 'motd'), 'A Minecraft Server')
	assert.equal(getProperty(reparsed, 'white-list'), 'false')
})

test('supports colon separators and whitespace', () => {
	const entries = parseProperties('gamemode : creative')
	const pair = entries[0]
	assert.equal(pair.type, 'pair')
	if (pair.type === 'pair') {
		assert.equal(pair.key, 'gamemode')
		assert.equal(pair.value, 'creative')
		assert.equal(pair.separator, ':')
	}
})

test('escapes and unescapes values', () => {
	const entries = parseProperties('motd=Line one\\nLine two')
	assert.equal(getProperty(entries, 'motd'), 'Line one\nLine two')
	const serialized = serializeProperties(entries)
	assert.equal(getProperty(parseProperties(serialized), 'motd'), 'Line one\nLine two')
})

test('setProperty updates existing keys and appends new ones', () => {
	const entries = parseProperties('difficulty=easy')
	const updated = setProperty(entries, 'difficulty', 'hard')
	assert.equal(getProperty(updated, 'difficulty'), 'hard')
	const appended = setProperty(updated, 'new-key', 'value')
	assert.equal(getProperty(appended, 'new-key'), 'value')
	assert.equal(appended.length, updated.length + 1)
})

test('handles values containing escaped separators', () => {
	const entries = parseProperties('level-name=World\\=Two')
	assert.equal(getProperty(entries, 'level-name'), 'World=Two')
})
