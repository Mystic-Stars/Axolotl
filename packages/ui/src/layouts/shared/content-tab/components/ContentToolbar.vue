<script setup lang="ts">
import { CompassIcon, GitGraphIcon, RefreshCwIcon, SearchIcon } from '@modrinth/assets'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import StyledInput from '#ui/components/base/StyledInput.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages, formatContentTypeSentence } from '#ui/utils/common-messages'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	searchPlaceholder: {
		id: 'content.page-layout.search-placeholder',
		defaultMessage: 'Search {count, number} {contentType}...',
	},
	browseContent: {
		id: 'content.page-layout.browse-content',
		defaultMessage: 'Browse content',
	},
	viewDependencies: {
		id: 'content.page-layout.view-dependencies',
		defaultMessage: 'View dependencies',
	},
})

const searchQuery = defineModel<string>('searchQuery', { required: true })

const props = withDefaults(
	defineProps<{
		searchableItemCount: number
		contentTypeLabel: string
		busy?: boolean
		busyTooltip?: string | null
		disableAddContent?: boolean
		disableAddContentTooltip?: string
		refreshing?: boolean
		viewDependencies?: boolean
	}>(),
	{
		busy: false,
		busyTooltip: null,
		disableAddContent: false,
		disableAddContentTooltip: undefined,
		refreshing: false,
		viewDependencies: false,
	},
)

const emit = defineEmits<{
	browse: []
	refresh: []
	viewDependencies: []
}>()
</script>

<template>
	<div class="flex flex-wrap items-center gap-2">
		<StyledInput
			v-model="searchQuery"
			:icon="SearchIcon"
			type="text"
			autocomplete="off"
			:spellcheck="false"
			input-class="!h-10"
			wrapper-class="flex-1 min-w-0"
			clearable
			:placeholder="
				formatMessage(messages.searchPlaceholder, {
					count: props.searchableItemCount,
					contentType: formatContentTypeSentence(
						formatMessage,
						props.contentTypeLabel,
						props.searchableItemCount,
					),
				})
			"
		/>

		<div class="flex items-center gap-2">
			<ButtonStyled color="brand">
				<button
					v-tooltip="
						props.busyTooltip ??
						(props.disableAddContent ? props.disableAddContentTooltip : undefined)
					"
					:disabled="props.busy || props.disableAddContent"
					class="!h-10 flex items-center gap-2"
					@click="emit('browse')"
				>
					<CompassIcon class="size-5" />
					<span>{{ formatMessage(messages.browseContent) }}</span>
				</button>
			</ButtonStyled>
			<ButtonStyled v-if="props.viewDependencies" type="outlined">
				<button
					v-tooltip="formatMessage(messages.viewDependencies)"
					:disabled="props.busy"
					class="!h-10 flex items-center gap-2"
					@click="emit('viewDependencies')"
				>
					<GitGraphIcon class="size-5" />
					<span>{{ formatMessage(messages.viewDependencies) }}</span>
				</button>
			</ButtonStyled>
			<ButtonStyled type="outlined">
				<button
					v-tooltip="props.busyTooltip"
					:disabled="props.refreshing"
					class="!h-10"
					@click="emit('refresh')"
				>
					<RefreshCwIcon :class="['size-5', { 'animate-spin': props.refreshing }]" />
					{{ formatMessage(commonMessages.refreshButton) }}
				</button>
			</ButtonStyled>
		</div>
	</div>
</template>
