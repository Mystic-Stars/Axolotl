import { Hono } from 'hono'

import { redact, truncateUtf8 } from './redact'
import { batchSchema, type TelemetryBatch } from './schema'

const MAX_REQUEST_BYTES = 64 * 1024
const HARD_MAX_BATCHES_PER_INSTALLATION_PER_DAY = 25
const HARD_MAX_ACCEPTED_BATCHES_PER_DAY = 100_000

export interface Bindings {
	DB: D1Database
	INSTALLATION_HMAC_SECRET: string
	INGEST_ENABLED?: string
	MAX_BATCHES_PER_INSTALLATION_PER_DAY?: string
	MAX_ACCEPTED_BATCHES_PER_DAY?: string
}

type Variables = { requestId: string }
type IngestionReservation = 'reserved' | 'duplicate' | 'installation_limit' | 'global_limit'

const app = new Hono<{ Bindings: Bindings; Variables: Variables }>()

app.use('*', async (context, next) => {
	context.set('requestId', crypto.randomUUID())
	await next()
	context.header('Cache-Control', 'no-store')
})

app.get('/health', (context) =>
	context.json({ status: 'ok', schema_version: 1 }, 200, { 'Cache-Control': 'no-store' }),
)

app.post('/v1/batch', async (context) => {
	const contentLength = Number(context.req.header('content-length') ?? '0')
	if (Number.isFinite(contentLength) && contentLength > MAX_REQUEST_BYTES) {
		return context.json({ error: 'request_too_large' }, 413)
	}

	const raw = await context.req.arrayBuffer()
	if (raw.byteLength > MAX_REQUEST_BYTES) return context.json({ error: 'request_too_large' }, 413)

	let input: unknown
	try {
		input = JSON.parse(new TextDecoder().decode(raw))
	} catch {
		return context.json({ error: 'invalid_json' }, 400)
	}

	const parsed = batchSchema.safeParse(input)
	if (!parsed.success) {
		// Do not accept or log legacy error payloads after error reporting was removed.
		if (containsLegacyErrorEvent(input))
			return context.json({ error: 'error_reporting_disabled' }, 400)
		return context.json(
			{
				error: 'invalid_batch',
				issues: parsed.error.issues.map((issue) => ({
					path: issue.path.join('.'),
					code: issue.code,
				})),
			},
			400,
		)
	}
	if (context.env.INGEST_ENABLED === 'false') {
		return context.json({ error: 'ingest_disabled' }, 503, { 'Retry-After': '60' })
	}
	if (!context.env.INSTALLATION_HMAC_SECRET || context.env.INSTALLATION_HMAC_SECRET.length < 32) {
		return context.json({ error: 'service_unavailable' }, 503)
	}

	try {
		const accepted = await context.env.DB.prepare(
			'SELECT 1 FROM accepted_batches WHERE batch_id = ? LIMIT 1',
		)
			.bind(parsed.data.batch_id)
			.first()
		if (accepted) return context.json({ accepted: true, duplicate: true })

		const installationHash = await hmacInstallationId(
			context.env.INSTALLATION_HMAC_SECRET,
			parsed.data.installation_id,
		)
		const limits = ingestionLimits(context.env)
		const reservation = await reserveIngestion(
			context.env.DB,
			parsed.data.batch_id,
			installationHash,
			limits,
		)
		if (reservation === 'duplicate') return context.json({ accepted: true, duplicate: true })
		if (reservation === 'installation_limit') {
			return context.json({ error: 'installation_batch_limit' }, 429, {
				'Retry-After': retryAfterSeconds().toString(),
			})
		}
		if (reservation === 'global_limit') {
			console.error('Telemetry ingestion budget exhausted', { limit: limits.global })
			return context.json({ error: 'global_batch_limit' }, 429, {
				'Retry-After': retryAfterSeconds().toString(),
			})
		}

		try {
			await persistBatch(context.env.DB, sanitizeBatch(parsed.data), installationHash)
		} catch (error) {
			await rollbackIngestion(context.env.DB, parsed.data.batch_id, installationHash)
			throw error
		}
		return context.json({ accepted: true, duplicate: false })
	} catch (error) {
		console.error('Telemetry ingestion failed', {
			requestId: context.get('requestId'),
			error: error instanceof Error ? error.message : String(error),
		})
		return context.json({ error: 'temporarily_unavailable' }, 503)
	}
})

app.notFound((context) => context.json({ error: 'not_found' }, 404))
app.onError((error, context) => {
	console.error('Unhandled telemetry worker error', {
		requestId: context.get('requestId'),
		error: error.message,
	})
	return context.json({ error: 'temporarily_unavailable' }, 503)
})

function containsLegacyErrorEvent(input: unknown): boolean {
	if (!input || typeof input !== 'object') return false
	const events = (input as { events?: unknown }).events
	return (
		Array.isArray(events) &&
		events.some(
			(event) =>
				Boolean(event) &&
				typeof event === 'object' &&
				(event as { type?: unknown }).type === 'error',
		)
	)
}

async function hmacInstallationId(secret: string, installationId: string): Promise<string> {
	const key = await crypto.subtle.importKey(
		'raw',
		new TextEncoder().encode(secret),
		{ name: 'HMAC', hash: 'SHA-256' },
		false,
		['sign'],
	)
	const signature = await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(installationId))
	return [...new Uint8Array(signature)].map((byte) => byte.toString(16).padStart(2, '0')).join('')
}

function ingestionLimits(env: Bindings): { installation: number; global: number } {
	return {
		installation: Math.min(
			positiveInteger(
				env.MAX_BATCHES_PER_INSTALLATION_PER_DAY,
				HARD_MAX_BATCHES_PER_INSTALLATION_PER_DAY,
			),
			HARD_MAX_BATCHES_PER_INSTALLATION_PER_DAY,
		),
		global: Math.min(
			positiveInteger(env.MAX_ACCEPTED_BATCHES_PER_DAY, HARD_MAX_ACCEPTED_BATCHES_PER_DAY),
			HARD_MAX_ACCEPTED_BATCHES_PER_DAY,
		),
	}
}

async function reserveIngestion(
	db: D1Database,
	batchId: string,
	installationHash: string,
	limits: { installation: number; global: number },
): Promise<IngestionReservation> {
	const day = utcDay()
	const batch = await db
		.prepare(
			'INSERT OR IGNORE INTO accepted_batches (batch_id, installation_hash, accepted_at) VALUES (?, ?, unixepoch())',
		)
		.bind(batchId, installationHash)
		.run()
	if (batch.meta.changes !== 1) return 'duplicate'

	const installation = await db
		.prepare(
			`INSERT INTO ingestion_daily (day, installation_hash, accepted_batches)
			VALUES (?, ?, 1)
			ON CONFLICT (day, installation_hash) DO UPDATE
			SET accepted_batches = accepted_batches + 1
			WHERE accepted_batches < ?`,
		)
		.bind(day, installationHash, limits.installation)
		.run()
	if (installation.meta.changes !== 1) {
		await db.prepare('DELETE FROM accepted_batches WHERE batch_id = ?').bind(batchId).run()
		return 'installation_limit'
	}

	const global = await db
		.prepare(
			`INSERT INTO ingestion_global_daily (day, accepted_batches)
			VALUES (?, 1)
			ON CONFLICT (day) DO UPDATE
			SET accepted_batches = accepted_batches + 1
			WHERE accepted_batches < ?`,
		)
		.bind(day, limits.global)
		.run()
	if (global.meta.changes !== 1) {
		await db.batch([
			db
				.prepare(
					'UPDATE ingestion_daily SET accepted_batches = accepted_batches - 1 WHERE day = ? AND installation_hash = ?',
				)
				.bind(day, installationHash),
			db.prepare('DELETE FROM accepted_batches WHERE batch_id = ?').bind(batchId),
		])
		return 'global_limit'
	}

	const currentGlobal = await db
		.prepare('SELECT accepted_batches FROM ingestion_global_daily WHERE day = ?')
		.bind(day)
		.first<{ accepted_batches: number }>()
	if (currentGlobal) warnIngestionThresholds(currentGlobal.accepted_batches, limits.global)
	return 'reserved'
}

async function rollbackIngestion(
	db: D1Database,
	batchId: string,
	installationHash: string,
): Promise<void> {
	const day = utcDay()
	await db.batch([
		db
			.prepare('DELETE FROM accepted_batches WHERE batch_id = ? AND installation_hash = ?')
			.bind(batchId, installationHash),
		db
			.prepare(
				'UPDATE ingestion_daily SET accepted_batches = accepted_batches - 1 WHERE day = ? AND installation_hash = ?',
			)
			.bind(day, installationHash),
		db
			.prepare(
				'UPDATE ingestion_global_daily SET accepted_batches = accepted_batches - 1 WHERE day = ?',
			)
			.bind(day),
	])
}

function sanitizeBatch(batch: TelemetryBatch): TelemetryBatch {
	return {
		...batch,
		app: {
			...batch.app,
			version: truncateUtf8(redact(batch.app.version), 64),
			platform: truncateUtf8(redact(batch.app.platform), 32),
			arch: truncateUtf8(redact(batch.app.arch), 32),
		},
		events: batch.events,
	}
}

async function persistBatch(
	db: D1Database,
	batch: TelemetryBatch,
	installationHash: string,
): Promise<void> {
	const acceptedDay = utcDay()
	const statements: D1PreparedStatement[] = [
		db
			.prepare(
				`INSERT OR IGNORE INTO installations (
					installation_hash, first_seen_at, last_seen_at,
					first_seen_day, app_version, platform, arch
				) VALUES (?, unixepoch(), unixepoch(), ?, ?, ?, ?)`,
			)
			.bind(installationHash, acceptedDay, batch.app.version, batch.app.platform, batch.app.arch),
		db.prepare('INSERT OR IGNORE INTO platforms (platform) VALUES (?)').bind(batch.app.platform),
	]

	for (const event of batch.events) {
		statements.push(
			db
				.prepare(
					'INSERT OR IGNORE INTO daily_active (day, installation_hash, app_version, platform, arch) VALUES (?, ?, ?, ?, ?)',
				)
				.bind(event.day, installationHash, batch.app.version, batch.app.platform, batch.app.arch),
		)
	}
	await db.batch(statements)
}

async function runMaintenance(db: D1Database): Promise<void> {
	const now = new Date()
	const yesterday = daysAgo(now, 1)
	const dayMinus = (days: number) => daysAgo(now, days)

	// Usage-only maintenance. Error aggregate/R2 work was intentionally removed.
	await db.batch([
		db
			.prepare(
				`INSERT INTO daily_totals (day, new_installations, active_installations)
				VALUES (?,
					(SELECT COUNT(*) FROM installations WHERE first_seen_day = ?),
					(SELECT COUNT(*) FROM daily_active WHERE day = ?))
				ON CONFLICT (day) DO UPDATE SET
					new_installations = excluded.new_installations,
					active_installations = excluded.active_installations`,
			)
			.bind(yesterday, yesterday, yesterday),
		db
			.prepare(
				`DELETE FROM wau_seen WHERE NOT EXISTS (
					SELECT 1 FROM daily_active da
					WHERE da.installation_hash = wau_seen.installation_hash AND da.day >= ?
				)`,
			)
			.bind(dayMinus(6)),
		db
			.prepare(
				`DELETE FROM mau_seen WHERE NOT EXISTS (
					SELECT 1 FROM daily_active da
					WHERE da.installation_hash = mau_seen.installation_hash AND da.day >= ?
				)`,
			)
			.bind(dayMinus(29)),
		db.prepare("DELETE FROM daily_active WHERE day < date('now', '-35 days')"),
		db.prepare("DELETE FROM accepted_batches WHERE accepted_at < unixepoch('now', '-8 days')"),
		db.prepare("DELETE FROM ingestion_daily WHERE day < date('now', '-8 days')"),
		db.prepare("DELETE FROM ingestion_global_daily WHERE day < date('now', '-8 days')"),
		db.prepare("DELETE FROM daily_active_dims WHERE day < date('now', '-365 days')"),
	])
}

function daysAgo(now: Date, days: number): string {
	const date = new Date(now)
	date.setUTCDate(date.getUTCDate() - days)
	return date.toISOString().slice(0, 10)
}

function positiveInteger(value: string | undefined, fallback: number): number {
	const parsed = Number(value)
	return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : fallback
}

function utcDay(): string {
	return new Date().toISOString().slice(0, 10)
}

function warnIngestionThresholds(count: number, limit: number): void {
	for (const ratio of [0.8, 0.9, 0.95]) {
		if (count === Math.ceil(limit * ratio)) {
			console.warn('Telemetry ingestion budget threshold reached', {
				count,
				limit,
				percent: ratio * 100,
			})
		}
	}
}

function retryAfterSeconds(now = new Date()): number {
	const nextDay = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() + 1)
	return Math.max(1, Math.ceil((nextDay - now.getTime()) / 1_000))
}

export { app, hmacInstallationId, runMaintenance, sanitizeBatch }

export default {
	fetch: app.fetch,
	async scheduled(_controller: ScheduledController, env: Bindings, context: ExecutionContext) {
		context.waitUntil(runMaintenance(env.DB))
	},
} satisfies ExportedHandler<Bindings>
