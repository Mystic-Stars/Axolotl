import type { Labrinth } from '@modrinth/api-client'
import type { BrowseInstallPreferences, BrowseSelectedProject } from '@modrinth/ui'
import { createContext, defineMessages } from '@modrinth/ui'
import type { ComputedRef, Ref } from 'vue'

import type ContentInstallPreviewModal from '@/components/ui/ContentInstallPreviewModal.vue'
import type {
	ContentInstallPreviewDependency,
	ContentInstallPreviewPrimary,
	ContentInstallPreviewSkipped,
} from '@/components/ui/ContentInstallPreviewModal.vue'
import type { ContentIdentity } from '@/helpers/content-identity'
import type { CurseForgeInstallPreview } from '@/helpers/curseforge'
import type { ResolveContentPlan } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import type { DownloadManager } from '@/providers/download-manager'

export type ContentSelectionProvider = 'modrinth' | 'curseforge'
export type ContentSelectionType = 'mod' | 'resourcepack' | 'datapack' | 'shader' | 'world'
export type ContentSelectionState = 'idle' | 'validating' | 'reviewing' | 'queueing' | 'error'

export interface ContentSelectionItem {
	key: string
	provider: ContentSelectionProvider
	projectId: string
	providerProjectId: string
	versionId?: string
	contentType: ContentSelectionType
	title: string
	iconUrl?: string | null
	preferences?: BrowseInstallPreferences
	targetInstanceId?: string
	slug?: string | null
	fileName?: string | null
	sha1?: string | null
	identity?: ContentIdentity
	versionPending?: boolean
}

export interface PreparedSelection {
	item: ContentSelectionItem
	primary: ContentInstallPreviewPrimary
	dependencies: ContentInstallPreviewDependency[]
	skipped: ContentInstallPreviewSkipped[]
	modrinthPlan?: ResolveContentPlan
	curseForgePreview?: CurseForgeInstallPreview
}

export interface ContentSelectionContext {
	instances: Ref<GameInstance[]>
	targetInstance: Ref<GameInstance | null>
	items: Ref<Map<string, ContentSelectionItem>>
	selectedProjects: ComputedRef<BrowseSelectedProject[]>
	selectedCount: ComputedRef<number>
	state: Ref<ContentSelectionState>
	progress: Ref<{ completed: number; total: number }>
	errorKeys: Ref<Set<string>>
	refreshInstances: (preferredId?: string | null) => Promise<GameInstance | null>
	refreshInstalledIdentities: () => Promise<void>
	setTarget: (instance: GameInstance | null) => void
	add: (item: ContentSelectionItem) => Promise<boolean>
	remove: (key: string) => void
	clear: () => void
	isSelected: (key: string) => boolean
	isInstalledIdentity: (
		provider: ContentSelectionProvider,
		projectId: string,
		slug?: string | null,
	) => boolean
	isInstalling: (key: string) => boolean
	hasPendingVersions: ComputedRef<boolean>
	updateVersion: (
		key: string,
		versionId: string,
		preferences?: BrowseInstallPreferences,
		sha1?: string,
	) => void
	installSelected: () => Promise<boolean>
	setPreviewModal: (modal: InstanceType<typeof ContentInstallPreviewModal> | null) => void
}

export interface CreateContentSelectionOptions {
	addNotification: (notification: { title: string; type: 'error' }) => void
	handleError: (error: unknown) => void
	downloadManager: DownloadManager
}

export const [injectContentSelection, provideContentSelection] =
	createContext<ContentSelectionContext>('App', 'contentSelection')

export const contentSelectionMessages = defineMessages({
	previewFailed: {
		id: 'app.content-selection.preview-failed',
		defaultMessage: 'Some selected content could not be prepared. Remove it or try again.',
	},
	queueFailed: {
		id: 'app.content-selection.queue-failed',
		defaultMessage: 'Some content could not be added to the install queue. It remains selected.',
	},
	dependencyConflict: {
		id: 'app.content-selection.dependency-conflict',
		defaultMessage: '{dependency} resolves to conflicting versions in this selection.',
	},
	unknownDependency: {
		id: 'app.content-selection.unknown-dependency',
		defaultMessage: 'Dependency {id}',
	},
	unknownReason: {
		id: 'app.content-selection.unknown-reason',
		defaultMessage: 'Could not be resolved',
	},
	targetChanged: {
		id: 'app.content-selection.target-changed',
		defaultMessage:
			'The selected content belongs to another instance. Switch back or clear it first.',
	},
	duplicateContent: {
		id: 'app.content-selection.duplicate-content',
		defaultMessage: '{project} is already installed or selected from another source.',
	},
	conflictUnavailable: {
		id: 'app.content-selection.conflict-unavailable',
		defaultMessage: 'Could not verify whether this content duplicates another source.',
	},
})

export function curseForgeLoaderType(loader: string): number | undefined {
	if (loader === 'forge') return 1
	if (loader === 'fabric') return 4
	if (loader === 'quilt') return 5
	if (loader === 'neoforge') return 6
	return undefined
}

export function toModrinthContentType(
	contentType: ContentSelectionType,
): Labrinth.Content.v3.ContentType {
	return contentType as Labrinth.Content.v3.ContentType
}
