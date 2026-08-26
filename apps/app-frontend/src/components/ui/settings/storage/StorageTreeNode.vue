<script setup lang="ts">
import { computed, ref } from 'vue'

import type { StorageNode } from './storageData'
import { sortStorageChildren } from './storageData'
import StorageTreeRow from './StorageTreeRow.vue'

defineOptions({ name: 'StorageTreeNode' })

const props = defineProps<{
	node: StorageNode
	depth: number
	parentTotal: number
}>()

const emit = defineEmits<{
	action: [node: StorageNode]
}>()

const hasChildren = computed(() => (props.node.children?.length ?? 0) > 0)
const expanded = ref(false)
const totalSize = computed(() => props.node.size.actual + props.node.size.symlink)
const visibleChildren = computed(() => sortStorageChildren(props.node.children))

function onToggle(event: Event) {
	expanded.value = (event.target as HTMLDetailsElement).open
}
</script>

<template>
	<!-- 有子节点时使用原生 <details>/<summary> 管理展开收起 -->
	<details v-if="hasChildren" class="flex flex-col" @toggle="onToggle">
		<summary class="tree-row-reveal">
			<StorageTreeRow
				:node="node"
				:depth="depth"
				:parent-total="parentTotal"
				:expanded="expanded"
				@action="emit('action', $event)"
			/>
		</summary>

		<div class="tree-children">
			<StorageTreeNode
				v-for="child in visibleChildren"
				:key="child.id"
				:node="child"
				:depth="depth + 1"
				:parent-total="totalSize"
				@action="emit('action', $event)"
			/>
		</div>
	</details>
	<div v-else class="flex flex-col">
		<!-- 叶子节点不需要展开，直接渲染行 -->
		<StorageTreeRow
			:node="node"
			:depth="depth"
			:parent-total="parentTotal"
			:expanded="false"
			@action="emit('action', $event)"
		/>
	</div>
</template>

<style scoped>
/* 原生的 <summary> 需要去掉默认的展开三角形与缩进 */
.tree-row-reveal {
	display: block;
	list-style: none;
	user-select: none;
	cursor: pointer;
}

.tree-row-reveal::-webkit-details-marker {
	display: none;
}

.tree-row-reveal::marker {
	content: none;
}
</style>
