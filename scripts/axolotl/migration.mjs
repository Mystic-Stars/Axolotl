// Create or renumber a launcher database migration without leaving the
// revert mapping (or the immutable-migration guard) out of step.
//
//   node scripts/axolotl/migration.mjs new add-my-setting
//   node scripts/axolotl/migration.mjs renumber 20260909010000 20260912120000
//   node scripts/axolotl/migration.mjs next-version
//
// Versions are 14-digit sqlx timestamps. `new` picks max(existing)+1 so a
// concurrent PR that already merged a higher version cannot produce an
// OUT-OF-ORDER migration. `renumber` only renames the file: the downgrade
// script derives columns from the SQL itself.

import { spawnSync } from 'node:child_process'
import { existsSync, renameSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { MIGRATIONS_DIR, listMigrationFiles } from './migration-revert.mjs'

function fail(message) {
	console.error(`error: ${message}`)
	process.exit(1)
}

function nextVersion(dir = MIGRATIONS_DIR) {
	const files = listMigrationFiles(dir)
	if (files.length === 0) fail(`no migrations found in ${dir}`)
	const max = files[files.length - 1].version
	return max + 1
}

function formatVersion(version) {
	return String(version).padStart(14, '0')
}

function normalizeSlug(slug) {
	const cleaned = slug
		.trim()
		.replace(/([a-z0-9])([A-Z])/g, '$1-$2')
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
	if (!/^[a-z0-9][a-z0-9-]*$/.test(cleaned)) {
		fail(`slug must look like add-my-setting, got ${slug}`)
	}
	return cleaned
}

function usage() {
	console.log(
		[
			'Create or renumber a database migration.',
			'',
			'  node scripts/axolotl/migration.mjs new <slug>',
			'  node scripts/axolotl/migration.mjs renumber <from> <to>',
			'  node scripts/axolotl/migration.mjs next-version',
			'',
			'`new` writes packages/app-lib/migrations/<max+1>_<slug>.sql.',
			'`renumber` renames an existing file when the immutable-migration',
			'guard requires a version past ones already on main.',
		].join('\n'),
	)
}

function commandNew(slug) {
	const normalized = normalizeSlug(slug)
	const version = nextVersion()
	const name = `${formatVersion(version)}_${normalized}.sql`
	const path = join(MIGRATIONS_DIR, name)
	if (existsSync(path)) fail(`${name} already exists`)

	const body = [
		`-- ${normalized.replace(/-/g, ' ')}`,
		'--',
		'-- ADD COLUMN statements are picked up automatically by',
		'-- scripts/axolotl/downgrade-app-db.mjs when undoing this migration.',
		'',
		'',
	].join('\n')

	writeFileSync(path, body)
	console.log(`created ${path}`)
	console.log(`version ${formatVersion(version)}`)
}

function commandRenumber(from, to) {
	if (!/^\d{14}$/.test(from) || !/^\d{14}$/.test(to)) {
		fail('renumber expects two 14-digit versions')
	}

	const files = listMigrationFiles()
	const source = files.find((entry) => entry.version === Number(from))
	if (!source) fail(`no migration with version ${from} in ${MIGRATIONS_DIR}`)
	if (files.some((entry) => entry.version === Number(to))) {
		fail(`version ${to} already exists`)
	}

	const others = files.filter((entry) => entry.version !== Number(from))
	const maxOther = others.length === 0 ? 0 : others[others.length - 1].version
	if (Number(to) <= maxOther) {
		fail(
			`${to} is not greater than the highest other migration (${formatVersion(maxOther)}); ` +
				`pick a version past that or the guard will still fail`,
		)
	}

	const targetName = `${formatVersion(Number(to))}${source.name.slice(14)}`
	const fromPath = join(MIGRATIONS_DIR, source.name)
	const toPath = join(MIGRATIONS_DIR, targetName)

	// Prefer git mv so history follows the file when the worktree is a git checkout.
	const moved = spawnSync('git', ['mv', fromPath, toPath], { encoding: 'utf8' })
	if (moved.status !== 0) {
		renameSync(fromPath, toPath)
	}

	console.log(`renamed ${source.name} -> ${targetName}`)
	console.log('downgrade-app-db.mjs derives columns from the SQL, so no mapping edit is needed')
	console.log('local databases that already applied the old version need a downgrade or a fresh suffix')
}

const [command, ...rest] = process.argv.slice(2)

if (command === 'new') {
	if (rest.length !== 1) fail('new expects exactly one slug')
	commandNew(rest[0])
} else if (command === 'renumber') {
	if (rest.length !== 2) fail('renumber expects <from> <to>')
	commandRenumber(rest[0], rest[1])
} else if (command === 'next-version') {
	console.log(formatVersion(nextVersion()))
} else {
	usage()
	if (command !== undefined && command !== '--help' && command !== '-h') process.exit(2)
}
