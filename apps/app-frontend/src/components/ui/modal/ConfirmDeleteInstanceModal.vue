<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" fade="danger" max-width="500px">
		<Admonition
			v-if="!symlinkTarget"
			type="critical"
			:header="formatMessage(messages.admonitionHeader)"
		>
			{{ formatMessage(messages.admonitionBody) }}
		</Admonition>
		<Admonition v-else type="critical">
			{{ formatMessage(messages.symlinkDeleteWarning, { path: symlinkTarget }) }}
		</Admonition>

		<template #actions>
			<div class="flex gap-2 justify-end">
				<ButtonStyled type="outlined">
					<button @click="modal?.hide()">
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="red">
					<button @click="confirm">
						<TrashIcon />
						{{ formatMessage(messages.deleteButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { TrashIcon, XIcon } from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	commonMessages,
	defineMessages,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

const { formatMessage } = useVIntl()

const props = defineProps<{
	symlinkTarget?: string | null
}>()

const messages = defineMessages({
	header: {
		id: 'app.instance.confirm-delete.header',
		defaultMessage: 'Delete instance',
	},
	admonitionHeader: {
		id: 'app.instance.confirm-delete.admonition-header',
		defaultMessage: 'This action cannot be undone',
	},
	admonitionBody: {
		id: 'app.instance.confirm-delete.admonition-body',
		defaultMessage:
			'All data for your instance will be permanently deleted, including your worlds, configs, and all installed content.',
	},
	symlinkDeleteWarning: {
		id: 'app.instance.confirm-delete.symlink-warning',
		defaultMessage:
			'This is a shared instance linked to "{path}". Only the link will be removed; the original files will not be deleted.',
	},
	deleteButton: {
		id: 'app.instance.confirm-delete.delete-button',
		defaultMessage: 'Delete instance',
	},
})

const emit = defineEmits<{
	(e: 'delete'): void
}>()

const modal = ref<InstanceType<typeof NewModal>>()

function show() {
	modal.value?.show()
}

function confirm() {
	modal.value?.hide()
	emit('delete')
}

defineExpose({
	show,
})
</script>
