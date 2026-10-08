<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'

const { formatMessage } = useVIntl()
const messages = defineMessages({
    worlds: { id: 'app.worlds.title', defaultMessage: 'Worlds' },
})
</script>
<template>
    <div class="p-6 flex flex-col gap-2">{{ formatMessage(messages.worlds) }}</div>
</template>
