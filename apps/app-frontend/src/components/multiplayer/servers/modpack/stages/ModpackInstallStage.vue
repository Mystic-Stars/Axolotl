<script setup lang="ts">
import { CheckCircleIcon, SpinnerIcon } from '@modrinth/assets'
import { Admonition, defineMessages, ProgressBar, useVIntl } from '@modrinth/ui'
import { computed, onMounted } from 'vue'

import { injectCreateServerFlow } from '../../create-server-flow'

const { formatMessage } = useVIntl()
const ctx = injectCreateServerFlow()

const messages = defineMessages({
	downloading: {
		id: 'app.servers.modpack.downloading',
		defaultMessage: 'Downloading modpack files...',
	},
	preparing: {
		id: 'app.servers.modpack.preparing',
		defaultMessage: 'Preparing server...',
	},
	done: { id: 'app.servers.modpack.done', defaultMessage: 'Installation complete' },
	failed: { id: 'app.servers.wizard.failed', defaultMessage: 'Setup failed' },
	installLog: { id: 'app.servers.wizard.log', defaultMessage: 'Output' },
	currentFile: {
		id: 'app.servers.modpack.current-file',
		defaultMessage: 'Now installing {file}',
	},
	backgroundHint: {
		id: 'app.servers.modpack.background-hint',
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
		case 'preparing':
			return formatMessage(messages.preparing)
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

const currentFile = computed(() => {
	const match = [...ctx.installLog.value]
		.map((line) => /^Downloading (.+)$/.exec(line)?.[1])
		.filter(Boolean)
		.at(-1)
	return match ?? null
})

const isBusy = computed(
	() =>
		ctx.installPhase.value === 'preparing' ||
		ctx.installPhase.value === 'downloading',
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
			v-if="currentFile && ctx.installPhase.value === 'downloading'"
			class="m-0 -mt-2 truncate text-xs font-medium text-secondary"
		>
			{{ formatMessage(messages.currentFile, { file: currentFile }) }}
		</p>

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

		<div
			v-if="ctx.installPhase.value === 'error'"
			class="flex flex-col gap-2"
		>
			<span class="text-sm font-semibold text-secondary">
				{{ formatMessage(messages.installLog) }}
			</span>
			<pre
				class="max-h-56 overflow-y-auto whitespace-pre-wrap rounded-xl border border-solid border-surface-4 bg-surface-3 p-3 font-mono text-xs leading-relaxed text-primary"
				>{{ ctx.installLog.value.slice(-40).join('\n') }}</pre
			>
		</div>
	</div>
</template>
