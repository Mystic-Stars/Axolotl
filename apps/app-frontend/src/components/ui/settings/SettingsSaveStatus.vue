<script setup lang="ts">
import { SpinnerIcon } from '@modrinth/assets'
import { defineMessages, NewButton as Button, useVIntl } from '@modrinth/ui'

const props = withDefaults(
	defineProps<{
		status?: 'idle' | 'saving' | 'saved' | 'error'
		retry?: (() => void) | undefined
	}>(),
	{
		status: 'idle',
		retry: undefined,
	},
)

const { formatMessage } = useVIntl()
const messages = defineMessages({
	saving: { id: 'app.settings.save-status.saving', defaultMessage: 'Saving…' },
	saved: { id: 'app.settings.save-status.saved', defaultMessage: 'Saved' },
	error: { id: 'app.settings.save-status.error', defaultMessage: 'Could not save' },
	retry: { id: 'app.settings.save-status.retry', defaultMessage: 'Retry' },
})

const statusMessage = {
	saving: messages.saving,
	saved: messages.saved,
	error: messages.error,
} as const
</script>

<template>
	<div
		v-if="props.status !== 'idle'"
		class="settings-save-status inline-flex items-center gap-1 text-xs text-secondary"
		role="status"
	>
		<SpinnerIcon v-if="props.status === 'saving'" class="size-3.5 animate-spin" />
		<span>{{ props.status === 'idle' ? '' : formatMessage(statusMessage[props.status]) }}</span>
		<Button v-if="props.status === 'error' && props.retry" type="quiet" @click="props.retry">
			{{ formatMessage(messages.retry) }}
		</Button>
	</div>
</template>

<style scoped>
.settings-save-status :deep(.btn) {
	padding: 0.25rem 0.5rem;
	font-size: 0.75rem;
}
</style>
