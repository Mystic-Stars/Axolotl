import type { ContentItem } from '@modrinth/ui'

export function matchesContentItem(
    item: ContentItem,
    target: ContentItem,
    originalFileName: string,
    originalFilePath?: string,
) {
    if (
        item.file_name === originalFileName ||
        item.file_path === originalFilePath ||
        item.file_path === target.file_path
    )
        return true

    const projectId = target.project?.id
    if (!projectId || item.project?.id !== projectId) return false

    const versionId = target.version?.id
    return !versionId || item.version?.id === versionId
}

export function applyContentItemUpdates(
    items: ContentItem[],
    target: ContentItem,
    originalFileName: string,
    originalFilePath: string | undefined,
    updates: Partial<ContentItem>,
) {
    for (const item of items) {
        if (matchesContentItem(item, target, originalFileName, originalFilePath)) {
            Object.assign(item, updates)
        }
    }
    Object.assign(target, updates)
}
