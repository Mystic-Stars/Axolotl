// Exercises migration.mjs against a throwaway migrations directory.
//
// Run directly: node scripts/axolotl/migration.test.mjs

import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('migration.mjs', import.meta.url))
const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'axolotl-migration-'))

function check(name, condition, detail) {
	assert.ok(condition, detail ? `${name}: ${detail}` : name)
	console.log(`  ok  ${name}`)
}

function run(args, env = {}) {
	const result = spawnSync(process.execPath, [script, ...args], {
		encoding: 'utf8',
		env: { ...process.env, ...env },
	})
	return { status: result.status, output: `${result.stdout}${result.stderr}` }
}

console.log('next-version')
{
	const empty = run(['next-version'], { AXOLOTL_MIGRATIONS_DIR: directory })
	check('refuses an empty migrations directory', empty.status === 1)

	fs.writeFileSync(path.join(directory, '20260101000000_init.sql'), 'SELECT 1;\n')
	fs.writeFileSync(path.join(directory, '20260102000000_more.sql'), 'SELECT 1;\n')
	const next = run(['next-version'], { AXOLOTL_MIGRATIONS_DIR: directory })
	check('exits zero', next.status === 0, next.output)
	check('picks max+1', next.output.includes('20260102000001'), next.output)
}

console.log('new')
{
	const created = run(['new', 'Add My Setting'], { AXOLOTL_MIGRATIONS_DIR: directory })
	check('creates a migration file', created.status === 0, created.output)
	check(
		'names it max+1_slug',
		fs.existsSync(path.join(directory, '20260102000001_add-my-setting.sql')),
		created.output,
	)
}

console.log('renumber')
{
	const renumbered = run(
		['renumber', '20260102000001', '20260103000000'],
		{ AXOLOTL_MIGRATIONS_DIR: directory },
	)
	check('renames the file', renumbered.status === 0, renumbered.output)
	check(
		'leaves only the new name',
		fs.existsSync(path.join(directory, '20260103000000_add-my-setting.sql')) &&
			!fs.existsSync(path.join(directory, '20260102000001_add-my-setting.sql')),
		renumbered.output,
	)

	const behind = run(['renumber', '20260101000000', '20260102000000'], {
		AXOLOTL_MIGRATIONS_DIR: directory,
	})
	check('refuses a version that is still behind', behind.status === 1, behind.output)

	const duplicate = run(['renumber', '20260101000000', '20260103000000'], {
		AXOLOTL_MIGRATIONS_DIR: directory,
	})
	check('refuses a duplicate version', duplicate.status === 1, duplicate.output)
}

console.log('argument validation')
{
	check('rejects an unknown command', run(['nope']).status === 2)
	check('rejects a bad slug', run(['new', '!!!']).status === 1)
	check(
		'rejects a short version',
		run(['renumber', '2026', '20260103000000']).status === 1,
	)
}

fs.rmSync(directory, { recursive: true, force: true })
console.log('\nAll migration script checks passed.')
