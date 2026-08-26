import type { ContentItem } from '../types'

export type ContentSortMode =
	| 'project-name-asc'
	| 'project-name-desc'
	| 'file-name-asc'
	| 'file-name-desc'
	| 'date-added-newest'
	| 'date-added-oldest'

function fileNameSortKey(fileName: string): string {
	return fileName.replace(/\.disabled$/i, '')
}

function compareText(left: string, right: string, locale?: string): number {
	return left.localeCompare(right, locale, {
		numeric: true,
		sensitivity: 'base',
	})
}

function compareByFileName(
	left: ContentItem,
	right: ContentItem,
	locale?: string,
	getStableId: (item: ContentItem) => string = (item) => item.id,
): number {
	return (
		compareText(fileNameSortKey(left.file_name), fileNameSortKey(right.file_name), locale) ||
		compareText(left.file_name, right.file_name, locale) ||
		compareText(left.project?.title ?? '', right.project?.title ?? '', locale) ||
		compareText(getStableId(left), getStableId(right), locale)
	)
}

function compareByProjectName(
	left: ContentItem,
	right: ContentItem,
	locale?: string,
	getStableId?: (item: ContentItem) => string,
): number {
	return (
		compareText(
			left.project?.title ?? left.file_name,
			right.project?.title ?? right.file_name,
			locale,
		) || compareByFileName(left, right, locale, getStableId)
	)
}

export function sortContentItems(
	items: ContentItem[],
	mode: ContentSortMode,
	locale?: string,
	getStableId?: (item: ContentItem) => string,
): ContentItem[] {
	const sorted = [...items]

	return sorted.sort((left, right) => {
		switch (mode) {
			case 'project-name-desc':
				return -compareByProjectName(left, right, locale, getStableId)
			case 'file-name-asc':
				return compareByFileName(left, right, locale, getStableId)
			case 'file-name-desc':
				return -compareByFileName(left, right, locale, getStableId)
			case 'date-added-newest':
				return (
					(right.date_added ?? '').localeCompare(left.date_added ?? '') ||
					compareByFileName(left, right, locale, getStableId)
				)
			case 'date-added-oldest':
				return (
					(left.date_added ?? '').localeCompare(right.date_added ?? '') ||
					compareByFileName(left, right, locale, getStableId)
				)
			case 'project-name-asc':
			default:
				return compareByProjectName(left, right, locale, getStableId)
		}
	})
}
