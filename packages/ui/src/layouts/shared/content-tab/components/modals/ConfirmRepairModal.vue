<template>
	<NewModal
		ref="modal"
		:header="
			formatMessage(messages.header, {
				type: formatMessage(server ? messages.serverLabel : messages.instanceLabel),
			})
		"
		max-width="500px"
	>
		<Admonition
			v-if="symlinkTarget"
			type="warning"
			:header="formatMessage(messages.symlinkWarningHeader)"
		>
			{{ formatMessage(messages.symlinkWarningBody, { path: symlinkTarget }) }}
		</Admonition>
		<span class="text-primary">
			{{ formatMessage(server ? messages.serverBody : messages.instanceBody) }}
		</span>

		<template #actions>
			<div class="flex gap-2 justify-end">
				<ButtonStyled type="outlined">
					<button @click="modal?.hide()">
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="green">
					<button @click="confirm">
						<HammerIcon />
						{{ formatMessage(messages.repairButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { HammerIcon, XIcon } from '@modrinth/assets'
import { ref } from 'vue'

import Admonition from '#ui/components/base/Admonition.vue'
import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { useDebugLogger } from '#ui/composables/debug-logger'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

defineProps<{
	server?: boolean
	symlinkTarget?: string
}>()

const { formatMessage } = useVIntl()
const debug = useDebugLogger('ConfirmRepairModal')

const messages = defineMessages({
	header: {
		id: 'instance.confirm-repair.header',
		defaultMessage: 'Repair {type}',
	},
	instanceBody: {
		id: 'instance.confirm-repair.body.instance',
		defaultMessage:
			'Repairing reinstalls the loader and Minecraft dependencies without deleting your content. This may resolve issues if your game is not launching due to launcher-related errors.',
	},
	serverBody: {
		id: 'instance.confirm-repair.body.server',
		defaultMessage:
			'Repairing reinstalls the loader and Minecraft dependencies without deleting your content. This may resolve issues if your server is not starting correctly.',
	},
	repairButton: {
		id: 'instance.confirm-repair.repair-button',
		defaultMessage: 'Repair',
	},
	instanceLabel: {
		id: 'instance.confirm-repair.instance-label',
		defaultMessage: 'instance',
	},
	serverLabel: {
		id: 'instance.confirm-repair.server-label',
		defaultMessage: 'server',
	},
	symlinkWarningHeader: { id: 'app.symlink-warning.write.header' },
	symlinkWarningBody: { id: 'app.symlink-warning.write.body' },
})

const emit = defineEmits<{
	(e: 'repair'): void
}>()

const modal = ref<InstanceType<typeof NewModal>>()

function show() {
	debug('show: called', { hasModalRef: !!modal.value })
	modal.value?.show()
	debug('show: returned from modal.show', { hasModalRef: !!modal.value })
}

function confirm() {
	debug('confirm: called', { hasModalRef: !!modal.value })
	modal.value?.hide()
	emit('repair')
	debug('confirm: emitted repair')
}

defineExpose({
	show,
})
</script>
