<template>
	<div class="flex items-center gap-1">
		<Button
			v-if="showClear && hasLogs"
			v-tooltip="clearDisabled ? clearDisabledTooltip : undefined"
			type="quiet"
			:disabled="clearDisabled"
			@click="emit('clear')"
			><XIcon />
			{{ formatMessage(commonMessages.clearButton) }}
		</Button>
		<ButtonStyled v-if="showDelete" type="transparent" hover-color-fill="background" color="red">
			<button
				v-tooltip="deleteDisabled ? deleteDisabledTooltip : undefined"
				:disabled="deleteDisabled"
				@click="emit('delete')"
			>
				<TrashIcon />
				{{ formatMessage(commonMessages.deleteLabel) }}
			</button>
		</ButtonStyled>
		<Button
			v-if="hasLogs"
			v-tooltip="shareDisabled ? shareDisabledTooltip : undefined"
			type="quiet"
			:disabled="shareDisabled || sharing"
			@click="emit('share')"
			><SpinnerIcon v-if="sharing" class="animate-spin" />
			<ShareIcon v-else />
			{{ formatMessage(messages.share) }}
		</Button>
		<Button type="quiet" @click="emit('toggle-fullscreen')"
			><ContractIcon v-if="fullscreen" />
			<ExpandIcon v-else />
			{{ formatMessage(fullscreen ? messages.collapse : messages.expand) }}
		</Button>
	</div>
</template>

<script setup lang="ts">
import {
	ContractIcon,
	ExpandIcon,
	ShareIcon,
	SpinnerIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'

import Button from '#ui/components/base/buttons/Button.vue'
import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

defineProps<{
	showClear?: boolean
	hasLogs?: boolean
	shareDisabled?: boolean
	shareDisabledTooltip?: string
	sharing?: boolean
	fullscreen?: boolean
	clearDisabled?: boolean
	clearDisabledTooltip?: string
	showDelete?: boolean
	deleteDisabled?: boolean
	deleteDisabledTooltip?: string
}>()

const emit = defineEmits<{
	clear: []
	share: []
	'toggle-fullscreen': []
	delete: []
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	share: { id: 'console.action.share', defaultMessage: 'Share' },
	expand: { id: 'console.action.expand', defaultMessage: 'Expand' },
	collapse: { id: 'console.action.collapse', defaultMessage: 'Collapse' },
})
</script>
