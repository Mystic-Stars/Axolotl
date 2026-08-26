<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onMounted, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import ServerPropertiesEditor from '@/components/multiplayer/servers/ServerPropertiesEditor.vue'

import { injectCreateServerFlow } from '../create-server-flow'

const { formatMessage } = useVIntl()
const ctx = injectCreateServerFlow()

const messages = defineMessages({
	heading: {
		id: 'app.servers.wizard.configure-heading',
		defaultMessage: 'Adjust the server settings, or finish to edit them later.',
	},
})

const editor = useTemplateRef<ComponentExposed<typeof ServerPropertiesEditor>>('editor')

onMounted(() => {
	ctx.saveServerProperties.value = () => editor.value?.save() ?? Promise.resolve(true)
})

const serverId = computed(() => ctx.createdServer.value?.id ?? '')
</script>

<template>
	<div class="flex flex-col gap-4">
		<p class="m-0 text-sm text-secondary">
			{{ formatMessage(messages.heading) }}
		</p>

		<div class="max-h-[32rem] overflow-y-auto pr-2">
			<ServerPropertiesEditor v-if="serverId !== ''" ref="editor" :server-id="serverId" />
		</div>
	</div>
</template>
