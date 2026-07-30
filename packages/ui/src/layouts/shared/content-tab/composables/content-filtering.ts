import type { ClientWarningType, ContentItem } from '../types'

const CLIENT_ONLY_ENVIRONMENTS = new Set(['client_only', 'singleplayer_only'])

export function isClientOnlyEnvironment(env?: string | null): boolean {
	return !!env && CLIENT_ONLY_ENVIRONMENTS.has(env)
}

export function getClientWarningType(item: ContentItem): ClientWarningType | null {
	if (item.pack_client_retained) return 'retained'
	if (item.pack_client_depends) return 'depends'
	if (isClientOnlyEnvironment(item.environment)) return 'environment'
	return null
}

export interface ContentFilterOption {
	id: string
	label: string
}
