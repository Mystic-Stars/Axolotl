import assert from 'node:assert/strict'
import test from 'node:test'

import type { ContentItem } from '@modrinth/ui'

import {
	buildDependencyGraph,
	dependencyGraphMetrics,
	getConnectedComponents,
	getDependencyTreeRows,
	getRelatedNodeIds,
	layoutDependencyGraph,
} from './dependency-graph.ts'

type Ref = { provider: 'modrinth'; projectId: string; releaseId: string }

type ItemOptions = {
	title?: string
	requires?: Ref[]
	requiredBy?: Ref[]
	autoDependency?: boolean
	entryId?: string
	filePath?: string
}

function item(projectId: string, versionId: string, options: ItemOptions = {}): ContentItem {
	return {
		id: projectId,
		file_name: `${projectId}.jar`,
		file_path: options.filePath ?? `mods/${projectId}.jar`,
		size: 1,
		enabled: true,
		project_type: 'mod',
		project: { id: projectId, slug: projectId, title: options.title ?? projectId, icon_url: null },
		version: { id: versionId, version_number: versionId, file_name: `${projectId}.jar` },
		update: null,
		origin_provider: 'modrinth',
		provider_refs: [{ provider: 'modrinth', project_id: projectId, version_id: versionId }],
		instanceEntryId: options.entryId,
		dependency: {
			autoDependency: options.autoDependency ?? false,
			requires: options.requires ?? [],
			requiredBy: options.requiredBy ?? [],
			orphaned: false,
		},
	} as ContentItem
}

const ref = (projectId: string, releaseId = '1') => ({
	provider: 'modrinth' as const,
	projectId,
	releaseId,
})

function nodeId(projectId: string) {
	return `item:modrinth:${projectId}:1`
}

test('builds dependency edges, roots, shared nodes, and relationship layout', () => {
	const graph = buildDependencyGraph([
		item('a', '1', { requires: [ref('b')] }),
		item('c', '1', { requires: [ref('b')] }),
		item('b', '1'),
	])

	assert.equal(graph.edges.length, 2)
	assert.equal(graph.rootIds.length, 2)
	assert.equal(graph.nodeById.get(nodeId('b'))?.shared, true)
	assert.equal(layoutDependencyGraph(graph).edges.length, 2)
})

test('keeps unresolved dependency targets visible', () => {
	const graph = buildDependencyGraph([item('a', '1', { requires: [ref('missing', '9')] })])
	assert.equal(graph.unresolvedIds.size, 1)
	assert.equal(graph.edges[0]?.resolved, false)
	assert.equal(graph.nodeById.get('missing:modrinth:missing:9')?.title, 'missing')
})

test('deduplicates edges and stops tree traversal at cycles and shared references', () => {
	const graph = buildDependencyGraph([
		item('a', '1', { requires: [ref('b'), ref('b')] }),
		item('b', '1', { requires: [ref('a')] }),
	])
	assert.equal(graph.edges.length, 2)
	assert.equal(graph.cycleIds.size, 2)

	const rows = getDependencyTreeRows(graph, new Set([nodeId('a'), nodeId('b')]))
	assert.ok(rows.some((row) => row.kind === 'cycle'))
	assert.ok(rows.length < 6)
})

test('preserves isolated content as a root without dependency edges', () => {
	const graph = buildDependencyGraph([item('standalone', '1')])
	assert.deepEqual(graph.rootIds, [nodeId('standalone')])
	assert.deepEqual(getDependencyTreeRows(graph, new Set()), [
		{
			id: `node:${nodeId('standalone')}:0`,
			nodeId: nodeId('standalone'),
			depth: 0,
			kind: 'node',
			hasChildren: false,
			expanded: false,
		},
	])
	assert.equal(layoutDependencyGraph(graph).nodes.length, 0)
})

test('keeps duplicate installed copies as distinct nodes and connects each matching copy', () => {
	const graph = buildDependencyGraph([
		item('parent', '1', { requires: [ref('library')] }),
		item('library', '1', { entryId: 'library-a', filePath: 'mods/library-a.jar' }),
		item('library', '1', { entryId: 'library-b', filePath: 'mods/library-b.jar' }),
	])

	assert.equal(graph.nodes.filter((node) => node.projectId === 'library').length, 2)
	assert.equal(graph.edges.length, 2)
	assert.ok(graph.nodeById.has('item:entry:library-a'))
	assert.ok(graph.nodeById.has('item:entry:library-b'))
})

test('partitions unrelated relationships into compact graph components', () => {
	const graph = buildDependencyGraph([
		item('a', '1', { requires: [ref('b')] }),
		item('b', '1'),
		item('c', '1', { requires: [ref('d')] }),
		item('d', '1'),
		item('isolated', '1'),
	])
	const relationshipIds = new Set(graph.edges.flatMap((edge) => [edge.source, edge.target]))
	const components = getConnectedComponents(graph, relationshipIds)
	const layout = layoutDependencyGraph(graph)

	assert.equal(components.length, 2)
	assert.deepEqual(
		components.map((component) => component.nodeIds.length),
		[2, 2],
	)
	assert.equal(layout.components.length, 2)
	assert.equal(layout.nodes.length, 4)
	assert.equal(
		layout.nodes.some((node) => node.id === nodeId('isolated')),
		false,
	)
})

test('returns the complete relationship context for a filtered node', () => {
	const graph = buildDependencyGraph([
		item('a', '1', { requires: [ref('b')] }),
		item('b', '1', { requires: [ref('c')] }),
		item('c', '1'),
		item('isolated', '1'),
	])
	const related = getRelatedNodeIds(graph, new Set([nodeId('b')]))

	assert.deepEqual(related, new Set([nodeId('a'), nodeId('b'), nodeId('c')]))
})

test('connects each edge from a source output port to a target input port with an HTML connector', () => {
	const graph = buildDependencyGraph([item('a', '1', { requires: [ref('b')] }), item('b', '1')])
	const layout = layoutDependencyGraph(graph)
	const source = layout.nodes.find((node) => node.id === nodeId('a'))!
	const target = layout.nodes.find((node) => node.id === nodeId('b'))!
	const edge = layout.edges[0]!
	const expectedStartX =
		source.x + dependencyGraphMetrics.nodeWidth + dependencyGraphMetrics.edgeClearance
	const expectedStartY = source.y + dependencyGraphMetrics.nodeHeight / 2
	const expectedEndX = target.x - dependencyGraphMetrics.edgeClearance
	const expectedEndY = target.y + dependencyGraphMetrics.nodeHeight / 2

	assert.equal(edge.connector.x, expectedStartX)
	assert.equal(edge.connector.y, expectedStartY)
	assert.equal(
		edge.connector.length,
		Math.hypot(expectedEndX - expectedStartX, expectedEndY - expectedStartY),
	)
	assert.equal(
		edge.connector.rotation,
		(Math.atan2(expectedEndY - expectedStartY, expectedEndX - expectedStartX) * 180) / Math.PI,
	)
})

test('keeps output and input ports stable for reverse cycle connectors', () => {
	const graph = buildDependencyGraph([
		item('a', '1', { requires: [ref('b')] }),
		item('b', '1', { requires: [ref('a')] }),
	])
	const layout = layoutDependencyGraph(graph)
	const reverseEdge = layout.edges.find((edge) => edge.source === nodeId('b'))!
	const source = layout.nodes.find((node) => node.id === reverseEdge.source)!
	const target = layout.nodes.find((node) => node.id === reverseEdge.target)!

	assert.equal(
		reverseEdge.connector.x,
		source.x + dependencyGraphMetrics.nodeWidth + dependencyGraphMetrics.edgeClearance,
	)
	assert.equal(reverseEdge.connector.y, source.y + dependencyGraphMetrics.nodeHeight / 2)
	assert.equal(
		reverseEdge.connector.rotation,
		(Math.atan2(
			target.y + dependencyGraphMetrics.nodeHeight / 2 - reverseEdge.connector.y,
			target.x - dependencyGraphMetrics.edgeClearance - reverseEdge.connector.x,
		) *
			180) /
			Math.PI,
	)
	assert.ok(reverseEdge.connector.length > 0)
	assert.equal(
		reverseEdge.connector.length,
		Math.hypot(
			target.x - dependencyGraphMetrics.edgeClearance - reverseEdge.connector.x,
			target.y + dependencyGraphMetrics.nodeHeight / 2 - reverseEdge.connector.y,
		),
	)
})

test('uses dragged final coordinates for connectors and canvas bounds', () => {
	const graph = buildDependencyGraph([item('a', '1', { requires: [ref('b')] }), item('b', '1')])
	const offsets = new Map([[nodeId('b'), { x: 420, y: 180 }]])
	const layout = layoutDependencyGraph(graph, undefined, offsets)
	const source = layout.nodes.find((node) => node.id === nodeId('a'))!
	const target = layout.nodes.find((node) => node.id === nodeId('b'))!
	const edge = layout.edges[0]!

	assert.ok(target.x >= dependencyGraphMetrics.canvasPadding + 420)
	assert.ok(target.y >= dependencyGraphMetrics.canvasPadding + 180)
	assert.equal(
		edge.connector.length,
		Math.hypot(
			target.x - dependencyGraphMetrics.edgeClearance - edge.connector.x,
			target.y + dependencyGraphMetrics.nodeHeight / 2 - edge.connector.y,
		),
	)
	assert.equal(
		edge.connector.x,
		source.x + dependencyGraphMetrics.nodeWidth + dependencyGraphMetrics.edgeClearance,
	)
	assert.ok(
		layout.width >=
			target.x + dependencyGraphMetrics.nodeWidth + dependencyGraphMetrics.canvasPadding,
	)
	assert.ok(
		layout.height >=
			target.y + dependencyGraphMetrics.nodeHeight + dependencyGraphMetrics.canvasPadding,
	)
})
