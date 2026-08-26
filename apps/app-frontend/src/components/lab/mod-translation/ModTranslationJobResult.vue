<script setup lang="ts">
import { CheckCircleIcon, XIcon } from '@modrinth/assets'
import { computed } from 'vue'

import type { ModTranslationJob } from '@/lab/mod-translation/types.ts'

const props = defineProps<{ job: ModTranslationJob }>()
const hasWarnings = computed(
	() => props.job.status === 'completed' && Boolean(props.job.report?.warnings?.length),
)
</script>

<template>
	<section
		v-if="job.status !== 'running'"
		class="result"
		:class="[`result--${job.status}`, { 'result--warning': hasWarnings }]"
	>
		<div class="result-head flex items-center gap-[0.45rem] text-green">
			<span class="result-badge">
				<CheckCircleIcon v-if="job.status === 'completed'" />
				<XIcon v-else />
			</span>
			<strong>{{
				job.status === 'completed'
					? hasWarnings
						? '已生成，但仍有未覆盖文本'
						: '翻译完成'
					: job.error?.code || 'UNKNOWN_ERROR'
			}}</strong>
		</div>
		<template v-if="job.status === 'completed'">
			<div v-if="job.report?.modName?.name" class="result-row">
				<span>模组</span><strong>{{ job.report.modName.name }}</strong>
			</div>
			<div class="result-stats">
				<div>
					<span>语言条目</span
					><strong
						>{{ job.report?.languageAccepted ?? 0 }}/{{
							job.report?.languageAttempted ?? 0
						}}</strong
					>
				</div>
				<div v-if="job.report?.classTotal">
					<span>Class 文本</span
					><strong>{{ job.report.classResolved }}/{{ job.report.classTotal }}</strong>
				</div>
			</div>
			<span v-if="job.report?.classChangedFiles?.length" class="secondary"
				>改动文件：{{ job.report.classChangedFiles.join('、') }}</span
			>
			<ul v-if="job.report?.warnings?.length" class="warnings">
				<li v-for="warning in job.report.warnings" :key="warning">{{ warning }}</li>
			</ul>
			<span class="path" :title="job.outputPath">{{ job.outputPath }}</span>
		</template>
		<template v-else>
			<p>{{ job.error?.message || job.message }}</p>
			<pre v-if="job.error?.details">{{ JSON.stringify(job.error.details, null, 2) }}</pre>
		</template>
	</section>
</template>

<style scoped>
.result {
	display: flex;
	flex-direction: column;
	gap: 0.55rem;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-md);
	padding: 0.7rem 0.75rem;
	font-size: 0.72rem;
}

.result--completed {
	border-color: color-mix(in srgb, var(--color-green) 24%, var(--color-divider));
	background: color-mix(in srgb, var(--color-green) 7%, var(--color-button-bg));
}

.result--failed {
	border-color: color-mix(in srgb, var(--color-red) 24%, var(--color-divider));
	background: color-mix(in srgb, var(--color-red) 7%, var(--color-button-bg));
}

.result--warning {
	border-color: color-mix(in srgb, var(--color-orange) 30%, var(--color-divider));
	background: color-mix(in srgb, var(--color-orange) 7%, var(--color-button-bg));
}

.result--failed .result-head {
	color: var(--color-red);
}

.result--warning .result-head {
	color: var(--color-orange);
}

.result-badge {
	display: grid;
	width: 1.45rem;
	height: 1.45rem;
	place-items: center;
	border-radius: 999px;
	background: color-mix(in srgb, currentColor 14%, transparent);
}

.result-badge :deep(svg) {
	width: 0.82rem;
	height: 0.82rem;
}

.result-head strong {
	font-size: 0.78rem;
	font-weight: 800;
}

.result-row {
	display: flex;
	justify-content: space-between;
	gap: 0.75rem;
	color: var(--color-text-secondary);
}

.result-row strong {
	color: var(--color-contrast);
}

.result-stats {
	display: grid;
	grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
	gap: 0.45rem;
}

.result-stats div {
	display: flex;
	flex-direction: column;
	gap: 0.12rem;
	border-radius: var(--radius-sm);
	background: var(--surface-3);
	padding: 0.45rem 0.55rem;
}

.result-stats span,
.secondary,
.path,
.result p {
	color: var(--color-text-secondary);
}

.warnings {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
	margin: 0;
	padding-left: 1.1rem;
	color: var(--color-orange);
	line-height: 1.45;
}

.result-stats strong {
	font-size: 0.82rem;
	color: var(--color-contrast);
}

.path {
	overflow: hidden;
	font-family: var(--font-mono, monospace);
	font-size: 0.64rem;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.result p {
	margin: 0;
	line-height: 1.5;
	word-break: break-word;
}

.result pre {
	max-height: 14rem;
	overflow: auto;
	margin: 0;
	border-radius: var(--radius-sm);
	background: var(--surface-1);
	padding: 0.55rem;
	color: var(--color-text-secondary);
	font-size: 0.64rem;
	white-space: pre-wrap;
}
</style>
