import { AbstractFeature, type RequestContext } from '@modrinth/api-client'

import { AxolotlBrandConfig, getOfficialLabrinthBaseUrl, MODRINTH_MIRROR_BASE_URL } from '@/config'

const SENSITIVE_HEADERS = new Set([
	'authorization',
	'cookie',
	'modrinth-download-meta',
	'proxy-authorization',
	'x-api-key',
])

function mirrorRequestSuffix(url: string) {
	try {
		const mirrorBase = new URL(MODRINTH_MIRROR_BASE_URL)
		const requestUrl = new URL(url)
		const basePath = mirrorBase.pathname.replace(/\/$/, '')
		if (
			requestUrl.origin !== mirrorBase.origin ||
			(requestUrl.pathname !== basePath && !requestUrl.pathname.startsWith(`${basePath}/`))
		) {
			return undefined
		}

		return `${requestUrl.pathname.slice(basePath.length) || '/'}${requestUrl.search}`
	} catch {
		return undefined
	}
}

function hasSensitiveHeaders(headers: Record<string, string> | undefined) {
	return Object.keys(headers ?? {}).some((name) => SENSITIVE_HEADERS.has(name.toLowerCase()))
}

function isPublicLabrinthPath(path: string) {
	const segments = path.split('/').filter(Boolean)
	const [version = '', resource = '', identifier = '', subresource = '', detail = ''] = segments

	if (version === 'v2') {
		switch (resource) {
			case 'search':
			case 'projects':
			case 'projects_random':
			case 'versions':
			case 'version_files':
			case 'users':
			case 'game_version':
			case 'loader':
			case 'donation_platform':
				return segments.length === 2
			case 'project':
				return (
					Boolean(identifier) &&
					(segments.length === 3 ||
						(segments.length === 4 &&
							['check', 'dependencies', 'gallery', 'version'].includes(subresource)) ||
						(segments.length === 5 && subresource === 'version' && Boolean(detail)))
				)
			case 'version':
			case 'version_file':
				return Boolean(identifier) && segments.length === 3
			case 'user':
				return (
					Boolean(identifier) &&
					(segments.length === 3 || (segments.length === 4 && subresource === 'projects'))
				)
			case 'team':
				return Boolean(identifier) && segments.length === 4 && subresource === 'members'
			case 'tag':
				return Boolean(identifier) && (segments.length === 3 || segments.length === 4)
			default:
				return false
		}
	}

	if (version === 'v3') {
		switch (resource) {
			case 'projects':
			case 'versions':
			case 'users':
			case 'organizations':
			case 'teams':
				return segments.length === 2
			case 'project':
				return (
					Boolean(identifier) &&
					(segments.length === 3 ||
						(segments.length === 4 &&
							['dependencies', 'members', 'organization', 'version'].includes(subresource)) ||
						(segments.length === 5 && subresource === 'version' && Boolean(detail)))
				)
			case 'version':
				return Boolean(identifier) && segments.length === 3
			case 'organization':
				return (
					Boolean(identifier) &&
					(segments.length === 3 || (segments.length === 4 && subresource === 'projects'))
				)
			case 'user':
				return (
					Boolean(identifier) &&
					(segments.length === 3 || (segments.length === 4 && subresource === 'projects'))
				)
			default:
				return false
		}
	}

	return false
}

function canUseMirror(context: RequestContext, path: string) {
	return (
		(context.options.method ?? 'GET') === 'GET' &&
		!hasSensitiveHeaders(context.options.headers) &&
		(!AxolotlBrandConfig.capabilities.privateModrinthServices ||
			context.options.skipAuth === true) &&
		isPublicLabrinthPath(path)
	)
}

function officialUrl(suffix: string) {
	return `${getOfficialLabrinthBaseUrl()}${suffix}`
}

export class ModrinthMirrorFallbackFeature extends AbstractFeature {
	shouldApply(context: RequestContext) {
		return (
			super.shouldApply(context) &&
			context.options.api === 'labrinth' &&
			mirrorRequestSuffix(context.url) !== undefined
		)
	}

	async execute<T>(next: () => Promise<T>, context: RequestContext): Promise<T> {
		const mirrorUrl = context.url
		const suffix = mirrorRequestSuffix(mirrorUrl)
		if (!suffix) return await next()

		const originalHeaders = context.options.headers
		if (!canUseMirror(context, suffix)) {
			context.url = officialUrl(suffix)
			console.info('[modrinth-mirror] Bypassing mirror for official-only request', {
				source: 'Official',
				method: context.options.method ?? 'GET',
				url: context.url,
				route: 1,
			})
			try {
				return await next()
			} finally {
				context.url = mirrorUrl
				context.options.headers = originalHeaders
			}
		}

		const mirrorStarted = performance.now()
		console.info('[modrinth-mirror] Attempting Modrinth API request', {
			source: 'Mirror',
			method: context.options.method ?? 'GET',
			url: mirrorUrl,
			route: 1,
		})

		try {
			const result = await next()
			console.info('[modrinth-mirror] Completed Modrinth API request', {
				source: 'Mirror',
				url: mirrorUrl,
				elapsedMs: Math.round(performance.now() - mirrorStarted),
			})
			return result
		} catch (error) {
			context.url = officialUrl(suffix)
			context.options.headers = originalHeaders
			console.warn('[modrinth-mirror] Mirror request failed; falling back to official source', {
				url: mirrorUrl,
				elapsedMs: Math.round(performance.now() - mirrorStarted),
				error,
			})
			const officialStarted = performance.now()
			console.info('[modrinth-mirror] Attempting Modrinth API request', {
				source: 'Official',
				method: context.options.method ?? 'GET',
				url: context.url,
				route: 2,
			})
			try {
				const result = await next()
				console.info('[modrinth-mirror] Completed Modrinth API request', {
					source: 'Official',
					url: context.url,
					elapsedMs: Math.round(performance.now() - officialStarted),
				})
				return result
			} catch (officialError) {
				console.error('[modrinth-mirror] Official Modrinth API request failed', {
					url: context.url,
					elapsedMs: Math.round(performance.now() - officialStarted),
					error: officialError,
				})
				throw officialError
			}
		} finally {
			context.url = mirrorUrl
			context.options.headers = originalHeaders
		}
	}
}
