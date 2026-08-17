<template>
	<NewModal
		ref="modal"
		:header="formatMessage(count > 1 ? messages.batchHeader : messages.header)"
		fade="danger"
		max-width="500px"
	>
		<Admonition
			v-if="!symlinkTarget && count <= 1"
			type="critical"
			:header="formatMessage(messages.admonitionHeader)"
		>
			{{ formatMessage(messages.admonitionBody) }}
		</Admonition>
		<Admonition
			v-else-if="!symlinkTarget"
			type="critical"
			:header="formatMessage(messages.admonitionHeader)"
		>
			{{ formatMessage(messages.batchAdmonitionBody, { count }) }}
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
						{{
							formatMessage(count > 1 ? messages.batchDeleteButton : messages.deleteButton, {
								count,
							})
						}}
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

withDefaults(
	defineProps<{
		symlinkTarget?: string | null
		count?: number
	}>(),
	{
		count: 1,
	},
)

const messages = defineMessages({
	header: {
		id: 'app.instance.confirm-delete.header',
		defaultMessage: 'Delete instance',
	},
	batchHeader: {
		id: 'app.instance.confirm-delete.batch-header',
		defaultMessage: 'Delete instances',
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
	batchAdmonitionBody: {
		id: 'app.instance.confirm-delete.batch-admonition-body',
		defaultMessage:
			'{count, plural, one {# instance} other {# instances}} will be permanently deleted, including worlds, configs, and all installed content.',
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
	batchDeleteButton: {
		id: 'app.instance.confirm-delete.batch-delete-button',
		defaultMessage: 'Delete {count, plural, one {# instance} other {# instances}}',
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
