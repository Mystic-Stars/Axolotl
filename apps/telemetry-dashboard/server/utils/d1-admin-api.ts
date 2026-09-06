import type {
	ActivityDto,
	AdminRange,
	DistributionItemDto,
	DistributionsDto,
	ErrorDetailDto,
	ErrorFiltersDto,
	ErrorRowDto,
	ErrorSampleDto,
	ErrorsPageDto,
	OverviewDto,
	ServiceCheckDto,
	SystemDto,
} from '../../shared/types/telemetry'
import type { TelemetryAdminApi } from './admin-api'
import { unavailable } from './errors'
import { type ErrorQuery, rangeDays, startDay } from './validation'

interface CountRow {
	count: number
}

interface OverviewRow {
	total_installations: number
	dau: number
	wau: number
	mau: number
	new_today: number
	error_occurrences: number
	distinct_groups: number
	r2_today: number
}

interface ActivityRow {
	day: string
	active_installations: number
	new_installations: number
	error_occurrences: number
}

interface DistributionRow {
	label: string
	value: number
}

interface ErrorRow {
	fingerprint: string
	error_type: string
	latest_message: string
	app_version: string
	first_seen: string
	last_seen: string
	occurrence_count: number
	affected_installations: number
	has_sample: number
}

interface SampleRegistration {
	object_key: string
}

export interface TelemetryQueryResult<T = Record<string, unknown>> {
	results: T[]
}

export interface TelemetryStatement {
	bind(...values: unknown[]): TelemetryStatement
	first<T = Record<string, unknown>>(): Promise<T | null>
	all<T = Record<string, unknown>>(): Promise<TelemetryQueryResult<T>>
}

export interface TelemetryDatabase {
	prepare(sql: string): TelemetryStatement
	batch(statements: TelemetryStatement[]): Promise<TelemetryQueryResult[]>
}

export interface TelemetryObject {
	body: ReadableStream
	httpMetadata?: { contentEncoding?: string }
}

export interface TelemetryObjectStore {
	get(
		key: string,
		options?: { range?: { offset: number; length: number } },
	): Promise<TelemetryObject | null>
}

const SAMPLE_UNCOMPRESSED_LIMIT = 32 * 1024
const ANALYTICS_ENDPOINT = 'https://api.cloudflare.com/client/v4/graphql'
const WORKERS_FREE_DAILY_REQUESTS = 100_000
const D1_FREE_DAILY_ROWS_READ = 5_000_000
const D1_FREE_DAILY_ROWS_WRITTEN = 100_000
const R2_FREE_MONTHLY_CLASS_A_OPS = 1_000_000
const USAGE_WARN_RATIO = 0.9

const CACHE_TTL_OVERVIEW = 600_000
const CACHE_TTL_ACTIVITY = 600_000
const CACHE_TTL_DISTRIBUTIONS = 900_000
const CACHE_TTL_ERRORS = 1_800_000
const CACHE_TTL_FILTERS = 900_000
const CACHE_TTL_DETAIL = 600_000
const CACHE_TTL_SAMPLE = 600_000
const CACHE_TTL_SYSTEM = 600_000

export interface CloudflareAnalyticsSettings {
	accountTag: string
	apiToken: string
}

interface AnalyticsResponse {
	data?: Record<string, unknown> | null
	errors?: Array<{ message?: string }>
}

interface CacheEntry {
	expiresAt: number
	value: unknown
}

const responseCache = new Map<string, CacheEntry>()

function utcDay(now = new Date()): string {
	return now.toISOString().slice(0, 10)
}

function daysAgoUtc(now: Date, days: number): string {
	const date = new Date(now)
	date.setUTCDate(date.getUTCDate() - days)
	return date.toISOString().slice(0, 10)
}

function formatCount(value: number): string {
	return Math.round(value).toLocaleString('en-US')
}

function errorMessage(error: unknown): string {
	const message = error instanceof Error ? error.message : String(error)
	return message.slice(0, 160)
}

async function withTimeout<T>(
	milliseconds: number,
	run: (signal: AbortSignal) => Promise<T>,
): Promise<T> {
	const controller = new AbortController()
	const timer = setTimeout(() => controller.abort(), milliseconds)
	try {
		return await run(controller.signal)
	} finally {
		clearTimeout(timer)
	}
}

async function queryAnalytics(
	fetcher: typeof fetch,
	settings: CloudflareAnalyticsSettings,
	query: string,
): Promise<Record<string, unknown>> {
	const response = await withTimeout(10_000, (signal) =>
		fetcher(ANALYTICS_ENDPOINT, {
			method: 'POST',
			headers: {
				authorization: `Bearer ${settings.apiToken}`,
				'content-type': 'application/json',
			},
			body: JSON.stringify({ query }),
			signal,
		}),
	)
	let body: AnalyticsResponse = {}
	try {
		body = (await response.json()) as AnalyticsResponse
	} catch {
		// A non-JSON error body is reported through the HTTP status below.
	}
	if (!response.ok) {
		throw new Error(`GraphQL 请求失败（HTTP ${response.status}）`)
	}
	if (body.errors?.length) {
		throw new Error(
			body.errors
				.map((entry) => entry.message ?? '未知错误')
				.join('；')
				.slice(0, 160),
		)
	}
	if (!body.data) throw new Error('GraphQL 响应缺少 data')
	return body.data
}

function analyticsRows(data: Record<string, unknown>, dataset: string): unknown[] {
	const accounts = (data.viewer as { accounts?: unknown } | undefined)?.accounts
	if (!Array.isArray(accounts) || accounts.length === 0) return []
	const node = (accounts[0] as Record<string, unknown> | undefined)?.[dataset]
	return Array.isArray(node) ? node : []
}

function sumOf(row: unknown, path: string[]): number {
	let current: unknown = row
	for (const field of path) {
		if (typeof current !== 'object' || current === null) return 0
		current = (current as Record<string, unknown>)[field]
	}
	const parsed = Number(current)
	return Number.isFinite(parsed) ? parsed : 0
}

function requireRow(rows: unknown[]): Record<string, unknown> {
	if (rows.length === 0) throw new Error('数据集暂无数据')
	return rows[0] as Record<string, unknown>
}

function mapError(row: ErrorRow): ErrorRowDto {
	return {
		fingerprint: row.fingerprint,
		errorType: row.error_type,
		latestMessage: row.latest_message,
		appVersion: row.app_version,
		firstSeen: row.first_seen,
		lastSeen: row.last_seen,
		occurrenceCount: Number(row.occurrence_count),
		affectedInstallations: Number(row.affected_installations),
		hasSample: Boolean(row.has_sample),
	}
}

function fillActivity(range: AdminRange, rows: ActivityRow[], now = new Date()): ActivityDto {
	const values = new Map(rows.map((row) => [row.day, row]))
	const start = new Date(`${startDay(range, now)}T00:00:00.000Z`)
	const points = Array.from({ length: rangeDays(range) }, (_, index) => {
		const day = new Date(start)
		day.setUTCDate(start.getUTCDate() + index)
		const key = day.toISOString().slice(0, 10)
		const row = values.get(key)
		return {
			day: key,
			activeInstallations: Number(row?.active_installations ?? 0),
			newInstallations: Number(row?.new_installations ?? 0),
			errorOccurrences: Number(row?.error_occurrences ?? 0),
		}
	})
	return { range, points }
}

function service(
	status: ServiceCheckDto['status'],
	label: string,
	detail: string,
): ServiceCheckDto {
	return { status, label, detail }
}

export class D1TelemetryAdminApi implements TelemetryAdminApi {
	constructor(
		private readonly db: TelemetryDatabase,
		private readonly r2: TelemetryObjectStore | undefined,
		private readonly options: {
			storeErrorContext: boolean
			healthUrl: string
			analytics?: CloudflareAnalyticsSettings
			fetcher?: typeof fetch
			now?: () => Date
			cacheTtlMs?: number
		},
	) {}

	private now(): Date {
		return this.options.now?.() ?? new Date()
	}

	private async cached<T>(key: string, ttlMs: number, run: () => Promise<T>): Promise<T> {
		const effectiveTtl = this.options.cacheTtlMs ?? ttlMs
		if (effectiveTtl <= 0) return run()
		const entry = responseCache.get(key)
		if (entry && entry.expiresAt > Date.now()) return entry.value as T
		const value = await run()
		if (responseCache.size > 256) {
			for (const [cachedKey, cachedEntry] of responseCache) {
				if (cachedEntry.expiresAt <= Date.now()) responseCache.delete(cachedKey)
			}
		}
		responseCache.set(key, { expiresAt: Date.now() + effectiveTtl, value })
		return value
	}

	async overview(range: AdminRange): Promise<OverviewDto> {
		return this.cached(`overview:${range}`, CACHE_TTL_OVERVIEW, async () => {
			const today = utcDay(this.now())
			const start = startDay(range, this.now())
			const yesterday = daysAgoUtc(this.now(), 1)
			const groupCountSql =
				range === '365d'
					? '(SELECT COUNT(*) FROM error_groups)'
					: '(SELECT COUNT(*) FROM error_range_stats WHERE range_days = ?)'
			const groupCountBindings = range === '365d' ? [] : [rangeDays(range)]
			const row = await this.db
				.prepare(
					`SELECT
						(SELECT COUNT(*) FROM installations) AS total_installations,
						(SELECT COUNT(*) FROM daily_active WHERE day = ?) AS dau,
						(SELECT COUNT(DISTINCT installation_hash) FROM daily_active
							WHERE day >= date(?, '-6 days') AND day <= ?) AS wau,
						(SELECT COUNT(DISTINCT installation_hash) FROM daily_active
							WHERE day >= date(?, '-29 days') AND day <= ?) AS mau,
						(SELECT COUNT(*) FROM installations WHERE first_seen_day = ?) AS new_today,
						(SELECT COALESCE(SUM(error_occurrences), 0) FROM daily_totals
							WHERE day >= ? AND day <= ?)
							+ (SELECT COALESCE(SUM(occurrence_count), 0) FROM error_daily WHERE day = ?) AS error_occurrences,
						${groupCountSql}
							+ (SELECT COUNT(*) FROM error_daily WHERE day = ?) AS distinct_groups,
						(SELECT COALESCE(object_count, 0) FROM error_context_budget WHERE day = ?) AS r2_today`,
				)
				.bind(today, today, today, today, today, today, start, yesterday, today, ...groupCountBindings, today, today)
				.first<OverviewRow>()
			if (!row) throw unavailable()
			const metric = (value: number, label: string) => ({ value: Number(value), label })
			return {
				range,
				generatedAt: this.now().toISOString(),
				metrics: {
					totalInstallations: metric(row.total_installations, '历史累计主动同意遥测的安装'),
					dau: metric(row.dau, '今日 UTC 唯一活跃安装'),
					wau: metric(row.wau, '最近 7 天唯一活跃安装'),
					mau: metric(row.mau, '最近 30 天唯一活跃安装'),
					newInstallationsToday: metric(row.new_today, '今日 UTC 首次出现'),
					errorOccurrences: metric(row.error_occurrences, `${range} 范围内发生次数`),
					distinctErrorGroups: metric(row.distinct_groups, `${range} 范围内错误指纹`),
					r2SamplesToday: metric(row.r2_today, '今日 UTC 已存储样本'),
				},
			}
		})
	}

	async activity(range: AdminRange): Promise<ActivityDto> {
		return this.cached(`activity:${range}`, CACHE_TTL_ACTIVITY, async () => {
			const today = utcDay(this.now())
			const yesterday = daysAgoUtc(this.now(), 1)
			const rows = await this.db
				.prepare(
					`SELECT day, active_installations, new_installations, error_occurrences
					FROM daily_totals WHERE day >= ? AND day <= ? ORDER BY day ASC`,
				)
				.bind(startDay(range, this.now()), yesterday)
				.all<ActivityRow>()
			const todayRow = await this.db
				.prepare(
					`SELECT
						(SELECT COUNT(*) FROM daily_active WHERE day = ?) AS active_installations,
						(SELECT COUNT(*) FROM installations WHERE first_seen_day = ?) AS new_installations,
						(SELECT COALESCE(SUM(occurrence_count), 0) FROM error_daily WHERE day = ?) AS error_occurrences`,
				)
				.bind(today, today, today)
				.first<ActivityRow>()
			const allRows = todayRow ? [...rows.results, todayRow] : rows.results
			return fillActivity(range, allRows, this.now())
		})
	}

	async distributions(range: AdminRange): Promise<DistributionsDto> {
		return this.cached(`distributions:${range}`, CACHE_TTL_DISTRIBUTIONS, async () => {
			const start = startDay(range, this.now())
			const today = utcDay(this.now())
			const query = async (
					field: 'app_version' | 'platform' | 'arch',
				): Promise<DistributionItemDto[]> => {
					const result = await this.db
						.prepare(
							`SELECT ${field} AS label, COUNT(DISTINCT installation_hash) AS value
							FROM daily_active WHERE day >= ? AND day <= ?
							GROUP BY ${field} ORDER BY value DESC, label ASC LIMIT 12`,
						)
						.bind(start, today)
					.all<DistributionRow>()
				return result.results.map((row) => ({ label: row.label, value: Number(row.value) }))
			}
			const [versions, platforms, architectures] = await Promise.all([
				query('app_version'),
				query('platform'),
				query('arch'),
			])
			return { range, versions, platforms, architectures }
		})
	}

	async errors(query: ErrorQuery): Promise<ErrorsPageDto> {
		return this.cached(`errors:${JSON.stringify(query)}`, CACHE_TTL_ERRORS, async () => {
			const today = utcDay(this.now())
			const start = startDay(query.range, this.now())

			const rangeSource =
				query.range === '365d'
					? {
							sql: `SELECT fingerprint, app_version, first_seen_day AS first_seen, last_seen_day AS last_seen,
									occurrence_count, installation_count, latest_error_type, latest_message,
									CASE WHEN sample_object_key IS NULL THEN 0 ELSE 1 END AS has_sample
									FROM error_groups`,
							bindings: [] as unknown[],
						}
					: {
							sql: `SELECT fingerprint, app_version, first_seen, last_seen,
									occurrence_count, installation_count, latest_error_type, latest_message,
									has_sample
									FROM error_range_stats WHERE range_days = ?`,
							bindings: [rangeDays(query.range)] as unknown[],
						}

			const liveSql = `SELECT ed.fingerprint, ed.app_version, ed.day AS first_seen, ed.day AS last_seen,
					SUM(ed.occurrence_count) AS occurrence_count,
					SUM(ed.installation_count) AS installation_count,
					MAX(ed.latest_error_type) AS latest_error_type,
					MAX(ed.latest_message) AS latest_message,
					MAX(ed.has_sample) AS has_sample
				FROM error_daily ed WHERE ed.day = ?
				GROUP BY ed.fingerprint, ed.app_version`

			const scopedSql = `SELECT fingerprint, app_version, first_seen, last_seen,
						occurrence_count, installation_count, latest_error_type, latest_message,
						has_sample
					FROM (${rangeSource.sql})
					UNION ALL
					${liveSql}`

			const bindings: unknown[] = [...rangeSource.bindings, today]
			const conditions: string[] = []
			if (query.search) {
				const escaped = query.search.toLowerCase().replace(/[\\%_]/g, '\\$&')
				conditions.push(
					"(LOWER(scoped.fingerprint) LIKE ? ESCAPE '\\' OR LOWER(scoped.latest_message) LIKE ? ESCAPE '\\' OR LOWER(scoped.latest_error_type) LIKE ? ESCAPE '\\')",
				)
				bindings.push(`%${escaped}%`, `%${escaped}%`, `%${escaped}%`)
			}
			if (query.version) {
				conditions.push('scoped.app_version = ?')
				bindings.push(query.version)
			}
			if (query.errorType) {
				conditions.push('scoped.latest_error_type = ?')
				bindings.push(query.errorType)
			}
			if (query.platform) {
				conditions.push(
					`EXISTS (SELECT 1 FROM error_reports er
							WHERE er.fingerprint = scoped.fingerprint AND er.app_version = scoped.app_version
								AND er.platform = ? AND er.day >= ? AND er.day <= ?)`,
				)
				bindings.push(query.platform, start, today)
			}
			if (query.hasSample !== null) {
				conditions.push(query.hasSample ? 'scoped.has_sample = 1' : 'scoped.has_sample = 0')
			}
			const where = conditions.length ? conditions.join(' AND ') : '1 = 1'

			const totalRow = await this.db
				.prepare(
					`WITH scoped AS (${scopedSql})
						SELECT COUNT(*) AS count FROM (
							SELECT fingerprint FROM scoped WHERE ${where} GROUP BY fingerprint
						)`,
				)
				.bind(...bindings)
				.first<CountRow>()

			const sortColumns: Record<ErrorQuery['sort'], string> = {
				lastSeen: 'last_seen',
				firstSeen: 'first_seen',
				occurrences: 'occurrence_count',
				installations: 'affected_installations',
			}
			const direction = query.direction === 'asc' ? 'ASC' : 'DESC'
			const offset = (query.page - 1) * query.pageSize
			const rows = await this.db
				.prepare(
					`WITH scoped AS (${scopedSql})
						SELECT
							fingerprint,
							MIN(first_seen) AS first_seen,
							MAX(last_seen) AS last_seen,
							SUM(occurrence_count) AS occurrence_count,
							SUM(installation_count) AS affected_installations,
							COALESCE((SELECT eg.latest_error_type FROM error_groups eg
								WHERE eg.fingerprint = scoped.fingerprint
								ORDER BY eg.last_seen_day DESC LIMIT 1), MAX(latest_error_type)) AS error_type,
							COALESCE((SELECT eg.latest_message FROM error_groups eg
								WHERE eg.fingerprint = scoped.fingerprint
								ORDER BY eg.last_seen_day DESC LIMIT 1), MAX(latest_message)) AS latest_message,
							COALESCE((SELECT eg.app_version FROM error_groups eg
								WHERE eg.fingerprint = scoped.fingerprint
								ORDER BY eg.last_seen_day DESC LIMIT 1), MAX(app_version)) AS app_version,
							MAX(has_sample) AS has_sample
						FROM scoped
						WHERE ${where}
						GROUP BY fingerprint
						ORDER BY ${sortColumns[query.sort]} ${direction}, fingerprint ASC
						LIMIT ? OFFSET ?`,
				)
				.bind(...bindings, query.pageSize, offset)
				.all<ErrorRow>()
			const filters = await this.errorFilters()
			const total = Number(totalRow?.count ?? 0)
			return {
				items: rows.results.map(mapError),
				page: query.page,
				pageSize: query.pageSize,
				total,
				totalPages: Math.max(1, Math.ceil(total / query.pageSize)),
				filters,
			}
		})
	}

	private async errorFilters(): Promise<ErrorFiltersDto> {
		return this.cached('errorFilters', CACHE_TTL_FILTERS, async () => {
			const [versions, platforms, errorTypes] = await Promise.all([
				this.db
					.prepare(
						'SELECT DISTINCT app_version AS value FROM error_groups ORDER BY value DESC LIMIT 100',
					)
					.all<{ value: string }>(),
				this.db
					.prepare('SELECT platform AS value FROM platforms ORDER BY value ASC LIMIT 100')
					.all<{ value: string }>(),
				this.db
					.prepare(
						'SELECT DISTINCT latest_error_type AS value FROM error_groups ORDER BY value ASC LIMIT 100',
					)
					.all<{ value: string }>(),
			])
			return {
				versions: versions.results.map((row) => row.value),
				platforms: platforms.results.map((row) => row.value),
				errorTypes: errorTypes.results.map((row) => row.value),
			}
		})
	}

	async errorDetail(fingerprint: string): Promise<ErrorDetailDto | null> {
		return this.cached(`errorDetail:${fingerprint}`, CACHE_TTL_DETAIL, async () => {
			const row = await this.db
				.prepare(
					`SELECT fingerprint, latest_error_type AS error_type, latest_message,
						app_version, first_seen_day AS first_seen, last_seen_day AS last_seen,
						occurrence_count, installation_count AS affected_installations,
						CASE WHEN sample_object_key IS NULL THEN 0 ELSE 1 END AS has_sample
					FROM error_groups WHERE fingerprint = ? ORDER BY last_seen_day DESC LIMIT 1`,
				)
				.bind(fingerprint)
				.first<ErrorRow>()
			if (row) return { ...mapError(row), route: null, command: null, stack: null }

			const live = await this.db
				.prepare(
					`SELECT
						ed.fingerprint,
						MAX(ed.latest_error_type) AS error_type,
						MAX(ed.latest_message) AS latest_message,
						ed.app_version,
						MIN(ed.day) AS first_seen,
						MAX(ed.day) AS last_seen,
						SUM(ed.occurrence_count) AS occurrence_count,
						SUM(ed.installation_count) AS affected_installations,
						MAX(ed.has_sample) AS has_sample
					FROM error_daily ed
					WHERE ed.fingerprint = ?
					GROUP BY ed.fingerprint, ed.app_version
					ORDER BY last_seen DESC LIMIT 1`,
				)
				.bind(fingerprint)
				.first<ErrorRow>()
			if (!live) return null
			return { ...mapError(live), route: null, command: null, stack: null }
		})
	}

	async errorSample(fingerprint: string): Promise<ErrorSampleDto | null> {
		if (!this.r2) return null
		return this.cached(`errorSample:${fingerprint}`, CACHE_TTL_SAMPLE, async () => {
			const registration = await this.db
				.prepare(
					`SELECT object_key FROM error_context_reservations
					WHERE fingerprint = ?
					ORDER BY created_at DESC LIMIT 1`,
				)
				.bind(fingerprint)
				.first<SampleRegistration>()
			if (!registration) return null
			const object = await this.r2!.get(registration.object_key)
			if (!object?.body) return null
			let stream: ReadableStream = object.body
			if (
				object.httpMetadata?.contentEncoding === 'gzip' ||
				registration.object_key.endsWith('.gz')
			) {
				stream = stream.pipeThrough(new DecompressionStream('gzip'))
			}
			const bytes = await new Response(stream).arrayBuffer()
			if (bytes.byteLength > SAMPLE_UNCOMPRESSED_LIMIT) throw unavailable()
			let value: Record<string, unknown>
			try {
				value = JSON.parse(new TextDecoder().decode(bytes)) as Record<string, unknown>
			} catch {
				throw unavailable()
			}
			const app =
				typeof value.app === 'object' && value.app ? (value.app as Record<string, unknown>) : {}
			const text = (input: unknown, limit: number): string =>
				typeof input === 'string' ? input.slice(0, limit) : ''
			const optional = (input: unknown, limit: number): string | null => text(input, limit) || null
			return {
				fingerprint,
				occurredAt: text(value.occurred_at, 64),
				appVersion: text(app.version, 64),
				platform: text(app.platform, 32),
				architecture: text(app.arch, 32),
				errorType: text(value.error_type, 128),
				message: text(value.message, 1_024),
				stack: optional(value.stack, 8_192),
				route: optional(value.route, 256),
				command: optional(value.command, 256),
				context: optional(value.context, 16_384),
			}
		})
	}

	private async workersRequests24h(
		settings: CloudflareAnalyticsSettings,
		fetcher: typeof fetch,
		now: Date,
	): Promise<number> {
		const start = new Date(now.getTime() - 24 * 60 * 60 * 1_000).toISOString()
		const data = await queryAnalytics(
			fetcher,
			settings,
			`{ viewer { accounts(filter: { accountTag: "${settings.accountTag}" }) {
				workersInvocationsAdaptive(
					filter: { datetime_geq: "${start}", datetime_lt: "${now.toISOString()}" }
					limit: 1
				) { sum { requests } }
			} } }`,
		)
		return sumOf(requireRow(analyticsRows(data, 'workersInvocationsAdaptive')), ['sum', 'requests'])
	}

	private async d1UsageToday(
		settings: CloudflareAnalyticsSettings,
		fetcher: typeof fetch,
		now: Date,
	): Promise<{ rowsRead: number; rowsWritten: number }> {
		const today = utcDay(now)
		const data = await queryAnalytics(
			fetcher,
			settings,
			`{ viewer { accounts(filter: { accountTag: "${settings.accountTag}" }) {
				d1AnalyticsAdaptiveGroups(
					filter: { date_geq: "${today}", date_leq: "${today}" }
					limit: 1
				) { sum { rowsRead rowsWritten } }
			} } }`,
		)
		const row = requireRow(analyticsRows(data, 'd1AnalyticsAdaptiveGroups'))
		return {
			rowsRead: sumOf(row, ['sum', 'rowsRead']),
			rowsWritten: sumOf(row, ['sum', 'rowsWritten']),
		}
	}

	private async r2Operations30d(
		settings: CloudflareAnalyticsSettings,
		fetcher: typeof fetch,
		now: Date,
	): Promise<number> {
		const start = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1_000).toISOString().slice(0, 10)
		const data = await queryAnalytics(
			fetcher,
			settings,
			`{ viewer { accounts(filter: { accountTag: "${settings.accountTag}" }) {
				r2OperationsAdaptiveGroups(
					filter: { date_geq: "${start}", date_leq: "${utcDay(now)}" }
					limit: 1
				) { sum { requests } }
			} } }`,
		)
		return sumOf(requireRow(analyticsRows(data, 'r2OperationsAdaptiveGroups')), ['sum', 'requests'])
	}

	private async accountUsage(settings: CloudflareAnalyticsSettings): Promise<ServiceCheckDto> {
		if (!/^[0-9a-f]{32}$/i.test(settings.accountTag)) {
			return service('unavailable', '未配置', 'CLOUDFLARE_ACCOUNT_ID 格式无效（应为 32 位账户 ID）')
		}
		const fetcher = this.options.fetcher ?? fetch
		const now = this.now()
		const parts: string[] = []
		const failures: string[] = []
		let workersRatio = 0
		let d1WriteRatio = 0
		let r2Ratio = 0

		try {
			const requests = await this.workersRequests24h(settings, fetcher, now)
			workersRatio = requests / WORKERS_FREE_DAILY_REQUESTS
			parts.push(
				`Workers ${formatCount(requests)}/${formatCount(WORKERS_FREE_DAILY_REQUESTS)} 请求（24h）`,
			)
		} catch (error) {
			failures.push(`Workers 用量查询失败：${errorMessage(error)}`)
		}
		try {
			const { rowsRead, rowsWritten } = await this.d1UsageToday(settings, fetcher, now)
			d1WriteRatio = rowsWritten / D1_FREE_DAILY_ROWS_WRITTEN
			parts.push(
				`D1 读 ${formatCount(rowsRead)}/${formatCount(D1_FREE_DAILY_ROWS_READ)}、写 ${formatCount(rowsWritten)}/${formatCount(D1_FREE_DAILY_ROWS_WRITTEN)} 行（今日 UTC）`,
			)
		} catch (error) {
			failures.push(`D1 用量查询失败：${errorMessage(error)}`)
		}
		try {
			const requests = await this.r2Operations30d(settings, fetcher, now)
			r2Ratio = requests / R2_FREE_MONTHLY_CLASS_A_OPS
			parts.push(
				`R2 ${formatCount(requests)}/${formatCount(R2_FREE_MONTHLY_CLASS_A_OPS)} 操作（30 天，Class A+B 合计）`,
			)
		} catch (error) {
			failures.push(`R2 用量查询失败：${errorMessage(error)}`)
		}

		if (parts.length === 0) {
			return service('unavailable', '查询失败', failures.join('；') || 'Analytics API 无可用数据')
		}
		const detail = failures.length
			? `${parts.join(' · ')}；${failures.join('；')}`
			: parts.join(' · ')
		if (failures.length > 0) return service('degraded', '部分可用', detail)
		if (Math.max(workersRatio, d1WriteRatio, r2Ratio) >= USAGE_WARN_RATIO) {
			return service('degraded', '接近配额', detail)
		}
		return service('available', '额度充足', detail)
	}

	async system(): Promise<SystemDto> {
		return this.cached('system', CACHE_TTL_SYSTEM, async () => {
			const now = this.now()
			let publicWorker = service('unavailable', '不可用', '健康检查端点未响应')
			try {
				const response = await withTimeout(3_000, (signal) =>
					(this.options.fetcher ?? fetch)(this.options.healthUrl, {
						signal,
						headers: { accept: 'application/json' },
					}),
				)
				if (response.ok) publicWorker = service('available', '运行正常', '公开采集服务健康检查通过')
			} catch {
				publicWorker = service('unavailable', '不可用', '健康检查端点未响应')
			}

			let d1 = service('unavailable', '不可用', 'D1 查询失败')
			let latestDataDay: string | null = null
			let budget = 0
			let sampleKey: string | null = null
			try {
				const result = await this.db.batch([
					this.db.prepare('SELECT MAX(day) AS value FROM daily_totals'),
					this.db
						.prepare(
							'SELECT COALESCE(object_count, 0) AS value FROM error_context_budget WHERE day = ?',
						)
						.bind(utcDay(now)),
					this.db.prepare(
						'SELECT object_key AS value FROM error_context_reservations ORDER BY created_at DESC LIMIT 1',
					),
				])
				latestDataDay =
					(result[0].results[0] as { value?: string | null } | undefined)?.value ?? null
				budget = Number((result[1].results[0] as { value?: number } | undefined)?.value ?? 0)
				sampleKey = (result[2].results[0] as { value?: string | null } | undefined)?.value ?? null
				d1 = service('available', '可查询', '只读遥测查询可用')
			} catch {
				d1 = service('unavailable', '不可用', 'D1 查询失败')
			}

			let r2 = service('unavailable', '不可用', '没有可读取的登记样本')
			if (!this.options.storeErrorContext) {
				r2 = service('degraded', '已停用', '错误上下文存储已停用')
			} else if (this.r2 && sampleKey) {
				try {
					const sample = await this.r2.get(sampleKey, { range: { offset: 0, length: 1 } })
					if (sample) {
						try {
							await sample.body.cancel()
						} catch {
							// The probe reads a single byte; cancellation failures are irrelevant.
						}
						r2 = service('available', '可读取', '已确认一个登记样本可读取')
					}
				} catch {
					r2 = service('unavailable', '不可用', '登记样本无法读取')
				}
			} else if (this.r2) {
				r2 = service('degraded', '暂无样本', 'R2 binding 正常，但当前没有登记样本')
			}

			const yesterday = daysAgoUtc(now, 1)
			const cron = !latestDataDay
				? service('unavailable', '不可用', '没有可用的聚合日期')
				: latestDataDay >= yesterday
					? service('available', '数据最新', `最近聚合日期：${latestDataDay}（UTC）`)
					: service('degraded', '数据滞后', `最近聚合日期：${latestDataDay}（UTC）`)

			const accountUsage = !this.options.analytics
				? service(
						'unavailable',
						'未配置',
						'尚未配置 Cloudflare Analytics API：请为 dashboard Worker 设置 CLOUDFLARE_ACCOUNT_ID 与 CLOUDFLARE_ANALYTICS_TOKEN（Account Analytics: Read）',
					)
				: await this.accountUsage(this.options.analytics)

			return {
				generatedAt: now.toISOString(),
				publicWorker,
				d1,
				r2,
				storeErrorContext: this.options.storeErrorContext,
				r2Budget: { used: budget, limit: 2000 },
				limits: {
					samplesPerGroup: 3,
					dailyActiveRetentionDays: 35,
					errorReportsRetentionDays: 30,
					r2RetentionDays: 30,
					errorAggregatesRetentionDays: 365,
				},
				latestDataDay,
				cron,
				accountUsage,
			}
		})
	}
}

export { fillActivity }
