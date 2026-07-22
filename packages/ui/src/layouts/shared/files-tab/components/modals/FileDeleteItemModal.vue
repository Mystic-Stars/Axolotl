<template>
	<NewModal ref="modal" fade="danger" :header="formatMessage(isBulk ? messages.bulkHeader : messages.header)" max-width="500px">
		<Admonition
			v-if="symlinkTarget"
			type="warning"
			:header="formatMessage(messages.symlinkWarningHeader)"
		>
			{{ formatMessage(messages.symlinkWarningBody, { path: symlinkTarget }) }}
		</Admonition>
		<Admonition type="critical" class="md:min-w-[400px]">
			<template #header>{{
				isBulk
					? formatMessage(messages.deletingMultiple, { count: bulkCount })
					: formatMessage(messages.deletingName, { name: item?.name })
			}}</template>
			{{
				isBulk
					? formatMessage(messages.bulkWarning)
					: formatMessage(
							item?.type === 'directory' ? messages.deleteFolderWarning : messages.deleteFileWarning,
						)
			}}
		</Admonition>
		<template #actions>
			<div class="flex gap-2 justify-end">
				<ButtonStyled type="outlined">
					<button @click="hide">
						<XIcon class="h-5 w-5" />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="red">
					<button @click="handleSubmit">
						<TrashIcon class="h-5 w-5" />
						{{ formatMessage(commonMessages.deleteLabel) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { TrashIcon, XIcon } from '@modrinth/assets'
import { ref } from 'vue'

import Admonition from '#ui/components/base/Admonition.vue'
import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

import type { FileItem } from '../../types'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: {
		id: 'files.delete-modal.header',
		defaultMessage: 'Delete file',
	},
	bulkHeader: {
		id: 'files.delete-modal.bulk-header',
		defaultMessage: 'Delete multiple items',
	},
	deletingName: {
		id: 'files.delete-modal.deleting-name',
		defaultMessage: 'Deleting "{name}"',
	},
	deletingMultiple: {
		id: 'files.delete-modal.deleting-multiple',
		defaultMessage: 'Deleting {count} items',
	},
	deleteFileWarning: {
		id: 'files.delete-modal.warning.file',
		defaultMessage: 'This file will be permanently deleted. This action cannot be undone.',
	},
	deleteFolderWarning: {
		id: 'files.delete-modal.warning.folder',
		defaultMessage:
			'This folder and all its contents will be permanently deleted. This action cannot be undone.',
	},
	bulkWarning: {
		id: 'files.delete-modal.bulk-warning',
		defaultMessage:
			'The selected items will be permanently deleted. This action cannot be undone.',
	},
	symlinkWarningHeader: {
		id: 'files.delete-modal.symlink-warning-header',
		defaultMessage: 'Shared instance',
	},
	symlinkWarningBody: {
		id: 'files.delete-modal.symlink-warning-body',
		defaultMessage:
			'You are modifying files in a shared instance linked to "{path}". Changes will affect the original instance.',
	},
})

const props = defineProps<{
	item: Pick<FileItem, 'name' | 'type'> | null
	symlinkTarget?: string | null
}>()

const emit = defineEmits<{
	delete: []
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const isBulk = ref(false)
const bulkCount = ref(0)

const handleSubmit = () => {
	emit('delete')
	hide()
}

const show = () => {
	isBulk.value = false
	bulkCount.value = 0
	modal.value?.show()
}

const showBulk = (count: number) => {
	isBulk.value = true
	bulkCount.value = count
	modal.value?.show()
}

const hide = () => {
	modal.value?.hide()
}

defineExpose({ show, showBulk, hide })
</script>
