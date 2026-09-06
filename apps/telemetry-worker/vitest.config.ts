import { defineWorkersConfig, readD1Migrations } from '@cloudflare/vitest-pool-workers/config'

const migrations = await readD1Migrations('./migrations')

export default defineWorkersConfig({
	test: {
		setupFiles: ['./test/setup.ts'],
		poolOptions: {
			workers: {
				main: './src/index.ts',
				singleWorker: true,
				miniflare: {
					compatibilityDate: '2025-08-13',
					d1Databases: ['DB'],
					bindings: {
						INSTALLATION_HMAC_SECRET: 'test-only-secret-that-is-longer-than-thirty-two-bytes',
						INGEST_ENABLED: 'true',
						MAX_BATCHES_PER_INSTALLATION_PER_DAY: '25',
						MAX_ACCEPTED_BATCHES_PER_DAY: '100000',
						TEST_MIGRATIONS: migrations,
					},
				},
			},
		},
	},
})
