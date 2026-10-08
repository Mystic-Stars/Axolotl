import { env } from 'cloudflare:test'
import { beforeEach, describe, expect, it } from 'vitest'

import { D1TelemetryAdminApi } from '../../server/utils/d1-admin-api'
import { parseErrorQuery } from '../../server/utils/validation'

const NOW = new Date('2026-08-14T10:00:00.000Z')
const ANALYTICS_URL = 'https://api.cloudflare.com/client/v4/graphql'
const FIXTURE_ACCOUNT_TAG = 'a7659e62e4d157aba4a45e4829b24e91'

async function gzip(input: string): Promise<ArrayBuffer> {
    return new Response(
        new Blob([input]).stream().pipeThrough(new CompressionStream('gzip')),
    ).arrayBuffer()
}

async function seed(): Promise<void> {
    await env.DB.batch([
        env.DB.prepare(
            `INSERT INTO installations
			(installation_hash, first_seen_at, last_seen_at, first_seen_day, app_version, platform, arch)
			VALUES ('fixture-hash-a', 1, 2, '2026-08-13', '9.4.0-fixture', 'windows-fixture', 'x86_64-fixture')`,
        ),
        env.DB.prepare(
            `INSERT INTO installations
			(installation_hash, first_seen_at, last_seen_at, first_seen_day, app_version, platform, arch)
			VALUES ('fixture-hash-b', 1, 2, '2026-08-14', '9.3.2-fixture', 'linux-fixture', 'aarch64-fixture')`,
        ),
        env.DB.prepare(
            `INSERT INTO daily_active (day, installation_hash, app_version, platform, arch)
			VALUES ('2026-08-14', 'fixture-hash-a', '9.4.0-fixture', 'windows-fixture', 'x86_64-fixture')`,
        ),
        env.DB.prepare(
            `INSERT INTO daily_active (day, installation_hash, app_version, platform, arch)
			VALUES ('2026-08-14', 'fixture-hash-b', '9.3.2-fixture', 'linux-fixture', 'aarch64-fixture')`,
        ),
        env.DB.prepare(
            `INSERT INTO error_reports
			(event_id, installation_hash, day, occurred_at, fingerprint, app_version, platform, arch,
			error_type, message, occurrence_count, object_key, created_at)
			VALUES ('fixture-event-a', 'fixture-hash-a', '2026-08-14', '2026-08-14T08:00:00Z',
			'fixture-render-01', '9.4.0-fixture', 'windows-fixture', 'x86_64-fixture',
			'RenderFixtureError', 'Fixture render failure', 4,
			'errors/2026-08-14/fixture-render-01/fixture-event-a.json.gz', 1)`,
        ),
        env.DB.prepare(
            `INSERT INTO error_context_reservations
			(event_id, day, fingerprint, app_version, object_key, created_at)
			VALUES ('fixture-event-a', '2026-08-14', 'fixture-render-01', '9.4.0-fixture',
			'errors/2026-08-14/fixture-render-01/fixture-event-a.json.gz', 1)`,
        ),
        env.DB.prepare(
            `INSERT INTO error_daily
			(day, fingerprint, app_version, occurrence_count, installation_count,
			latest_error_type, latest_message, has_sample)
			VALUES ('2026-08-14', 'fixture-render-01', '9.4.0-fixture', 4, 0,
			'RenderFixtureError', 'Fixture render failure', 1)`,
        ),
        env.DB.prepare(
            `INSERT INTO error_daily_installations
			(day, fingerprint, app_version, installation_hash)
			VALUES ('2026-08-14', 'fixture-render-01', '9.4.0-fixture', 'fixture-hash-a')`,
        ),
        env.DB.prepare(
            `INSERT INTO daily_totals (day, new_installations, active_installations, error_occurrences, distinct_error_groups)
			VALUES ('2026-08-13', 1, 0, 0, 0)`,
        ),
        env.DB.prepare(`INSERT INTO platforms (platform) VALUES ('windows-fixture')`),
    ])
    await env.ERROR_CONTEXTS.put(
        'errors/2026-08-14/fixture-render-01/fixture-event-a.json.gz',
        await gzip(
            JSON.stringify({
                occurred_at: '2026-08-14T08:00:00Z',
                fingerprint: 'fixture-render-01',
                app: {
                    version: '9.4.0-fixture',
                    platform: 'windows-fixture',
                    arch: 'x86_64-fixture',
                },
                error_type: 'RenderFixtureError',
                message: 'Fixture render failure',
                stack: 'Fixture stack',
                route: '/fixture',
                command: 'fixture-command',
                context: 'synthetic fixture context',
                installation_hash: 'must-not-leak',
                object_key: 'must-not-leak',
            }),
        ),
        { httpMetadata: { contentEncoding: 'gzip', contentType: 'application/json' } },
    )
}

function api(analytics?: 'configured' | 'partial' | 'unconfigured'): D1TelemetryAdminApi {
    const fetcher: typeof fetch = async (input, init) => {
        const url = String(input)
        if (url === 'https://fixture.invalid/health') {
            return new Response(JSON.stringify({ status: 'ok' }))
        }
        if (url === ANALYTICS_URL) {
            const query = JSON.parse(String(init?.body ?? '{}')).query as string
            if (analytics === 'partial' && query.includes('workersInvocationsAdaptive')) {
                return new Response(
                    JSON.stringify({
                        data: null,
                        errors: [{ message: 'fixture analytics failure' }],
                    }),
                )
            }
            if (query.includes('workersInvocationsAdaptive')) {
                return new Response(
                    JSON.stringify({
                        data: {
                            viewer: {
                                accounts: [
                                    { workersInvocationsAdaptive: [{ sum: { requests: 12_345 } }] },
                                ],
                            },
                        },
                    }),
                )
            }
            if (query.includes('d1AnalyticsAdaptiveGroups')) {
                return new Response(
                    JSON.stringify({
                        data: {
                            viewer: {
                                accounts: [
                                    {
                                        d1AnalyticsAdaptiveGroups: [
                                            { sum: { rowsRead: 45_012, rowsWritten: 3_201 } },
                                        ],
                                    },
                                ],
                            },
                        },
                    }),
                )
            }
            if (query.includes('r2OperationsAdaptiveGroups')) {
                return new Response(
                    JSON.stringify({
                        data: {
                            viewer: {
                                accounts: [
                                    { r2OperationsAdaptiveGroups: [{ sum: { requests: 8_412 } }] },
                                ],
                            },
                        },
                    }),
                )
            }
            return new Response(
                JSON.stringify({ data: null, errors: [{ message: 'unknown fixture query' }] }),
            )
        }
        return new Response('not found', { status: 404 })
    }
    return new D1TelemetryAdminApi(env.DB, env.ERROR_CONTEXTS, {
        storeErrorContext: true,
        healthUrl: 'https://fixture.invalid/health',
        fetcher,
        now: () => NOW,
        cacheTtlMs: 0,
        analytics:
            analytics === 'unconfigured'
                ? undefined
                : { accountTag: FIXTURE_ACCOUNT_TAG, apiToken: 'fixture-analytics-token' },
    })
}

describe('D1TelemetryAdminApi', () => {
    beforeEach(seed)

    it('maps D1 aggregates to stable overview and DAU/WAU/MAU DTOs', async () => {
        const overview = await api().overview('30d')
        expect(overview.metrics.totalInstallations.value).toBe(2)
        expect(overview.metrics.dau.value).toBe(2)
        expect(overview.metrics.wau.value).toBe(2)
        expect(overview.metrics.mau.value).toBe(2)
        expect(overview.metrics.errorOccurrences.value).toBe(4)
        expect(JSON.stringify(overview)).not.toContain('installation_hash')
    })

    it('supports bound search, filters, sorting, and server pagination', async () => {
        const result = await api().errors(
            parseErrorQuery({
                range: '30d',
                search: 'render',
                platform: 'windows-fixture',
                page: '1',
                pageSize: '25',
                sort: 'occurrences',
                direction: 'desc',
            }),
        )
        expect(result.total).toBe(1)
        expect(result.items[0]).toMatchObject({
            fingerprint: 'fixture-render-01',
            occurrenceCount: 4,
            hasSample: true,
        })
        expect(JSON.stringify(result)).not.toMatch(/installation_hash|object_key/)
    })

    it('reads only a D1-registered sample and never returns internal keys', async () => {
        await env.ERROR_CONTEXTS.put(
            'errors/unregistered.json.gz',
            await gzip('{"message":"unregistered"}'),
        )
        const registered = await api().errorSample('fixture-render-01')
        expect(registered).toMatchObject({
            fingerprint: 'fixture-render-01',
            message: 'Fixture render failure',
        })
        expect(JSON.stringify(registered)).not.toMatch(/installation_hash|object_key|must-not-leak/)
        expect(await api().errorSample('fixture-unregistered')).toBeNull()
    })

    it('reports the unconfigured state when analytics credentials are absent', async () => {
        const system = await api('unconfigured').system()
        expect(system.accountUsage).toMatchObject({ status: 'unavailable', label: '未配置' })
        expect(system.accountUsage.detail).toContain('尚未配置 Cloudflare Analytics API')
    })

    it('aggregates Workers, D1, and R2 usage into one account usage check', async () => {
        const system = await api('configured').system()
        expect(system.accountUsage.status).toBe('available')
        expect(system.accountUsage.detail).toContain('Workers 12,345/100,000 请求')
        expect(system.accountUsage.detail).toContain('写 3,201/100,000 行')
        expect(system.accountUsage.detail).toContain('R2 8,412/1,000,000 操作')
    })

    it('degrades when one analytics dataset fails but others respond', async () => {
        const system = await api('partial').system()
        expect(system.accountUsage).toMatchObject({ status: 'degraded', label: '部分可用' })
        expect(system.accountUsage.detail).toContain('fixture analytics failure')
        expect(system.accountUsage.detail).toContain('D1')
        expect(system.accountUsage.detail).toContain('R2')
    })
})
