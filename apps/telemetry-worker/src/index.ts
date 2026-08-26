import { Hono } from 'hono'

import { byteLength, redact, truncateUtf8 } from './redact'
import { batchSchema, type ErrorEvent, type TelemetryBatch } from './schema'

const MAX_REQUEST_BYTES = 64 * 1024
const MAX_CONTEXT_OBJECT_BYTES = 16 * 1024
const HARD_MAX_CONTEXTS_PER_DAY = 2_000
const HARD_MAX_SAMPLES_PER_GROUP = 3
const DETAIL_SAMPLES_PER_GROUP_PER_DAY = 2
const HARD_MAX_BATCHES_PER_INSTALLATION_PER_DAY = 25
const HARD_MAX_ACCEPTED_BATCHES_PER_DAY = 100_000

export interface Bindings {
	DB: D1Database
	ERROR_CONTEXTS: R2Bucket
	INSTALLATION_HMAC_SECRET: string
	STORE_ERROR_CONTEXT?: string
	MAX_ERROR_CONTEXTS_PER_DAY?: string
	MAX_ERROR_SAMPLES_PER_GROUP?: string
	INGEST_ENABLED?: string
	DROP_DOWNLOAD_STALL?: string
	MAX_BATCHES_PER_INSTALLATION_PER_DAY?: string
	MAX_ACCEPTED_BATCHES_PER_DAY?: string
}

type Variables = {
	requestId: string
}

interface ErrorGroupKey {
	day: string
	fingerprint: string
	appVersion: string
	occurrenceCount: number
	latestErrorType: string
	latestMessage: string
	hasSample: boolean
}

type IngestionReservation = 'reserved' | 'duplicate' | 'installation_limit' | 'global_limit'

const app = new Hono<{ Bindings: Bindings; Variables: Variables }>()

app.use('*', async (context, next) => {
	context.set('requestId', crypto.randomUUID())
	await next()
	context.header('Cache-Control', 'no-store')
})

app.get('/health', (context) =>
	context.json({ status: 'ok', schema_version: 1 }, 200, {
		'Cache-Control': 'no-store',
	}),
)

app.post('/v1/batch', async (context) => {
	const contentLength = Number(context.req.header('content-length') ?? '0')
	if (Number.isFinite(contentLength) && contentLength > MAX_REQUEST_BYTES) {
		return context.json({ error: 'request_too_large' }, 413)
	}

	const raw = await context.req.arrayBuffer()
	if (raw.byteLength > MAX_REQUEST_BYTES) {
		return context.json({ error: 'request_too_large' }, 413)
	}

	let input: unknown
	try {
		input = JSON.parse(new TextDecoder().decode(raw))
	} catch {
		return context.json({ error: 'invalid_json' }, 400)
	}

	const parsed = batchSchema.safeParse(input)
	if (!parsed.success) {
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
		const filtered = filterDroppedEvents(parsed.data, context.env.DROP_DOWNLOAD_STALL === 'true')
		if (filtered.events.length === 0) {
			return context.json({ accepted: true, duplicate: false, dropped: true })
		}

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
			console.error('Telemetry ingestion budget exhausted', {
				limit: limits.global,
			})
			return context.json({ error: 'global_batch_limit' }, 429, {
				'Retry-After': retryAfterSeconds().toString(),
			})
		}

		try {
			const sanitized = sanitizeBatch(filtered)
			const objectKeys = await storeErrorContexts(context.env, sanitized)
			await persistBatch(context.env.DB, sanitized, installationHash, objectKeys)
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

function filterDroppedEvents(batch: TelemetryBatch, dropDownloadStall: boolean): TelemetryBatch {
	if (!dropDownloadStall) return batch
	return {
		...batch,
		events: batch.events.filter(
			(event) => event.type !== 'error' || event.error_type !== 'download_stall',
		),
	}
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
		installation_id: batch.installation_id,
		app: {
			...batch.app,
			version: truncateUtf8(redact(batch.app.version), 64),
			platform: truncateUtf8(redact(batch.app.platform), 32),
			arch: truncateUtf8(redact(batch.app.arch), 32),
		},
		events: batch.events.map((event) => {
			if (event.type === 'heartbeat') return event
			return {
				...event,
				error_type: truncateUtf8(redact(event.error_type), 128),
				message: truncateUtf8(redact(event.message), 1_024),
				stack: event.stack ? truncateUtf8(redact(event.stack), 8_192) : event.stack,
				route: event.route ? truncateUtf8(redact(event.route), 256) : event.route,
				command: event.command ? truncateUtf8(redact(event.command), 256) : event.command,
				context: event.context
					? truncateUtf8(redact(event.context), MAX_CONTEXT_OBJECT_BYTES)
					: event.context,
			}
		}),
	}
}

async function storeErrorContexts(
	env: Bindings,
	batch: TelemetryBatch,
): Promise<Map<string, string>> {
	const objectKeys = new Map<string, string>()
	if (env.STORE_ERROR_CONTEXT !== 'true') return objectKeys

	const dailyLimit = Math.min(
		positiveInteger(env.MAX_ERROR_CONTEXTS_PER_DAY, HARD_MAX_CONTEXTS_PER_DAY),
		HARD_MAX_CONTEXTS_PER_DAY,
	)
	const sampleLimit = Math.min(
		positiveInteger(env.MAX_ERROR_SAMPLES_PER_GROUP, HARD_MAX_SAMPLES_PER_GROUP),
		HARD_MAX_SAMPLES_PER_GROUP,
	)
	const day = utcDay()

	for (const event of batch.events) {
		if (event.type !== 'error' || !event.context) continue
		const objectDay = eventDay(event)
		const objectKey = `errors/${objectDay}/${event.fingerprint}/${event.event_id}.json.gz`
		await env.DB.prepare(
			`INSERT OR IGNORE INTO error_context_reservations (
				event_id, day, fingerprint, app_version, object_key, created_at
			)
			SELECT ?, ?, ?, ?, ?, unixepoch()
			WHERE COALESCE((
				SELECT object_count FROM error_context_budget WHERE day = ?
			), 0) < ?
			AND COALESCE((
				SELECT sample_count FROM error_context_samples
				WHERE day = ? AND fingerprint = ? AND app_version = ?
			), 0) < ?`,
		)
			.bind(
				event.event_id,
				day,
				event.fingerprint,
				batch.app.version,
				objectKey,
				day,
				dailyLimit,
				day,
				event.fingerprint,
				batch.app.version,
				sampleLimit,
			)
			.run()

		const reservation = await env.DB.prepare(
			`SELECT r.object_key, b.object_count
			FROM error_context_reservations r
			JOIN error_context_budget b ON b.day = r.day
			WHERE r.event_id = ?`,
		)
			.bind(event.event_id)
			.first<{ object_key: string; object_count: number }>()
		if (!reservation) continue

		const serialized = buildContextObject(batch, event)
		const compressed = await gzip(serialized)
		await env.ERROR_CONTEXTS.put(reservation.object_key, compressed, {
			httpMetadata: { contentType: 'application/json', contentEncoding: 'gzip' },
			customMetadata: { schema_version: '1', day: objectDay },
		})
		objectKeys.set(event.event_id, reservation.object_key)
		warnAtThresholds(reservation.object_count, dailyLimit)
	}

	return objectKeys
}

function buildContextObject(batch: TelemetryBatch, event: ErrorEvent): string {
	const value = {
		schema_version: 1,
		event_id: event.event_id,
		occurred_at: event.occurred_at,
		fingerprint: event.fingerprint,
		app: batch.app,
		error_type: event.error_type,
		message: event.message,
		stack: event.stack,
		route: event.route,
		command: event.command,
		context: event.context,
	}
	let serialized = JSON.stringify(value)
	if (byteLength(serialized) <= MAX_CONTEXT_OBJECT_BYTES) return serialized

	const shrink = (field: 'context' | 'stack' | 'message' | 'route' | 'command' | 'error_type') => {
		const current = value[field] ?? ''
		const overflow = byteLength(serialized) - MAX_CONTEXT_OBJECT_BYTES
		value[field] = truncateUtf8(current, Math.max(0, byteLength(current) - overflow))
		serialized = JSON.stringify(value)
	}

	for (const field of ['context', 'stack', 'message', 'route', 'command', 'error_type'] as const) {
		if (byteLength(serialized) <= MAX_CONTEXT_OBJECT_BYTES) break
		shrink(field)
	}
	if (byteLength(serialized) > MAX_CONTEXT_OBJECT_BYTES) {
		return JSON.stringify({
			schema_version: 1,
			event_id: event.event_id,
			fingerprint: event.fingerprint,
		})
	}
	return serialized
}

async function gzip(input: string): Promise<ArrayBuffer> {
	const stream = new Blob([input]).stream().pipeThrough(new CompressionStream('gzip'))
	return await new Response(stream).arrayBuffer()
}

async function persistBatch(
	db: D1Database,
	batch: TelemetryBatch,
	installationHash: string,
	objectKeys: Map<string, string>,
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

	const groups = new Map<string, ErrorGroupKey>()
	const detailInserts: D1PreparedStatement[] = []

	for (const event of batch.events) {
		const day = eventDay(event)
		if (event.type === 'heartbeat') {
			statements.push(
				db
					.prepare(
						'INSERT OR IGNORE INTO daily_active (day, installation_hash, app_version, platform, arch) VALUES (?, ?, ?, ?, ?)',
					)
					.bind(day, installationHash, batch.app.version, batch.app.platform, batch.app.arch),
			)
			continue
		}

		const key = `${day}\u0000${event.fingerprint}\u0000${batch.app.version}`
		const group = groups.get(key) ?? {
			day,
			fingerprint: event.fingerprint,
			appVersion: batch.app.version,
			occurrenceCount: 0,
			latestErrorType: 'Unknown',
			latestMessage: '',
			hasSample: false,
		}
		group.occurrenceCount += event.occurrence_count
		group.latestErrorType = event.error_type
		group.latestMessage = event.message
		if (objectKeys.has(event.event_id)) group.hasSample = true
		groups.set(key, group)

		detailInserts.push(
			db
				.prepare(
					`INSERT INTO error_reports (
						event_id, installation_hash, day, occurred_at, fingerprint, app_version,
						platform, arch, error_type, message, occurrence_count, object_key, created_at
					)
					SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, unixepoch()
					WHERE (SELECT COUNT(*) FROM error_reports
						WHERE fingerprint = ? AND app_version = ? AND day = ?) < ?`,
				)
				.bind(
					event.event_id,
					installationHash,
					day,
					event.occurred_at,
					event.fingerprint,
					batch.app.version,
					batch.app.platform,
					batch.app.arch,
					event.error_type,
					event.message,
					event.occurrence_count,
					objectKeys.get(event.event_id) ?? null,
					event.fingerprint,
					batch.app.version,
					day,
					DETAIL_SAMPLES_PER_GROUP_PER_DAY,
				),
		)
	}

	for (const group of groups.values()) {
		statements.push(
			db
				.prepare(
					`INSERT OR IGNORE INTO error_daily_installations (
						day, fingerprint, app_version, installation_hash
					) VALUES (?, ?, ?, ?)`,
				)
				.bind(group.day, group.fingerprint, group.appVersion, installationHash),
			db
				.prepare(
					`INSERT INTO error_daily (
						day, fingerprint, app_version, occurrence_count, installation_count,
						latest_error_type, latest_message, has_sample
					) VALUES (?, ?, ?, ?, (SELECT changes()), ?, ?, ?)
					ON CONFLICT (day, fingerprint, app_version) DO UPDATE SET
						occurrence_count = occurrence_count + excluded.occurrence_count,
						installation_count = installation_count + (SELECT changes()),
						latest_error_type = excluded.latest_error_type,
						latest_message = excluded.latest_message,
						has_sample = MAX(error_daily.has_sample, excluded.has_sample)`,
				)
				.bind(
					group.day,
					group.fingerprint,
					group.appVersion,
					group.occurrenceCount,
					group.latestErrorType,
					group.latestMessage,
					group.hasSample ? 1 : 0,
				),
		)
	}

	statements.push(...detailInserts)
	await db.batch(statements)
}

function rangeStatStatements(
	db: D1Database,
	rangeDays: number,
	start: string,
	end: string,
): D1PreparedStatement[] {
	return [
		db
			.prepare(
				`INSERT INTO error_range_stats (
					range_days, fingerprint, app_version, first_seen, last_seen,
					occurrence_count, installation_count, latest_error_type, latest_message,
					has_sample
				)
				SELECT
					?, ed.fingerprint, ed.app_version, MIN(ed.day), MAX(ed.day),
					SUM(ed.occurrence_count), SUM(ed.installation_count),
					COALESCE((SELECT eg.latest_error_type FROM error_groups eg
						WHERE eg.fingerprint = ed.fingerprint AND eg.app_version = ed.app_version), 'Unknown'),
					COALESCE((SELECT eg.latest_message FROM error_groups eg
						WHERE eg.fingerprint = ed.fingerprint AND eg.app_version = ed.app_version), ''),
					CASE WHEN EXISTS (SELECT 1 FROM error_groups eg
						WHERE eg.fingerprint = ed.fingerprint AND eg.app_version = ed.app_version
							AND eg.sample_object_key IS NOT NULL) THEN 1 ELSE 0 END
				FROM error_daily ed
				WHERE ed.day >= ? AND ed.day <= ?
				GROUP BY ed.fingerprint, ed.app_version`,
			)
			.bind(rangeDays, start, end),
	]
}

async function runMaintenance(db: D1Database): Promise<void> {
	const now = new Date()
	const yesterday = daysAgo(now, 1)
	const dayMinus = (days: number) => daysAgo(now, days)

	await db.batch([
		db
			.prepare(
				`UPDATE error_daily SET
					installation_count = (
						SELECT COUNT(*) FROM error_daily_installations edi
						WHERE edi.day = error_daily.day
							AND edi.fingerprint = error_daily.fingerprint
							AND edi.app_version = error_daily.app_version
					),
					latest_error_type = COALESCE((
						SELECT er.error_type FROM error_reports er
						WHERE er.fingerprint = error_daily.fingerprint
							AND er.app_version = error_daily.app_version
						ORDER BY er.occurred_at DESC, er.event_id DESC LIMIT 1
					), error_daily.latest_error_type),
					latest_message = COALESCE((
						SELECT er.message FROM error_reports er
						WHERE er.fingerprint = error_daily.fingerprint
							AND er.app_version = error_daily.app_version
						ORDER BY er.occurred_at DESC, er.event_id DESC LIMIT 1
					), error_daily.latest_message),
					has_sample = CASE WHEN EXISTS (
						SELECT 1 FROM error_context_reservations r
						WHERE r.fingerprint = error_daily.fingerprint
							AND r.app_version = error_daily.app_version
					) THEN 1 ELSE error_daily.has_sample END
				WHERE day = ?`,
			)
			.bind(yesterday),
		db
			.prepare(
				`INSERT OR IGNORE INTO error_group_installations (
					fingerprint, app_version, installation_hash, first_seen_day
				)
				SELECT fingerprint, app_version, installation_hash, ?
				FROM error_daily_installations WHERE day = ?`,
			)
			.bind(yesterday, yesterday),
		db
			.prepare(
				`UPDATE error_groups SET
					last_seen_day = ?,
					occurrence_count = occurrence_count + (
						SELECT SUM(occurrence_count) FROM error_daily ed
						WHERE ed.day = ? AND ed.fingerprint = error_groups.fingerprint
							AND ed.app_version = error_groups.app_version
					),
					installation_count = (
						SELECT COUNT(*) FROM error_group_installations gi
						WHERE gi.fingerprint = error_groups.fingerprint
							AND gi.app_version = error_groups.app_version
					),
					latest_error_type = (
						SELECT er.error_type FROM error_reports er
						WHERE er.fingerprint = error_groups.fingerprint
							AND er.app_version = error_groups.app_version
							AND er.day = ?
						ORDER BY er.occurred_at DESC, er.event_id DESC LIMIT 1
					),
					latest_message = (
						SELECT er.message FROM error_reports er
						WHERE er.fingerprint = error_groups.fingerprint
							AND er.app_version = error_groups.app_version
							AND er.day = ?
						ORDER BY er.occurred_at DESC, er.event_id DESC LIMIT 1
					),
					sample_object_key = COALESCE(error_groups.sample_object_key, (
						SELECT r.object_key FROM error_context_reservations r
						WHERE r.fingerprint = error_groups.fingerprint
							AND r.app_version = error_groups.app_version
						ORDER BY r.created_at ASC LIMIT 1
					))
				WHERE EXISTS (
					SELECT 1 FROM error_daily ed
					WHERE ed.day = ? AND ed.fingerprint = error_groups.fingerprint
						AND ed.app_version = error_groups.app_version
				)`,
			)
			.bind(yesterday, yesterday, yesterday, yesterday, yesterday),
		db
			.prepare(
				`INSERT OR IGNORE INTO error_groups (
					fingerprint, app_version, first_seen_day, last_seen_day, occurrence_count,
					installation_count, latest_error_type, latest_message, sample_object_key
				)
				SELECT
					ed.fingerprint, ed.app_version, ?, ?, ed.occurrence_count,
					(SELECT COUNT(*) FROM error_group_installations gi
						WHERE gi.fingerprint = ed.fingerprint AND gi.app_version = ed.app_version),
					COALESCE((SELECT er.error_type FROM error_reports er
						WHERE er.fingerprint = ed.fingerprint AND er.app_version = ed.app_version
						ORDER BY er.occurred_at DESC, er.event_id DESC LIMIT 1), 'Unknown'),
					COALESCE((SELECT er.message FROM error_reports er
						WHERE er.fingerprint = ed.fingerprint AND er.app_version = ed.app_version
						ORDER BY er.occurred_at DESC, er.event_id DESC LIMIT 1), ''),
					(SELECT r.object_key FROM error_context_reservations r
						WHERE r.fingerprint = ed.fingerprint AND r.app_version = ed.app_version
						ORDER BY r.created_at ASC LIMIT 1)
				FROM error_daily ed WHERE ed.day = ?`,
			)
			.bind(yesterday, yesterday, yesterday),
		db
			.prepare(
				`INSERT INTO daily_totals (
					day, new_installations, active_installations, error_occurrences, distinct_error_groups
				) VALUES (
					?,
					(SELECT COUNT(*) FROM installations WHERE first_seen_day = ?),
					(SELECT COUNT(*) FROM daily_active WHERE day = ?),
					(SELECT COALESCE(SUM(occurrence_count), 0) FROM error_daily WHERE day = ?),
					(SELECT COUNT(*) FROM error_daily WHERE day = ?)
				)
				ON CONFLICT (day) DO UPDATE SET
					new_installations = excluded.new_installations,
					active_installations = excluded.active_installations,
					error_occurrences = excluded.error_occurrences,
					distinct_error_groups = excluded.distinct_error_groups`,
			)
			.bind(yesterday, yesterday, yesterday, yesterday, yesterday),
		db.prepare('DELETE FROM error_range_stats'),
		...rangeStatStatements(db, 7, dayMinus(7), yesterday),
		...rangeStatStatements(db, 30, dayMinus(30), yesterday),
		...rangeStatStatements(db, 90, dayMinus(90), yesterday),
		db
			.prepare(
				`DELETE FROM wau_seen WHERE NOT EXISTS (
					SELECT 1 FROM daily_active da
					WHERE da.installation_hash = wau_seen.installation_hash
						AND da.day >= ?
				)`,
			)
			.bind(dayMinus(6)),
		db
			.prepare(
				`DELETE FROM mau_seen WHERE NOT EXISTS (
					SELECT 1 FROM daily_active da
					WHERE da.installation_hash = mau_seen.installation_hash
						AND da.day >= ?
				)`,
			)
			.bind(dayMinus(29)),
		db
			.prepare(
				`DELETE FROM platforms WHERE NOT EXISTS (
					SELECT 1 FROM error_reports er
					WHERE er.platform = platforms.platform AND er.day >= ?
				)`,
			)
			.bind(dayMinus(29)),
		db.prepare(
			`DELETE FROM error_group_installations WHERE NOT EXISTS (
				SELECT 1 FROM error_groups eg
				WHERE eg.fingerprint = error_group_installations.fingerprint
					AND eg.app_version = error_group_installations.app_version
			)`,
		),
		db.prepare("DELETE FROM daily_active WHERE day < date('now', '-35 days')"),
		db.prepare("DELETE FROM error_reports WHERE day < date('now', '-30 days')"),
		db.prepare("DELETE FROM error_context_reservations WHERE day < date('now', '-30 days')"),
		db.prepare("DELETE FROM error_context_samples WHERE day < date('now', '-30 days')"),
		db.prepare("DELETE FROM error_context_budget WHERE day < date('now', '-30 days')"),
		db.prepare("DELETE FROM error_daily WHERE day < date('now', '-365 days')"),
		db.prepare("DELETE FROM error_groups WHERE last_seen_day < date('now', '-365 days')"),
		db.prepare("DELETE FROM accepted_batches WHERE accepted_at < unixepoch('now', '-8 days')"),
		db.prepare("DELETE FROM ingestion_daily WHERE day < date('now', '-8 days')"),
		db.prepare("DELETE FROM ingestion_global_daily WHERE day < date('now', '-8 days')"),
		db.prepare("DELETE FROM error_daily_installations WHERE day < date('now', '-1 days')"),
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

function eventDay(event: TelemetryBatch['events'][number]): string {
	if (event.type === 'heartbeat') return event.day
	return new Date(event.occurred_at).toISOString().slice(0, 10)
}

function warnAtThresholds(count: number, limit: number): void {
	const thresholds = [0.5, 0.75, 0.9].map((ratio) => Math.ceil(limit * ratio))
	if (thresholds.includes(count)) {
		console.warn('R2 telemetry context budget threshold reached', { count, limit })
	}
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

export { app, filterDroppedEvents, hmacInstallationId, runMaintenance, sanitizeBatch }

export default {
	fetch: app.fetch,
	async scheduled(_controller: ScheduledController, env: Bindings, context: ExecutionContext) {
		context.waitUntil(runMaintenance(env.DB))
	},
} satisfies ExportedHandler<Bindings>
