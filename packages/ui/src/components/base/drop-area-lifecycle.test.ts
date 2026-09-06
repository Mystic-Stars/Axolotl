import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

test('drop area removes its document listener when unmounted', () => {
	const source = readFileSync(new URL('./DropArea.vue', import.meta.url), 'utf8')

	assert.match(source, /document\.addEventListener\('dragenter', allowDrag\)/)
	assert.match(source, /document\.removeEventListener\('dragenter', allowDrag\)/)
})
