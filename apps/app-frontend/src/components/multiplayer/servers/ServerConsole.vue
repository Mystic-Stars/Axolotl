<script setup lang="ts">
import {
	ConsolePageLayout,
	createConsoleState,
	defineMessages,
	provideConsoleManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'

import { hydrateLog, type ServerView, useServers } from '@/composables/useServers'
import { servers } from '@/helpers/servers'

const props = defineProps<{
	server: ServerView
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	notRunning: {
		id: 'app.servers.console.not-running',
		defaultMessage: 'The server is not running',
	},
	commandEcho: {
		id: 'app.servers.console.command-echo',
		defaultMessage: '> {command}',
	},
})

const { logLines, sendCommand } = useServers()
const consoleState = createConsoleState()
const loading = ref(true)
let consumedLines = 0

onMounted(async () => {
	await hydrateLog(props.server.id)
	const buffer = logLines[props.server.id] ?? []
	if (buffer.length > 0) await consoleState.addLegacyLog(buffer.join('\n'))
	consumedLines = buffer.length
	loading.value = false
})

watch(
	() => (logLines[props.server.id] ?? []).length,
	(count) => {
		if (loading.value) return
		const lines = logLines[props.server.id] ?? []
		if (count < consumedLines) {
			consoleState.clear()
			consumedLines = 0
		}
		const fresh = lines.slice(consumedLines)
		consumedLines = lines.length
		if (fresh.length === 0) return
		for (const line of fresh) {
			void consoleState.addLegacyLog(line)
		}
	},
)

async function handleSendCommand(command: string) {
	const sent = await sendCommand(props.server.id, command)
	if (sent) void consoleState.addLegacyLog(formatMessage(messages.commandEcho, { command }))
}

provideConsoleManager({
	logLines: consoleState.output,
	sendCommand: (command: string) => void handleSendCommand(command),
	showCommandInput: computed(() => props.server.running),
	disableCommandInput: computed(() => !props.server.running),
	disableCommandInputTooltip: computed(() => formatMessage(messages.notRunning)),
	loading,
	emptyStateType: 'server',
	onClear: () => {
		consoleState.clear()
		consumedLines = 0
		// Drop the shared frontend buffer too, otherwise the next incoming log
		// line replays the entire pre-clear history back into the console.
		logLines[props.server.id] = []
		void servers.clearLog(props.server.id).catch(() => {})
	},
})
</script>

<template>
	<div data-onboarding-id="server-console" class="flex h-full min-h-0 flex-col">
		<ConsolePageLayout />
	</div>
</template>
