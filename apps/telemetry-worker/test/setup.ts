import { applyD1Migrations, env, type D1Migration } from 'cloudflare:test'
import { beforeAll } from 'vitest'

declare module 'cloudflare:test' {
    interface ProvidedEnv {
        DB: D1Database
        TEST_MIGRATIONS: D1Migration[]
    }
}

beforeAll(async () => {
    await applyD1Migrations(env.DB, env.TEST_MIGRATIONS)
})
