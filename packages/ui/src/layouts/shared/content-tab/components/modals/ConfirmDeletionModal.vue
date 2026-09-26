<template>
	<NewModal
		ref="modal"
		:header="
			formatMessage(messages.header, {
				itemType: formatContentTypeSentence(formatMessage, visibleItemType, visibleCount),
			})
		"
		fade="warning"
		max-width="500px"
	>
		<div class="flex flex-col gap-6">
			<SymlinkWarningAdmonition :symlink-target="symlinkTarget" />
			<Admonition type="warning" :header="formatMessage(messages.admonitionHeader)">
				{{ formatMessage(messages.admonitionBody) }}
			</Admonition>
		</div>

		<template #actions>
			<div class="flex gap-2 justify-end">
				<Button type="outlined" @click="modal?.hide()"
					><XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button
					v-tooltip="props.actionDisabled ? props.actionDisabledTooltip : undefined"
					type="colored"
					color="orange"
					:disabled="props.actionDisabled"
					@click="confirm"
					><TrashIcon />
					{{
						formatMessage(messages.deleteButton, {
							count: visibleCount,
							itemType: formatContentTypeSentence(formatMessage, visibleItemType, visibleCount),
						})
					}}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { TrashIcon, XIcon } from '@modrinth/assets'
import { nextTick, ref } from 'vue'

import Admonition from '#ui/components/base/Admonition.vue'
import Button from '#ui/components/base/buttons/Button.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages, formatContentTypeSentence } from '#ui/utils/common-messages'

import SymlinkWarningAdmonition from './SymlinkWarningAdmonition.vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: {
		id: 'content.confirm-deletion.header',
		defaultMessage: 'Delete {itemType}',
	},
	admonitionHeader: {
		id: 'content.confirm-deletion.admonition-header',
		defaultMessage: 'Deletion warning',
	},
	admonitionBody: {
		id: 'content.confirm-deletion.admonition-body',
		defaultMessage:
			'Deleting a mod can permanently affect your world and may cause missing content or unexpected issues when it loads again.',
	},
	deleteButton: {
		id: 'content.confirm-deletion.delete-button',
		defaultMessage: 'Delete {count, number} {itemType}',
	},
})

const props = withDefaults(
	defineProps<{
		count: number
		itemType: string
		variant?: 'instance' | 'server'
		backupTip?: string
		actionDisabled?: boolean
		actionDisabledTooltip?: string
		symlinkTarget?: string
	}>(),
	{
		variant: 'instance',
		backupTip: undefined,
		actionDisabled: false,
		actionDisabledTooltip: undefined,
		symlinkTarget: undefined,
	},
)

const emit = defineEmits<{
	(e: 'delete'): void
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const visibleCount = ref(props.count)
const visibleItemType = ref(props.itemType)

async function show() {
	await nextTick()
	visibleCount.value = props.count
	visibleItemType.value = props.itemType
	modal.value?.show()
}

function confirm() {
	if (props.actionDisabled) return
	modal.value?.hide()
	emit('delete')
}

defineExpose({
	show,
})
</script>
