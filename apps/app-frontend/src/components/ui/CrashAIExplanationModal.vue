<script setup lang="ts">
import {
	Admonition,
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { renderHighlightedString } from '@modrinth/utils/highlightjs'
import { computed, ref } from 'vue'

import { explain_crash_with_ai } from '@/helpers/logs'

const modal = ref<InstanceType<typeof NewModal>>()
const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const loading = ref(false)
const output = ref('')
const errorMessage = ref('')

const messages = defineMessages({
	title: { id: 'app.crash-analysis.ai.title', defaultMessage: 'AI crash explanation' },
	disclaimer: {
		id: 'app.crash-analysis.ai.disclaimer',
		defaultMessage:
			'A sanitized and shortened crash context is sent directly to the AI provider configured in this launcher. AI output may be inaccurate.',
	},
	analyzing: { id: 'app.crash-analysis.ai.analyzing', defaultMessage: 'Explaining the crash...' },
	error: { id: 'app.crash-analysis.ai.error', defaultMessage: 'AI explanation failed: {message}' },
	copy: { id: 'app.crash-analysis.ai.copy', defaultMessage: 'Copy explanation' },
	copied: {
		id: 'app.crash-analysis.ai.copied',
		defaultMessage: 'AI explanation copied to your clipboard',
	},
	close: { id: 'app.crash-analysis.ai.close', defaultMessage: 'Close' },
})

const renderedOutput = computed(() => renderHighlightedString(output.value))

async function show(instanceId: string): Promise<void> {
	output.value = ''
	errorMessage.value = ''
	loading.value = true
	modal.value?.show()
	try {
		const result = await explain_crash_with_ai(instanceId)
		output.value = result.content
	} catch (error) {
		errorMessage.value = error instanceof Error ? error.message : String(error)
	} finally {
		loading.value = false
	}
}

async function copy(): Promise<void> {
	try {
		await navigator.clipboard.writeText(output.value)
		addNotification({ title: formatMessage(messages.copied), type: 'success' })
	} catch (error) {
		errorMessage.value = error instanceof Error ? error.message : String(error)
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)" max-width="720px">
		<div class="flex flex-col gap-4">
			<Admonition type="warning" :header="formatMessage(messages.title)">
				{{ formatMessage(messages.disclaimer) }}
			</Admonition>
			<div v-if="loading" class="text-secondary">{{ formatMessage(messages.analyzing) }}</div>
			<div v-else-if="errorMessage" class="rounded-lg bg-red-500/10 p-3 text-secondary">
				{{ formatMessage(messages.error, { message: errorMessage }) }}
			</div>
			<div
				v-else-if="output"
				class="markdown-body max-h-[55vh] overflow-y-auto rounded-lg bg-surface-2 p-4"
				v-html="renderedOutput"
			/>
		</div>
		<template #actions>
			<div class="flex flex-wrap justify-end gap-2">
				<ButtonStyled v-if="output" type="outlined">
					<button @click="copy">{{ formatMessage(messages.copy) }}</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="loading" @click="modal?.hide()">
						{{ formatMessage(messages.close) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
