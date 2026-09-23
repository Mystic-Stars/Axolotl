<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" fade="warning" max-width="500px">
		<div class="flex flex-col gap-6">
			<SymlinkWarningAdmonition :symlink-target="symlinkTarget" />
			<Admonition type="warning" :header="formatMessage(messages.admonitionHeader)">
				{{
					props.scopeDescription ?? formatMessage(messages.admonitionBody, { count: visibleCount })
				}}
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
					><DownloadIcon />
					{{ props.actionLabel ?? formatMessage(messages.updateButton, { count: visibleCount }) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { DownloadIcon, XIcon } from '@modrinth/assets'
import { nextTick, ref } from 'vue'

import Admonition from '#ui/components/base/Admonition.vue'
import Button from '#ui/components/base/buttons/Button.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

import SymlinkWarningAdmonition from './SymlinkWarningAdmonition.vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: {
		id: 'content.confirm-bulk-update.header',
		defaultMessage: 'Update projects',
	},
	admonitionHeader: {
		id: 'content.confirm-bulk-update.admonition-header',
		defaultMessage: 'Update warning',
	},
	admonitionBody: {
		id: 'content.confirm-bulk-update.admonition-body',
		defaultMessage:
			"Are you sure you want to update {count, plural, one {# project} other {# projects}} to their latest compatible version? It's recommended to update content one-by-one.",
	},
	updateButton: {
		id: 'content.confirm-bulk-update.update-button',
		defaultMessage: 'Update {count, plural, one {# project} other {# projects}}',
	},
})

const props = defineProps<{
	count: number
	server?: boolean
	backupTip?: string
	actionDisabled?: boolean
	actionDisabledTooltip?: string
	symlinkTarget?: string
	actionLabel?: string
	scopeDescription?: string
}>()

const emit = defineEmits<{
	(e: 'update'): void
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const visibleCount = ref(props.count)

async function show() {
	await nextTick()
	visibleCount.value = props.count
	modal.value?.show()
}

function confirm() {
	if (props.actionDisabled) return
	modal.value?.hide()
	emit('update')
}

defineExpose({
	show,
})
</script>
