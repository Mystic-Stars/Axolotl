import { applyD1Migrations, type D1Migration, env } from 'cloudflare:test'
import { beforeAll, beforeEach } from 'vitest'

declare module 'cloudflare:test' {
    interface ProvidedEnv {
        DB: D1Database
        ERROR_CONTEXTS: R2Bucket
        TEST_MIGRATIONS: D1Migration[]
    }
}

const tables = [
    'error_context_reservations',
    'error_context_samples',
    'error_context_budget',
    'error_reports',
    'error_daily',
    'error_groups',
    'daily_active',
    'daily_totals',
    'accepted_batches',
    'installations',
]

beforeAll(async () => {
    await applyD1Migrations(env.DB, env.TEST_MIGRATIONS)
})

beforeEach(async () => {
    await env.DB.batch(tables.map((table) => env.DB.prepare(`DELETE FROM ${table}`)))
})
