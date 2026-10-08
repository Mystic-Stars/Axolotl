import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

test('context menu removes global listeners from their registration targets', () => {
    const source = readFileSync(new URL('./ContextMenu.vue', import.meta.url), 'utf8')

    assert.match(source, /window\.addEventListener\('click', handleClickOutside\)/)
    assert.match(source, /window\.removeEventListener\('click', handleClickOutside\)/)
    assert.match(source, /document\.body\.addEventListener\('keyup', onEscKeyRelease\)/)
    assert.match(source, /document\.body\.removeEventListener\('keyup', onEscKeyRelease\)/)
    assert.match(source, /window\.addEventListener\('resize', updatePosition\)/)
    assert.match(source, /window\.removeEventListener\('resize', updatePosition\)/)
    assert.match(source, /window\.addEventListener\('scroll', updatePosition, true\)/)
    assert.match(source, /window\.removeEventListener\('scroll', updatePosition, true\)/)
    assert.match(source, /resizeObserver\?\.disconnect\(\)/)
})
