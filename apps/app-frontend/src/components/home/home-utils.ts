export type HomeTimeBucket = 'late-night' | 'dawn' | 'morning' | 'afternoon' | 'evening' | 'night'
export type PlaytimeView = 'month' | 'year'

export type HeatmapDay = {
	date: Date
	dateKey: string
	inPeriod: boolean
}

export type MinecraftAccountLike = {
	account_type?: string
	profile?: {
		id?: string
		name?: string
	}
}

export function getTimeBucket(date: Date): HomeTimeBucket {
	const hour = date.getHours()
	if (hour < 5) return 'late-night'
	if (hour < 8) return 'dawn'
	if (hour < 12) return 'morning'
	if (hour < 17) return 'afternoon'
	if (hour < 21) return 'evening'
	return 'night'
}

export function stableGreetingIndex(seed: string, count: number): number {
	if (count <= 0) return 0

	let hash = 0
	for (const character of seed) {
		hash = (hash * 31 + character.charCodeAt(0)) | 0
	}
	return Math.abs(hash) % count
}

export function toDateKey(date: Date): string {
	const year = date.getFullYear()
	const month = String(date.getMonth() + 1).padStart(2, '0')
	const day = String(date.getDate()).padStart(2, '0')
	return `${year}-${month}-${day}`
}

export function dateFromKey(dateKey: string): Date {
	const [year, month, day] = dateKey.split('-').map(Number)
	return new Date(year, month - 1, day, 12)
}

export function startOfPeriod(anchor: Date, view: PlaytimeView): Date {
	return view === 'month'
		? new Date(anchor.getFullYear(), anchor.getMonth(), 1, 12)
		: new Date(anchor.getFullYear(), 0, 1, 12)
}

export function endOfPeriod(anchor: Date, view: PlaytimeView): Date {
	return view === 'month'
		? new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0, 12)
		: new Date(anchor.getFullYear(), 11, 31, 12)
}

export function shiftPeriod(anchor: Date, view: PlaytimeView, amount: number): Date {
	return view === 'month'
		? new Date(anchor.getFullYear(), anchor.getMonth() + amount, 1, 12)
		: new Date(anchor.getFullYear() + amount, 0, 1, 12)
}

export function buildHeatmapDays(anchor: Date, view: PlaytimeView): HeatmapDay[] {
	const periodStart = startOfPeriod(anchor, view)
	const periodEnd = endOfPeriod(anchor, view)
	const periodStartKey = toDateKey(periodStart)
	const periodEndKey = toDateKey(periodEnd)
	const gridStart = new Date(periodStart)
	gridStart.setDate(periodStart.getDate() - ((periodStart.getDay() + 6) % 7))
	const gridEnd = new Date(periodEnd)
	gridEnd.setDate(periodEnd.getDate() + ((7 - gridEnd.getDay()) % 7))

	const days: HeatmapDay[] = []
	const cursor = new Date(gridStart)
	while (cursor <= gridEnd) {
		const date = new Date(cursor)
		const dateKey = toDateKey(date)
		days.push({
			date,
			dateKey,
			inPeriod: dateKey >= periodStartKey && dateKey <= periodEndKey,
		})
		cursor.setDate(cursor.getDate() + 1)
	}
	return days
}

export function getPlaytimeLevel(seconds: number): number {
	if (seconds <= 0) return 0
	if (seconds <= 30 * 60) return 1
	if (seconds <= 90 * 60) return 2
	if (seconds <= 180 * 60) return 3
	return 4
}

export function getActivePlayerName(
	selectedUser: string | null | undefined,
	accounts: readonly MinecraftAccountLike[],
): string | null {
	if (!selectedUser) return null
	const account = accounts.find(
		(candidate) =>
			candidate.profile?.id === selectedUser &&
			(candidate.account_type === 'microsoft' || candidate.account_type === 'yggdrasil'),
	)
	return account?.profile?.name ?? null
}
