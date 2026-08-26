<script setup lang="ts">
import { defineMessages, EmptyState, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import { countModTranslationJobs } from '@/lab/mod-translation/job-state.ts'
import type { ModTranslationJob } from '@/lab/mod-translation/types.ts'

import ModTranslationJobCard from './ModTranslationJobCard.vue'

const props = defineProps<{
	jobs: ModTranslationJob[]
}>()

const emit = defineEmits<{
	cancel: [taskId: string]
	remove: [taskId: string]
	openOutput: [job: ModTranslationJob]
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	noJobs: {
		id: 'app.lab.mod-translation.no-jobs',
		defaultMessage: 'Started jobs will appear here.',
	},
	activeCount: {
		id: 'app.lab.mod-translation.active-count',
		defaultMessage: '{count} running',
	},
	failedCount: {
		id: 'app.lab.mod-translation.failed-count',
		defaultMessage: '{count} failed',
	},
	completedCount: {
		id: 'app.lab.mod-translation.completed-count',
		defaultMessage: '{count} completed',
	},
	allFinished: {
		id: 'app.lab.mod-translation.all-finished',
		defaultMessage: 'All finished',
	},
})

const counts = computed(() => countModTranslationJobs(props.jobs))
const headerState = computed(() => {
	if (counts.value.running > 0) return 'running'
	if (counts.value.failed > 0) return 'failed'
	return 'completed'
})
</script>

<template>
	<div class="flex flex-col gap-2">
		<template v-if="jobs.length">
			<div class="job-list-header">
				<span
					class="w-2 h-2 flex-none rounded-full"
					:class="
						headerState === 'running'
							? 'live-dot--running'
							: headerState === 'completed'
								? 'bg-green'
								: 'bg-red'
					"
				/>
				<span v-if="counts.running" class="header-chip header-chip--running">
					{{ formatMessage(messages.activeCount, { count: counts.running }) }}
				</span>
				<span v-if="counts.failed" class="header-chip header-chip--failed">
					{{ formatMessage(messages.failedCount, { count: counts.failed }) }}
				</span>
				<span
					v-if="counts.completed && (counts.running || counts.failed)"
					class="header-chip header-chip--done"
				>
					{{ formatMessage(messages.completedCount, { count: counts.completed }) }}
				</span>
				<span v-if="!counts.running && !counts.failed" class="header-chip header-chip--done">
					{{ formatMessage(messages.allFinished) }}
				</span>
			</div>
			<TransitionGroup name="job-list" tag="div" class="flex flex-col gap-3">
				<ModTranslationJobCard
					v-for="job in jobs"
					:key="job.taskId"
					:job="job"
					@cancel="emit('cancel', $event)"
					@remove="emit('remove', $event)"
					@open-output="emit('openOutput', $event)"
				/>
			</TransitionGroup>
		</template>
		<EmptyState v-else :heading="formatMessage(messages.noJobs)" type="empty" />
	</div>
</template>

<style scoped>
.job-list-header {
	display: flex;
	align-items: center;
	gap: 0.5rem;
}

.live-dot--running {
	background: var(--color-brand);
	animation: list-pulse 1.6s ease-out infinite;
}

@keyframes list-pulse {
	0% {
		box-shadow: 0 0 0 0 color-mix(in srgb, var(--color-brand) 50%, transparent);
	}

	70%,
	100% {
		box-shadow: 0 0 0 0.5rem transparent;
	}
}

.header-chip {
	flex: 0 0 auto;
	font-size: 0.68rem;
	font-weight: 700;
	font-variant-numeric: tabular-nums;
	white-space: nowrap;
}

.header-chip--running {
	color: var(--color-brand);
}

.header-chip--done {
	color: var(--color-green);
}

.header-chip--failed {
	color: var(--color-red);
}

.job-list-enter-active {
	transition:
		opacity 0.35s ease,
		transform 0.35s ease;
}

.job-list-enter-from {
	opacity: 0;
	transform: translateY(10px) scale(0.985);
}

.job-list-leave-active {
	transition:
		opacity 0.2s ease,
		transform 0.2s ease;
}

.job-list-leave-to {
	opacity: 0;
	transform: scale(0.985);
}
</style>
