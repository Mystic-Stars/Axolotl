import type { useVIntl } from '@modrinth/ui'

import type { CurseForgeInstallResult, CurseForgeManualDownloadImport } from '@/helpers/curseforge'
import type { CurseForgeManualDownloadItem } from '@/helpers/curseforge-manual'

import * as contentInstallMessages from './content-install-messages'

type FormatMessage = ReturnType<typeof useVIntl>['formatMessage']

export type ContentInstallNotification = {
	title: string
	text?: string
	type?: 'error' | 'warning' | 'success' | 'info'
	supportData?: Record<string, unknown>
}

export function mapInstallResultToManualItems(
	result: CurseForgeInstallResult,
): CurseForgeManualDownloadItem[] {
	return (result.manualDownloads ?? []).map((item) => ({
		projectId: item.projectId,
		fileId: item.fileId,
		fileName: item.fileName,
		websiteUrl: item.websiteUrl,
		projectType: item.projectType,
		projectSlug: item.projectSlug,
		targetFolder: item.targetFolder,
		hashes: item.hashes,
		fileLength: item.fileLength,
		fileFingerprint: item.fileFingerprint,
		ownershipKind: item.ownershipKind,
		operationKind: item.operationKind,
	}))
}

export function filterRemainingManualDownloads(
	items: CurseForgeManualDownloadItem[],
	imported: CurseForgeManualDownloadImport[],
): CurseForgeManualDownloadItem[] {
	const importedKeys = new Set(imported.map((item) => `${item.projectId}:${item.fileId}`))
	return items.filter((item) => !importedKeys.has(`${item.projectId}:${item.fileId}`))
}

export function formatCurseForgeFileListText(
	formatMessage: FormatMessage,
	fileNames: string[],
	totalCount: number,
	options?: { emptyFallbackToCount?: boolean },
): string {
	const names = fileNames
		.slice(0, 5)
		.map((name) => name)
		.filter(Boolean)
	const extra = totalCount > names.length ? totalCount - names.length : 0
	if (names.length === 0) {
		return options?.emptyFallbackToCount
			? formatMessage(contentInstallMessages.manualDownloadsFilesCountMessage, {
					count: totalCount,
				})
			: ''
	}
	if (extra > 0) {
		return formatMessage(contentInstallMessages.manualDownloadsListAndMoreMessage, {
			list: names.join(', '),
			count: extra,
		})
	}
	return names.join(', ')
}

export function formatManualDownloadsNotification(
	formatMessage: FormatMessage,
	summary: { installed: number; manual: number },
	fileNames: string[],
): ContentInstallNotification {
	const listText = formatCurseForgeFileListText(formatMessage, fileNames, summary.manual, {
		emptyFallbackToCount: true,
	})
	return {
		title: formatMessage(contentInstallMessages.manualDownloadsTitleMessage),
		text:
			summary.installed > 0
				? formatMessage(contentInstallMessages.manualDownloadsPartialMessage, {
						installed: summary.installed,
						manual: summary.manual,
						list: listText,
					})
				: formatMessage(contentInstallMessages.manualDownloadsFailedMessage, {
						manual: summary.manual,
						list: listText,
					}),
		type: summary.installed > 0 ? 'warning' : 'error',
	}
}

export function formatAutomaticDownloadsFailedNotification(
	formatMessage: FormatMessage,
	fileNames: string[],
): ContentInstallNotification {
	const names = fileNames
		.slice(0, 5)
		.map((name) => name)
		.filter(Boolean)
	const extra = fileNames.length - names.length
	const listText =
		extra > 0
			? formatMessage(contentInstallMessages.manualDownloadsListAndMoreMessage, {
					list: names.join(', '),
					count: extra,
				})
			: names.join(', ')
	return {
		title: formatMessage(contentInstallMessages.automaticDownloadsFailedTitleMessage),
		text: formatMessage(contentInstallMessages.automaticDownloadsFailedMessage, {
			failed: fileNames.length,
			list: listText,
		}),
		type: 'error',
	}
}

export function formatDependencyNotesNotification(
	formatMessage: FormatMessage,
	counts: { optional: number; incompatible: number; skipped: number },
): ContentInstallNotification | null {
	const { optional, incompatible, skipped } = counts
	if (optional === 0 && incompatible === 0 && skipped === 0) return null
	return {
		title: formatMessage(contentInstallMessages.dependencyNotesTitleMessage),
		text: formatMessage(contentInstallMessages.dependencyNotesMessage, {
			optional,
			incompatible,
			skipped,
		}),
		type: 'info',
	}
}

export function formatImportedManualDownloadsNotification(
	formatMessage: FormatMessage,
	count: number,
): ContentInstallNotification {
	return {
		title: formatMessage(contentInstallMessages.manualDownloadsImportedTitleMessage),
		text: formatMessage(contentInstallMessages.manualDownloadsImportedMessage, { count }),
		type: 'success',
	}
}
