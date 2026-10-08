import { defineWorkersConfig, readD1Migrations } from '@cloudflare/vitest-pool-workers/config'

const migrations = await readD1Migrations('../telemetry-worker/migrations')

export default defineWorkersConfig({
    test: {
        include: ['test/server/**/*.test.ts'],
        setupFiles: ['./test/server/setup.ts'],
        poolOptions: {
            workers: {
                main: './test/server/worker.ts',
                singleWorker: true,
                miniflare: {
                    compatibilityDate: '2025-08-13',
                    d1Databases: ['DB'],
                    r2Buckets: ['ERROR_CONTEXTS'],
                    bindings: { TEST_MIGRATIONS: migrations },
                },
            },
        },
    },
})
