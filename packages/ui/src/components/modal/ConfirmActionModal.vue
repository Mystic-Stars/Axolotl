<template>
	<NewModal ref="modal" :header="header" :fade="fade" :max-width="maxWidth">
		<div class="flex flex-col gap-6">
			<slot name="warning" />
			<slot />
		</div>

		<template #actions>
			<div class="flex gap-2 justify-end">
				<Button type="outlined" @click="modal?.hide()"
					><XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<ButtonStyled :color="confirmColor">
					<button
						v-tooltip="confirmDisabled ? confirmDisabledTooltip : undefined"
						:disabled="confirmDisabled"
						@click="onConfirm"
					>
						<component :is="confirmIcon" />
						{{ confirmLabel }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { XIcon } from '@modrinth/assets'
import { ref } from 'vue'

import { useVIntl } from '../../composables/i18n'
import { commonMessages } from '../../utils/common-messages'
import Button from '../base/buttons/Button.vue'
import ButtonStyled from '../base/ButtonStyled.vue'
import NewModal from './NewModal.vue'

const props = withDefaults(
	defineProps<{
		header: string
		fade?: 'standard' | 'danger' | 'warning'
		maxWidth?: string
		confirmColor?: 'red' | 'green' | 'orange' | 'brand' | 'standard'
		confirmIcon?: object
		confirmLabel: string
		confirmDisabled?: boolean
		confirmDisabledTooltip?: string
	}>(),
	{
		fade: 'standard',
		maxWidth: '500px',
		confirmColor: 'brand',
		confirmIcon: undefined,
		confirmDisabled: false,
		confirmDisabledTooltip: undefined,
	},
)

const emit = defineEmits<{
	(e: 'confirm'): void
}>()

const { formatMessage } = useVIntl()
const modal = ref<InstanceType<typeof NewModal>>()

function show() {
	modal.value?.show()
}

function onConfirm() {
	if (props.confirmDisabled) return
	modal.value?.hide()
	emit('confirm')
}

defineExpose({ show })
</script>
