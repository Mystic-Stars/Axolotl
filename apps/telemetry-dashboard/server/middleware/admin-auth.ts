import { getHeader } from 'h3'

import { toAdminHttpError } from '../utils/admin-http-error'
import {
    accessSettings,
    authenticateAccessToken,
    mockAuthEnabled,
    mockScenario,
    mockSession,
} from '../utils/auth'
import { AdminApiError, unauthorized, unavailable } from '../utils/errors'
import type { AdminEventContext } from '../utils/service'

export default defineEventHandler(async (event) => {
    if (!event.path.startsWith('/api/admin/')) return
    try {
        const config = useRuntimeConfig(event)
        if (mockAuthEnabled(config)) {
            ;(event.context as AdminEventContext).adminSession = mockSession(mockScenario(config))
            return
        }
        const settings = accessSettings(config)
        if (!settings) {
            if (process.env.NODE_ENV === 'production') throw unavailable()
            throw unauthorized()
        }
        const token = getHeader(event, 'cf-access-jwt-assertion')
        ;(event.context as AdminEventContext).adminSession = await authenticateAccessToken(
            token,
            settings,
        )
    } catch (error) {
        if (error instanceof AdminApiError) throw toAdminHttpError(error)
        throw error
    }
})
