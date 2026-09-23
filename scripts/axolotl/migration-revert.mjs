// Derive the schema a downgrade must undo, straight from the migration SQL.
//
// The hand-maintained REVERTIBLE_COLUMNS table used to live inside
// downgrade-app-db.mjs. Renumbering a migration (to clear the immutable-
// migration guard) silently desynchronized that table and blocked recovery.
// Reading the SQL removes the second source of truth.

import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

export const MIGRATIONS_DIR =
	process.env.AXOLOTL_MIGRATIONS_DIR ||
	fileURLToPath(new URL('../../packages/app-lib/migrations', import.meta.url))

// Only migrations at or after this version are treated as revertible. Older
// ones predate the documented recovery path and stay unmapped so a downgrade
// into them refuses rather than guesses.
export const OLDEST_REVERTIBLE_VERSION = 20260903120000

const ADD_COLUMN_RE =
	/ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?["'`]?([A-Za-z_][A-Za-z0-9_]*)["'`]?\s+ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?["'`]?([A-Za-z_][A-Za-z0-9_]*)["'`]?/gi

const CREATE_TABLE_RE =
	/CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?["'`]?([A-Za-z_][A-Za-z0-9_]*)["'`]?/gi

// SQLite has no DROP COLUMN RENAME counterpart, so a rename is found rather
// than reversed: the table it left behind is the one worth dropping.
const RENAME_TABLE_RE =
	/ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?["'`]?([A-Za-z_][A-Za-z0-9_]*)["'`]?\s+RENAME\s+TO\s+["'`]?([A-Za-z_][A-Za-z0-9_]*)["'`]?/gi

// Remove line/block comments and string literals so DDL recognizers only see
// executable SQL. A commented-out CREATE TABLE must not look like a real table.
export function stripSqlNoise(sql) {
	let out = ''
	let index = 0
	while (index < sql.length) {
		const rest = sql.slice(index)
		if (rest.startsWith('--')) {
			const newline = sql.indexOf('\n', index)
			index = newline === -1 ? sql.length : newline + 1
			out += '\n'
			continue
		}
		if (rest.startsWith('/*')) {
			const end = sql.indexOf('*/', index + 2)
			index = end === -1 ? sql.length : end + 2
			out += ' '
			continue
		}
		const quote = sql[index]
		if (quote === "'" || quote === '"' || quote === '`') {
			const start = index
			index += 1
			while (index < sql.length) {
				if (sql[index] === quote) {
					if (sql[index + 1] === quote) {
						index += 2
						continue
					}
					index += 1
					break
				}
				index += 1
			}
			// Keep identifier quotes so "table"."column" still matches DDL regexes;
			// string literals become empty quotes and cannot fake identifiers.
			const raw = sql.slice(start, index)
			out += quote === "'" ? "''" : raw
			continue
		}
		out += sql[index]
		index += 1
	}
	return out
}

function collect(sql, pattern, apply) {
	const source = stripSqlNoise(sql)
	const found = []
	pattern.lastIndex = 0
	let match
	while ((match = pattern.exec(source)) !== null) {
		apply(match, found)
	}
	return found
}

function pushUnique(list, value) {
	if (!list.includes(value)) list.push(value)
}

export function parseAddColumns(sql) {
	return collect(sql, ADD_COLUMN_RE, (match, found) => {
		const entry = { table: match[1], column: match[2] }
		if (!found.some((other) => other.table === entry.table && other.column === entry.column)) {
			found.push(entry)
		}
	})
}

export function parseCreatedTables(sql) {
	return collect(sql, CREATE_TABLE_RE, (match, found) => pushUnique(found, match[1]))
}

export function parseRenamedTables(sql) {
	return collect(sql, RENAME_TABLE_RE, (match, found) => {
		const entry = { from: match[1], to: match[2] }
		if (!found.some((other) => other.from === entry.from && other.to === entry.to)) {
			found.push(entry)
		}
	})
}

export function migrationVersionFromName(name) {
	if (!name.endsWith('.sql')) return null
	const match = /^(\d{14})_[a-z0-9][a-z0-9_-]*\.sql$/.exec(name)
	return match ? Number(match[1]) : null
}

export function listMigrationFiles(dir = MIGRATIONS_DIR) {
	return readdirSync(dir)
		.filter((name) => name.endsWith('.sql'))
		.map((name) => ({ name, version: migrationVersionFromName(name) }))
		.filter((entry) => entry.version !== null)
		.sort((left, right) => left.version - right.version)
}

/**
 * Splits created tables into the ones the migration alone owns and the ones it
 * rebuilt. A rebuild creates a scratch table, drops a pre-existing one and
 * renames the scratch table onto its name, so the surviving table carries data
 * that predates the migration. Dropping it would delete that data, so a rebuild
 * is reported but never offered for dropping.
 */
function classifyTables(created, renames) {
	const createdTables = []
	const rebuiltTables = []

	for (const table of created) {
		let name = table
		for (const { from, to } of renames) {
			if (name === from) name = to
		}

		const renamed = name !== table
		if (renamed) rebuiltTables.push(name)
		else createdTables.push(name)
	}

	return { createdTables, rebuiltTables }
}

/**
 * version -> { name, columns: [{table, column}], createdTables: [string],
 * rebuiltTables: [string], renamedTables: [{from, to}] } for every migration at
 * or after OLDEST_REVERTIBLE_VERSION.
 *
 * A created table means the downgrade cannot undo the migration on its own:
 * SQLite has no DROP TABLE counterpart to leave behind safely, so the caller
 * refuses unless a table is explicitly allowed. Rebuilt tables are a separate
 * list because they hold data a drop would destroy.
 */
export function loadRevertibleMigrations(dir = MIGRATIONS_DIR) {
	const map = new Map()
	for (const { name, version } of listMigrationFiles(dir)) {
		if (version < OLDEST_REVERTIBLE_VERSION) continue
		const sql = readFileSync(join(dir, name), 'utf8')
		const renamedTables = parseRenamedTables(sql)
		const { createdTables, rebuiltTables } = classifyTables(parseCreatedTables(sql), renamedTables)
		map.set(version, {
			name,
			columns: parseAddColumns(sql),
			createdTables,
			rebuiltTables,
			renamedTables,
		})
	}
	return map
}
