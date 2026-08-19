<script setup lang="ts">
import { ChevronRightIcon, FileIcon, FolderIcon, FolderOpenIcon } from '@modrinth/assets'
import { useFormatBytes, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import type { StorageNode, StorageNodeType } from './storageData'
import { storageMessages } from './storageMessages'

type MessageDescriptor = (typeof storageMessages)['total']

defineOptions({ name: 'StorageTreeRow' })

const props = defineProps<{
	node: StorageNode
	depth: number
	parentTotal: number
	expanded: boolean
}>()

const emit = defineEmits<{
	action: [node: StorageNode]
}>()

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()

const typeLabels: Record<StorageNodeType, MessageDescriptor> = {
	instances: storageMessages.instanceData,
	cache: storageMessages.cacheData,
	meta: storageMessages.metaData,
	database: storageMessages.database,
	other: storageMessages.other,
	instance: storageMessages.instance,
	mods: storageMessages.mods,
	replay: storageMessages.replay,
	resourcepacks: storageMessages.resourcepacks,
	saves: storageMessages.saves,
	world: storageMessages.world,
	schematics: storageMessages.schematics,
	screenshots: storageMessages.screenshots,
	shaderpacks: storageMessages.shaderpacks,
	minimap: storageMessages.minimap,
	'distant-horizons': storageMessages.distantHorizons,
	'db-file': storageMessages.dbFile,
	'db-backup': storageMessages.dbBackup,
}

const hasChildren = computed(() => (props.node.children?.length ?? 0) > 0)
const isDirectory = computed(() => (props.node.paths[0]?.kind ?? 'directory') !== 'file')

const actualShare = computed(() =>
	props.parentTotal > 0 ? props.node.size.actual / props.parentTotal : 0,
)

const symShare = computed(() =>
	props.parentTotal > 0 ? props.node.size.symlink / props.parentTotal : 0,
)

const totalShare = computed(() => Math.min(1, actualShare.value + symShare.value))

const percent = computed(() => Math.round(totalShare.value * 100))

const displayLabel = computed(() => props.node.name ?? formatMessage(typeLabels[props.node.type]))

const tooltipText = computed(() => props.node.paths.map((path) => path.path).join('\n'))

const nameTooltip = computed(() => ({
	content: tooltipText.value,
	popperClass: 'storage-tooltip',
}))

const actualSizeTooltip = computed(() =>
	formatMessage(storageMessages.actualSizeTooltip, {
		size: formatBytes(props.node.size.actual),
	}),
)

const symlinkSizeTooltip = computed(() =>
	formatMessage(storageMessages.symlinkSizeTooltip, {
		size: formatBytes(props.node.size.symlink),
	}),
)

const progressTitle = computed(() =>
	props.node.size.symlink > 0
		? `${actualSizeTooltip.value}\n${symlinkSizeTooltip.value}`
		: actualSizeTooltip.value,
)

const progressTooltip = computed(() => ({
	content: progressTitle.value,
	popperClass: 'storage-tooltip',
}))

const progressLabel = computed(() =>
	props.node.size.symlink > 0
		? `${actualSizeTooltip.value} ${symlinkSizeTooltip.value}`
		: actualSizeTooltip.value,
)

const actualSizeText = computed(() => formatBytes(props.node.size.actual))
const symlinkSizeText = computed(() => formatBytes(props.node.size.symlink))

const openActionLabel = computed(() => formatMessage(storageMessages.openAction))
</script>

<template>
	<div class="tree-row" :class="{ clickable: hasChildren, 'is-expanded': expanded }">
		<!-- 左侧树层级与名称部分 -->
		<div class="tree-main-col">
			<!-- 根据 depth 生成层级缩进和导轨线 -->
			<div class="indent-guides" aria-hidden="true">
				<span v-for="i in depth" :key="i" class="guide-line" />
			</div>

			<!-- 展开/折叠 箭头（装饰性，由原生的 <summary> 负责展开） -->
			<span v-if="hasChildren" class="chevron-btn chevron-icon" aria-hidden="true">
				<ChevronRightIcon class="size-3.5" />
			</span>
			<span v-else class="chevron-placeholder" aria-hidden="true" />

			<!-- 文件/文件夹按钮：打开位置（不展开树） -->
			<button
				type="button"
				class="node-type-btn"
				v-tooltip="openActionLabel"
				:aria-label="`${displayLabel}: ${openActionLabel}`"
				@click.stop="emit('action', node)"
			>
				<FolderOpenIcon v-if="hasChildren && expanded" class="node-type-icon text-brand" />
				<FolderIcon v-else-if="isDirectory" class="node-type-icon" />
				<FileIcon v-else class="node-type-icon" />
			</button>

			<!-- 节点名称 -->
			<span class="node-name" v-tooltip="nameTooltip">
				{{ displayLabel }}
			</span>

			<span v-if="node.count !== undefined" class="count-badge">
				{{ node.count }}
			</span>
		</div>

		<!-- 右侧数据与进度列 -->
		<div class="tree-meta-col">
			<div class="storage-size">
				{{ actualSizeText }}
				<span v-if="node.size.symlink > 0" class="text-secondary"> + {{ symlinkSizeText }} </span>
			</div>

			<div class="storage-percent">{{ percent }}%</div>

			<progress
				class="storage-progress"
				v-tooltip="progressTooltip"
				:value="totalShare"
				:max="1"
				:aria-label="progressLabel"
			/>
		</div>
	</div>
</template>

<style scoped>
/* 整体行布局：去卡片化、去分割线 */
.tree-row {
	display: flex;
	align-items: center;
	justify-content: space-between;
	height: 2rem;
	padding: 0 0.5rem;
	border: none;
	background: transparent;
	user-select: none;
	transition: background-color 0.1s ease;
}

.tree-row.clickable {
	cursor: pointer;
}

.tree-row:hover {
	background-color: var(--surface-2);
}

/* 左侧层级结构列 */
.tree-main-col {
	display: flex;
	align-items: center;
	min-width: 0;
	flex: 1;
	height: 100%;
}

/* 竖向缩进线容器 */
.indent-guides {
	display: flex;
	height: 100%;
	flex-shrink: 0;
}

/* 垂直导轨线：跟随主题 surface 描边色，浅色/深色模式都清晰可见 */
.guide-line {
	display: block;
	width: 1.25rem;
	height: 100%;
	position: relative;
}

.guide-line::before {
	content: '';
	position: absolute;
	left: 0.5rem;
	top: 0;
	bottom: 0;
	width: 1px;
	background-color: var(--surface-5);
	opacity: 1;
}

/* 展开状态下高亮当前父级的引导线，使用主题品牌色 */
.tree-row.is-expanded .indent-guides .guide-line:last-child::before {
	background-color: var(--color-brand);
	opacity: 1;
}

/* 折叠/展开 箭头图标 */
.chevron-btn {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	width: 1.25rem;
	height: 1.25rem;
	flex-shrink: 0;
	padding: 0;
	color: var(--color-secondary);
}

.chevron-icon {
	transition: transform 0.15s ease;
}

.tree-row.is-expanded .chevron-icon {
	transform: rotate(90deg);
}

.chevron-placeholder {
	width: 1.25rem;
	flex-shrink: 0;
}

/* 文件/文件夹打开按钮 */
.node-type-btn {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	padding: 0;
	margin-right: 0.375rem;
	border: 0;
	border-radius: 0.25rem;
	background: transparent;
	color: var(--color-secondary);
	cursor: pointer;
}

.node-type-icon {
	width: 1.125rem;
	height: 1.125rem;
	flex-shrink: 0;
}

.node-type-btn:hover {
	background: var(--surface-3);
	color: var(--color-contrast);
}

.node-type-btn:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: -1px;
}

.node-name {
	min-width: 0;
	overflow: hidden;
	font-size: 0.8125rem;
	font-weight: 400;
	color: var(--color-contrast);
	text-overflow: ellipsis;
	white-space: nowrap;
}

.count-badge {
	margin-left: 0.375rem;
	padding: 0 0.35rem;
	height: 1rem;
	border-radius: 0.25rem;
	background: var(--surface-3);
	color: var(--color-secondary);
	font-size: 0.6875rem;
	line-height: 1rem;
	font-variant-numeric: tabular-nums;
	flex-shrink: 0;
}

/* 右侧数据列组合 */
.tree-meta-col {
	display: flex;
	align-items: center;
	gap: 1rem;
	flex-shrink: 0;
	margin-left: 1rem;
}

.storage-size {
	font-size: 0.8125rem;
	font-variant-numeric: tabular-nums;
	color: var(--color-primary);
	white-space: nowrap;
	text-align: right;
	min-width: 5rem;
}

.storage-percent {
	font-size: 0.75rem;
	font-variant-numeric: tabular-nums;
	color: var(--color-secondary);
	text-align: right;
	width: 2.25rem;
}

/* 原生 <progress> 作为共享占用进度条 */
.storage-progress {
	appearance: none;
	-webkit-appearance: none;
	width: 6rem;
	height: 0.25rem;
	flex-shrink: 0;
	border: 0;
	border-radius: 9999px;
	background: var(--surface-3);
	overflow: hidden;
}

.storage-progress::-webkit-progress-bar {
	background: var(--surface-3);
	border-radius: 9999px;
}

.storage-progress::-webkit-progress-value {
	background: var(--color-brand);
	border-radius: 9999px;
}

.storage-progress::-moz-progress-bar {
	background: var(--color-brand);
	border-radius: 9999px;
}

@media (max-width: 900px) {
	.storage-progress {
		display: none;
	}
}

@media (max-width: 640px) {
	.storage-percent {
		display: none;
	}
}

/* 存储页多行 tooltip：内容换行并限制宽度 */
:global(.v-popper__popper.storage-tooltip .v-popper__inner) {
	white-space: pre-line;
	max-width: 22rem;
}
</style>
