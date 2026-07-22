<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { Admonition } from '@modrinth/ui'

const props = defineProps<{
	symlinkTarget?: string | null
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	symlinkWarningHeader: {
		id: 'app.symlink-warning.header',
	},
	symlinkWarningBody: {
		id: 'app.symlink-warning.body',
	},
})
</script>

<template>
	<Admonition
		v-if="symlinkTarget"
		type="warning"
		:header="formatMessage(messages.symlinkWarningHeader)"
	>
		{{ formatMessage(messages.symlinkWarningBody, { path: symlinkTarget }) }}
	</Admonition>
</template>
