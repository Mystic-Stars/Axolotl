<script setup lang="ts">
import {
    ConsolePageLayout,
    createConsoleState,
    defineMessages,
    JLineCommandInput,
    provideConsoleManager,
    useVIntl,
} from '@modrinth/ui'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import { ServerConsoleBuffer } from '@/composables/server-console-buffer'
import {
    hydrateLog,
    type ServerView,
    subscribeServerConsoleOutput,
    useServers,
} from '@/composables/useServers'
import { servers } from '@/helpers/servers'

const props = defineProps<{
    server: ServerView
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
    forgeCommandPlaceholder: {
        id: 'app.servers.console.forge-command-placeholder',
        defaultMessage: 'Send a command - Tab completion supported',
    },
    notRunning: {
        id: 'app.servers.console.not-running',
        defaultMessage: 'The server is not running',
    },
})

const { logLines, sendCommand } = useServers()
const consoleState = createConsoleState()
const loading = ref(true)
const hasLogs = computed(() => consoleState.output.value.length > 0)
const isForge = computed(() => props.server.serverType === 'forge')
const jlineInput = ref<InstanceType<typeof JLineCommandInput> | null>(null)
let consumedLines = 0
// Guards the live length-watcher from double-appending while we rebuild the
// console from the buffer during (re)hydration. Without it, the async
// hydrate fetch and the streamed `logLines` updates race, dropping or
// duplicating the earliest startup lines.
let hydrating = false
let unsubscribeConsoleOutput: (() => void) | null = null
const PENDING_CONSOLE_OUTPUT_CAPACITY = 64 * 1024
let pendingConsoleOutput = new ServerConsoleBuffer(PENDING_CONSOLE_OUTPUT_CAPACITY)

function flushConsoleOutput() {
    for (const data of pendingConsoleOutput.values()) jlineInput.value?.write(data)
    pendingConsoleOutput = new ServerConsoleBuffer(PENDING_CONSOLE_OUTPUT_CAPACITY)
}

watch(
    () => props.server.id,
    (serverId) => {
        unsubscribeConsoleOutput?.()
        unsubscribeConsoleOutput = subscribeServerConsoleOutput(serverId, (data) => {
            if (jlineInput.value) {
                jlineInput.value.write(data)
            } else {
                pendingConsoleOutput.push(data)
            }
        })
    },
    { immediate: true },
)

async function hydrateAndDisplay() {
    hydrating = true
    try {
        await hydrateLog(props.server.id)
        const buffer = logLines[props.server.id] ?? []
        if (buffer.length > 0) await consoleState.addLegacyLog(buffer.join('\n'))
        consumedLines = buffer.length
    } finally {
        hydrating = false
    }
}

// The per-line `server` events can drop during heavy bursts, so we
// periodically reconcile the displayed log against the lossless backend
// buffer. This guarantees the console always shows the complete history,
// including the server's startup and command responses that arrived in a
// single fast burst.
let syncTimer: ReturnType<typeof setInterval> | null = null
function startSync() {
    stopSync()
    syncTimer = setInterval(() => {
        if (hydrating) return
        if (!props.server.running) {
            stopSync()
            return
        }
        void hydrateLog(props.server.id)
    }, 1000)
}
function stopSync() {
    if (syncTimer) {
        clearInterval(syncTimer)
        syncTimer = null
    }
}

onMounted(async () => {
    flushConsoleOutput()
    await hydrateAndDisplay()
    loading.value = false
    startSync()
})

onUnmounted(() => {
    stopSync()
    unsubscribeConsoleOutput?.()
})

watch(
    () => (logLines[props.server.id] ?? []).length,
    (count) => {
        if (loading.value || hydrating) return
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
    // The server echoes the command into its own log (e.g. "> time set 0"),
    // which the console already shows, so we don't echo it a second time here.
    await sendCommand(props.server.id, command)
}

// Starting a server always resets the console to a clean slate and resumes
// bottom-following. The displayed `consoleState` is cleared here, but the
// shared `logLines` buffer is intentionally preserved: the global listener may
// have already streamed the earliest startup lines, and discarding them (or
// letting the async hydrate overwrite them) is what made the launch appear to
// have "no startup info". We rebuild the view from whatever `logLines` already
// holds, then continue following new lines.
const consoleLayout = ref<InstanceType<typeof ConsolePageLayout> | null>(null)
watch(
    () => props.server.running,
    async (running, previousRunning) => {
        if (!running) {
            pendingConsoleOutput = new ServerConsoleBuffer(PENDING_CONSOLE_OUTPUT_CAPACITY)
            return
        }
        if (previousRunning) return
        await nextTick()
        flushConsoleOutput()
        consoleState.clear()
        consumedLines = 0
        // Drop the previous run's lines from the shared buffer too; the backend
        // cleared its own buffer at launch, so without this the old history
        // would be rehydrated into the fresh console on every restart.
        logLines[props.server.id] = []
        await hydrateAndDisplay()
        consoleLayout.value?.scrollToBottom()
    },
)

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
    <div
        data-onboarding-id="server-console"
        class="flex flex-col pb-3"
        :class="hasLogs ? 'h-[calc(100dvh-80px)] shrink-0' : 'h-full min-h-[240px]'"
    >
        <ConsolePageLayout ref="consoleLayout" :custom-command-input="isForge">
            <template #command-input="{ disabled }">
                <JLineCommandInput
                    ref="jlineInput"
                    :disabled="disabled"
                    :placeholder="formatMessage(messages.forgeCommandPlaceholder)"
                    :send-command="handleSendCommand"
                    :send-input="(data) => servers.sendConsoleInput(server.id, data)"
                    :resize-console="(cols, rows) => servers.resizeConsole(server.id, cols, rows)"
                />
            </template>
        </ConsolePageLayout>
    </div>
</template>
