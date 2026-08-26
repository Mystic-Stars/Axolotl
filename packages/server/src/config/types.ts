import type { PropertiesEntry } from '../properties.ts'

export type ConfigFieldKind = 'boolean' | 'integer' | 'number' | 'string' | 'enum'

export interface ConfigFieldDefinition {
	key: string
	kind: ConfigFieldKind
	/** Allowed values for `enum` fields. */
	options?: string[]
	min?: number
	max?: number
}

/**
 * Describes an editable configuration file. Formats plug in through the
 * `entries` accessors so new files (bukkit.yml, whitelist.json, ...) only need
 * a schema definition and, when needed, their own parse/serialize functions.
 */
export interface ConfigFileDefinition {
	id: string
	filename: string
	fields: ConfigFieldDefinition[]
	/** Fallback inference for keys not present in `fields`. */
	inferFieldKind: (key: string, value: string) => ConfigFieldKind
}

export interface ResolvedConfigField extends ConfigFieldDefinition {
	/** True when the kind was inferred from the value rather than declared. */
	inferred: boolean
}

const TRUE_VALUES = new Set(['true', 'false'])
const INTEGER_RE = /^-?\d+$/
const NUMBER_RE = /^-?\d+(\.\d+)?$/

export function inferFieldKindFromValue(value: string): ConfigFieldKind {
	if (TRUE_VALUES.has(value.toLowerCase())) return 'boolean'
	if (INTEGER_RE.test(value)) return 'integer'
	if (NUMBER_RE.test(value)) return 'number'
	return 'string'
}

export function resolveConfigField(
	definition: ConfigFileDefinition,
	key: string,
	value: string,
): ResolvedConfigField {
	const declared = definition.fields.find((field) => field.key === key)
	if (declared) return { ...declared, inferred: false }
	return { key, kind: definition.inferFieldKind(key, value), inferred: true }
}

/** Humanizes a config key (`server-port` -> `Server port`) as a fallback label. */
export function configFieldLabel(key: string): string {
	const parts = key.split(/[-_.]/)
	if (parts.length === 0) return key
	const first = parts[0]
	return (
		(first.length > 0 ? first[0].toUpperCase() + first.slice(1) : first) +
		' ' +
		parts.slice(1).join(' ')
	).trimEnd()
}

export interface ConfigFileDocument {
	definition: ConfigFileDefinition
	entries: PropertiesEntry[]
}

export function getRawValue(document: ConfigFileDocument, key: string): string | undefined {
	const entry = document.entries.find(
		(entry): entry is Extract<PropertiesEntry, { type: 'pair' }> =>
			entry.type === 'pair' && entry.key === key,
	)
	return entry?.value
}

export function setRawValue(document: ConfigFileDocument, key: string, value: string): void {
	const index = document.entries.findIndex((entry) => entry.type === 'pair' && entry.key === key)
	if (index === -1) {
		document.entries.push({ type: 'pair', key, value, separator: '=' })
		return
	}
	document.entries[index] = { type: 'pair', key, value, separator: '=' }
}
