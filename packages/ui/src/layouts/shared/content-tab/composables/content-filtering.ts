import type { ContentItem } from '../types'

export function isPresentContentItem(item: ContentItem): boolean {
	return (
		item.instanceMaterializationState == null || item.instanceMaterializationState === 'present'
	)
}

export function isEnabledContentItem(item: ContentItem): boolean {
	return isPresentContentItem(item) && item.enabled === true
}

export function isDisabledContentItem(item: ContentItem): boolean {
	return isPresentContentItem(item) && item.enabled === false
}

export function canToggleContentItem(item: ContentItem): boolean {
	return (
		isPresentContentItem(item) &&
		item.enabled !== undefined &&
		item.instanceCapabilities?.canToggle !== false
	)
}

export interface ContentFilterOption {
	id: string
	label: string
}
