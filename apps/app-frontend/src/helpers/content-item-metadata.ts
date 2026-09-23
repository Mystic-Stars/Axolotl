import type { ContentItem } from '@modrinth/ui'

export function mergeContentItemMetadata(item: ContentItem, metadata: ContentItem): ContentItem {
	const merged = { ...item, ...metadata }
	if (item.instanceCapabilities) {
		merged.instanceCapabilities = {
			...item.instanceCapabilities,
			canUpdate: merged.update != null && merged.instanceMaterializationState === 'present',
		}
	}
	return merged
}
