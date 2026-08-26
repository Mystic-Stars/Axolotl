/**
 * Provider-aware query normalization and variant expansion.
 *
 * Modrinth (Labrinth) search tokenizes queries and prefix-matches tokens, so a
 * query like `examplemod` can never match a project named "Example Mod", while
 * CurseForge's `searchFilter` is only reliable in slug form (`example-mod`).
 * Since neither server can be changed, the desktop client rewrites the query
 * before it leaves the app: the primary form preserves what the user typed,
 * and fallback variants cover the forms the providers actually understand.
 */

export interface SearchQueryExpansion {
	/** Trimmed, diacritics-stripped, lowercased query. */
	normalized: string
	/** True when the query contains no whitespace or separator characters. */
	compact: boolean
	/**
	 * Ordered Modrinth query candidates, primary first. Each candidate is a
	 * normalized, deduplicated, non-empty string.
	 */
	modrinthVariants: string[]
	/** Ordered CurseForge query candidates, primary (slug form) first. */
	curseforgeVariants: string[]
}

const SEPARATOR_PATTERN = /[-_.+~,;:'"()[\]{}]+/gu

function dedupe(values: string[]): string[] {
	return [...new Set(values.filter((value) => value.length > 0))]
}

/**
 * Normalizes free text: trims, collapses whitespace, strips combining marks
 * (diacritics) and lowercases. Punctuation is preserved.
 */
export function normalizeSearchText(value: string): string {
	return value
		.trim()
		.normalize('NFD')
		.replace(/\p{M}/gu, '')
		.toLocaleLowerCase()
		.replace(/\s+/g, ' ')
}

/**
 * Removes every non-letter/non-digit character (spaces, hyphens, punctuation),
 * producing the compact form of a query (`example mod` → `examplemod`).
 */
export function compactSearchText(value: string): string {
	return compactSearchTextPreservingCase(value).toLocaleLowerCase()
}

function compactSearchTextPreservingCase(value: string): string {
	return value
		.normalize('NFD')
		.replace(/\p{M}/gu, '')
		.replace(/[^\p{L}\p{N}]+/gu, '')
}

/**
 * Converts free text into a CurseForge-friendly slug form
 * (`Example Mod!` → `example-mod`). CurseForge's search filter matches slugs,
 * where spaces and punctuation do not work.
 */
export function slugifySearchText(value: string): string {
	return normalizeSearchText(value)
		.replace(SEPARATOR_PATTERN, ' ')
		.replace(/[^\p{L}\p{N}]+/gu, '-')
		.replace(/-{2,}/g, '-')
		.replace(/^-+|-+$/g, '')
}

/**
 * Splits concatenated camelCase/PascalCase text into words
 * (`SodiumExtra` → `Sodium Extra`). The result is not lowercased; callers
 * normalize it as needed.
 */
export function splitCamelCaseSearchText(value: string): string {
	return value
		.replace(/([\p{Ll}\p{N}])(\p{Lu})/gu, '$1 $2')
		.replace(/(\p{Lu})(\p{Lu}\p{Ll})/gu, '$1 $2')
}

/**
 * Builds the ordered Modrinth query candidates for a base query: primary is
 * the query as normalized, then separator-free forms (spaces for hyphens,
 * camelCase splits, and the fully compact form) for engines that tokenize
 * differently. At most three candidates are returned.
 */
export function modrinthQueryVariants(base: string): string[] {
	const normalized = normalizeSearchText(base)
	if (!normalized) return []
	const compact = compactSearchText(base)
	const compactCased = compactSearchTextPreservingCase(base)
	const spaced = normalized.replace(SEPARATOR_PATTERN, ' ').replace(/\s+/g, ' ')
	const camelCaseSplit = splitCamelCaseSearchText(compactCased).toLocaleLowerCase().trim()
	return dedupe([normalized, spaced, camelCaseSplit, compact]).slice(0, 3)
}

/**
 * Builds the ordered CurseForge query candidates for a base query. The slug
 * form is primary because CurseForge search filters match project slugs;
 * the other variants cover cases where CurseForge accepts plain text.
 */
export function curseForgeQueryVariants(base: string): string[] {
	const normalized = normalizeSearchText(base)
	if (!normalized) return []
	const slug = slugifySearchText(normalized)
	const camelCaseSlug = slugifySearchText(
		splitCamelCaseSearchText(
			compactSearchTextPreservingCase(base),
		).toLocaleLowerCase(),
	)
	const compact = compactSearchText(base)
	return dedupe([slug, normalized, camelCaseSlug, compact]).slice(0, 3)
}

/**
 * Expands a raw browse query into provider-specific search candidates.
 * Returns `null` for empty queries (browsing without a query must stay
 * unfiltered).
 */
export function expandSearchQuery(raw: string): SearchQueryExpansion | null {
	const normalized = normalizeSearchText(raw)
	if (!normalized) return null
	const compact = compactSearchText(raw)
	const isCompact = !/\s/u.test(normalized) && normalized === compact
	return {
		normalized,
		compact: isCompact,
		modrinthVariants: modrinthQueryVariants(normalized),
		curseforgeVariants: curseForgeQueryVariants(normalized),
	}
}