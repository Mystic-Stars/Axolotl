<template>
	<NewModal
		ref="modal"
		:noblur="noblur"
		:fade="danger ? 'danger' : 'standard'"
		:on-hide="onHide"
		max-width="550px"
	>
		<template #title>
			<slot name="title">
				<span class="font-extrabold text-[var(--color-text-primary)] text-lg">{{
					title || formatMessage(messages.noTitle)
				}}</span>
			</slot>
		</template>
		<div class="flex flex-col gap-4">
			<template v-if="description">
				<div
					v-if="markdown"
					class="markdown-body max-w-[35rem]"
					v-html="renderString(description)"
				/>
				<p v-else class="max-w-[35rem] m-0">
					{{ description }}
				</p>
			</template>
			<slot />
			<label v-if="hasToType" for="confirmation">
				<span>
					<IntlFormatted :message-id="messages.confirmationPrompt" :values="{ confirmationText }">
						<template #confirmation-text="{ children }">
							<span class="italic font-bold"><component :is="() => children" /></span>
						</template>
					</IntlFormatted>
				</span>
			</label>
			<StyledInput
				v-if="hasToType"
				id="confirmation"
				v-model="confirmation_typed"
				:placeholder="formatMessage(messages.confirmationPlaceholder)"
				wrapper-class="max-w-[20rem]"
			/>
			<div class="flex gap-2 justify-end">
				<Button class="!shadow-none" @click="hide()"
					><XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<ButtonStyled :color="danger ? 'red' : 'brand'">
					<button :disabled="action_disabled" @click="proceed">
						<component :is="proceedIcon" />
						{{ proceedLabel || formatMessage(messages.proceed) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>

<script setup>
import { TrashIcon, XIcon } from '@modrinth/assets'
import { renderString } from '@modrinth/utils'
import { computed, ref } from 'vue'

import { defineMessages, useVIntl } from '../../composables/i18n'
import { commonMessages } from '../../utils/common-messages'
import Button from '../base/buttons/Button.vue'
import ButtonStyled from '../base/ButtonStyled.vue'
import IntlFormatted from '../base/IntlFormatted.vue'
import StyledInput from '../base/StyledInput.vue'
import NewModal from './NewModal.vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	confirmationPrompt: {
		id: 'modal.confirm.confirmation-prompt',
		defaultMessage:
			'To confirm you want to proceed, type <confirmation-text>{confirmationText}</confirmation-text> below:',
	},
	confirmationPlaceholder: {
		id: 'modal.confirm.confirmation-placeholder',
		defaultMessage: 'Type here...',
	},
	noTitle: { id: 'modal.confirm.no-title', defaultMessage: 'No title defined' },
	proceed: { id: 'modal.confirm.proceed', defaultMessage: 'Proceed' },
})

const props = defineProps({
	confirmationText: {
		type: String,
		default: '',
	},
	hasToType: {
		type: Boolean,
		default: false,
	},
	title: {
		type: String,
		default: undefined,
		required: true,
	},
	description: {
		type: String,
		default: undefined,
		required: false,
	},
	proceedIcon: {
		type: Object,
		default: () => TrashIcon,
	},
	proceedLabel: {
		type: String,
		default: undefined,
	},
	noblur: {
		type: Boolean,
		default: false,
	},
	danger: {
		type: Boolean,
		default: true,
	},
	onHide: {
		type: Function,
		default() {
			return () => {}
		},
	},
	markdown: {
		type: Boolean,
		default: true,
	},
})

const emit = defineEmits(['proceed'])
const modal = ref(null)

const confirmation_typed = ref('')

const action_disabled = computed(
	() =>
		props.hasToType &&
		confirmation_typed.value.toLowerCase() !== props.confirmationText.toLowerCase(),
)

function proceed() {
	modal.value.hide()
	confirmation_typed.value = ''
	emit('proceed')
}

function show() {
	modal.value.show()
}
function hide() {
	modal.value.hide()
}

defineExpose({ show, hide })
</script>
