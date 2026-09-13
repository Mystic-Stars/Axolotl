<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'

defineOptions({ name: 'AxolotlNavWorlds' }) // KeepAlive cache key (App.vue CACHED_LEFT_NAV_PAGES)

const { formatMessage } = useVIntl()
const messages = defineMessages({
	worlds: { id: 'app.worlds.title', defaultMessage: 'Worlds' },
})
</script>
<template>
	<div class="p-6 flex flex-col gap-2">{{ formatMessage(messages.worlds) }}</div>
</template>
