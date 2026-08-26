<script setup lang="ts">
import type { ModTranslationJob } from '@/lab/mod-translation/types.ts'

import ModTranslationJobProgress from './ModTranslationJobProgress.vue'
import ModTranslationJobResult from './ModTranslationJobResult.vue'
import ModTranslationJobSummary from './ModTranslationJobSummary.vue'
import ModTranslationTaskTimeline from './ModTranslationTaskTimeline.vue'
import ModTranslationTechnicalDetails from './ModTranslationTechnicalDetails.vue'

const props = defineProps<{ job: ModTranslationJob }>()

const emit = defineEmits<{
	cancel: [taskId: string]
	remove: [taskId: string]
	openOutput: [job: ModTranslationJob]
}>()
</script>

<template>
	<article
		class="job-card flex min-w-0 flex-col gap-[0.7rem] rounded-[var(--radius-lg)] bg-surface-3 p-[0.95rem_1rem]"
		:class="`job-card--${job.status}`"
	>
		<ModTranslationJobSummary
			:job="props.job"
			@cancel="emit('cancel', $event)"
			@remove="emit('remove', $event)"
			@open-output="emit('openOutput', $event)"
		/>
		<ModTranslationJobProgress :job="props.job" />
		<ModTranslationTaskTimeline :entries="job.timeline" />
		<ModTranslationJobResult :job="props.job" />
		<ModTranslationTechnicalDetails :job="props.job" />
	</article>
</template>

<style scoped>
.job-card--running {
	background:
		linear-gradient(
			180deg,
			color-mix(in srgb, var(--color-brand) 7%, transparent),
			transparent 4.5rem
		),
		var(--surface-3);
}

.job-card--completed {
	background:
		linear-gradient(
			180deg,
			color-mix(in srgb, var(--color-green) 6%, transparent),
			transparent 4.5rem
		),
		var(--surface-3);
}

.job-card--failed {
	background:
		linear-gradient(
			180deg,
			color-mix(in srgb, var(--color-red) 6%, transparent),
			transparent 4.5rem
		),
		var(--surface-3);
}
</style>
