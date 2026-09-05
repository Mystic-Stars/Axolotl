<script setup lang="ts">
import { CheckCircleIcon, SpinnerIcon } from '@modrinth/assets'
import { Admonition, defineMessages, ProgressBar, useVIntl } from '@modrinth/ui'
import { computed, nextTick, onMounted, ref, watch } from 'vue'

import { injectCreateServerFlow } from '../create-server-flow'

const { formatMessage } = useVIntl()
const ctx = injectCreateServerFlow()
const logOutput = ref<HTMLElement | null>(null)
const followLogOutput = ref(true)

const messages = defineMessages({
	downloading: {
		id: 'app.servers.wizard.downloading',
		defaultMessage: 'Downloading server files...',
	},
	firstRun: { id: 'app.servers.wizard.first-run', defaultMessage: 'Running first start...' },
	eulaWait: {
		id: 'app.servers.wizard.eula-wait',
		defaultMessage: 'Waiting for EULA confirmation',
	},
	done: { id: 'app.servers.wizard.done', defaultMessage: 'Server ready' },
	failed: { id: 'app.servers.wizard.failed', defaultMessage: 'Setup failed' },
	installLog: { id: 'app.servers.wizard.log', defaultMessage: 'Output' },
	backgroundHint: {
		id: 'app.servers.wizard.background-hint',
		defaultMessage: 'You can close this window — the download continues in the background.',
	},
})

onMounted(() => {
	if (ctx.installPhase.value === 'idle' || ctx.installPhase.value === 'error') {
		void ctx.beginInstall()
	}
})

const phaseText = computed(() => {
	switch (ctx.installPhase.value) {
		case 'first-run':
			return formatMessage(messages.firstRun)
		case 'eula':
			return formatMessage(messages.eulaWait)
		case 'done':
			return formatMessage(messages.done)
		case 'error':
			return formatMessage(messages.failed)
		default:
			return formatMessage(messages.downloading)
	}
})

const progressPercent = computed(() => {
	const progress = ctx.downloadProgress.value
	if (!progress || !progress.total) return 0
	return Math.min(100, (progress.downloaded / progress.total) * 100)
})

const isBusy = computed(
	() =>
		ctx.installPhase.value === 'preparing' ||
		ctx.installPhase.value === 'downloading' ||
		ctx.installPhase.value === 'first-run',
)

function handleLogScroll() {
	const output = logOutput.value
	if (!output) return
	followLogOutput.value = output.scrollHeight - output.scrollTop - output.clientHeight < 24
}

watch(
	() => ctx.installLog.value.slice(-40),
	async (lines) => {
		if (lines.length === 0) {
			followLogOutput.value = true
			return
		}
		if (!followLogOutput.value) return
		await nextTick()
		if (logOutput.value) logOutput.value.scrollTop = logOutput.value.scrollHeight
	},
	{ immediate: true },
)
</script>

<template>
	<div class="flex flex-col gap-5">
		<div class="flex items-center gap-3">
			<SpinnerIcon v-if="isBusy" class="size-6 shrink-0 animate-spin text-orange" />
			<CheckCircleIcon
				v-else-if="ctx.installPhase.value === 'done'"
				class="size-6 shrink-0 text-green"
			/>
			<span class="text-lg font-semibold text-contrast">{{ phaseText }}</span>
		</div>

		<ProgressBar
			v-if="ctx.installPhase.value === 'downloading'"
			full-width
			:progress="progressPercent"
			:max="100"
			:waiting="progressPercent === 0"
			:label="formatMessage(messages.downloading)"
			show-progress
		/>

		<p
			v-if="ctx.installPhase.value === 'downloading'"
			class="m-0 text-xs font-medium text-secondary"
		>
			{{ formatMessage(messages.backgroundHint) }}
		</p>

		<Admonition
			v-if="ctx.installPhase.value === 'error'"
			type="critical"
			:header="formatMessage(messages.failed)"
		>
			{{ ctx.installError.value }}
		</Admonition>

		<div v-if="ctx.installLog.value.length > 0" class="flex flex-col gap-2">
			<span class="text-sm font-semibold text-secondary">
				{{ formatMessage(messages.installLog) }}
			</span>
			<pre
				ref="logOutput"
				class="max-h-56 overflow-y-auto whitespace-pre-wrap rounded-xl border border-solid border-surface-4 bg-surface-3 p-3 font-mono text-xs leading-relaxed text-primary"
				@scroll="handleLogScroll"
				>{{ ctx.installLog.value.slice(-40).join('\n') }}</pre
			>
		</div>
	</div>
</template>
