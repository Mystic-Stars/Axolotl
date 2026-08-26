<script setup lang="ts">
import {
	ArrowDownAZIcon,
	ArrowUpZAIcon,
	CheckIcon,
	ClockArrowDownIcon,
	ClockArrowUpIcon,
	DownloadIcon,
	PinIcon,
	RotateCounterClockwiseIcon,
} from '@modrinth/assets'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import PopoutMenu from '#ui/components/base/PopoutMenu.vue'

import type { ContentSortMode } from '../composables'

export interface ContentSortOption {
	id: ContentSortMode
	label: string
}

const props = withDefaults(
	defineProps<{
		sortMode: ContentSortMode
		sortLabel: string
		sortOptions: ContentSortOption[]
		viewOptionsLabel: string
		pinned: boolean
		pinTooltip: string
		resetTooltip: string
		hasBulkUpdateSupport?: boolean
		hasOutdatedProjects?: boolean
		bulkUpdateTooltip?: string
		isBulkOperating?: boolean
	}>(),
	{
		hasBulkUpdateSupport: false,
		hasOutdatedProjects: false,
		bulkUpdateTooltip: undefined,
		isBulkOperating: false,
	},
)

const emit = defineEmits<{
	selectSort: [mode: ContentSortMode]
	togglePin: []
	resetView: []
	updateAll: []
}>()
</script>

<template>
	<div class="flex items-center justify-end gap-2">
		<PopoutMenu :tooltip="props.sortLabel" placement="bottom-end">
			<ButtonStyled circular type="transparent">
				<button :aria-label="props.sortLabel">
					<ArrowUpZAIcon
						v-if="props.sortMode === 'project-name-desc' || props.sortMode === 'file-name-desc'"
					/>
					<ClockArrowDownIcon v-else-if="props.sortMode === 'date-added-newest'" />
					<ClockArrowUpIcon v-else-if="props.sortMode === 'date-added-oldest'" />
					<ArrowDownAZIcon v-else />
				</button>
			</ButtonStyled>
			<template #menu>
				<div class="flex w-56 flex-col gap-1 p-1" role="menu" :aria-label="props.viewOptionsLabel">
					<ButtonStyled
						v-for="option in props.sortOptions"
						:key="option.id"
						:type="props.sortMode === option.id ? 'filled' : 'transparent'"
					>
						<button
							class="flex w-full items-center gap-2 !justify-start text-left"
							role="menuitemradio"
							:aria-checked="props.sortMode === option.id"
							@click="emit('selectSort', option.id)"
						>
							<CheckIcon
								class="size-4 shrink-0"
								:class="props.sortMode === option.id ? 'opacity-100' : 'opacity-0'"
							/>
							<span>{{ option.label }}</span>
						</button>
					</ButtonStyled>
					<div class="my-1 h-px bg-surface-5" />
					<ButtonStyled type="transparent">
						<button
							class="flex w-full items-center gap-2 !justify-start text-left"
							@click="emit('resetView')"
						>
							<RotateCounterClockwiseIcon class="size-4" />
							<span>{{ props.resetTooltip }}</span>
						</button>
					</ButtonStyled>
				</div>
			</template>
		</PopoutMenu>

		<ButtonStyled
			circular
			:type="props.pinned ? 'chip' : 'transparent'"
			:color="props.pinned ? 'brand' : 'standard'"
		>
			<button
				v-tooltip="props.pinTooltip"
				:aria-label="props.pinTooltip"
				@click="emit('togglePin')"
			>
				<PinIcon />
			</button>
		</ButtonStyled>

		<ButtonStyled
			v-if="props.hasBulkUpdateSupport && props.hasOutdatedProjects"
			circular
			color="green"
			type="transparent"
			color-fill="text"
			hover-color-fill="background"
		>
			<button
				v-tooltip="props.bulkUpdateTooltip"
				:disabled="props.isBulkOperating"
				@click="emit('updateAll')"
			>
				<DownloadIcon />
			</button>
		</ButtonStyled>
	</div>
</template>
