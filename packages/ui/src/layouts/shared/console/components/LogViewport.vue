<template>
	<div
		ref="viewportRef"
		class="log-viewport font-mono"
		:class="{ 'log-viewport-wrap': wrap, 'overflow-x-hidden': wrap }"
		:style="{ fontSize: fontSize + 'px' }"
		@scroll="handleScroll"
	>
		<div v-if="lines.length === 0" class="flex items-center justify-center h-full">
			<EmptyState
				v-if="emptyStateType === 'instance'"
				:heading="formatMessage(consoleMessages.emptyInstanceTitle)"
				:description="formatMessage(consoleMessages.emptyInstanceDescription)"
			/>
			<EmptyState
				v-else-if="emptyStateType === 'server'"
				:heading="formatMessage(consoleMessages.emptyServerTitle)"
				:description="formatMessage(consoleMessages.emptyServerDescription)"
			/>
		</div>

		<div v-else class="log-viewport-spacer relative w-full min-w-max" :style="{ height: totalHeight + 'px' }">
			<div
				class="absolute inset-x-0 top-0"
				:style="{ transform: 'translateY(' + topOffset + 'px)' }"
			>
				<div
					v-for="item in windowItems"
					:key="item.originalIndex"
					:data-line="item.originalIndex + 1"
					class="log-line flex items-stretch whitespace-pre"
					:class="entryClass(item.line)"
					:style="{ height: estimateHeight(item) + 'px' }"
				>
					<span
						class="shrink-0 w-[52px] pr-2.5 text-right text-secondary bg-surface-3 border-r border-solid border-surface-5 select-none overflow-hidden"
						>{{ item.originalIndex + 1 }}</span
					>
					<span
						class="log-line-content flex-1 px-2 break-all [overflow-wrap:anywhere]"
						v-html="renderLine(item)"
					></span>
				</div>
			</div>
		</div>

		<Transition name="scroll-to-bottom-fade">
			<div v-if="lines.length > 0 && !stickToBottom" class="absolute bottom-4 right-4 z-10">
				<ButtonStyled circular type="highlight" size="large">
					<button aria-label="Scroll to bottom" @click="scrollToBottom">
						<ChevronDownIcon />
					</button>
				</ButtonStyled>
			</div>
		</Transition>
	</div>
</template>

<script setup lang="ts">
// 日志查看器：交互与布局参考 LogShare-Web-UI (src/views/LogView.vue)，
// 逐行正则高亮来自 ./composables/log-highlight.ts（移植自 logParser.worker.ts）。
// LogShare-Web-UI 为 MIT License, Copyright (c) 2024 LogShare.CN Team，详见 packages/ui/COPYING.md。
import { ChevronDownIcon } from '@modrinth/assets'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import EmptyState from '#ui/components/base/EmptyState.vue'
import { useVIntl } from '#ui/composables/i18n'

import { highlightLine } from '../composables/log-highlight'
import { consoleMessages } from '../messages'
import type { LogLine } from '../types'

const { formatMessage } = useVIntl()

interface ViewportLine {
	line: LogLine
	originalIndex: number
}

const props = withDefaults(
	defineProps<{
		lines: ViewportLine[]
		searchQuery?: string
		wrap?: boolean
		fontSize?: number
		emptyStateType?: 'server' | 'instance'
	}>(),
	{
		searchQuery: '',
		wrap: false,
		fontSize: 12,
		emptyStateType: undefined,
	},
)

const viewportRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const viewportHeight = ref(0)
const stickToBottom = ref(true)

// 行高：单行 = 字号 × 1.4（与等宽字体匹配），wrap 时按估算折行数放大
const lineHeightPx = computed(() => Math.round(props.fontSize * 1.4))
// wrap 折行估算：0.6em 为等宽字符平均宽，乘 0.9 留保守余量（行高宁高勿矮，避免内容溢出重叠）
const charsPerLine = computed(() => {
	const vp = viewportRef.value
	if (!vp) return 120
	return Math.max(20, Math.floor((vp.clientWidth / (props.fontSize * 0.6)) * 0.9))
})

function estimateHeight(item: ViewportLine): number {
	if (!props.wrap) return lineHeightPx.value
	const lines = Math.max(1, Math.ceil(item.line.text.length / charsPerLine.value))
	return lines * lineHeightPx.value
}

// 高度前缀和缓存：lines/wrap/fontSize 变化时重建（O(n)），滚动时二分查找（O(log n)）
let heightPrefix: number[] | null = null
let heightTotal = 0

function rebuildHeights() {
	const n = props.lines.length
	if (!props.wrap) {
		heightPrefix = null
		heightTotal = n * lineHeightPx.value
		return
	}
	heightPrefix = new Array(n)
	let acc = 0
	for (let i = 0; i < n; i++) {
		heightPrefix[i] = acc
		acc += estimateHeight(props.lines[i]!)
	}
	heightTotal = acc
}

watch(
	() => [props.lines, props.wrap, props.fontSize] as const,
	() => {
		rebuildHeights()
		if (stickToBottom.value) {
			nextTick(scrollToBottom)
		}
	},
	{ immediate: true },
)

const totalHeight = computed(() => heightTotal)

// 虚拟窗口：可见行 + 上下缓冲
const WINDOW_BUFFER = 15

function computeWindow(): { items: ViewportLine[]; startIndex: number } {
	const n = props.lines.length
	if (n === 0) return { items: [], startIndex: 0 }

	let start = 0
	let end = n - 1

	if (n > WINDOW_BUFFER * 2) {
		if (props.wrap && heightPrefix) {
			let lo = 0
			let hi = n - 1
			while (lo < hi) {
				const mid = (lo + hi + 1) >> 1
				if (heightPrefix[mid]! <= scrollTop.value) lo = mid
				else hi = mid - 1
			}
			start = Math.max(0, lo - WINDOW_BUFFER)
		} else {
			const first = Math.floor(scrollTop.value / lineHeightPx.value)
			start = Math.max(0, first - WINDOW_BUFFER)
		}
		end = Math.min(
			n - 1,
			start + Math.ceil(viewportHeight.value / lineHeightPx.value) + WINDOW_BUFFER * 2,
		)
	}

	return { items: props.lines.slice(start, end + 1), startIndex: start }
}

const windowState = computed(computeWindow)
const windowItems = computed(() => windowState.value.items)

const topOffset = computed(() => {
	const { startIndex } = windowState.value
	if (startIndex === 0) return 0
	if (props.wrap && heightPrefix) return heightPrefix[startIndex]!
	return startIndex * lineHeightPx.value
})

function entryClass(line: LogLine): string {
	if (line.level === 'error') return 'entry-error'
	if (line.level === 'warn') return 'entry-warning'
	return 'entry-no-error'
}

function renderLine(item: ViewportLine): string {
	let text = item.line.text
	if (props.searchQuery) {
		const terms = props.searchQuery
			.trim()
			.toLowerCase()
			.split(/\s+/)
			.filter((t) => t.length > 0)
		if (terms.length > 0) {
			for (const term of [...terms].sort((a, b) => b.length - a.length)) {
				const escaped = term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
				text = text.replace(new RegExp(`(${escaped})`, 'gi'), '<mark>$1</mark>')
			}
		}
	}
	return highlightLine(text)
}

function handleScroll() {
	const vp = viewportRef.value
	if (!vp) return
	scrollTop.value = vp.scrollTop
	viewportHeight.value = vp.clientHeight
	stickToBottom.value = vp.scrollTop + vp.clientHeight >= vp.scrollHeight - lineHeightPx.value * 2
}

function scrollToBottom() {
	const vp = viewportRef.value
	if (!vp) return
	vp.scrollTop = vp.scrollHeight
	scrollTop.value = vp.scrollTop
	stickToBottom.value = true
}

function syncViewportSize() {
	const vp = viewportRef.value
	if (!vp) return
	viewportHeight.value = vp.clientHeight
	// 窗口宽度影响 wrap 折行估算，resize 时重建高度缓存
	if (props.wrap) rebuildHeights()
}

let resizeObserver: ResizeObserver | null = null

onMounted(() => {
	syncViewportSize()
	if (stickToBottom.value) nextTick(scrollToBottom)
	resizeObserver = new ResizeObserver(syncViewportSize)
	if (viewportRef.value) resizeObserver.observe(viewportRef.value)
	window.addEventListener('resize', syncViewportSize)
})

onBeforeUnmount(() => {
	resizeObserver?.disconnect()
	resizeObserver = null
	window.removeEventListener('resize', syncViewportSize)
})

defineExpose({
	scrollToBottom,
})
</script>

<style>
.log-viewport {
	height: 100%;
	overflow-y: auto;
	overflow-x: auto;
	background-color: var(--surface-2);
	color: var(--color-text-default);
	line-height: 1.4;
	-webkit-font-smoothing: antialiased;
	text-rendering: optimizeLegibility;
	user-select: text;
}

.scroll-to-bottom-fade-enter-active,
.scroll-to-bottom-fade-leave-active {
	transition: opacity 250ms ease-in-out;
}

.scroll-to-bottom-fade-enter-from,
.scroll-to-bottom-fade-leave-to {
	opacity: 0;
}

.log-viewport-wrap .log-viewport-spacer {
	min-width: 0;
}

.log-viewport-wrap .log-line {
	white-space: normal;
}

.log-viewport-wrap .log-line-content {
	min-width: 0;
}

.log-line.entry-error {
	background-color: color-mix(in srgb, var(--color-red) 12%, transparent);
}

.log-line.entry-warning {
	background-color: color-mix(in srgb, var(--color-orange) 12%, transparent);
}

[data-theme='dark'] .log-line.entry-error {
	background-color: color-mix(in srgb, var(--color-red) 18%, transparent);
}

[data-theme='dark'] .log-line.entry-warning {
	background-color: color-mix(in srgb, var(--color-orange) 18%, transparent);
}

.log-line mark {
	padding: 0 0.1em;
	background-color: color-mix(in srgb, var(--color-blue) 45%, transparent);
	color: var(--color-text-primary);
	border-radius: 2px;
	font-weight: 500;
}

/* ===== LogShare token 高亮（LogsAnalysis.css 移植，前景色用主题变量） ===== */

.level {
	white-space: pre-wrap;
	word-break: break-all;
	overflow-wrap: anywhere;
}

.level-error,
.level-critical,
.level-emergency {
	color: var(--color-red);
	font-weight: 600;
}

.level-warning {
	color: var(--color-orange);
}

.level-fatal {
	color: var(--color-red);
	font-weight: 700;
	background-color: color-mix(in srgb, var(--color-red) 8%, transparent);
}

[data-theme='dark'] .level-fatal {
	background-color: color-mix(in srgb, var(--color-red) 15%, transparent);
}

.level-debug,
.level-notice {
	color: var(--color-text-secondary);
	background-color: color-mix(in srgb, var(--color-blue) 5%, transparent);
}

.level-notice {
	background-color: color-mix(in srgb, var(--color-blue) 10%, transparent);
}

.level-timestamp {
	color: var(--color-blue);
	font-weight: 500;
}

.level-info-prefix {
	color: var(--color-green);
	font-weight: 500;
}

.level-thread {
	color: var(--color-blue);
	opacity: 0.85;
}

.level-error-word {
	color: var(--color-red);
	font-weight: 600;
}

.level-warning-tag {
	color: var(--color-orange);
	font-weight: 600;
}

.level-plugin {
	color: var(--color-green);
	opacity: 0.85;
}

.level-filepath {
	color: var(--color-blue);
	opacity: 0.85;
}

.level-dimmed {
	color: var(--color-text-tertiary);
	opacity: 0.75;
}

.level-stack-frame {
	color: var(--color-text-secondary);
}

.level-stack-class {
	color: var(--color-orange);
	font-weight: 500;
}

.level-stack-location {
	color: var(--color-blue);
}

.level-stack-caused-by {
	color: var(--color-red);
	font-weight: 600;
}

.level-stack-exception {
	color: var(--color-red);
	font-weight: 600;
}

.level-mod-header {
	color: var(--color-purple);
	font-weight: 700;
}

.level-mod-id {
	color: var(--color-blue);
	font-weight: 500;
}

.level-mod-version {
	color: #d4a72c;
	font-weight: 500;
}

.level-mod-name {
	color: var(--color-text-secondary);
}

.level-mod-status {
	color: var(--color-text-tertiary);
}

.level-mod-status-ok {
	color: var(--color-green);
	font-weight: 600;
}

.level-mod-status-error {
	color: var(--color-red);
	font-weight: 600;
}

.level-mod-status-warn {
	color: var(--color-orange);
	font-weight: 600;
}

.level-mod-tree {
	color: var(--color-blue);
}

.level-mod-dim {
	color: var(--color-green);
}

.level-env-key {
	color: var(--color-blue);
	font-weight: 500;
}

.level-section-header {
	color: var(--color-purple);
	font-weight: 600;
}

.level-arg-flag {
	color: #d4a72c;
	font-weight: 500;
}

.level-section-marker {
	color: var(--color-orange);
	font-weight: 600;
}

/* Minecraft § 颜色码 */

.format-black {
	color: #000000;
}

.format-darkblue {
	color: #0000aa;
}

.format-darkgreen {
	color: #00aa00;
}

.format-darkaqua {
	color: #00aaaa;
}

.format-darkred {
	color: #aa0000;
}

.format-darkpurple {
	color: #aa00aa;
}

.format-gold {
	color: #ffaa00;
}

.format-gray {
	color: #aaaaaa;
}

.format-darkgray {
	color: #555555;
}

.format-blue {
	color: #5555ff;
}

.format-green {
	color: #55ff55;
}

.format-aqua {
	color: #55ffff;
}

.format-red {
	color: #ff5555;
}

.format-lightpurple {
	color: #ff55ff;
}

.format-yellow {
	color: #ffff55;
}

.format-white {
	color: #ffffff;
}

.format-reset {
	color: var(--color-text-default);
	font-weight: normal;
	text-decoration: none;
	font-style: normal;
}

.format-bold {
	font-weight: bold;
}

.format-underline {
	text-decoration: underline;
}

.format-italic {
	font-style: italic;
}

.format-strike {
	text-decoration: line-through;
}
</style>
