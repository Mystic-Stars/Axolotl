<script setup lang="ts">
import {
	ArrowLeftRightIcon,
	ChevronDownIcon,
	ChevronRightIcon,
	ClockIcon,
	DownloadIcon,
	MoreVerticalIcon,
	SkullIcon,
	SpinnerIcon,
	TrashExclamationIcon,
	TrashIcon,
	TriangleAlertIcon,
	UndoIcon,
	UserIcon,
} from '@modrinth/assets'
import { useMagicKeys } from '@vueuse/core'
import { computed, getCurrentInstance, ref } from 'vue'
import type { RouteLocationRaw } from 'vue-router'

import AutoLink from '#ui/components/base/AutoLink.vue'
import Avatar from '#ui/components/base/Avatar.vue'
import BulletDivider from '#ui/components/base/BulletDivider.vue'
import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import Checkbox from '#ui/components/base/Checkbox.vue'
import type { Option as OverflowMenuOption } from '#ui/components/base/OverflowMenu.vue'
import TeleportOverflowMenu from '#ui/components/base/TeleportOverflowMenu.vue'
import Toggle from '#ui/components/base/Toggle.vue'
import { useRelativeTime } from '#ui/composables/how-ago'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'
import { truncatedTooltip } from '#ui/utils/truncate'

import type {
	ClientWarningType,
	ContentCardProject,
	ContentCardVersion,
	ContentOwner,
	ContentRowInlineAction,
	ContentWorldGroupMeta,
} from '../types'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	selectProject: {
		id: 'content.card.select-project',
		defaultMessage: 'Select {project}',
	},
	pendingManualDownload: {
		id: 'content.card.pending-manual-download',
		defaultMessage: 'Manual download required',
	},
	duplicateMod: {
		id: 'content.card.duplicate-mod',
		defaultMessage: 'This mod is installed {count, number} times.',
	},
	rollbackTooltip: {
		id: 'content.card.rollback-tooltip',
		defaultMessage: 'Roll back to {fileName}',
	},
	dependencyBadge: {
		id: 'content.card.dependency-badge',
		defaultMessage: 'Dependency',
	},
	orphanedDependencyBadge: {
		id: 'content.card.orphaned-dependency-badge',
		defaultMessage: 'Orphaned dependency',
	},
	notPlayedYet: {
		id: 'content.card.group.not-played-yet',
		defaultMessage: 'Not played yet',
	},
	hardcore: {
		id: 'content.card.group.hardcore',
		defaultMessage: 'Hardcore mode',
	},
})

interface Props {
	project: ContentCardProject
	projectLink?: string | RouteLocationRaw
	version?: ContentCardVersion
	versionLink?: string | RouteLocationRaw
	owner?: ContentOwner
	enabled?: boolean
	installing?: boolean
	pendingManualDownload?: boolean
	duplicateCount?: number
	hasUpdate?: boolean
	rollbackFileName?: string
	isClientOnly?: boolean
	clientWarning?: ClientWarningType | null
	hideSwitchVersion?: boolean
	overflowOptions?: OverflowMenuOption[]
	inlineActions?: ContentRowInlineAction[]
	disabled?: boolean
	disabledTooltip?: string | null
	postUpgradeWarningTooltip?: string | null
	toggleDisabled?: boolean
	toggleDisabledTooltip?: string | null
	dependencyBadge?: {
		autoDependency: boolean
		orphaned: boolean
	} | null
	showCheckbox?: boolean
	hideDelete?: boolean
	hideActions?: boolean
	inline?: boolean
	isGroupHeader?: boolean
	groupDepth?: number
	groupItemCount?: number
	groupExpanded?: boolean
	groupSwitchVersion?: () => void
	isGroupChild?: boolean
	groupKind?: 'folder' | 'world'
	groupMeta?: ContentWorldGroupMeta
	groupCheckboxIndeterminate?: boolean
	downloads?: number | null
	categories?: Array<{
		name: string
		icon?: string
		action?: (event: MouseEvent) => void
	}>
}

const props = withDefaults(defineProps<Props>(), {
	projectLink: undefined,
	version: undefined,
	versionLink: undefined,
	owner: undefined,
	enabled: undefined,
	installing: false,
	pendingManualDownload: false,
	duplicateCount: undefined,
	hasUpdate: false,
	rollbackFileName: undefined,
	isClientOnly: false,
	clientWarning: null,
	hideSwitchVersion: false,
	overflowOptions: undefined,
	inlineActions: undefined,
	disabled: false,
	disabledTooltip: undefined,
	postUpgradeWarningTooltip: undefined,
	toggleDisabled: false,
	toggleDisabledTooltip: undefined,
	dependencyBadge: null,
	showCheckbox: false,
	hideDelete: false,
	hideActions: false,
	inline: false,
	isGroupHeader: false,
	groupDepth: 0,
	groupItemCount: 0,
	groupExpanded: false,
	groupSwitchVersion: undefined,
	isGroupChild: false,
	groupKind: 'folder',
	groupMeta: undefined,
	groupCheckboxIndeterminate: false,
	downloads: null,
	categories: undefined,
})

const selected = defineModel<boolean>('selected')

const emit = defineEmits<{
	'update:enabled': [value: boolean]
	select: [value: boolean, event?: MouseEvent]
	delete: [event: MouseEvent]
	update: []
	switchVersion: []
	rollback: []
	toggleExpand: []
}>()

const instance = getCurrentInstance()
const hasDeleteListener = computed(() => typeof instance?.vnode.props?.onDelete === 'function')
const hasUpdateListener = computed(() => typeof instance?.vnode.props?.onUpdate === 'function')
const hasRollbackListener = computed(() => typeof instance?.vnode.props?.onRollback === 'function')
const hasSwitchVersionListener = computed(
	() => typeof instance?.vnode.props?.onSwitchVersion === 'function',
)

const formatCompact = (n: number | undefined | null) => {
	if (n == null) return ''
	return new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 2 }).format(n)
}

const formatTimeAgo = useRelativeTime()

const versionNumberRef = ref<HTMLElement | null>(null)
const fileNameRef = ref<HTMLElement | null>(null)

const isDisabled = computed(() => props.disabled || props.installing)
const isToggleDisabled = computed(() => isDisabled.value || props.toggleDisabled)

const interactiveSelectors = 'a, button, input, select, [role="checkbox"], [role="button"]'

function handleRowClick(event: MouseEvent) {
	const target = event.target as HTMLElement
	if (target.closest(interactiveSelectors)) return
	emit('toggleExpand')
}

const { shift: shiftHeld } = useMagicKeys()
const deleteHovered = ref(false)
</script>

<template>
	<div
		v-if="isGroupHeader"
		role="row"
		class="flex h-[74px] cursor-pointer items-center justify-between gap-4"
		:class="[
			{ 'opacity-50': disabled },
			groupKind === 'world'
				? selected
					? 'card-shadow !bg-surface-2.5 rounded-lg p-3'
					: 'card-shadow !bg-bg-raised rounded-lg p-3 hover:!bg-bg-raised'
				: 'px-3 hover:bg-[hsl(230deg,6.98%,16.86%,60%)]',
		]"
		:style="
			groupDepth && groupKind !== 'world' ? { paddingLeft: `${groupDepth * 2.5}rem` } : undefined
		"
		@click="handleRowClick"
	>
		<div
			class="flex min-w-0 items-center gap-4"
			:class="
				hideActions ? 'flex-1' : 'flex-1 @[800px]:w-[45%] @[800px]:shrink-0 @[800px]:flex-none'
			"
		>
			<Checkbox
				v-if="showCheckbox"
				:model-value="selected ?? false"
				:indeterminate="groupCheckboxIndeterminate"
				:aria-label="formatMessage(messages.selectProject, { project: project.title })"
				class="shrink-0"
				@update:model-value="(value, event) => emit('select', value, event)"
			/>

			<div class="flex min-w-0 items-center gap-3">
				<Avatar
					:src="project.icon_url"
					:alt="project.title"
					size="3rem"
					no-shadow
					class="rounded-2xl border border-surface-5"
				/>
				<div class="flex min-w-0 flex-col gap-0.5">
					<div class="flex min-w-0 items-center gap-1">
						<TriangleAlertIcon
							v-if="postUpgradeWarningTooltip"
							v-tooltip="postUpgradeWarningTooltip"
							class="size-4 shrink-0 text-brand-orange"
							aria-hidden="true"
						/>
						<AutoLink
							:target="
								typeof projectLink === 'string' && projectLink.startsWith('http')
									? '_blank'
									: undefined
							"
							:to="projectLink"
							class="truncate text-contrast !decoration-contrast"
							:class="[
								groupKind === 'world' ? 'text-lg font-bold' : 'font-semibold leading-6',
								{ 'hover:underline': projectLink },
							]"
						>
							{{ project.title }}
						</AutoLink>
						<span
							v-if="groupKind === 'world'"
							class="flex items-center gap-1 whitespace-nowrap text-sm font-semibold text-secondary"
						>
							<UserIcon
								aria-hidden="true"
								class="h-4 w-4 shrink-0 text-secondary"
								stroke-width="3px"
							/>
							{{ formatMessage(commonMessages.singleplayerLabel) }}
						</span>
						<span class="shrink-0 text-sm font-medium text-secondary">
							({{ groupItemCount }})
						</span>
					</div>
					<div class="flex min-w-0 items-center gap-1">
						<template v-if="groupKind === 'world'">
							<template v-if="groupMeta?.last_played">
								<ClockIcon class="size-4 shrink-0 text-secondary" />
								<span class="truncate text-sm leading-5 text-secondary">
									{{
										formatMessage(commonMessages.playedLabel, {
											ago: formatTimeAgo(new Date(groupMeta.last_played)),
										})
									}}
								</span>
							</template>
							<span v-else class="truncate text-sm leading-5 text-secondary">
								{{ formatMessage(messages.notPlayedYet) }}
							</span>
						</template>
						<template v-else>
							<AutoLink
								v-if="owner"
								:target="
									typeof owner.link === 'string' && owner.link.startsWith('http')
										? '_blank'
										: undefined
								"
								:to="owner.link"
								class="flex shrink-0 items-center gap-1 !decoration-secondary"
								:class="{ 'hover:underline': owner.link }"
							>
								<Avatar
									:src="owner.avatar_url"
									:alt="owner.name"
									size="1.5rem"
									:circle="owner.type === 'user'"
									no-shadow
									class="shrink-0"
								/>
								<span class="text-sm leading-5 text-secondary">{{ owner.name }}</span>
							</AutoLink>
							<template v-if="version">
								<BulletDivider class="shrink-0 @[800px]:hidden" />
								<AutoLink
									:target="
										typeof versionLink === 'string' && versionLink.startsWith('http')
											? '_blank'
											: undefined
									"
									:to="versionLink"
									class="truncate text-sm leading-5 text-secondary !decoration-secondary @[800px]:hidden"
									:class="{ 'hover:underline': versionLink }"
								>
									{{ version.version_number }}
								</AutoLink>
								<template v-if="version.date_published">
									<BulletDivider class="shrink-0 @[800px]:hidden" />
									<ClockIcon class="size-4 shrink-0 text-secondary @[800px]:hidden" />
									<span class="shrink-0 text-sm leading-5 text-secondary @[800px]:hidden">
										{{ formatTimeAgo(new Date(version.date_published)) }}
									</span>
								</template>
							</template>
						</template>
					</div>
				</div>
			</div>
		</div>

		<div
			class="hidden flex-col gap-0.5 @[800px]:flex"
			:class="hideActions ? 'flex-1' : 'flex-1 min-w-0'"
		>
			<template v-if="groupKind === 'world'">
				<div class="flex min-w-0 items-center gap-1.5 font-medium leading-6 text-contrast">
					<template v-if="groupMeta?.hardcore">
						<SkullIcon aria-hidden="true" class="h-4 w-4 shrink-0 text-red" />
						<span class="text-red">{{ formatMessage(messages.hardcore) }}</span>
					</template>
					<span v-else-if="groupMeta?.game_mode" class="text-secondary">
						{{ groupMeta.game_mode.charAt(0).toUpperCase() + groupMeta.game_mode.slice(1) }}
					</span>
				</div>
			</template>
			<template v-else-if="version">
				<div class="flex min-w-0 items-center gap-1.5 font-medium leading-6 text-contrast">
					<AutoLink
						:target="
							typeof versionLink === 'string' && versionLink.startsWith('http')
								? '_blank'
								: undefined
						"
						:to="versionLink"
						class="truncate self-start !decoration-contrast"
						:class="{ 'hover:underline': versionLink, 'cursor-pointer': versionLink }"
					>
						{{ version.version_number }}
					</AutoLink>
					<template v-if="version.date_published">
						<ClockIcon class="hidden size-4 shrink-0 text-secondary @[600px]:inline" />
						<span
							class="hidden shrink-0 text-sm font-normal leading-6 text-secondary @[600px]:inline"
						>
							{{ formatTimeAgo(new Date(version.date_published)) }}
						</span>
					</template>
				</div>
				<span class="flex min-w-0 leading-6 text-secondary">
					<span class="truncate">{{ version.file_name }}</span>
				</span>
			</template>
			<div v-if="downloads != null" class="flex flex-nowrap items-center gap-3 overflow-hidden">
				<div v-if="downloads != null" class="flex items-center gap-2 text-secondary">
					<DownloadIcon class="size-4" />
					<span class="text-sm font-medium">{{ formatCompact(downloads) }}</span>
				</div>
			</div>
		</div>

		<div v-if="!hideActions" class="flex min-w-[160px] shrink-0 items-center justify-end gap-2">
			<template v-if="groupKind !== 'world'">
				<ButtonStyled
					v-if="hasUpdate"
					circular
					type="transparent"
					color="green"
					color-fill="text"
					hover-color-fill="background"
				>
					<button
						v-tooltip="
							isDisabled && disabledTooltip
								? disabledTooltip
								: formatMessage(commonMessages.updateAvailableLabel)
						"
						:disabled="isDisabled"
						@click.stop="emit('update')"
					>
						<DownloadIcon class="size-5" />
					</button>
				</ButtonStyled>
				<ButtonStyled v-else-if="groupSwitchVersion" circular type="transparent">
					<button
						v-tooltip="formatMessage(commonMessages.switchVersionButton)"
						@click.stop="groupSwitchVersion"
					>
						<ArrowLeftRightIcon class="size-5" />
					</button>
				</ButtonStyled>
			</template>
			<ButtonStyled circular type="transparent">
				<button
					class="flex items-center text-secondary hover:text-primary transition-colors"
					@click.stop="emit('toggleExpand')"
				>
					<ChevronDownIcon v-if="groupExpanded" class="size-5" />
					<ChevronRightIcon v-else class="size-5" />
				</button>
			</ButtonStyled>
		</div>
	</div>

	<div
		v-else
		role="row"
		class="flex items-center justify-between"
		:class="{
			'h-[74px] gap-4 px-3': !inline,
			'gap-3': inline,
			'opacity-50 grayscale': disabled && !installing,
			'opacity-50': installing,
			'pl-10': isGroupChild && !inline,
		}"
		:style="
			isGroupChild && !inline && groupDepth > 1
				? { paddingLeft: `${groupDepth * 2.5}rem` }
				: undefined
		"
	>
		<div
			class="flex min-w-0 items-center gap-4"
			:class="
				hideActions ? 'flex-1' : 'flex-1 @[800px]:w-[45%] @[800px]:shrink-0 @[800px]:flex-none'
			"
		>
			<Checkbox
				v-if="showCheckbox"
				:model-value="selected ?? false"
				:aria-label="formatMessage(messages.selectProject, { project: project.title })"
				class="shrink-0"
				@update:model-value="(value, event) => emit('select', value, event)"
			/>

			<div
				class="flex min-w-0 items-center gap-3 transition-[filter,opacity] duration-200"
				:class="enabled === false && !disabled ? 'grayscale opacity-50' : ''"
			>
				<div
					v-tooltip="
						installing
							? formatMessage(commonMessages.installingLabel)
							: pendingManualDownload
								? formatMessage(messages.pendingManualDownload)
								: undefined
					"
					class="relative flex shrink-0 items-center"
				>
					<Avatar
						:src="project.icon_url"
						:alt="project.title"
						size="3rem"
						no-shadow
						class="rounded-2xl border border-surface-5"
					/>
					<div
						v-if="installing"
						class="absolute inset-0 flex items-center justify-center rounded-2xl bg-black/20"
					>
						<SpinnerIcon class="size-5 animate-spin text-white" />
					</div>
					<div
						v-else-if="pendingManualDownload"
						class="absolute -right-1 -top-1 flex size-5 items-center justify-center rounded-full bg-orange text-white"
					>
						<TriangleAlertIcon class="size-3.5" />
					</div>
				</div>
				<div class="flex min-w-0 flex-col gap-0.5">
					<div class="flex min-w-0 items-center gap-1">
						<AutoLink
							:target="
								typeof projectLink === 'string' && projectLink.startsWith('http')
									? '_blank'
									: undefined
							"
							:to="projectLink"
							class="truncate font-semibold leading-6 text-contrast !decoration-contrast"
							:class="{ 'hover:underline': projectLink }"
						>
							{{ project.title }}
						</AutoLink>
						<TriangleAlertIcon
							v-if="postUpgradeWarningTooltip"
							v-tooltip="postUpgradeWarningTooltip"
							class="size-4 shrink-0 text-brand-orange"
							aria-hidden="true"
						/>
						<TriangleAlertIcon
							v-if="duplicateCount && duplicateCount > 1"
							v-tooltip="formatMessage(messages.duplicateMod, { count: duplicateCount })"
							class="size-4 shrink-0 text-red"
							aria-hidden="true"
						/>
						<span
							v-if="dependencyBadge"
							v-tooltip="
								dependencyBadge.orphaned
									? formatMessage(messages.orphanedDependencyBadge)
									: formatMessage(messages.dependencyBadge)
							"
							class="shrink-0 rounded-md bg-surface-3 px-1.5 py-0.5 text-xs font-medium leading-4 text-secondary"
						>
							{{
								dependencyBadge.orphaned
									? formatMessage(messages.orphanedDependencyBadge)
									: formatMessage(messages.dependencyBadge)
							}}
						</span>
						<slot name="title-badges" />
					</div>

					<div class="flex min-w-0 items-center gap-1">
						<span
							v-if="project.description"
							class="truncate text-sm leading-5 text-secondary"
							:title="project.description"
						>
							{{ project.description }}
						</span>
						<AutoLink
							v-if="owner && !project.description"
							:target="
								typeof owner.link === 'string' && owner.link.startsWith('http')
									? '_blank'
									: undefined
							"
							:to="owner.link"
							class="flex shrink-0 items-center gap-1 !decoration-secondary"
							:class="{ 'hover:underline': owner.link }"
						>
							<Avatar
								:src="owner.avatar_url"
								:alt="owner.name"
								size="1.5rem"
								:circle="owner.type === 'user'"
								no-shadow
								class="shrink-0"
							/>
							<span class="text-sm leading-5 text-secondary">{{ owner.name }}</span>
						</AutoLink>
						<template v-if="version && !project.description">
							<BulletDivider class="shrink-0 @[800px]:hidden" />
							<AutoLink
								:target="
									typeof versionLink === 'string' && versionLink.startsWith('http')
										? '_blank'
										: undefined
								"
								:to="versionLink"
								class="truncate text-sm leading-5 text-secondary !decoration-secondary @[800px]:hidden"
								:class="{ 'hover:underline': versionLink }"
							>
								{{ version.version_number }}
							</AutoLink>
						</template>
						<template v-else-if="version && project.description">
							<BulletDivider class="shrink-0 @[800px]:hidden" />
							<AutoLink
								:target="
									typeof versionLink === 'string' && versionLink.startsWith('http')
										? '_blank'
										: undefined
								"
								:to="versionLink"
								class="truncate text-sm leading-5 text-secondary !decoration-secondary @[800px]:hidden"
								:class="{ 'hover:underline': versionLink }"
							>
								{{ version.version_number }}
							</AutoLink>
						</template>
					</div>
				</div>
			</div>
		</div>

		<div
			class="hidden flex-col gap-0.5 transition-[filter,opacity] duration-200 @[800px]:flex"
			:class="[
				hideActions ? 'flex-1' : 'flex-1 min-w-0',
				enabled === false && !disabled ? 'grayscale opacity-50' : '',
			]"
		>
			<template v-if="version">
				<AutoLink
					v-tooltip="truncatedTooltip(versionNumberRef, version.version_number)"
					:target="
						typeof versionLink === 'string' && versionLink.startsWith('http') ? '_blank' : undefined
					"
					:to="versionLink"
					class="inline-flex self-start font-medium leading-6 text-contrast !decoration-contrast"
					:class="{ 'hover:underline': versionLink, 'cursor-pointer': versionLink }"
				>
					<span ref="versionNumberRef" class="truncate">{{
						version.version_number.slice(0, Math.ceil(version.version_number.length / 2))
					}}</span
					><span class="shrink-0">{{
						version.version_number.slice(Math.ceil(version.version_number.length / 2))
					}}</span>
				</AutoLink>
				<span
					v-tooltip="truncatedTooltip(fileNameRef, version.file_name)"
					class="flex min-w-0 leading-6 text-secondary"
				>
					<span ref="fileNameRef" class="truncate">{{
						version.file_name.slice(0, Math.ceil(version.file_name.length / 2))
					}}</span
					><span class="shrink-0">{{
						version.file_name.slice(Math.ceil(version.file_name.length / 2))
					}}</span>
				</span>
			</template>
		</div>

		<div
			v-if="!hideActions"
			class="flex min-w-[160px] shrink-0 items-center justify-end gap-2 transition-colors duration-200"
		>
			<slot name="additionalButtonsLeft" />

			<ButtonStyled v-if="hasRollbackListener && rollbackFileName" circular type="transparent">
				<button
					v-tooltip="formatMessage(messages.rollbackTooltip, { fileName: rollbackFileName })"
					:aria-label="formatMessage(messages.rollbackTooltip, { fileName: rollbackFileName })"
					:disabled="isDisabled"
					@click="emit('rollback')"
				>
					<UndoIcon class="size-5" />
				</button>
			</ButtonStyled>

			<!-- Fixed width container to reserve space for update/switch version button -->
			<div
				v-if="hasUpdateListener || hasSwitchVersionListener"
				class="flex w-8 items-center justify-center"
			>
				<ButtonStyled
					v-if="hasUpdate"
					circular
					type="transparent"
					color="green"
					color-fill="text"
					hover-color-fill="background"
				>
					<button
						v-tooltip="
							isDisabled && disabledTooltip
								? disabledTooltip
								: formatMessage(commonMessages.updateAvailableLabel)
						"
						:disabled="isDisabled"
						@click="emit('update')"
					>
						<DownloadIcon class="size-5" />
					</button>
				</ButtonStyled>
				<ButtonStyled
					v-else-if="hasSwitchVersionListener && version && !hideSwitchVersion"
					circular
					type="transparent"
				>
					<button
						v-tooltip="
							isDisabled && disabledTooltip
								? disabledTooltip
								: formatMessage(commonMessages.switchVersionButton)
						"
						:disabled="isDisabled"
						@click="emit('switchVersion')"
					>
						<ArrowLeftRightIcon class="size-5" />
					</button>
				</ButtonStyled>
			</div>

			<template v-for="action in inlineActions" :key="action.id">
				<ButtonStyled circular type="transparent">
					<button
						v-tooltip="action.label"
						:aria-label="action.label"
						:disabled="isDisabled"
						@click="action.action"
					>
						<component :is="action.icon" class="size-5" />
					</button>
				</ButtonStyled>
			</template>

			<Toggle
				v-if="enabled !== undefined"
				v-tooltip="
					isToggleDisabled && (toggleDisabledTooltip || disabledTooltip)
						? (toggleDisabledTooltip ?? disabledTooltip)
						: undefined
				"
				:model-value="enabled"
				:disabled="isToggleDisabled"
				:aria-label="project.title"
				class="my-auto"
				@update:model-value="(val) => emit('update:enabled', val as boolean)"
			/>

			<ButtonStyled v-if="hasDeleteListener && !props.hideDelete" circular type="transparent">
				<button
					v-tooltip="
						isDisabled && disabledTooltip
							? disabledTooltip
							: formatMessage(
									shiftHeld && deleteHovered
										? commonMessages.deleteImmediatelyLabel
										: commonMessages.deleteLabel,
								)
					"
					:disabled="isDisabled"
					@click="emit('delete', $event)"
					@mouseenter="deleteHovered = true"
					@mouseleave="deleteHovered = false"
				>
					<span class="relative size-5">
						<TrashIcon
							class="absolute inset-0 size-5 text-secondary transition-opacity duration-200"
							:class="shiftHeld && deleteHovered ? 'opacity-0' : 'opacity-100'"
						/>
						<TrashExclamationIcon
							class="absolute inset-0 size-5 text-red transition-opacity duration-200"
							:class="shiftHeld && deleteHovered ? 'opacity-100' : 'opacity-0'"
						/>
					</span>
				</button>
			</ButtonStyled>

			<slot name="additionalButtonsRight" />

			<ButtonStyled circular type="transparent">
				<TeleportOverflowMenu
					v-if="overflowOptions?.length"
					:options="overflowOptions"
					:disabled="isDisabled"
				>
					<MoreVerticalIcon class="size-5" />
				</TeleportOverflowMenu>
			</ButtonStyled>
		</div>
	</div>
</template>
