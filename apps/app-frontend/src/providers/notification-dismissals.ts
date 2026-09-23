export function loadNotificationDismissals(storageKeys: string[], fieldCount: number) {
	const keys = new Set<string>()
	let clearedAt: number | null = null
	for (const storageKey of storageKeys) {
		try {
			const value = JSON.parse(localStorage.getItem(storageKey) ?? 'null')
			if (Number.isFinite(value?.clearedAt)) {
				clearedAt = Math.max(clearedAt ?? 0, value.clearedAt)
			}
			const entries = Array.isArray(value) ? value : value?.keys
			if (!Array.isArray(entries)) continue
			for (const entry of entries) {
				if (typeof entry !== 'string') continue
				try {
					const fields = JSON.parse(entry)
					if (!Array.isArray(fields) || fields.length < 3) continue
					keys.add(
						JSON.stringify(
							Array.from(
								{ length: Math.max(fields.length, fieldCount) },
								(_, index) => fields[index] ?? '',
							),
						),
					)
				} catch {
					// A damaged entry must not discard other dismissal records.
				}
			}
		} catch {
			// Read each storage version independently so valid migration data survives.
		}
	}
	return { keys, clearedAt }
}
