// Derive which columns a downgrade must drop, straight from the migration SQL.
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
			out += '""'
			continue
		}
		out += sql[index]
		index += 1
	}
	return out
}

export function parseAddColumns(sql) {
	const source = stripSqlNoise(sql)
	const columns = []
	ADD_COLUMN_RE.lastIndex = 0
	let match
	while ((match = ADD_COLUMN_RE.exec(source)) !== null) {
		const table = match[1]
		const column = match[2]
		if (!columns.some((entry) => entry.table === table && entry.column === column)) {
			columns.push({ table, column })
		}
	}
	return columns
}

export function parseCreatedTables(sql) {
	const source = stripSqlNoise(sql)
	const tables = []
	CREATE_TABLE_RE.lastIndex = 0
	let match
	while ((match = CREATE_TABLE_RE.exec(source)) !== null) {
		const table = match[1]
		if (!tables.includes(table)) tables.push(table)
	}
	return tables
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
 * version -> { columns: [{table, column}], createdTables: [string] }
 * for every migration at or after OLDEST_REVERTIBLE_VERSION.
 */
export function loadRevertibleMigrations(dir = MIGRATIONS_DIR) {
	const map = new Map()
	for (const { name, version } of listMigrationFiles(dir)) {
		if (version < OLDEST_REVERTIBLE_VERSION) continue
		const sql = readFileSync(join(dir, name), 'utf8')
		map.set(version, {
			name,
			columns: parseAddColumns(sql),
			createdTables: parseCreatedTables(sql),
		})
	}
	return map
}

export function isRevertibleVersion(version, revertible = loadRevertibleMigrations()) {
	return revertible.has(Number(version))
}
