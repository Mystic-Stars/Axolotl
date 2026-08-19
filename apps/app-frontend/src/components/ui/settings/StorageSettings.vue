<script setup lang="ts">
import { HelpCircleIcon, RefreshCwIcon, SpinnerIcon } from '@modrinth/assets'
import { useFormatBytes, useVIntl } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import { list as fetchInstances } from '@/helpers/instance'
import {
	listenStorageScan,
	openStoragePaths,
	startStorageScan,
	type StorageScanEvent,
} from '@/helpers/storage'

import type { StorageNode, StorageNodeType, StorageSize, StorageTree } from './storage/storageData'
import { sortStorageChildren } from './storage/storageData'
import { storageMessages } from './storage/storageMessages'
import StorageTreeNode from './storage/StorageTreeNode.vue'

type MessageDescriptor = (typeof storageMessages)['total']

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const router = useRouter()

const tree = ref<StorageTree | null>(null)
const loading = ref(true)
const lastUpdated = ref<Date | null>(null)
let listenerUnsubscribe: (() => void) | null = null

const categories = computed(() => tree.value?.categories ?? [])

const mainCategories = computed<StorageNode[]>(() => {
	const list = categories.value.filter(
		(category) => category.type !== 'other' && category.size.actual + category.size.symlink > 0,
	)
	const rootOther = tree.value?.rootOther
	if (rootOther && rootOther.size.actual + rootOther.size.symlink > 0) {
		list.push(rootOther)
	}
	return list
})

const instancesCategory = computed(
	() => categories.value.find((category) => category.type === 'instances') ?? null,
)

const categoryLabels: Record<StorageNodeType, MessageDescriptor> = {
	instances: storageMessages.instanceData,
	cache: storageMessages.cacheData,
	meta: storageMessages.metaData,
	database: storageMessages.database,
	other: storageMessages.other,
}

// 醒目高对比度色彩配置
const categoryColors: Record<StorageNodeType, { actual: string; symlink: string }> = {
	instances: { actual: '#10b981', symlink: '#06b6d4' }, // 翡翠绿 / 亮青
	cache: { actual: '#f59e0b', symlink: '#eab308' }, // 琥珀黄 / 明黄
	meta: { actual: '#8b5cf6', symlink: '#ec4899' }, // 靛紫 / 靓粉
	database: { actual: '#3b82f6', symlink: '#6366f1' }, // 靛蓝 / 靛青
	other: { actual: '#6b7280', symlink: '#9ca3af' }, // 中灰 / 浅灰
}

const hoveredId = ref<string | null>(null)

const symlinkHelpTooltipOptions = computed(() => ({
	content: formatMessage(storageMessages.symlinkHelpTooltip),
	popperClass: 'storage-tooltip',
}))

function sizeTotal(size: StorageSize) {
	return size.actual + size.symlink
}

function formatSize(size: StorageSize) {
	const actual = formatBytes(size.actual)
	if (size.symlink <= 0) return actual
	return `${actual} + ${formatBytes(size.symlink)}`
}

function emptyTree(): StorageTree {
	return {
		total: { actual: 0, symlink: 0 },
		categories: [],
		rootOther: null,
	}
}

function applyStorageEvent(event: StorageScanEvent) {
	switch (event.kind) {
		case 'started':
			loading.value = true
			break
		case 'category': {
			const category = event.payload.category
			if (!tree.value) tree.value = emptyTree()
			if (category.type === 'other') {
				tree.value!.rootOther = category
			} else {
				const index = tree.value!.categories.findIndex((child) => child.id === category.id)
				if (index >= 0) tree.value!.categories[index] = category
				else tree.value!.categories.push(category)
			}
			break
		}
		case 'complete': {
			tree.value = event.payload.tree
			lastUpdated.value = tree.value.scannedAt ? new Date(tree.value.scannedAt) : new Date()
			loading.value = false
			break
		}
		case 'error': {
			loading.value = false
			console.warn('[storage] Scan failed:', event.payload.message)
			break
		}
	}
}

async function loadStorage(force: boolean) {
	loading.value = true
	try {
		await startStorageScan(force)
	} catch (error) {
		loading.value = false
		console.warn('[storage] Failed to start storage scan', error)
	}
}

async function updateStorage() {
	await loadStorage(true)
}

onMounted(async () => {
	try {
		listenerUnsubscribe = await listenStorageScan(applyStorageEvent)
	} catch (error) {
		console.warn('[storage] Failed to subscribe to storage scan events', error)
	}
	await loadStorage(false)
})

onUnmounted(() => {
	listenerUnsubscribe?.()
})

function findInstanceParent(target: StorageNode): StorageNode | null {
	const roots: StorageNode[] = [...(tree.value?.categories ?? [])]
	if (tree.value?.rootOther) roots.push(tree.value.rootOther)

	const stack: { node: StorageNode; parentInstance: StorageNode | null }[] = roots.map(
		(node) => ({ node, parentInstance: null }),
	)

	while (stack.length > 0) {
		const { node, parentInstance } = stack.pop()!
		if (node === target) return node.type === 'instance' ? node : parentInstance

		const nextInstance = node.type === 'instance' ? node : parentInstance
		for (const child of node.children ?? []) {
			stack.push({ node: child, parentInstance: nextInstance })
		}
	}

	return null
}

async function resolveInstanceIdByName(instanceNode: StorageNode): Promise<string | null> {
	const name = instanceNode.name?.trim() ?? ''
	if (!name) return null

	try {
		const instances = await fetchInstances()
		const normalizedName = name.toLowerCase()
		return (
			instances.find(
				(instance) =>
					instance.path === name || instance.name.trim().toLowerCase() === normalizedName,
			)?.id ?? null
		)
	} catch (error) {
		console.warn('[storage] Failed to resolve instance id, falling back to filesystem', error)
		return null
	}
}

async function resolveInstanceId(node: StorageNode): Promise<string | null> {
	const instanceNode = findInstanceParent(node)
	const directId = node.instance_id ?? instanceNode?.instance_id ?? null
	if (directId) return directId
	return instanceNode ? resolveInstanceIdByName(instanceNode) : null
}

function launcherRouteFor(node: StorageNode, instanceId: string): string | null {
	const encodedId = encodeURIComponent(instanceId)
	switch (node.type) {
		case 'instance':
		case 'mods':
			return `/instance/${encodedId}`
		case 'saves':
		case 'world':
			return `/instance/${encodedId}/worlds`
		case 'screenshots':
			return `/instance/${encodedId}/screenshots`
		default:
			return null
	}
}

async function tryNavigate(path: string): Promise<boolean> {
	try {
		await router.push(path)
		return true
	} catch (error) {
		console.warn('[storage] Launcher navigation failed, falling back to filesystem', error)
		return false
	}
}

async function openNodePaths(node: StorageNode) {
	if (node.paths.length === 0) return
	try {
		const result = await openStoragePaths(node.paths)
		for (const failure of result.failed) {
			console.warn(`[storage] Failed to open path: ${failure.path}`, failure.reason)
		}
	} catch (error) {
		console.warn('[storage] Failed to open storage paths', error)
	}
}

async function handleAction(node: StorageNode) {
	const instanceId = await resolveInstanceId(node)
	const route = instanceId ? launcherRouteFor(node, instanceId) : null

	if (route && (await tryNavigate(route))) return

	await openNodePaths(node)
}

interface ChartItem {
	id: string
	category: StorageNode
	label: string
	sizeBytes: number
	formattedSize: string
	color: string
	isSymlink: boolean
	startAngle: number
	endAngle: number
	pathData: string
	percentText: string
}

// 极其精准的 SVG Path 圆弧/环形生成算法（彻底解决 SVG circle dashoffset 错位乱套问题）
function getRingPath(
	cx: number,
	cy: number,
	rInner: number,
	rOuter: number,
	startAngle: number,
	endAngle: number,
) {
	// 防止 100% 比例下起点终点重合导致无法绘制
	const angleDiff = endAngle - startAngle
	const safeEndAngle = angleDiff >= 360 ? startAngle + 359.999 : endAngle

	const rad = (deg: number) => (deg - 90) * (Math.PI / 180)

	const x1 = cx + rOuter * Math.cos(rad(startAngle))
	const y1 = cy + rOuter * Math.sin(rad(startAngle))
	const x2 = cx + rOuter * Math.cos(rad(safeEndAngle))
	const y2 = cy + rOuter * Math.sin(rad(safeEndAngle))

	const x3 = cx + rInner * Math.cos(rad(safeEndAngle))
	const y3 = cy + rInner * Math.sin(rad(safeEndAngle))
	const x4 = cx + rInner * Math.cos(rad(startAngle))
	const y4 = cy + rInner * Math.sin(rad(startAngle))

	const largeArc = angleDiff > 180 ? 1 : 0

	return [
		`M ${x1} ${y1}`,
		`A ${rOuter} ${rOuter} 0 ${largeArc} 1 ${x2} ${y2}`,
		`L ${x3} ${y3}`,
		`A ${rInner} ${rInner} 0 ${largeArc} 0 ${x4} ${y4}`,
		'Z',
	].join(' ')
}

const chartSlices = computed(() => {
	const items: {
		id: string
		category: StorageNode
		label: string
		sizeBytes: number
		formattedSize: string
		color: string
		isSymlink: boolean
	}[] = []

	// 拆分实体与软链接
	for (const cat of mainCategories.value) {
		const baseLabel = formatMessage(categoryLabels[cat.type])

		if (cat.size.actual > 0) {
			items.push({
				id: `${cat.id}-actual`,
				category: cat,
				label: baseLabel,
				sizeBytes: cat.size.actual,
				formattedSize: formatBytes(cat.size.actual),
				color: categoryColors[cat.type].actual,
				isSymlink: false,
			})
		}

		if (cat.size.symlink > 0) {
			items.push({
				id: `${cat.id}-symlink`,
				category: cat,
				label: `${baseLabel} (${formatMessage(storageMessages.symlinkLabel)})`,
				sizeBytes: cat.size.symlink,
				formattedSize: formatBytes(cat.size.symlink),
				color: categoryColors[cat.type].symlink,
				isSymlink: true,
			})
		}
	}

	const total = items.reduce((acc, item) => acc + item.sizeBytes, 0) || 1
	let currentAngle = 0

	return items.map((item) => {
		const ratio = item.sizeBytes / total
		const angle = ratio * 360
		const startAngle = currentAngle
		const endAngle = currentAngle + angle
		currentAngle = endAngle

		return {
			...item,
			startAngle,
			endAngle,
			percentText: `${(ratio * 100).toFixed(1)}%`,
			pathData: getRingPath(50, 50, 26, 48, startAngle, endAngle), // 外径48，内径26，加粗环形
		} as ChartItem
	})
})

function formatDateTime(date: Date) {
	return new Intl.DateTimeFormat(undefined, {
		dateStyle: 'medium',
		timeStyle: 'short',
	}).format(date)
}
</script>

<template>
	<div class="storage-page">
		<div v-if="loading && !tree" class="storage-loading">
			<SpinnerIcon class="size-8 animate-spin text-brand" />
			<span>{{ formatMessage(storageMessages.scanning) }}</span>
		</div>

		<template v-else-if="tree">
			<!-- 顶部面板：总览 + 更新按钮 + 饼图 -->
			<section class="storage-dashboard">
				<div class="storage-total-card">
					<h2 class="storage-total-title">
						{{ formatMessage(storageMessages.total) }}
					</h2>

					<div class="storage-total-value">
						<span class="total-actual">{{ formatBytes(tree.total.actual) }}</span>
						<span v-if="tree.total.symlink > 0" class="total-symlink">
							+ {{ formatBytes(tree.total.symlink) }} ({{ formatMessage(storageMessages.symlinkLabel) }})
						</span>
					</div>

					<div class="storage-actions">
						<button class="btn min-w-max" :disabled="loading" @click="updateStorage">
							<SpinnerIcon v-if="loading" class="size-4 animate-spin" />
							<RefreshCwIcon v-else />
							{{ formatMessage(loading ? storageMessages.updating : storageMessages.update) }}
						</button>
						<span v-if="lastUpdated" class="storage-last-updated">
							<span>{{ formatMessage(storageMessages.lastUpdatedLabel) }}</span>
							<span class="storage-last-updated-time">{{ formatDateTime(lastUpdated) }}</span>
						</span>
					</div>
				</div>

				<div v-if="mainCategories.length > 0" class="storage-chart-section">
					<div class="storage-pie-wrapper">
						<svg class="storage-pie-svg" viewBox="0 0 100 100">
							<path
								v-for="slice in chartSlices"
								:key="slice.id"
								:d="slice.pathData"
								:fill="slice.color"
								class="pie-path"
								:class="{
									'is-hovered': hoveredId === slice.id,
									'is-dimmed': hoveredId !== null && hoveredId !== slice.id,
								}"
								@mouseenter="hoveredId = slice.id"
								@mouseleave="hoveredId = null"
								@click="handleAction(slice.category)"
							/>
						</svg>
					</div>

					<div class="storage-legend">
						<button
							v-for="slice in chartSlices"
							:key="slice.id"
							type="button"
							class="legend-item"
							:class="{
								'is-hovered': hoveredId === slice.id,
								'is-dimmed': hoveredId !== null && hoveredId !== slice.id,
							}"
							@mouseenter="hoveredId = slice.id"
							@mouseleave="hoveredId = null"
							@click="handleAction(slice.category)"
						>
							<span
								class="legend-dot"
								:class="{ 'is-symlink-dot': slice.isSymlink }"
								:style="{ backgroundColor: slice.color }"
							/>

							<div class="legend-info">
								<span class="legend-name">{{ slice.label }}</span>
								<span class="legend-size">{{ slice.formattedSize }}</span>
								<span class="legend-percent">{{ slice.percentText }}</span>
							</div>
						</button>
					</div>
				</div>
			</section>

			<!-- 实例树节点列表 -->
			<section v-if="instancesCategory" class="storage-instance-section">
				<div class="instance-help" v-tooltip="symlinkHelpTooltipOptions">
					<HelpCircleIcon class="instance-help-icon" aria-hidden="true" />
					<span>{{ formatMessage(storageMessages.symlinkHelp) }}</span>
				</div>

				<div class="instance-heading">
					<span class="storage-section-title">
						{{ formatMessage(storageMessages.instanceData) }}
					</span>

					<span class="storage-section-size">
						{{ formatSize(instancesCategory.size) }}
					</span>
				</div>

				<div class="storage-tree">
					<StorageTreeNode
						v-for="child in sortStorageChildren(instancesCategory.children)"
						:key="child.id"
						:node="child"
						:depth="0"
						:parent-total="sizeTotal(instancesCategory.size)"
						@action="handleAction"
					/>
				</div>
			</section>
		</template>
	</div>
</template>

<style scoped>
.storage-page {
	display: flex;
	flex-direction: column;
	width: 100%;
	color: var(--color-contrast);
}

.storage-loading {
	display: flex;
	align-items: center;
	justify-content: center;
	gap: 0.75rem;
	padding: 3rem 0;
	color: var(--color-secondary);
}

/* 顶栏卡片布局 */
.storage-dashboard {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 2rem;
	padding: 1.25rem 1.5rem;
	border: 1px solid var(--surface-5);
	border-radius: 0.75rem;
	background: var(--surface-1, transparent);
	overflow: hidden;
}

/* 左侧总大小区域 */
.storage-total-card {
	display: flex;
	flex-direction: column;
	gap: 0.375rem;
	flex-shrink: 0;
}

.storage-total-title {
	margin: 0;
	font-size: 0.875rem;
	font-weight: 500;
	color: var(--color-secondary);
}

.storage-total-value {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
	font-variant-numeric: tabular-nums;
}

.total-actual {
	font-size: 1.875rem;
	font-weight: 700;
	line-height: 1.1;
	color: var(--color-contrast);
}

.total-symlink {
	font-size: 0.8125rem;
	font-weight: 500;
	color: var(--color-secondary);
}

.storage-actions {
	display: flex;
	flex-direction: column;
	align-items: flex-start;
	gap: 0.375rem;
	margin-top: 0.5rem;
}

.storage-last-updated {
	display: flex;
	flex-direction: column;
	font-size: 0.75rem;
	color: var(--color-secondary);
}

.storage-last-updated-time {
	font-variant-numeric: tabular-nums;
}

/* 右侧核心区域：强制向右对齐 */
.storage-chart-section {
	display: flex;
	align-items: center;
	gap: 1.5rem;
	margin-left: auto;
}

/* SVG 饼图包裹层 (较大且加粗) */
.storage-pie-wrapper {
	position: relative;
	width: 130px;
	height: 130px;
	flex-shrink: 0;
}

.storage-pie-svg {
	width: 100%;
	height: 100%;
	overflow: visible;
}

.pie-path {
	cursor: pointer;
	transition:
		transform 150ms ease,
		opacity 150ms ease,
		filter 150ms ease;
	transform-origin: 50px 50px;
}

.pie-path.is-hovered {
	transform: scale(1.06);
	opacity: 1;
	filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.25));
}

.pie-path.is-dimmed {
	opacity: 0.3;
}

/* 图例布局 */
.storage-legend {
	display: grid;
	grid-template-columns: repeat(2, auto);
	gap: 0.625rem 1.25rem;
}

.legend-item {
	display: flex;
	align-items: flex-start;
	gap: 0.5rem;
	padding: 0.375rem 0.5rem;
	border: 0;
	border-radius: 0.375rem;
	background: transparent;
	color: inherit;
	text-align: left;
	cursor: pointer;
	transition:
		background-color 120ms ease,
		opacity 150ms ease;
}

.legend-item.is-hovered {
	background: var(--surface-3);
}

.legend-item.is-dimmed {
	opacity: 0.3;
}

.legend-item:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: -1px;
}

.legend-dot {
	width: 0.625rem;
	height: 0.625rem;
	margin-top: 0.25rem;
	border-radius: 50%;
	flex-shrink: 0;
}

.legend-dot.is-symlink-dot {
	border-radius: 2px;
}

.legend-info {
	display: flex;
	flex-direction: column;
	line-height: 1.25;
}

.legend-name {
	font-size: 0.8125rem;
	font-weight: 600;
	color: var(--color-contrast);
	white-space: nowrap;
}

.legend-size {
	font-size: 0.75rem;
	font-variant-numeric: tabular-nums;
	color: var(--color-secondary);
	white-space: nowrap;
}

/* 百分比单独一行显示，避免超过卡片 */
.legend-percent {
	display: block;
	font-size: 0.71875rem;
	font-weight: 500;
	font-variant-numeric: tabular-nums;
	color: var(--color-secondary);
	opacity: 0.8;
}

/* 实例树样式 */
.storage-instance-section {
	margin-top: 1.75rem;
}

.instance-heading {
	display: flex;
	align-items: baseline;
	gap: 0.75rem;
	min-width: 0;
}

.storage-section-title {
	font-size: 0.9375rem;
	font-weight: 600;
	line-height: 1.375rem;
	color: var(--color-contrast);
}

.storage-section-size {
	font-size: 0.8125rem;
	font-variant-numeric: tabular-nums;
	color: var(--color-secondary);
}

.instance-help {
	display: inline-flex;
	align-items: center;
	gap: 0.3125rem;
	margin-bottom: 0.375rem;
	font-size: 0.75rem;
	line-height: 1.25rem;
	color: var(--color-secondary);
	cursor: help;
}

.instance-help-icon {
	width: 0.875rem;
	height: 0.875rem;
	flex-shrink: 0;
}

.storage-tree {
	display: flex;
	flex-direction: column;
	margin-top: 0.25rem;
}

/* 响应式支持 */
@media (max-width: 860px) {
	.storage-dashboard {
		flex-direction: column;
		align-items: flex-start;
		gap: 1.25rem;
	}

	.storage-chart-section {
		margin-left: 0;
		width: 100%;
		justify-content: flex-start;
	}
}

@media (max-width: 520px) {
	.storage-chart-section {
		flex-direction: column;
		align-items: flex-start;
	}

	.storage-legend {
		grid-template-columns: 1fr;
	}
}

/* 存储页多行 tooltip：内容换行并限制宽度 */
:global(.v-popper__popper.storage-tooltip .v-popper__inner) {
	white-space: pre-line;
	max-width: 22rem;
}
</style>
