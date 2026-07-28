import type { Labrinth } from '@modrinth/api-client'
import type { RouteLocationRaw } from 'vue-router'

import type { Option as OverflowMenuOption } from '#ui/components/base/OverflowMenu.vue'

export type ContentCardProject = Pick<
	Labrinth.Projects.v2.Project,
	'id' | 'slug' | 'title' | 'icon_url'
>

export type ContentCardVersion = Pick<Labrinth.Versions.v2.Version, 'id' | 'version_number'> & {
	file_name: string
	date_published?: string
}

export interface ContentOwner {
	id: string
	name: string
	avatar_url?: string
	type: 'user' | 'organization'
	link?: string | RouteLocationRaw | (() => void)
}

export type ClientWarningType = 'retained' | 'depends' | 'environment'

export interface ContentCardTableItem {
	id: string
	project: ContentCardProject
	projectLink?: string | RouteLocationRaw
	version?: ContentCardVersion
	versionLink?: string | RouteLocationRaw
	owner?: ContentOwner
	enabled?: boolean
	disabled?: boolean
	disabledTooltip?: string | null
	toggleDisabled?: boolean
	toggleDisabledTooltip?: string | null
	installing?: boolean
	hasUpdate?: boolean
	isClientOnly?: boolean
	clientWarning?: ClientWarningType | null
	hideSwitchVersion?: boolean
	pendingManualDownload?: boolean
	overflowOptions?: OverflowMenuOption[]
	isGroupHeader?: boolean
	group?: string
	groupItemCount?: number
	groupSwitchVersion?: () => void
	groupChildIds?: string[]
	isGroupChild?: boolean
	downloads?: number | null
	followers?: number | null
	categories?: ContentModpackCardCategory[]
}

export type ContentCardTableSortColumn = 'project' | 'version'
export type ContentCardTableSortDirection = 'asc' | 'desc'

export interface BulkOperationStatus {
	message?: string
	progress?: number
	total?: number
	waiting?: boolean
}

/** Content item returned from the app backend API - maps to ContentCardTableItem for display */
export interface ContentItem extends Omit<
	ContentCardTableItem,
	'id' | 'projectLink' | 'disabled' | 'overflowOptions'
> {
	id: string
	file_name: string
	file_path?: string
	size?: number
	project_type: string
	/** Provider-qualified update returned by the launcher backend. */
	update:
		| {
				provider: 'modrinth'
				project_id: string
				current_version_id: string
				target_version_id: string
		  }
		| {
				provider: 'curseforge'
				project_id: number
				current_file_id: number
				target_file_id: number
		  }
		| null
	origin_provider: 'modrinth' | 'curseforge' | null
	date_added?: string
	environment?: string
	pack_client_retained?: boolean
	pack_client_depends?: boolean
	installing?: boolean
	pendingManualDownload?: boolean
	provider_refs: Array<
		| {
				provider: 'modrinth'
				project_id: string
				version_id: string | null
			}
		| {
				provider: 'curseforge'
				project_id: number
				file_id: number | null
			}
	>
}

export type ContentModpackCardProject = Pick<
	Labrinth.Projects.v2.Project,
	'id' | 'slug' | 'title' | 'icon_url' | 'description'
> & {
	downloads?: number | null
	followers?: number | null
	filename?: string | null
}

export type ContentModpackCardVersion = Pick<
	Labrinth.Versions.v2.Version,
	'id' | 'version_number' | 'date_published'
>

export type ContentModpackCardCategory = Labrinth.Tags.v2.Category & {
	action?: (event: MouseEvent) => void
}
