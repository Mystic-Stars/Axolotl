// Exercises the downgrade script against throwaway databases.
//
// The script rewrites a real installation's database, so every guard it has -
// refusing on unknown schema, refusing to guess the channel, refusing to touch
// a database it does not recognize - is worth a regression test.
//
// Run directly: node scripts/axolotl/downgrade-app-db.test.mjs

import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { DatabaseSync } from 'node:sqlite'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('downgrade-app-db.mjs', import.meta.url))
const migrationsDir = fileURLToPath(new URL('../../packages/app-lib/migrations', import.meta.url))
const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'axolotl-downgrade-'))

const CLOSE_BEHAVIOR = 20260903120000
const UNMAPPED = 20260101000000

// THESEUS_CONFIG_DIR takes priority over the platform directories, so the tests
// run the same way everywhere instead of only on Windows.
let settingsDir

function run(args, { configDir = settingsDir, env = {} } = {}) {
	const environment = { ...process.env, ...env }
	delete environment.THESEUS_CONFIG_DIR
	if (configDir) environment.THESEUS_CONFIG_DIR = configDir

	const result = spawnSync(process.execPath, [script, ...args], {
		encoding: 'utf8',
		env: environment,
	})
	return { status: result.status, output: `${result.stdout}${result.stderr}` }
}

function sampleDatabase(file, { withColumn = true, applied = [CLOSE_BEHAVIOR] } = {}) {
	fs.mkdirSync(path.dirname(file), { recursive: true })
	const database = new DatabaseSync(file)
	database.exec('CREATE TABLE settings (id INTEGER PRIMARY KEY, max_memory REAL)')
	if (withColumn) {
		database.exec(
			"ALTER TABLE settings ADD COLUMN close_behavior TEXT NOT NULL DEFAULT 'ask' CHECK (close_behavior IN ('ask', 'close', 'lightweight'))",
		)
	}
	database.exec('INSERT INTO settings (id, max_memory) VALUES (1, 4096)')
	database.exec(`CREATE TABLE _sqlx_migrations (
		version BIGINT PRIMARY KEY,
		description TEXT NOT NULL,
		installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
		success BOOLEAN NOT NULL,
		checksum BLOB NOT NULL,
		execution_time BIGINT NOT NULL
	)`)
	const insert = database.prepare(
		`INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
		 VALUES (?, ?, TRUE, X'00', 0)`,
	)
	for (const version of applied) insert.run(version, `migration-${version}`)
	database.close()
}

function migrations(file) {
	const database = new DatabaseSync(file, { readOnly: true })
	const rows = database
		.prepare('SELECT version FROM _sqlx_migrations ORDER BY version')
		.all()
		.map((row) => Number(row.version))
	database.close()
	return rows
}

function columns(file, table) {
	const database = new DatabaseSync(file, { readOnly: true })
	const names = database
		.prepare('SELECT name FROM pragma_table_info(?)')
		.all(table)
		.map((row) => row.name)
	database.close()
	return names
}

function backupsOf(file) {
	const dir = path.dirname(file)
	return fs
		.readdirSync(dir)
		.filter((name) => name.startsWith(`${path.basename(file)}.before-downgrade-`))
}

function check(name, condition, detail) {
	assert.ok(condition, detail ? `${name}: ${detail}` : name)
	console.log(`  ok  ${name}`)
}

// --- a database with an applied, revertible migration -------------------------

settingsDir = path.join(directory, 'settings')
const database = path.join(settingsDir, 'beta', 'app.db')
sampleDatabase(database)

console.log('dry run')
{
	const before = migrations(database)
	const result = run(['--to', String(CLOSE_BEHAVIOR)])
	check('reports success', result.status === 0)
	check('names the migration to drop', result.output.includes(String(CLOSE_BEHAVIOR)))
	check('announces the dry run', result.output.includes('dry run'))
	check('leaves the migrations alone', migrations(database).join() === before.join())
	check('leaves the column in place', columns(database, 'settings').includes('close_behavior'))
	check('writes no backup', backupsOf(database).length === 0)
}

console.log('apply')
{
	const result = run(['--to', String(CLOSE_BEHAVIOR), '--apply'])
	check('reports success', result.status === 0)
	check('removes the migration record', !migrations(database).includes(CLOSE_BEHAVIOR))
	check('drops the column', !columns(database, 'settings').includes('close_behavior'))
	check('keeps the remaining columns', columns(database, 'settings').includes('max_memory'))
	check(
		'keeps the row',
		new DatabaseSync(database, { readOnly: true })
			.prepare('SELECT COUNT(*) AS count FROM settings')
			.get().count === 1,
	)
	check('writes one backup', backupsOf(database).length === 1)
	check('explains how to restore it', result.output.includes('Restore the backup'))

	const [backup] = backupsOf(database)
	const backupPath = path.join(path.dirname(database), backup)
	check(
		'the backup is a database in its own right',
		migrations(backupPath).includes(CLOSE_BEHAVIOR),
	)
	check(
		'the backup still has the column',
		columns(backupPath, 'settings').includes('close_behavior'),
	)
	check(
		'the backup needs no sidecar files',
		!fs.existsSync(`${backupPath}-wal`) && !fs.existsSync(`${backupPath}-shm`),
	)
}

console.log('apply again')
{
	const before = migrations(database)
	const result = run(['--to', String(CLOSE_BEHAVIOR), '--apply'])
	check('reports success', result.status === 0)
	check('changes nothing', migrations(database).join() === before.join())
	check('writes no second backup', backupsOf(database).length === 1)
}

// --- refusals -----------------------------------------------------------------

console.log('refusals')
{
	const unknown = path.join(settingsDir, 'beta', 'unknown.db')
	sampleDatabase(unknown, { withColumn: false, applied: [UNMAPPED] })
	const before = migrations(unknown)

	const refused = run(['--db', unknown, '--to', String(UNMAPPED)])
	check('refuses a migration with no known schema', refused.status === 1)
	check('explains why', refused.output.includes('no known schema'))
	check('points at the override', refused.output.includes('--allow-unmapped'))
	check('changed nothing', migrations(unknown).join() === before.join())

	const allowed = run(['--db', unknown, '--to', String(UNMAPPED), '--allow-unmapped', '--apply'])
	check('proceeds with --allow-unmapped', allowed.status === 0)
	check('removes the record', !migrations(unknown).includes(UNMAPPED))
}

{
	// The column is already gone, so this script's mapping cannot describe the
	// database. Deleting the record anyway would leave a database no build opens.
	const mismatched = path.join(settingsDir, 'beta', 'mismatched.db')
	sampleDatabase(mismatched, { withColumn: false })

	const result = run(['--db', mismatched, '--to', String(CLOSE_BEHAVIOR)])
	check('refuses a mapping that does not match', result.status === 1)
	check('does not claim the column is fine', result.output.includes('NOT in the database'))
	check('keeps the migration record', migrations(mismatched).includes(CLOSE_BEHAVIOR))
}

{
	const stranger = path.join(settingsDir, 'beta', 'stranger.db')
	fs.mkdirSync(path.dirname(stranger), { recursive: true })
	new DatabaseSync(stranger).exec('CREATE TABLE unrelated (a)')

	const result = run(['--db', stranger, '--to', String(CLOSE_BEHAVIOR)])
	check("refuses a database that is not the launcher's", result.status === 1)
	check('says which table is missing', result.output.includes('_sqlx_migrations'))
}

console.log('argument validation')
{
	for (const [args, expected] of [
		[['--to', '2026'], '14 digit'],
		[['--to', '1e5'], '14 digit'],
		[['--to', String(CLOSE_BEHAVIOR), '--db'], 'needs a value'],
		[['--to', String(CLOSE_BEHAVIOR), '--channel', '../..'], 'must be one of'],
		[['--totally-bogus'], 'unknown argument'],
	]) {
		const result = run(args)
		check(`rejects ${args.join(' ')}`, result.status === 1 && result.output.includes(expected))
	}
}

// --- resolving which database to touch ---------------------------------------

console.log('resolving the settings directory')
{
	const suffixed = `${settingsDir}-pr538`
	sampleDatabase(path.join(suffixed, 'beta', 'app.db'))
	sampleDatabase(path.join(settingsDir, 'release', 'app.db'))

	const result = run(['--suffix', 'pr538', '--to', String(CLOSE_BEHAVIOR), '--apply'])
	check(
		'a suffix targets the built directory',
		result.status === 0 && result.output.includes('settings-pr538'),
	)
	check(
		'and leaves the unsuffixed directory untouched',
		migrations(path.join(settingsDir, 'release', 'app.db')).includes(CLOSE_BEHAVIOR),
	)

	const unknown = run(['--to', String(CLOSE_BEHAVIOR), '--suffix', 'does-not-exist'])
	check('an unknown suffix reports the database as missing', unknown.status === 1)
	check('without falling back to another directory', unknown.output.includes('does-not-exist'))
}

{
	// Without --db or --suffix, a database under a suffixed directory is worth
	// naming: that is the directory a suffixed build writes to, and silently
	// reporting "missing" would send the reader looking in the wrong place.
	const nearby = path.join(directory, 'nearby')
	sampleDatabase(path.join(nearby, 'beta', 'app.db'))
	sampleDatabase(path.join(`${nearby}-pr538`, 'beta', 'app.db'))
	fs.rmSync(path.join(nearby, 'beta', 'app.db'))

	const result = run(['--to', String(CLOSE_BEHAVIOR)], { configDir: nearby })
	check('a missing database names the suffixed one that exists', result.status === 1)
	check(
		'with its full path',
		result.output.includes(path.join(`${nearby}-pr538`, 'beta', 'app.db')),
	)
	check('and how to select it', result.output.includes('--suffix'))
}

{
	const ambiguous = path.join(directory, 'ambiguous')
	sampleDatabase(path.join(ambiguous, 'beta', 'app.db'))
	sampleDatabase(path.join(ambiguous, 'release', 'app.db'))

	const result = run(['--to', String(CLOSE_BEHAVIOR)], { configDir: ambiguous })
	check('two channels need an explicit choice', result.status === 1)
	check('and say so', result.output.includes('--channel'))
}

{
	const empty = path.join(directory, 'empty')
	fs.mkdirSync(empty, { recursive: true })

	const result = run(['--to', String(CLOSE_BEHAVIOR)], { configDir: empty })
	check('an empty directory needs an explicit channel', result.status === 1)
	check('and says which flag to pass', result.output.includes('--channel'))
}

// --- keeping the migration mapping honest -------------------------------------

console.log('keeping the migration mapping honest')
{
	const source = fs.readFileSync(script, 'utf8')
	const block = source.slice(source.indexOf('const REVERTIBLE_COLUMNS = {'))
	const body = block.slice(0, block.indexOf('\n}'))
	const registered = [...body.matchAll(/^\t(\d{14}):/gm)].map((match) => Number(match[1]))

	const files = fs.readdirSync(migrationsDir).filter((name) => name.endsWith('.sql'))
	const versions = files.map((name) => Number(name.slice(0, 14)))

	// A mapping for a migration that does not exist is dead weight: it can never
	// match anything, and it makes the table look considered when it is not.
	const unknown = registered.filter((version) => !versions.includes(version))
	check('every mapped migration exists', unknown.length === 0, unknown.join(', '))

	// The script refuses any applied migration it has no mapping for, so a
	// column added without one turns the documented recovery into a refusal.
	const oldest = Math.min(...registered)
	const addingColumns = files
		.filter((name) => /ADD COLUMN/i.test(fs.readFileSync(path.join(migrationsDir, name), 'utf8')))
		.map((name) => Number(name.slice(0, 14)))
		.filter((version) => version >= oldest)
	const unmapped = addingColumns.filter((version) => !registered.includes(version))
	check('every migration that adds a column is mapped', unmapped.length === 0, unmapped.join(', '))
}

// --- refusing a database a running launcher holds open ------------------------

if (process.platform !== 'win32') {
	console.log('skipping the running launcher checks outside Windows')
} else {
	console.log('refusing a database a running launcher holds open')

	const held = path.join(directory, 'held')
	const heldDb = path.join(held, 'beta', 'app.db')
	sampleDatabase(heldDb)

	// tasklist matches on the name of the executable, so the probe has to be a
	// process that is certainly running: the test itself. A filter covering only
	// the first candidate would miss it and rewrite the database regardless.
	const runningImage = path.basename(process.execPath)

	const running = run(['--to', String(CLOSE_BEHAVIOR), '--apply'], {
		configDir: held,
		env: { AXOLOTL_LAUNCHER_IMAGES: `definitely-not-running.exe,${runningImage}` },
	})
	check('a launcher running under a later candidate is found', running.status === 1)
	check('and named in the refusal', running.output.includes(`${runningImage} is running`))
	check('the migration record stays', migrations(heldDb).includes(CLOSE_BEHAVIOR))

	const absent = run(['--to', String(CLOSE_BEHAVIOR), '--apply'], {
		configDir: held,
		env: { AXOLOTL_LAUNCHER_IMAGES: 'definitely-not-running.exe' },
	})
	check('no launcher running lets the downgrade through', absent.status === 0)
	check('and the record goes', !migrations(heldDb).includes(CLOSE_BEHAVIOR))

	// A database with nothing left to remove returns before the launcher check,
	// so this needs one of its own.
	const namelessDir = path.join(directory, 'nameless')
	sampleDatabase(path.join(namelessDir, 'beta', 'app.db'))

	const nameless = run(['--to', String(CLOSE_BEHAVIOR), '--apply'], {
		configDir: namelessDir,
		env: { AXOLOTL_LAUNCHER_IMAGES: ' , ' },
	})
	check('a list naming no process is rejected', nameless.status === 1)
	check('and says so', nameless.output.includes('names no process'))
}

fs.rmSync(directory, { recursive: true, force: true })
console.log('\nAll downgrade script checks passed.')
