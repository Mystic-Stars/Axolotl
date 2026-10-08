import { createRequire } from 'node:module'

import type { PlatformProxy } from 'wrangler'

import type { DashboardBindings } from '../utils/bindings'

let proxy: PlatformProxy<DashboardBindings> | null = null
const require = createRequire(import.meta.url)

export default defineNitroPlugin(async (nitroApp) => {
    const { getPlatformProxy } = require('wrangler') as typeof import('wrangler')
    proxy = await getPlatformProxy<DashboardBindings>({
        configPath: 'wrangler.toml',
        persist: false,
        remoteBindings: true,
    })

    nitroApp.hooks.hook('request', (event) => {
        event.context.cloudflare = { env: proxy?.env ?? {} }
    })

    nitroApp.hooks.hook('close', async () => {
        await proxy?.dispose()
        proxy = null
    })
})
