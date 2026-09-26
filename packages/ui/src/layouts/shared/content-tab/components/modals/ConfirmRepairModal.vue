<template>
	<ConfirmActionModal
		ref="modal"
		:header="
			formatMessage(messages.header, {
				type: formatMessage(server ? messages.serverLabel : messages.instanceLabel),
			})
		"
		confirm-color="green"
		:confirm-icon="HammerIcon"
		:confirm-label="formatMessage(messages.repairButton)"
		@confirm="emit('repair')"
	>
		<SymlinkWarningAdmonition :symlink-target="symlinkTarget" />
		<span class="text-[var(--color-text-default)]">
			{{ formatMessage(server ? messages.serverBody : messages.instanceBody) }}
		</span>
	</ConfirmActionModal>
</template>

<script setup lang="ts">
import { HammerIcon } from '@modrinth/assets'
import { useTemplateRef } from 'vue'

import ConfirmActionModal from '#ui/components/modal/ConfirmActionModal.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

import SymlinkWarningAdmonition from './SymlinkWarningAdmonition.vue'

defineProps<{
	server?: boolean
	symlinkTarget?: string
}>()

const { formatMessage } = useVIntl()

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
})

const emit = defineEmits<{
	(e: 'repair'): void
}>()

const modal = useTemplateRef<InstanceType<typeof ConfirmActionModal>>('modal')

defineExpose({
	show: () => modal.value?.show(),
})
</script>
