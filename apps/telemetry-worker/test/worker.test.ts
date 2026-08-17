import { env, SELF } from 'cloudflare:test'
import { describe, expect, it } from 'vitest'

import { runMaintenance, type Bindings } from '../src'
import { batchSchema } from '../src/schema'

declare module 'cloudflare:test' {
	interface ProvidedEnv extends Bindings {}
}

const installationId = '018f6ee8-4cb1-7db3-8a8d-8df96f122d85'

function batch(
	batchId: string,
	events: Array<Record<string, unknown>>,
	clientInstallationId = installationId,
): Record<string, unknown> {
	return {
		schema_version: 1,
		batch_id: batchId,
		installation_id: clientInstallationId,
		app: {
			version: '1.7.1',
			environment: 'production',
			platform: 'windows',
			arch: 'x86_64',
		},
		events,
	}
}

async function post(payload: unknown): Promise<Response> {
	return await SELF.fetch('https://telemetry.example/v1/batch', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(payload),
	})
}

describe('telemetry worker', () => {
	it('serves health without storage access', async () => {
		const response = await SELF.fetch('https://telemetry.example/health')
		expect(response.status).toBe(200)
		expect(await response.json()).toEqual({ status: 'ok', schema_version: 1 })
	})

	it('rejects unknown structures and oversized requests', async () => {
		const invalid = batch('11111111-1111-4111-8111-111111111111', [
			{
				type: 'heartbeat',
				event_id: '21111111-1111-4111-8111-111111111111',
				occurred_at: new Date().toISOString(),
				day: '2026-08-14',
				unknown: true,
			},
		])
		expect(batchSchema.safeParse(invalid).success).toBe(false)
		expect((await post(invalid)).status).toBe(400)

		const oversized = await SELF.fetch('https://telemetry.example/v1/batch', {
			method: 'POST',
			body: 'x'.repeat(65 * 1024),
		})
		expect(oversized.status).toBe(413)
	})

	it('drops legacy download_stall batches before quota and storage', async () => {
		const payload = batch('12111111-1111-4111-8111-111111111111', [
			{
				type: 'error',
				event_id: '13111111-1111-4111-8111-111111111111',
				occurred_at: new Date().toISOString(),
				fingerprint: 'a'.repeat(64),
				occurrence_count: 1,
				error_type: 'download_stall',
				message: 'no progress',
			},
		])

		const response = await post(payload)
		expect(response.status).toBe(200)
		expect(await response.json()).toEqual({ accepted: true, duplicate: false, dropped: true })

		const stored = await env.DB.prepare(
			'SELECT COUNT(*) AS count FROM accepted_batches WHERE batch_id = ?',
		)
			.bind('12111111-1111-4111-8111-111111111111')
			.first<{ count: number }>()
		expect(stored?.count).toBe(0)
		expect(
			await env.DB.prepare('SELECT COUNT(*) AS count FROM error_reports').first<{
				count: number
			}>(),
		).toEqual({ count: 0 })
	})

	it('filters download_stall events from mixed legacy batches', async () => {
		const fingerprint = 'b'.repeat(64)
		const response = await post(
			batch('14111111-1111-4111-8111-111111111111', [
				{
					type: 'error',
					event_id: '15111111-1111-4111-8111-111111111111',
					occurred_at: new Date().toISOString(),
					fingerprint: 'c'.repeat(64),
					occurrence_count: 1,
					error_type: 'download_stall',
					message: 'no progress',
				},
				{
					type: 'error',
					event_id: '16111111-1111-4111-8111-111111111111',
					occurred_at: new Date().toISOString(),
					fingerprint,
					occurrence_count: 1,
					error_type: 'download_error',
					message: 'connection reset',
				},
			]),
		)
		expect(response.status).toBe(200)

		const reports = await env.DB.prepare(
			'SELECT error_type FROM error_reports WHERE fingerprint = ?',
		)
			.bind(fingerprint)
			.all<{ error_type: string }>()
		expect(reports.results).toEqual([{ error_type: 'download_error' }])
		expect(
			await env.DB.prepare(
				"SELECT COUNT(*) AS count FROM error_reports WHERE error_type = 'download_stall'",
			).first<{ count: number }>(),
		).toEqual({ count: 0 })
	})

	it('enforces the daily installation batch cap without charging duplicates', async () => {
		const cappedInstallation = '018f6ee8-4cb1-7db3-8a8d-8df96f122d99'
		const day = new Date().toISOString().slice(0, 10)
		const payloadFor = (index: number) =>
			batch(
				`60000000-0000-4000-8000-${index.toString().padStart(12, '0')}`,
				[
					{
						type: 'heartbeat',
						event_id: `70000000-0000-4000-8000-${index.toString().padStart(12, '0')}`,
						occurred_at: new Date().toISOString(),
						day,
					},
				],
				cappedInstallation,
			)

		for (let index = 0; index < 25; index++) {
			expect((await post(payloadFor(index))).status).toBe(200)
		}
		expect((await post(payloadFor(0))).status).toBe(200)

		const rejected = await post(payloadFor(25))
		expect(rejected.status).toBe(429)
		expect(rejected.headers.get('Retry-After')).not.toBeNull()

		const used = await env.DB.prepare(
			'SELECT accepted_batches FROM ingestion_daily WHERE day = ? ORDER BY accepted_batches DESC LIMIT 1',
		)
			.bind(day)
			.first<{ accepted_batches: number }>()
		expect(used?.accepted_batches).toBe(25)
	})

	it('opens the global ingestion circuit breaker at 100,000 accepted batches', async () => {
		const day = new Date().toISOString().slice(0, 10)
		await env.DB.prepare(
			`INSERT INTO ingestion_global_daily (day, accepted_batches)
			VALUES (?, 99999)
			ON CONFLICT (day) DO UPDATE SET accepted_batches = 99999`,
		)
			.bind(day)
			.run()

		const first = batch(
			'80000000-0000-4000-8000-000000000001',
			[
				{
					type: 'heartbeat',
					event_id: '81000000-0000-4000-8000-000000000001',
					occurred_at: new Date().toISOString(),
					day,
				},
			],
			'018f6ee8-4cb1-7db3-8a8d-8df96f122d98',
		)
		expect((await post(first)).status).toBe(200)

		const rejected = await post({
			...first,
			batch_id: '80000000-0000-4000-8000-000000000002',
			events: [
				{
					...(first.events as Array<Record<string, unknown>>)[0],
					event_id: '81000000-0000-4000-8000-000000000002',
				},
			],
		})
		expect(rejected.status).toBe(429)
		expect(rejected.headers.get('Retry-After')).not.toBeNull()
	})

	it('hashes installations and keeps heartbeat batches idempotent', async () => {
		const payload = batch('31111111-1111-4111-8111-111111111111', [
			{
				type: 'heartbeat',
				event_id: '41111111-1111-4111-8111-111111111111',
				occurred_at: new Date().toISOString(),
				day: '2026-08-14',
			},
		])
		expect((await post(payload)).status).toBe(200)
		expect((await post(payload)).status).toBe(200)

		const installation = await env.DB.prepare('SELECT installation_hash FROM installations').first<{
			installation_hash: string
		}>()
		expect(installation?.installation_hash).toMatch(/^[0-9a-f]{64}$/)
		expect(installation?.installation_hash).not.toBe(installationId)

		const active = await env.DB.prepare('SELECT COUNT(*) AS count FROM daily_active').first<{
			count: number
		}>()
		expect(active?.count).toBe(1)
	})

	it('keeps offline heartbeat dates for DAU, WAU, and MAU queries', async () => {
		const day = (daysAgo: number) =>
			new Date(Date.now() - daysAgo * 24 * 60 * 60 * 1_000).toISOString().slice(0, 10)
		const samples = [
			{
				batchId: '32111111-1111-4111-8111-111111111111',
				eventId: '42111111-1111-4111-8111-111111111111',
				installationId: '018f6ee8-4cb1-7db3-8a8d-8df96f122d81',
				daysAgo: 0,
			},
			{
				batchId: '33111111-1111-4111-8111-111111111111',
				eventId: '43111111-1111-4111-8111-111111111111',
				installationId: '018f6ee8-4cb1-7db3-8a8d-8df96f122d82',
				daysAgo: 6,
			},
			{
				batchId: '34111111-1111-4111-8111-111111111111',
				eventId: '44111111-1111-4111-8111-111111111111',
				installationId: '018f6ee8-4cb1-7db3-8a8d-8df96f122d83',
				daysAgo: 29,
			},
		]

		for (const sample of samples) {
			const heartbeatDay = day(sample.daysAgo)
			const response = await post(
				batch(
					sample.batchId,
					[
						{
							type: 'heartbeat',
							event_id: sample.eventId,
							occurred_at: `${heartbeatDay}T12:00:00.000Z`,
							day: heartbeatDay,
						},
					],
					sample.installationId,
				),
			)
			expect(response.status).toBe(200)
		}

		const counts = await env.DB.prepare(
			`SELECT
				COUNT(DISTINCT CASE WHEN day = ? THEN installation_hash END) AS dau,
				COUNT(DISTINCT CASE WHEN day >= date(?, '-6 days') THEN installation_hash END) AS wau,
				COUNT(DISTINCT CASE WHEN day >= date(?, '-29 days') THEN installation_hash END) AS mau
			FROM daily_active`,
		)
			.bind(day(0), day(0), day(0))
			.first<{ dau: number; wau: number; mau: number }>()
		expect(counts).toEqual({ dau: 1, wau: 2, mau: 3 })

		const seen = await env.DB.prepare(
			'SELECT (SELECT COUNT(*) FROM wau_seen) AS wau, (SELECT COUNT(*) FROM mau_seen) AS mau',
		).first<{ wau: number; mau: number }>()
		expect(seen).toEqual({ wau: 3, mau: 3 })
	})

	it('caps persisted error detail rows per group per day without losing counts', async () => {
		const fingerprint = 'c'.repeat(64)
		const day = new Date().toISOString().slice(0, 10)
		for (let index = 0; index < 4; index++) {
			const response = await post(
				batch(`91111111-1111-4111-8111-11111111111${index}`, [
					{
						type: 'error',
						event_id: `a1111111-1111-4111-8111-11111111111${index}`,
						occurred_at: new Date().toISOString(),
						fingerprint,
						occurrence_count: 1,
						error_type: 'sample_error',
						message: 'sampled',
					},
				]),
			)
			expect(response.status).toBe(200)
		}

		const reports = await env.DB.prepare('SELECT COUNT(*) AS count FROM error_reports').first<{
			count: number
		}>()
		expect(reports?.count).toBe(2)

		const daily = await env.DB.prepare(
			'SELECT occurrence_count FROM error_daily WHERE fingerprint = ? AND day = ?',
		)
			.bind(fingerprint, day)
			.first<{ occurrence_count: number }>()
		expect(daily?.occurrence_count).toBe(4)

		const seen = await env.DB.prepare(
			'SELECT COUNT(*) AS count FROM error_daily_installations WHERE fingerprint = ?',
		)
			.bind(fingerprint)
			.first<{ count: number }>()
		expect(seen?.count).toBe(1)
	})

	it('rolls up daily aggregates through runMaintenance', async () => {
		const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1_000).toISOString().slice(0, 10)
		const fingerprint = 'd'.repeat(64)
		const response = await post(
			batch('a2111111-1111-4111-8111-111111111111', [
				{
					type: 'error',
					event_id: 'a3111111-1111-4111-8111-111111111111',
					occurred_at: `${yesterday}T12:00:00.000Z`,
					fingerprint,
					occurrence_count: 3,
					error_type: 'maintenance_error',
					message: 'maintenance',
				},
			]),
		)
		expect(response.status).toBe(200)

		await runMaintenance(env.DB)

		const totals = await env.DB.prepare('SELECT error_occurrences FROM daily_totals WHERE day = ?')
			.bind(yesterday)
			.first<{ error_occurrences: number }>()
		expect(totals?.error_occurrences).toBe(3)

		const group = await env.DB.prepare(
			'SELECT occurrence_count, installation_count FROM error_groups WHERE fingerprint = ?',
		)
			.bind(fingerprint)
			.first<{ occurrence_count: number; installation_count: number }>()
		expect(group).toEqual({ occurrence_count: 3, installation_count: 1 })

		const stats = await env.DB.prepare(
			'SELECT occurrence_count FROM error_range_stats WHERE range_days = 7 AND fingerprint = ?',
		)
			.bind(fingerprint)
			.first<{ occurrence_count: number }>()
		expect(stats?.occurrence_count).toBe(3)
	})

	it('redacts error context and enforces the daily R2 reservation cap', async () => {
		const firstEvent = {
			type: 'error',
			event_id: '51111111-1111-4111-8111-111111111111',
			occurred_at: new Date().toISOString(),
			fingerprint: 'a'.repeat(64),
			occurrence_count: 2,
			error_type: 'window_error',
			message: 'Failed for user@example.com',
			stack: 'C:\\Users\\Alice\\launcher.js:1',
			route: '/settings?token=secret',
			command: null,
			context: `Authorization: Bearer abc.def ${'x'.repeat(16_000)}`,
		}
		const secondEvent = {
			...firstEvent,
			event_id: '61111111-1111-4111-8111-111111111111',
			fingerprint: 'b'.repeat(64),
		}

		expect((await post(batch('71111111-1111-4111-8111-111111111111', [firstEvent]))).status).toBe(
			200,
		)
		expect((await post(batch('81111111-1111-4111-8111-111111111111', [secondEvent]))).status).toBe(
			200,
		)

		const reservations = await env.DB.prepare(
			'SELECT COUNT(*) AS count FROM error_context_reservations',
		).first<{ count: number }>()
		expect(reservations?.count).toBe(1)

		const reservation = await env.DB.prepare(
			'SELECT object_key FROM error_context_reservations LIMIT 1',
		).first<{ object_key: string }>()
		const object = await env.ERROR_CONTEXTS.get(reservation!.object_key)
		expect(object).not.toBeNull()
		const decompressed = object!.body.pipeThrough(new DecompressionStream('gzip'))
		const text = await new Response(decompressed).text()
		expect(new TextEncoder().encode(text).byteLength).toBeLessThanOrEqual(16 * 1024)
		expect(text).not.toContain('user@example.com')
		expect(text).not.toContain('abc.def')
		expect(text).not.toContain('C:\\Users\\Alice')
	})
})
