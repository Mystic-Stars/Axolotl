<script setup lang="ts">
import { ChevronDownIcon, ChevronUpIcon, HistoryIcon } from '@modrinth/assets'
import { computed, nextTick, ref, watch } from 'vue'

import { groupTimelineByRepairPass } from '@/lab/mod-translation/timeline'
import type { ModTranslationTimelineEntry } from '@/lab/mod-translation/types.ts'

const props = defineProps<{ entries: ModTranslationTimelineEntry[] }>()
const open = ref(false)
const scrollEl = ref<HTMLElement | null>(null)
const stayAtBottom = ref(true)
const debugOpen = ref(new Set<string>())
const groups = computed(() => groupTimelineByRepairPass(props.entries))

function onScroll() {
	const element = scrollEl.value
	if (!element) return
	stayAtBottom.value = element.scrollHeight - element.scrollTop - element.clientHeight < 24
}

watch(
	() => props.entries.length,
	async () => {
		if (!open.value || !stayAtBottom.value) return
		await nextTick()
		if (scrollEl.value) scrollEl.value.scrollTop = scrollEl.value.scrollHeight
	},
)

function toggleDebug(id: string) {
	const next = new Set(debugOpen.value)
	if (next.has(id)) next.delete(id)
	else next.add(id)
	debugOpen.value = next
}

function copyDebug(value: unknown) {
	void navigator.clipboard.writeText(JSON.stringify(value, null, 2))
}
</script>

<template>
	<section class="flex flex-col gap-[0.35rem] border-t border-divider pt-[0.55rem]">
		<button type="button" class="toggle" :aria-expanded="open" @click="open = !open">
			<HistoryIcon />处理时间线 ({{ entries.length }}) <ChevronUpIcon v-if="open" /><ChevronDownIcon
				v-else
			/>
		</button>
		<div v-show="open" ref="scrollEl" class="timeline" @scroll="onScroll">
			<div v-if="!groups.length" class="empty">等待处理事件…</div>
			<section v-for="group in groups" :key="group.id" class="group">
				<h4 v-if="group.pass">Repair Pass {{ group.pass }}</h4>
				<div
					v-for="entry in group.entries"
					:key="entry.id"
					class="entry grid grid-cols-[auto_minmax(0,1fr)] gap-2"
					:class="`entry--${entry.status}`"
				>
					<span class="dot" />
					<div class="entry-body">
						<strong>{{ entry.title }}</strong>
						<span v-if="entry.summary">{{ entry.summary }}</span>
						<span v-if="entry.issueIds.length" class="issues"
							>{{ entry.issueIds.length }} issue IDs</span
						>
						<div v-if="entry.debug" class="debug">
							<button
								type="button"
								:aria-expanded="debugOpen.has(entry.id)"
								@click="toggleDebug(entry.id)"
							>
								技术详情
							</button>
							<button type="button" @click="copyDebug(entry.debug)">复制诊断</button>
							<pre v-if="debugOpen.has(entry.id)">{{ JSON.stringify(entry.debug, null, 2) }}</pre>
						</div>
					</div>
				</div>
			</section>
		</div>
	</section>
</template>

<style scoped>
.toggle {
	display: flex;
	width: 100%;
	align-items: center;
	gap: 0.4rem;
	border: 0;
	background: transparent;
	padding: 0;
	color: var(--color-text-secondary);
	font-size: 0.68rem;
	font-weight: 700;
	text-align: left;
	cursor: pointer;
}
.toggle:hover {
	color: var(--color-contrast);
}
.toggle:focus-visible,
.debug button:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: -2px;
}
.toggle :deep(svg) {
	width: 0.9rem;
	height: 0.9rem;
}
.toggle :deep(svg:last-child) {
	margin-left: auto;
}
.timeline {
	display: flex;
	max-height: 18rem;
	flex-direction: column;
	gap: 0.7rem;
	overflow-y: auto;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-md);
	background: var(--color-button-bg);
	padding: 0.65rem 0.7rem;
}
.empty {
	color: var(--color-text-secondary);
	font-size: 0.72rem;
}
.group {
	display: flex;
	flex-direction: column;
	gap: 0.45rem;
}
.group h4 {
	margin: 0;
	color: var(--color-brand);
	font-size: 0.68rem;
	text-transform: uppercase;
}
.dot {
	width: 0.5rem;
	height: 0.5rem;
	margin-top: 0.3rem;
	border-radius: 999px;
	background: var(--color-brand);
}
.entry--success .dot {
	background: var(--color-green);
}
.entry--warning .dot,
.entry--warn .dot {
	background: var(--color-orange);
}
.entry--error .dot {
	background: var(--color-red);
}
.entry-body {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: 0.12rem;
	font-size: 0.72rem;
}
.entry-body span {
	color: var(--color-text-secondary);
}
.issues {
	font-family: monospace;
	font-size: 0.64rem;
}
.debug {
	display: flex;
	flex-wrap: wrap;
	gap: 0.35rem;
	margin-top: 0.25rem;
}
.debug button {
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--surface-2);
	padding: 0.2rem 0.4rem;
	color: var(--color-text-secondary);
	font-size: 0.65rem;
}
.debug pre {
	width: 100%;
	max-height: 14rem;
	overflow: auto;
	margin: 0;
	border-radius: var(--radius-sm);
	background: var(--surface-1);
	padding: 0.55rem;
	font-size: 0.62rem;
	white-space: pre-wrap;
}
</style>
