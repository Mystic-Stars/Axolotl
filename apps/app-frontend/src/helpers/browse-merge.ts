export type BrowseMergeSort =
	| 'relevance'
	| 'downloads'
	| 'follows'
	| 'newest'
	| 'updated'
	| string

export interface BrowseMergeHit {
	provider?: 'modrinth' | 'curseforge' | string
	project_id: string
	downloads?: number | null
	follows?: number | null
	date_created?: string | null
	date_modified?: string | null
	chinese_search_score?: number | null
}

export interface MergeProviderResultsOptions<T extends BrowseMergeHit> {
	modrinthHits: T[]
	curseForgeHits: T[]
	sort: BrowseMergeSort | null | undefined
	query?: string | null
	limit: number
}

function providerKey(hit: BrowseMergeHit): string {
	return `${hit.provider ?? 'modrinth'}:${hit.project_id}`
}

function toTimestamp(value?: string | null): number {
	if (!value) return 0
	const time = Date.parse(value)
	return Number.isFinite(time) ? time : 0
}

function maxMetric(hits: BrowseMergeHit[], read: (hit: BrowseMergeHit) => number): number {
	let max = 0
	for (const hit of hits) {
		const value = read(hit)
		if (value > max) max = value
	}
	return max
}

function normalize(value: number, max: number): number {
	if (max <= 0) return 0
	return value / max
}

function providerTieBreak(hit: BrowseMergeHit): number {
	return hit.provider === 'curseforge' ? 1 : 0
}

function sortByMetric<T extends BrowseMergeHit>(
	hits: T[],
	read: (hit: BrowseMergeHit) => number,
	limit: number,
): T[] {
	return [...hits]
		.sort((left, right) => {
			const delta = read(right) - read(left)
			if (delta !== 0) return delta
			return providerTieBreak(left) - providerTieBreak(right)
		})
		.slice(0, limit)
}

/**
 * Merge Modrinth + CurseForge page results for "all sources".
 *
 * Dual-source pagination still uses the same offset on each provider (known
 * limitation: pages may have mild overlap/holes). Prefer metric order over
 * strict alternating so sort controls stay meaningful.
 */
export function mergeProviderResults<T extends BrowseMergeHit>(
	options: MergeProviderResultsOptions<T>,
): T[] {
	const { modrinthHits, curseForgeHits, sort, query, limit } = options
	const combined = [...modrinthHits, ...curseForgeHits]
	if (combined.length === 0 || limit <= 0) return []

	const effectiveSort = sort || 'relevance'
	const hasQuery = Boolean(query?.trim())

	if (effectiveSort === 'downloads') {
		return sortByMetric(combined, (hit) => hit.downloads ?? 0, limit)
	}

	if (effectiveSort === 'updated') {
		return sortByMetric(combined, (hit) => toTimestamp(hit.date_modified), limit)
	}

	if (effectiveSort === 'newest') {
		return sortByMetric(combined, (hit) => toTimestamp(hit.date_created), limit)
	}

	if (effectiveSort === 'follows') {
		const maxFollows = maxMetric(modrinthHits, (hit) => hit.follows ?? 0)
		const maxDownloads = maxMetric(curseForgeHits, (hit) => hit.downloads ?? 0)
		return [...combined]
			.map((hit) => {
				const score =
					hit.provider === 'curseforge'
						? normalize(hit.downloads ?? 0, maxDownloads)
						: normalize(hit.follows ?? 0, maxFollows)
				return { hit, score }
			})
			.sort((left, right) => {
				const delta = right.score - left.score
				if (delta !== 0) return delta
				return providerTieBreak(left.hit) - providerTieBreak(right.hit)
			})
			.slice(0, limit)
			.map(({ hit }) => hit)
	}

	// relevance (default) and unknown sorts: rank fusion + download prior
	const ranked = new Map<string, { hit: T; score: number }>()
	const maxDownloads = maxMetric(combined, (hit) => hit.downloads ?? 0)
	const maxChineseScore = maxMetric(combined, (hit) => hit.chinese_search_score ?? 0)
	const chineseWeight = hasQuery && maxChineseScore > 0 ? 0.65 : 0
	const rankWeight = chineseWeight > 0 ? 0.27 : hasQuery ? 0.82 : 0.55
	const downloadWeight = 1 - chineseWeight - rankWeight
	const rankBias = hasQuery ? 12 : 20

	for (const hits of [modrinthHits, curseForgeHits]) {
		hits.forEach((hit, index) => {
			const rankScore = 1 / (rankBias + index + 1)
			const downloadScore = normalize(Math.log1p(hit.downloads ?? 0), Math.log1p(maxDownloads))
			const chineseScore = normalize(hit.chinese_search_score ?? 0, maxChineseScore)
			const score =
				chineseWeight * chineseScore + rankWeight * rankScore + downloadWeight * downloadScore
			ranked.set(providerKey(hit), { hit, score })
		})
	}

	return [...ranked.values()]
		.sort((left, right) => {
			const delta = right.score - left.score
			if (delta !== 0) return delta
			return providerTieBreak(left.hit) - providerTieBreak(right.hit)
		})
		.slice(0, limit)
		.map(({ hit }) => hit)
}
