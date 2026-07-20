import { AbstractFeature, type RequestContext } from '@modrinth/api-client'

import { getOfficialLabrinthBaseUrl, MODRINTH_MIRROR_BASE_URL } from '@/config'

function withoutSensitiveHeaders(headers: Record<string, string> | undefined) {
	return Object.fromEntries(
		Object.entries(headers ?? {}).filter(([name]) => {
			const normalizedName = name.toLowerCase()
			return normalizedName !== 'authorization' && normalizedName !== 'modrinth-download-meta'
		}),
	)
}

export class ModrinthMirrorFallbackFeature extends AbstractFeature {
	shouldApply(context: RequestContext) {
		return (
			super.shouldApply(context) &&
			(context.options.method ?? 'GET') === 'GET' &&
			context.options.api === 'labrinth' &&
			context.url.startsWith(MODRINTH_MIRROR_BASE_URL)
		)
	}

	async execute<T>(next: () => Promise<T>, context: RequestContext): Promise<T> {
		const mirrorUrl = context.url
		const originalHeaders = context.options.headers
		context.options.headers = withoutSensitiveHeaders(originalHeaders)

		try {
			return await next()
		} catch (error) {
			context.url = `${getOfficialLabrinthBaseUrl()}${mirrorUrl.slice(MODRINTH_MIRROR_BASE_URL.length)}`
			context.options.headers = originalHeaders
			console.warn(
				'[modrinth-mirror] Mirror request failed; falling back to official source',
				error,
			)
			return await next()
		} finally {
			context.url = mirrorUrl
			context.options.headers = originalHeaders
		}
	}
}
