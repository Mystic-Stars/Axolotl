import type { H3Event } from 'h3'

import type { AdminSessionDto } from '../../shared/types/telemetry'
import type { TelemetryAdminApi } from './admin-api'
import { mockScenario } from './auth'
import { dashboardBindings } from './bindings'
import {
	D1TelemetryAdminApi,
	type TelemetryDatabase,
} from './d1-admin-api'
import { unavailable } from './errors'
import { MockTelemetryAdminApi } from './mock-admin-api'
import { remoteTelemetryDataSource } from './vercel-data-source'

export interface AdminEventContext {
	adminSession?: AdminSessionDto
	adminApi?: TelemetryAdminApi
}

export function requireSession(event: H3Event): AdminSessionDto {
	const session = (event.context as AdminEventContext).adminSession
	if (!session) throw unavailable()
	return session
}

export function getAdminApi(event: H3Event): TelemetryAdminApi {
	const context = event.context as AdminEventContext
	if (context.adminApi) return context.adminApi
	const config = useRuntimeConfig(event)
	const bindings = dashboardBindings(event as unknown as { context: Record<string, unknown> })
	const remote = remoteTelemetryDataSource()
	const db = (bindings.DB as unknown as TelemetryDatabase | undefined) ?? remote?.db
	const accountTag = String(bindings.CLOUDFLARE_ACCOUNT_ID ?? '').trim()
	const analyticsToken = String(bindings.CLOUDFLARE_ANALYTICS_TOKEN ?? '').trim()
	if (db) {
		context.adminApi = new D1TelemetryAdminApi(db, undefined, {
			storeErrorContext: false,
			healthUrl: String(config.publicWorkerHealthUrl),
			analytics:
				accountTag && analyticsToken ? { accountTag, apiToken: analyticsToken } : undefined,
		})
	} else if (context.adminSession?.mock) {
		context.adminApi = new MockTelemetryAdminApi(mockScenario(config))
	} else {
		throw unavailable()
	}
	return context.adminApi
}
