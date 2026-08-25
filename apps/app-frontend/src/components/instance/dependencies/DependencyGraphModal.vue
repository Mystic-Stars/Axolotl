<script setup lang="ts">
import {
	ChevronDownIcon,
	ChevronRightIcon,
	CircleAlertIcon,
	GitGraphIcon,
	ListIcon,
	RotateCounterClockwiseIcon,
	SearchIcon,
	ZoomInIcon,
	ZoomOutIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	type ContentItem,
	defineMessages,
	DropdownSelect,
	NewModal,
	StyledInput,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { RouterLink } from 'vue-router'

import {
	buildDependencyGraph,
	type DependencyDirection,
	type DependencyGraph,
	type DependencyGraphEdge,
	type DependencyGraphNode,
	getDependencyTreeRows,
	getRelatedNodeIds,
	layoutDependencyGraph,
} from './dependency-graph'

const { formatMessage } = useVIntl()

const props = withDefaults(
	defineProps<{
		instanceId?: string
		instanceName?: string
		instanceIconUrl?: string
	}>(),
	{
		instanceId: undefined,
		instanceName: undefined,
		instanceIconUrl: undefined,
	},
)

const messages = defineMessages({
	header: { id: 'app.instance.dependencies.header', defaultMessage: 'Dependency relationships' },
	search: { id: 'app.instance.dependencies.search', defaultMessage: 'Search dependencies...' },
	treeView: { id: 'app.instance.dependencies.tree-view', defaultMessage: 'Dependency tree' },
	graphView: { id: 'app.instance.dependencies.graph-view', defaultMessage: 'Relationship graph' },
	requires: { id: 'app.instance.dependencies.requires', defaultMessage: 'Dependencies' },
	requiredBy: { id: 'app.instance.dependencies.required-by', defaultMessage: 'Dependents' },
	onlyRelated: {
		id: 'app.instance.dependencies.only-related',
		defaultMessage: 'Only show related content',
	},
	showIsolated: {
		id: 'app.instance.dependencies.show-isolated',
		defaultMessage: 'Show isolated content',
	},
	isolatedSummary: {
		id: 'app.instance.dependencies.isolated-summary',
		defaultMessage: '{count, number} isolated items are kept out of the relationship graph.',
	},
	allTypes: { id: 'app.instance.dependencies.all-types', defaultMessage: 'All types' },
	allSources: { id: 'app.instance.dependencies.all-sources', defaultMessage: 'All sources' },
	allStatuses: { id: 'app.instance.dependencies.all-statuses', defaultMessage: 'All statuses' },
	packManaged: { id: 'app.instance.dependencies.pack-managed', defaultMessage: 'Pack managed' },
	userAdded: { id: 'app.instance.dependencies.user-added', defaultMessage: 'User added' },
	localDiscovered: { id: 'app.instance.dependencies.local-discovered', defaultMessage: 'Local' },
	disabled: { id: 'app.instance.dependencies.disabled', defaultMessage: 'Disabled' },
	missing: { id: 'app.instance.dependencies.missing', defaultMessage: 'Missing' },
	orphaned: { id: 'app.instance.dependencies.orphaned', defaultMessage: 'Orphaned' },
	cycle: { id: 'app.instance.dependencies.cycle', defaultMessage: 'Cycle' },
	unresolved: { id: 'app.instance.dependencies.unresolved', defaultMessage: 'Unresolved' },
	noContent: {
		id: 'app.instance.dependencies.no-content',
		defaultMessage: 'No content to analyze.',
	},
	noMatches: {
		id: 'app.instance.dependencies.no-matches',
		defaultMessage: 'No dependencies match these filters.',
	},
	noDependencies: {
		id: 'app.instance.dependencies.no-dependencies',
		defaultMessage: 'No dependency edges were found.',
	},
	noGraphRelations: {
		id: 'app.instance.dependencies.no-graph-relations',
		defaultMessage: 'No visible relationships match these filters.',
	},
	graphDirection: {
		id: 'app.instance.dependencies.graph-direction',
		defaultMessage: 'Arrows point from dependent content to the content it uses.',
	},
	graphHint: {
		id: 'app.instance.dependencies.graph-hint',
		defaultMessage: 'Drag the canvas to explore. Drag a card to arrange it.',
	},
	reference: { id: 'app.instance.dependencies.reference', defaultMessage: 'Referenced elsewhere' },
	cycleReference: {
		id: 'app.instance.dependencies.cycle-reference',
		defaultMessage: 'Cycle continues here',
	},
	unresolvedLabel: {
		id: 'app.instance.dependencies.unresolved-label',
		defaultMessage: 'Dependency details are unavailable.',
	},
	directDependencies: {
		id: 'app.instance.dependencies.direct-dependencies',
		defaultMessage: 'Direct dependencies',
	},
	directDependents: {
		id: 'app.instance.dependencies.direct-dependents',
		defaultMessage: 'Direct dependents',
	},
	version: { id: 'app.instance.dependencies.version', defaultMessage: 'Version {version}' },
	file: { id: 'app.instance.dependencies.file', defaultMessage: 'File {file}' },
	source: { id: 'app.instance.dependencies.source', defaultMessage: 'Source' },
	status: { id: 'app.instance.dependencies.status', defaultMessage: 'Status' },
	stats: {
		id: 'app.instance.dependencies.stats',
		defaultMessage: '{nodes, number} items, {edges, number} relationships',
	},
	resetZoom: { id: 'app.instance.dependencies.reset-zoom', defaultMessage: 'Fit graph to view' },
	zoomIn: { id: 'app.instance.dependencies.zoom-in', defaultMessage: 'Zoom in' },
	zoomOut: { id: 'app.instance.dependencies.zoom-out', defaultMessage: 'Zoom out' },
})

type ViewMode = 'tree' | 'graph'
type Filter =
	| 'all'
	| 'pack'
	| 'user'
	| 'local'
	| 'disabled'
	| 'missing'
	| 'orphaned'
	| 'cycle'
	| 'unresolved'
type Point = { x: number; y: number }

const minZoom = 0.32
const maxZoom = 1.6
const viewportPadding = 36
const modal = ref<InstanceType<typeof NewModal>>()
const items = ref<ContentItem[]>([])
const viewMode = ref<ViewMode>('tree')
const direction = ref<DependencyDirection>('requiredBy')
const searchQuery = ref('')
const sourceFilter = ref<'all' | 'pack' | 'user' | 'local'>('all')
const typeFilter = ref('all')
const statusFilter = ref<Filter>('all')
const onlyRelated = ref(false)
const showIsolated = ref(false)
const expandedIds = ref(new Set<string>())
const selectedNodeId = ref<string>()
const zoom = ref(1)
const pan = ref<Point>({ x: 0, y: 0 })
const draggedNodeId = ref<string>()
const nodeOffsets = ref(new Map<string, Point>())
const graphViewport = ref<HTMLElement | null>(null)
const graphCanvas = ref<HTMLElement | null>(null)
const dragState = ref<{ kind: 'pan' | 'node'; id?: string }>()
let activePan: Point | undefined
let lastPointerPosition: Point | undefined
let pendingPointerMove: { dx: number; dy: number } | undefined
let pointerFrame: number | undefined
let constrainFrame: number | undefined
let viewportObserver: ResizeObserver | undefined
let fitFrame: number | undefined

const graph = computed<DependencyGraph>(() => buildDependencyGraph(items.value))
const typeOptions = computed(() => [
	'all',
	...new Set(graph.value.nodes.map((node) => node.projectType)),
])
const sourceOptions = ['all', 'pack', 'user', 'local'] as const
const statusOptions = ['all', 'disabled', 'missing', 'orphaned', 'cycle', 'unresolved'] as const

function typeFilterLabel(type: string) {
	return type === 'all' ? formatMessage(messages.allTypes) : type
}

function sourceFilterLabel(source: (typeof sourceOptions)[number]) {
	if (source === 'all') return formatMessage(messages.allSources)
	if (source === 'pack') return formatMessage(messages.packManaged)
	if (source === 'user') return formatMessage(messages.userAdded)
	return formatMessage(messages.localDiscovered)
}

const statusFilterMessages = {
	all: messages.allStatuses,
	cycle: messages.cycle,
	disabled: messages.disabled,
	missing: messages.missing,
	orphaned: messages.orphaned,
	unresolved: messages.unresolved,
} as const

function statusFilterLabel(status: (typeof statusOptions)[number]) {
	return formatMessage(statusFilterMessages[status])
}

function nodeMatchesBaseFilter(node: DependencyGraphNode): boolean {
	const query = searchQuery.value.trim().toLocaleLowerCase()
	if (query && !`${node.title} ${node.fileName ?? ''}`.toLocaleLowerCase().includes(query)) {
		return false
	}
	if (typeFilter.value !== 'all' && node.projectType !== typeFilter.value) return false
	if (sourceFilter.value !== 'all') {
		const ownershipKind =
			sourceFilter.value === 'pack'
				? 'pack_managed'
				: sourceFilter.value === 'user'
					? 'user_added'
					: 'local_discovered'
		if (node.ownershipKind !== ownershipKind) return false
	}
	if (statusFilter.value === 'disabled' && node.enabled !== false) return false
	if (
		statusFilter.value === 'missing' &&
		!['missing', 'pending_manual', 'removed'].includes(node.materializationState ?? '')
	) {
		return false
	}
	if (statusFilter.value === 'orphaned' && !node.dependency.orphaned) return false
	if (statusFilter.value === 'cycle' && !node.cycle) return false
	if (statusFilter.value === 'unresolved' && node.resolved) return false
	return true
}

const matchedNodeIds = computed(
	() => new Set(graph.value.nodes.filter(nodeMatchesBaseFilter).map((node) => node.id)),
)

const relationshipNodeIds = computed(
	() => new Set(graph.value.edges.flatMap((edge) => [edge.source, edge.target])),
)

const filteredNodeIds = computed(() => {
	if (!onlyRelated.value) return matchedNodeIds.value
	return new Set([...matchedNodeIds.value].filter((id) => relationshipNodeIds.value.has(id)))
})

const filteredEdges = computed(() =>
	graph.value.edges.filter(
		(edge) => filteredNodeIds.value.has(edge.source) && filteredNodeIds.value.has(edge.target),
	),
)

const treeRows = computed(() =>
	getDependencyTreeRows(graph.value, expandedIds.value, direction.value).filter((row) => {
		if (!row.nodeId) return true
		return filteredNodeIds.value.has(row.nodeId)
	}),
)

const selectedNode = computed(() =>
	selectedNodeId.value ? graph.value.nodeById.get(selectedNodeId.value) : undefined,
)

const hasActiveGraphFilter = computed(
	() =>
		!!searchQuery.value.trim() ||
		typeFilter.value !== 'all' ||
		sourceFilter.value !== 'all' ||
		statusFilter.value !== 'all',
)

const graphNodeIds = computed(() => {
	if (!hasActiveGraphFilter.value) return relationshipNodeIds.value
	return getRelatedNodeIds(graph.value, matchedNodeIds.value)
})

const graphLayout = computed(() =>
	layoutDependencyGraph(graph.value, graphNodeIds.value, nodeOffsets.value),
)

const isolatedNodes = computed(() => {
	const candidates = hasActiveGraphFilter.value
		? matchedNodeIds.value
		: new Set(graph.value.nodes.map((node) => node.id))
	return graph.value.nodes.filter(
		(node) => candidates.has(node.id) && !relationshipNodeIds.value.has(node.id),
	)
})

const statsText = computed(() =>
	formatMessage(messages.stats, {
		nodes: filteredNodeIds.value.size,
		edges: filteredEdges.value.length,
	}),
)

const graphStatsText = computed(() =>
	formatMessage(messages.stats, {
		nodes: graphLayout.value.nodes.length,
		edges: graphLayout.value.edges.length,
	}),
)

const treeHasRows = computed(() => treeRows.value.length > 0)
const graphStructureKey = computed(
	() =>
		`${[...graphNodeIds.value].sort().join('|')}::${graph.value.edges
			.filter((edge) => graphNodeIds.value.has(edge.source) && graphNodeIds.value.has(edge.target))
			.map((edge) => edge.id)
			.sort()
			.join('|')}`,
)

function treeRootIdsForDirection(graphData: DependencyGraph, treeDirection: DependencyDirection) {
	if (treeDirection === 'requires') return graphData.rootIds
	return graphData.nodes
		.filter((node) => !(graphData.edgesBySource.get(node.id)?.length ?? 0))
		.map((node) => node.id)
		.sort((left, right) =>
			graphData.nodeById.get(left)!.title.localeCompare(graphData.nodeById.get(right)!.title),
		)
}

function toggleExpanded(nodeId: string) {
	const next = new Set(expandedIds.value)
	if (next.has(nodeId)) next.delete(nodeId)
	else next.add(nodeId)
	expandedIds.value = next
}

function selectNode(nodeId: string) {
	selectedNodeId.value = nodeId
}

function clamp(value: number, lower: number, upper: number): number {
	return Math.min(Math.max(value, lower), upper)
}

function applyCanvasTransform(nextPan = pan.value) {
	if (!graphCanvas.value) return
	graphCanvas.value.style.transform = `translate3d(${nextPan.x}px, ${nextPan.y}px, 0) scale(${zoom.value})`
}

function constrainedPan(nextPan: Point, nextZoom = zoom.value): Point {
	const viewport = graphViewport.value
	if (!viewport) return nextPan
	const scaledWidth = graphLayout.value.width * nextZoom
	const scaledHeight = graphLayout.value.height * nextZoom
	const centerX = (viewport.clientWidth - scaledWidth) / 2
	const centerY = (viewport.clientHeight - scaledHeight) / 2
	return {
		x: clamp(
			nextPan.x,
			scaledWidth <= viewport.clientWidth
				? centerX
				: viewport.clientWidth - scaledWidth - viewportPadding,
			scaledWidth <= viewport.clientWidth ? centerX : viewportPadding,
		),
		y: clamp(
			nextPan.y,
			scaledHeight <= viewport.clientHeight
				? centerY
				: viewport.clientHeight - scaledHeight - viewportPadding,
			scaledHeight <= viewport.clientHeight ? centerY : viewportPadding,
		),
	}
}

function fitGraph() {
	const viewport = graphViewport.value
	if (!viewport || !graphLayout.value.nodes.length) return
	const availableWidth = Math.max(1, viewport.clientWidth - viewportPadding * 2)
	const availableHeight = Math.max(1, viewport.clientHeight - viewportPadding * 2)
	zoom.value = clamp(
		Math.min(
			1,
			availableWidth / graphLayout.value.width,
			availableHeight / graphLayout.value.height,
		),
		minZoom,
		maxZoom,
	)
	pan.value = constrainedPan(
		{
			x: (viewport.clientWidth - graphLayout.value.width * zoom.value) / 2,
			y: (viewport.clientHeight - graphLayout.value.height * zoom.value) / 2,
		},
		zoom.value,
	)
	applyCanvasTransform()
}

function constrainGraphPan() {
	constrainFrame = undefined
	pan.value = constrainedPan(activePan ?? pan.value)
	activePan = undefined
	applyCanvasTransform()
}

function schedulePanConstraint() {
	if (constrainFrame) cancelAnimationFrame(constrainFrame)
	constrainFrame = requestAnimationFrame(constrainGraphPan)
}

function scheduleGraphFit() {
	if (fitFrame) cancelAnimationFrame(fitFrame)
	fitFrame = requestAnimationFrame(() => {
		fitFrame = undefined
		if (viewMode.value === 'graph') fitGraph()
	})
}

function resetGraphView() {
	nodeOffsets.value = new Map()
	nextTick(scheduleGraphFit)
}

function zoomTo(nextZoom: number, anchor?: Point) {
	const viewport = graphViewport.value
	const clampedZoom = clamp(nextZoom, minZoom, maxZoom)
	if (!viewport || clampedZoom === zoom.value) return
	const focalPoint = anchor ?? { x: viewport.clientWidth / 2, y: viewport.clientHeight / 2 }
	const graphPoint = {
		x: (focalPoint.x - pan.value.x) / zoom.value,
		y: (focalPoint.y - pan.value.y) / zoom.value,
	}
	zoom.value = clampedZoom
	pan.value = constrainedPan(
		{
			x: focalPoint.x - graphPoint.x * clampedZoom,
			y: focalPoint.y - graphPoint.y * clampedZoom,
		},
		clampedZoom,
	)
	applyCanvasTransform()
}

function handleWheel(event: WheelEvent) {
	const viewport = graphViewport.value
	if (!viewport) return
	event.preventDefault()
	const rect = viewport.getBoundingClientRect()
	zoomTo(zoom.value * (event.deltaY > 0 ? 0.88 : 1.12), {
		x: event.clientX - rect.left,
		y: event.clientY - rect.top,
	})
}

function startPan(event: PointerEvent) {
	if ((event.target as Element)?.closest('[data-dependency-node], [data-dependency-control]'))
		return
	selectedNodeId.value = undefined
	dragState.value = { kind: 'pan' }
	lastPointerPosition = { x: event.clientX, y: event.clientY }
	;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
}

function startNodeDrag(event: PointerEvent, nodeId: string) {
	event.stopPropagation()
	selectNode(nodeId)
	draggedNodeId.value = nodeId
	dragState.value = { kind: 'node', id: nodeId }
	lastPointerPosition = { x: event.clientX, y: event.clientY }
	;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
}

function applyPointerMove() {
	pointerFrame = undefined
	const state = dragState.value
	const movement = pendingPointerMove
	pendingPointerMove = undefined
	if (!state || !movement) return
	if (state.kind === 'pan') {
		activePan = constrainedPan({
			x: (activePan ?? pan.value).x + movement.dx,
			y: (activePan ?? pan.value).y + movement.dy,
		})
		applyCanvasTransform(activePan)
		return
	}
	if (!state.id) return
	const offset = nodeOffsets.value.get(state.id) ?? { x: 0, y: 0 }
	const nextOffsets = new Map(nodeOffsets.value)
	nextOffsets.set(state.id, {
		x: offset.x + movement.dx / zoom.value,
		y: offset.y + movement.dy / zoom.value,
	})
	nodeOffsets.value = nextOffsets
}

function movePointer(event: PointerEvent) {
	if (!dragState.value || !lastPointerPosition) return
	const dx = event.clientX - lastPointerPosition.x
	const dy = event.clientY - lastPointerPosition.y
	lastPointerPosition = { x: event.clientX, y: event.clientY }
	const pending = pendingPointerMove ?? { dx: 0, dy: 0 }
	pendingPointerMove = { dx: pending.dx + dx, dy: pending.dy + dy }
	if (!pointerFrame) pointerFrame = requestAnimationFrame(applyPointerMove)
}

function endPointer() {
	if (pointerFrame) {
		cancelAnimationFrame(pointerFrame)
		pointerFrame = undefined
	}
	applyPointerMove()
	if (activePan) {
		pan.value = activePan
		activePan = undefined
	}
	lastPointerPosition = undefined
	dragState.value = undefined
	draggedNodeId.value = undefined
}

function nodeLink(node: DependencyGraphNode) {
	if (!node.resolved || !node.projectId) return undefined
	if (node.provider === 'curseforge')
		return `/project/curseforge/${node.projectId.replace(/^curseforge:/, '')}`
	if (node.provider === 'modrinth') return `/project/${node.projectId}`
	return undefined
}

function nodeLinkQuery() {
	return props.instanceId ? { i: props.instanceId, from: 'instance-content' } : undefined
}

function statusLabel(node: DependencyGraphNode) {
	if (!node.resolved) return formatMessage(messages.unresolved)
	if (node.cycle) return formatMessage(messages.cycle)
	if (node.dependency.orphaned) return formatMessage(messages.orphaned)
	if (node.materializationState === 'missing' || node.materializationState === 'pending_manual') {
		return formatMessage(messages.missing)
	}
	if (node.enabled === false) return formatMessage(messages.disabled)
	return undefined
}

function sourceLabel(node: DependencyGraphNode) {
	if (node.ownershipKind === 'pack_managed') return formatMessage(messages.packManaged)
	if (node.ownershipKind === 'user_added') return formatMessage(messages.userAdded)
	if (node.ownershipKind === 'local_discovered') return formatMessage(messages.localDiscovered)
	return node.provider
}

function nodeStatusClass(node: DependencyGraphNode) {
	if (!node.resolved) return 'border-orange bg-surface-2 shadow-orange/15'
	if (node.cycle) return 'border-red bg-surface-2 shadow-red/15'
	if (node.enabled === false) return 'border-surface-4 bg-surface-2 opacity-70'
	return 'border-surface-4 bg-surface-2 shadow-black/20'
}

function edgeIsMuted(edge: DependencyGraphEdge) {
	return (
		!!selectedNodeId.value &&
		edge.source !== selectedNodeId.value &&
		edge.target !== selectedNodeId.value
	)
}

function edgeColor(edge: DependencyGraphEdge) {
	if (edgeIsMuted(edge)) return 'var(--surface-5)'
	return edge.resolved ? 'var(--color-brand)' : 'var(--color-orange)'
}

function show(contentItems: ContentItem[]) {
	items.value = [...contentItems]
	selectedNodeId.value = undefined
	searchQuery.value = ''
	typeFilter.value = 'all'
	sourceFilter.value = 'all'
	statusFilter.value = 'all'
	onlyRelated.value = false
	showIsolated.value = false
	viewMode.value = 'tree'
	direction.value = 'requiredBy'
	nodeOffsets.value = new Map()
	expandedIds.value = new Set(
		treeRootIdsForDirection(buildDependencyGraph(contentItems), 'requiredBy'),
	)
	zoom.value = 1
	pan.value = { x: 0, y: 0 }
	nextTick(() => modal.value?.show())
}

function setItems(contentItems: ContentItem[]) {
	items.value = [...contentItems]
	const ids = new Set(buildDependencyGraph(contentItems).nodes.map((node) => node.id))
	selectedNodeId.value = ids.has(selectedNodeId.value ?? '') ? selectedNodeId.value : undefined
	expandedIds.value = new Set([...expandedIds.value].filter((id) => ids.has(id)))
	nodeOffsets.value = new Map([...nodeOffsets.value].filter(([id]) => ids.has(id)))
	nextTick(scheduleGraphFit)
}

function hide() {
	modal.value?.hide()
}

watch(direction, () => {
	expandedIds.value = new Set()
})

watch(viewMode, (mode) => {
	if (mode === 'graph') nextTick(scheduleGraphFit)
})

watch(graphStructureKey, () => {
	nextTick(scheduleGraphFit)
})

watch(graphViewport, (viewport) => {
	viewportObserver?.disconnect()
	if (!viewport || typeof ResizeObserver === 'undefined') return
	viewportObserver = new ResizeObserver(schedulePanConstraint)
	viewportObserver.observe(viewport)
	nextTick(scheduleGraphFit)
})

onBeforeUnmount(() => {
	viewportObserver?.disconnect()
	if (fitFrame) cancelAnimationFrame(fitFrame)
	if (pointerFrame) cancelAnimationFrame(pointerFrame)
	if (constrainFrame) cancelAnimationFrame(constrainFrame)
})

defineExpose({ show, hide, setItems })
</script>

<template>
	<NewModal
		ref="modal"
		:max-width="'min(1180px, calc(100vw - 2rem))'"
		:width="'min(1180px, calc(100vw - 2rem))'"
		:no-padding="true"
	>
		<template #title>
			<Avatar
				v-if="props.instanceIconUrl"
				:src="props.instanceIconUrl"
				size="3rem"
				:tint-by="props.instanceName"
			/>
			<div class="flex min-w-0 flex-col">
				<span class="truncate text-lg font-extrabold text-contrast">{{
					formatMessage(messages.header)
				}}</span>
				<span v-if="props.instanceName" class="truncate text-sm font-medium text-secondary">{{
					props.instanceName
				}}</span>
			</div>
		</template>

		<div class="flex h-[min(640px,calc(100vh-8rem))] min-h-0 flex-col">
			<div
				class="flex flex-wrap items-center gap-3 border-0 border-b border-solid border-surface-4 px-6 py-4"
			>
				<StyledInput
					v-model="searchQuery"
					:icon="SearchIcon"
					:placeholder="formatMessage(messages.search)"
					clearable
					wrapper-class="min-w-[220px] flex-1"
				/>
				<div class="flex items-center gap-1 rounded-xl bg-surface-2 p-1" role="tablist">
					<ButtonStyled :type="viewMode === 'tree' ? 'chip' : 'transparent'" size="small">
						<button role="tab" :aria-selected="viewMode === 'tree'" @click="viewMode = 'tree'">
							<ListIcon class="size-4" /> {{ formatMessage(messages.treeView) }}
						</button>
					</ButtonStyled>
					<ButtonStyled :type="viewMode === 'graph' ? 'chip' : 'transparent'" size="small">
						<button role="tab" :aria-selected="viewMode === 'graph'" @click="viewMode = 'graph'">
							<GitGraphIcon class="size-4" /> {{ formatMessage(messages.graphView) }}
						</button>
					</ButtonStyled>
				</div>
			</div>

			<div
				class="flex flex-wrap items-center gap-2 border-0 border-b border-solid border-surface-4 px-6 py-3"
			>
				<DropdownSelect
					v-model="typeFilter"
					class="!w-44"
					name="dependency-type"
					:options="typeOptions"
					:display-name="typeFilterLabel"
					auto-placement
				/>
				<DropdownSelect
					v-model="sourceFilter"
					class="!w-44"
					name="dependency-source"
					:options="sourceOptions"
					:display-name="sourceFilterLabel"
					auto-placement
				/>
				<DropdownSelect
					v-model="statusFilter"
					class="!w-44"
					name="dependency-status"
					:options="statusOptions"
					:display-name="statusFilterLabel"
					auto-placement
				/>
				<div v-if="viewMode === 'tree'" class="flex items-center gap-2 text-sm text-secondary">
					<Toggle id="dependency-related-only" v-model="onlyRelated" small />
					<span>{{ formatMessage(messages.onlyRelated) }}</span>
				</div>
				<span class="ml-auto text-sm text-secondary">{{ statsText }}</span>
			</div>

			<div class="flex min-h-0 flex-1 flex-col overflow-hidden lg:flex-row">
				<div class="min-h-0 min-w-0 flex-1">
					<div
						v-if="items.length === 0"
						class="flex h-full items-center justify-center p-8 text-secondary"
					>
						{{ formatMessage(messages.noContent) }}
					</div>
					<div
						v-else-if="filteredNodeIds.size === 0"
						class="flex h-full items-center justify-center p-8 text-secondary"
					>
						{{ formatMessage(messages.noMatches) }}
					</div>
					<div v-else-if="viewMode === 'tree'" class="h-full overflow-auto p-4">
						<div class="mb-3 flex items-center gap-2">
							<ButtonStyled :type="direction === 'requires' ? 'chip' : 'transparent'" size="small">
								<button @click="direction = 'requires'">
									{{ formatMessage(messages.requires) }}
								</button>
							</ButtonStyled>
							<ButtonStyled
								:type="direction === 'requiredBy' ? 'chip' : 'transparent'"
								size="small"
							>
								<button @click="direction = 'requiredBy'">
									{{ formatMessage(messages.requiredBy) }}
								</button>
							</ButtonStyled>
						</div>
						<div
							v-if="!treeHasRows"
							class="rounded-xl border border-dashed border-surface-4 p-8 text-center text-secondary"
						>
							{{ formatMessage(messages.noDependencies) }}
						</div>
						<div v-else class="flex flex-col gap-1">
							<div
								v-for="row in treeRows"
								:key="row.id"
								role="treeitem"
								tabindex="0"
								class="group flex min-h-14 w-full items-center gap-2 rounded-xl px-2 text-left transition-colors hover:bg-surface-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
								:class="
									row.kind === 'cycle'
										? 'text-red'
										: row.kind === 'reference'
											? 'text-secondary'
											: 'text-primary'
								"
								:style="{ paddingLeft: `${row.depth * 1.5 + 0.5}rem` }"
								@click="row.nodeId && selectNode(row.nodeId)"
								@keydown.enter="row.nodeId && selectNode(row.nodeId)"
							>
								<span class="flex size-5 shrink-0 items-center justify-center">
									<button
										v-if="row.kind === 'node' && row.hasChildren"
										class="rounded p-0.5 hover:bg-surface-4"
										@click.stop="toggleExpanded(row.nodeId!)"
									>
										<ChevronDownIcon v-if="row.expanded" class="size-4" />
										<ChevronRightIcon v-else class="size-4" />
									</button>
									<CircleAlertIcon v-else-if="row.kind === 'cycle'" class="size-4" />
								</span>
								<template v-if="row.nodeId && graph.nodeById.get(row.nodeId)">
									<Avatar
										:src="graph.nodeById.get(row.nodeId)?.iconUrl"
										:alt="graph.nodeById.get(row.nodeId)?.title"
										size="2.5rem"
										no-shadow
									/>
									<div class="min-w-0 flex-1">
										<div class="truncate font-semibold text-contrast">
											{{ graph.nodeById.get(row.nodeId)?.title }}
										</div>
										<div class="mt-0.5 truncate text-xs text-secondary">
											{{
												graph.nodeById.get(row.nodeId)?.versionNumber ??
												graph.nodeById.get(row.nodeId)?.fileName
											}}
										</div>
									</div>
									<span v-if="row.kind === 'reference'" class="shrink-0 text-xs text-secondary">
										{{ formatMessage(messages.reference) }}
									</span>
									<span v-else-if="row.kind === 'cycle'" class="shrink-0 text-xs text-red">
										{{ formatMessage(messages.cycleReference) }}
									</span>
									<span
										v-else-if="statusLabel(graph.nodeById.get(row.nodeId)!)"
										class="shrink-0 rounded-md bg-surface-3 px-2 py-1 text-xs font-medium"
									>
										{{ statusLabel(graph.nodeById.get(row.nodeId)!) }}
									</span>
								</template>
							</div>
						</div>
					</div>

					<div v-else class="flex h-full min-h-0 flex-col p-4">
						<div
							class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-solid border-surface-5 bg-surface-1 shadow-sm"
						>
							<div
								class="flex shrink-0 flex-wrap items-center justify-between gap-3 border-0 border-b border-solid border-surface-4 bg-surface-2 px-4 py-3"
							>
								<div class="flex min-w-0 items-center gap-2">
									<span
										class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-brand/10 text-brand"
									>
										<GitGraphIcon class="size-4" />
									</span>
									<span class="truncate text-sm text-secondary">{{
										formatMessage(messages.graphDirection)
									}}</span>
								</div>
								<div class="flex items-center gap-3">
									<span class="shrink-0 text-xs text-secondary">{{ graphStatsText }}</span>
									<div
										v-if="isolatedNodes.length"
										class="flex items-center gap-2 text-xs text-secondary"
									>
										<Toggle id="dependency-show-isolated" v-model="showIsolated" small />
										<span>{{ formatMessage(messages.showIsolated) }}</span>
									</div>
								</div>
							</div>

							<div
								v-if="graphLayout.edges.length === 0"
								class="flex min-h-0 flex-1 items-center justify-center p-8 text-center text-secondary"
							>
								{{ formatMessage(messages.noGraphRelations) }}
							</div>
							<div
								v-else
								ref="graphViewport"
								class="dependency-graph-viewport relative min-h-0 flex-1 overflow-hidden touch-none"
								@wheel="handleWheel"
								@pointerdown="startPan"
								@pointermove="movePointer"
								@pointerup="endPointer"
								@pointercancel="endPointer"
							>
								<div
									data-dependency-control
									class="absolute right-3 top-3 z-20 flex items-center gap-1 rounded-xl border border-solid border-surface-4 bg-surface-2 p-1 shadow-lg"
								>
									<ButtonStyled circular type="transparent" size="small">
										<button
											:aria-label="formatMessage(messages.zoomOut)"
											@click="zoomTo(zoom - 0.1)"
										>
											<ZoomOutIcon />
										</button>
									</ButtonStyled>
									<span class="min-w-10 text-center text-xs tabular-nums text-secondary">
										{{ Math.round(zoom * 100) }}%
									</span>
									<ButtonStyled circular type="transparent" size="small">
										<button
											:aria-label="formatMessage(messages.zoomIn)"
											@click="zoomTo(zoom + 0.1)"
										>
											<ZoomInIcon />
										</button>
									</ButtonStyled>
									<ButtonStyled circular type="transparent" size="small">
										<button :aria-label="formatMessage(messages.resetZoom)" @click="resetGraphView">
											<RotateCounterClockwiseIcon />
										</button>
									</ButtonStyled>
								</div>

								<div
									class="pointer-events-none absolute bottom-3 left-3 z-20 max-w-[min(28rem,calc(100%-6rem))] rounded-lg border border-solid border-surface-4 bg-surface-2 px-2.5 py-1.5 text-xs text-secondary shadow-sm"
								>
									{{ formatMessage(messages.graphHint) }}
								</div>

								<div
									ref="graphCanvas"
									class="dependency-graph-canvas relative"
									:style="{
										width: `${graphLayout.width}px`,
										height: `${graphLayout.height}px`,
										transform: `translate3d(${pan.x}px, ${pan.y}px, 0) scale(${zoom})`,
										transformOrigin: 'top left',
									}"
								>
									<div
										v-for="component in graphLayout.components"
										:key="component.id"
										class="dependency-graph-component"
										:style="{
											left: `${component.x - 20}px`,
											top: `${component.y - 20}px`,
											width: `${component.width + 40}px`,
											height: `${component.height + 40}px`,
										}"
									/>
									<div
										v-for="edge in graphLayout.edges"
										:key="edge.id"
										class="dependency-graph-connector"
										:class="{
											'dependency-graph-connector-muted': edgeIsMuted(edge),
											'dependency-graph-connector-unresolved': !edge.resolved,
										}"
										:style="{
											'--connector-length': `${edge.connector.length}px`,
											left: `${edge.connector.x}px`,
											top: `${edge.connector.y}px`,
											transform: `rotate(${edge.connector.rotation}deg)`,
										}"
									>
										<span class="dependency-graph-connector-line" />
										<span class="dependency-graph-connector-arrow" />
									</div>

									<div
										v-for="node in graphLayout.nodes"
										:key="node.id"
										data-dependency-node
										class="dependency-graph-node absolute flex h-[76px] w-[228px] cursor-grab items-center gap-3 rounded-2xl border-2 px-3 shadow-lg transition-[box-shadow,opacity,transform] active:cursor-grabbing"
										:class="[
											nodeStatusClass(node),
											selectedNodeId && selectedNodeId !== node.id ? 'opacity-35' : '',
											draggedNodeId === node.id ? 'z-10 scale-[1.03] shadow-xl' : '',
										]"
										:style="{ left: `${node.x}px`, top: `${node.y}px` }"
										@pointerdown="(event) => startNodeDrag(event, node.id)"
										@click.stop="selectNode(node.id)"
									>
										<span class="dependency-graph-port dependency-graph-port-input" />
										<Avatar :src="node.iconUrl" :alt="node.title" size="2.75rem" no-shadow />
										<div class="min-w-0 flex-1">
											<div class="truncate text-sm font-bold text-contrast">{{ node.title }}</div>
											<div class="mt-0.5 truncate text-xs text-secondary">
												{{ statusLabel(node) ?? sourceLabel(node) }}
											</div>
										</div>
										<span class="dependency-graph-port dependency-graph-port-output" />
									</div>
								</div>
							</div>

							<div
								v-if="isolatedNodes.length"
								class="shrink-0 border-0 border-t border-solid border-surface-4 bg-surface-2 px-4 py-3"
							>
								<p class="m-0 text-xs text-secondary">
									{{ formatMessage(messages.isolatedSummary, { count: isolatedNodes.length }) }}
								</p>
								<div
									v-if="showIsolated"
									class="mt-3 flex max-h-24 flex-wrap gap-2 overflow-auto pr-1"
								>
									<button
										v-for="node in isolatedNodes"
										:key="node.id"
										class="flex max-w-52 items-center gap-2 rounded-lg border border-solid border-surface-4 bg-surface-1 px-2 py-1.5 text-left text-xs text-secondary transition-colors hover:border-brand hover:text-contrast"
										@click="selectNode(node.id)"
									>
										<Avatar :src="node.iconUrl" :alt="node.title" size="1.5rem" no-shadow />
										<span class="truncate">{{ node.title }}</span>
									</button>
								</div>
							</div>
						</div>
					</div>
				</div>

				<aside
					v-if="selectedNode"
					class="flex min-h-0 w-full shrink-0 flex-col gap-4 overflow-y-auto border-0 border-t border-solid border-surface-4 bg-surface-1 p-5 lg:w-[310px] lg:border-l lg:border-t-0"
				>
					<div class="flex items-center gap-3">
						<Avatar :src="selectedNode.iconUrl" :alt="selectedNode.title" size="3rem" no-shadow />
						<div class="min-w-0">
							<RouterLink
								v-if="nodeLink(selectedNode)"
								:to="{ path: nodeLink(selectedNode), query: nodeLinkQuery() }"
								class="block truncate font-semibold text-contrast hover:underline"
							>
								{{ selectedNode.title }}
							</RouterLink>
							<span v-else class="block truncate font-semibold text-contrast">{{
								selectedNode.title
							}}</span>
							<span class="text-sm text-secondary">{{ sourceLabel(selectedNode) }}</span>
						</div>
					</div>
					<p v-if="!selectedNode.resolved" class="m-0 text-sm text-orange">
						{{ formatMessage(messages.unresolvedLabel) }}
					</p>
					<div class="flex flex-col gap-2 text-sm">
						<span v-if="selectedNode.versionNumber">{{
							formatMessage(messages.version, { version: selectedNode.versionNumber })
						}}</span>
						<span v-if="selectedNode.fileName">{{
							formatMessage(messages.file, { file: selectedNode.fileName })
						}}</span>
						<span>{{ formatMessage(messages.source) }}: {{ sourceLabel(selectedNode) }}</span>
						<span v-if="statusLabel(selectedNode)">
							{{ formatMessage(messages.status) }}: {{ statusLabel(selectedNode) }}
						</span>
					</div>
					<div class="flex flex-col gap-2">
						<strong class="text-sm text-contrast">{{
							formatMessage(messages.directDependencies)
						}}</strong>
						<button
							v-for="edge in graph.edgesBySource.get(selectedNode.id)"
							:key="edge.id"
							class="truncate text-left text-sm text-brand hover:underline"
							@click="selectNode(edge.target)"
						>
							{{ graph.nodeById.get(edge.target)?.title }}
						</button>
						<span
							v-if="!graph.edgesBySource.get(selectedNode.id)?.length"
							class="text-sm text-secondary"
						>
							-
						</span>
					</div>
					<div class="flex flex-col gap-2">
						<strong class="text-sm text-contrast">{{
							formatMessage(messages.directDependents)
						}}</strong>
						<button
							v-for="edge in graph.edgesByTarget.get(selectedNode.id)"
							:key="edge.id"
							class="truncate text-left text-sm text-brand hover:underline"
							@click="selectNode(edge.source)"
						>
							{{ graph.nodeById.get(edge.source)?.title }}
						</button>
						<span
							v-if="!graph.edgesByTarget.get(selectedNode.id)?.length"
							class="text-sm text-secondary"
						>
							-
						</span>
					</div>
				</aside>
			</div>
		</div>
	</NewModal>
</template>

<style scoped>
.dependency-graph-viewport {
	background-color: var(--surface-1);
	background-image:
		linear-gradient(color-mix(in srgb, var(--surface-4) 76%, transparent) 1px, transparent 1px),
		linear-gradient(
			90deg,
			color-mix(in srgb, var(--surface-4) 76%, transparent) 1px,
			transparent 1px
		);
	background-position: -1px -1px;
	background-size: 24px 24px;
}

.dependency-graph-canvas {
	will-change: transform;
}

.dependency-graph-component {
	position: absolute;
	z-index: 0;
	border: 1px solid var(--surface-4);
	border-radius: 22px;
	background: color-mix(in srgb, var(--surface-2) 70%, transparent);
}

.dependency-graph-connector {
	position: absolute;
	z-index: 1;
	display: block;
	width: var(--connector-length);
	height: 0;
	transform-origin: 0 50%;
}

.dependency-graph-connector-line {
	position: absolute;
	inset: -5px 0 auto;
	display: block;
	height: 10px;
	border-radius: 999px;
	background: var(--surface-5);
}

.dependency-graph-connector-line::after {
	position: absolute;
	top: 3px;
	right: 0;
	left: 0;
	display: block;
	height: 4px;
	border-radius: inherit;
	background: var(--color-brand);
	content: '';
}

.dependency-graph-connector-arrow {
	position: absolute;
	top: -8px;
	right: -1px;
	width: 0;
	height: 0;
	border-top: 8px solid transparent;
	border-bottom: 8px solid transparent;
	border-left: 12px solid var(--color-brand);
}

.dependency-graph-connector-unresolved .dependency-graph-connector-line::after {
	background: var(--color-orange);
}

.dependency-graph-connector-unresolved .dependency-graph-connector-arrow {
	border-left-color: var(--color-orange);
}

.dependency-graph-connector-muted {
	opacity: 0.28;
}

.dependency-graph-node {
	z-index: 2;
}

.dependency-graph-port {
	position: absolute;
	top: 50%;
	display: block;
	width: 9px;
	height: 9px;
	border: 2px solid var(--surface-2);
	border-radius: 999px;
	background: var(--color-brand);
	box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-brand) 50%, transparent);
	transform: translateY(-50%);
}

.dependency-graph-port-input {
	left: -6px;
}

.dependency-graph-port-output {
	right: -6px;
}
</style>
