<script setup lang="ts">
import { ChevronDownIcon, ChevronUpIcon } from '@modrinth/assets'
import { useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import { modTranslationMessages } from '@/lab/mod-translation/i18n'
import type { ModTranslationJob } from '@/lab/mod-translation/types.ts'

const props = defineProps<{ job: ModTranslationJob }>()
const { formatMessage } = useVIntl()
const open = ref(false)
const diagnostic = computed(() => ({
	taskId: props.job.taskId,
	inputHash: props.job.inputHash,
	lastSequence: props.job.lastSequence,
	status: props.job.status,
	error: props.job.error,
	events: props.job.events,
}))

function copy() {
	void navigator.clipboard.writeText(JSON.stringify(diagnostic.value, null, 2))
}
</script>

<template>
	<section class="flex flex-col gap-[0.4rem]">
		<div class="flex justify-between gap-2">
			<button
				:aria-expanded="open"
				class="inline-flex items-center gap-[0.3rem] border-0 bg-transparent p-[0.2rem] text-[var(--color-text-tertiary)] text-[0.68rem]"
				@click="open = !open"
			>
				{{ formatMessage(modTranslationMessages.technicalDetails) }}
				<ChevronUpIcon v-if="open" /><ChevronDownIcon v-else />
			</button>
			<button
				class="inline-flex items-center gap-[0.3rem] border-0 bg-transparent p-[0.2rem] text-[var(--color-text-tertiary)] text-[0.68rem]"
				@click="copy"
			>
				{{ formatMessage(modTranslationMessages.copyDiagnosticsInfo) }}
			</button>
		</div>
		<pre v-if="open">{{ JSON.stringify(diagnostic, null, 2) }}</pre>
	</section>
</template>

<style scoped>
pre {
	max-height: 18rem;
	overflow: auto;
	margin: 0;
	border-radius: var(--radius-sm);
	background: var(--surface-1);
	padding: 0.65rem;
	font-size: 0.62rem;
	white-space: pre-wrap;
}
</style>
