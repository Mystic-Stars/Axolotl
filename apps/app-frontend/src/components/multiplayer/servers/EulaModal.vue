<script setup lang="ts">
import { CheckIcon, XIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

const props = defineProps<{
	text: string
}>()

const emit = defineEmits<{
	accept: []
	decline: []
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	title: { id: 'app.servers.eula.title', defaultMessage: 'Minecraft EULA' },
	description: {
		id: 'app.servers.eula.description',
		defaultMessage:
			'The server must accept the Minecraft End User License Agreement before it can start. Review the notice below:',
	},
	accept: {
		id: 'app.servers.eula.accept',
		defaultMessage: 'Accept and continue',
	},
	decline: { id: 'app.servers.eula.decline', defaultMessage: 'Cancel' },
})

const modal = useTemplateRef<ComponentExposed<typeof NewModal>>('modal')

defineExpose({
	show: (event?: MouseEvent) => modal.value?.show(event),
	hide: () => modal.value?.hide(),
})
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)">
		<div class="flex flex-col gap-4">
			<p class="m-0 text-secondary">
				{{ formatMessage(messages.description) }}
			</p>
			<pre
				class="max-h-48 overflow-y-auto whitespace-pre-wrap rounded-xl bg-surface-2 p-4 font-mono text-sm text-contrast"
				>{{ props.text }}</pre
			>
		</div>
		<template #actions>
			<div class="flex flex-col justify-end gap-2 sm:flex-row">
				<ButtonStyled type="outlined">
					<button type="button" @click="emit('decline')">
						<XIcon />
						{{ formatMessage(messages.decline) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button type="button" @click="emit('accept')">
						<CheckIcon />
						{{ formatMessage(messages.accept) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
