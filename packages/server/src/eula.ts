import { parseProperties, serializeProperties, type PropertiesEntry } from './properties.ts'

const EULA_KEY = 'eula'

export interface EulaDocument {
	entries: PropertiesEntry[]
	accepted: boolean
}

export function parseEula(text: string): EulaDocument {
	const entries = parseProperties(text)
	const value = entries.find(
		(entry): entry is Extract<PropertiesEntry, { type: 'pair' }> =>
			entry.type === 'pair' && entry.key === EULA_KEY,
	)
	return { entries, accepted: value?.value === 'true' }
}

export function setEulaAccepted(text: string, accepted: boolean): string {
	const { entries } = parseEula(text)
	const updated = entries.map((entry) =>
		entry.type === 'pair' && entry.key === EULA_KEY ? { ...entry, value: String(accepted) } : entry,
	)
	const hasEulaKey = updated.some((entry) => entry.type === 'pair' && entry.key === EULA_KEY)
	const finalEntries = hasEulaKey
		? updated
		: [
				...updated,
				{ type: 'pair' as const, key: EULA_KEY, value: String(accepted), separator: '=' },
			]
	return serializeProperties(finalEntries)
}
