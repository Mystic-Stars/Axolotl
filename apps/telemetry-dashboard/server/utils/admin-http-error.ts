import { createError } from 'h3'

import type { AdminApiError } from './errors'

export function toAdminHttpError(error: AdminApiError) {
    return createError({
        statusCode: error.statusCode,
        statusMessage: error.message,
        data: { error: { code: error.code, message: error.message } },
    })
}
