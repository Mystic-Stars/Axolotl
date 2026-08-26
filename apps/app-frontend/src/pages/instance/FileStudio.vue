<script setup lang="ts">
import {
	ChevronDownIcon,
	ChevronRightIcon,
	CodeIcon,
	CopyIcon,
	FileCodeIcon,
	FilePlusIcon,
	FolderIcon,
	FolderOpenIcon,
	RefreshCwIcon,
	SaveIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import {
	copyFile,
	mkdir,
	readDir,
	readFile,
	readTextFile,
	remove,
	rename,
	watch as watchFiles,
	writeFile,
	writeTextFile,
} from '@tauri-apps/plugin-fs'
import { NbtFile, NbtTag } from 'deepslate/nbt'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave, useRouter } from 'vue-router'

import NbtEditor from '@/components/instance/studio/NbtEditor.vue'
import StudioEditor from '@/components/instance/studio/StudioEditor.vue'
import StudioEditorTabs from '@/components/instance/studio/StudioEditorTabs.vue'
import {
	type StudioDocument,
	useStudioDocuments,
} from '@/components/instance/studio/useStudioDocuments'
import type { ServerView } from '@/composables/useServers'
import { get_full_path } from '@/helpers/instance'
import {
	listenStudioFilesChanged,
	readStudioBinary,
	readStudioText,
	registerStudioWatcher,
	trashStudioFile,
	unregisterStudioWatcher,
	writeStudioBinary,
} from '@/helpers/studio'
import type { GameInstance } from '@/helpers/types'
import { highlightInFolder, openPath } from '@/helpers/utils'

interface StudioTreeNode {
	name: string
	path: string
	type: 'directory' | 'file'
	depth: number
	expanded: boolean
	loaded: boolean
	loading: boolean
	children: StudioTreeNode[]
}

const props = defineProps<{
	instance?: GameInstance
	server?: ServerView
}>()

const messages = defineMessages({
	title: {
		id: 'instance.files.studio.title',
		defaultMessage: 'Studio',
	},
	files: {
		id: 'instance.files.studio.files',
		defaultMessage: 'Explorer',
	},
	backToFiles: {
		id: 'instance.files.studio.back-to-files',
		defaultMessage: 'Exit',
	},
	format: {
		id: 'instance.files.studio.format',
		defaultMessage: 'Format',
	},
	emptyTitle: {
		id: 'instance.files.studio.empty-title',
		defaultMessage: 'Select a configuration file',
	},
	emptyDescription: {
		id: 'instance.files.studio.empty-description',
		defaultMessage: 'Open any text file from the explorer.',
	},
	loadDirectoryFailed: {
		id: 'instance.files.studio.load-directory-failed',
		defaultMessage: 'Could not load directory',
	},
	loadFileFailed: {
		id: 'instance.files.studio.load-file-failed',
		defaultMessage: 'Could not open file',
	},
	nbtLoadFailed: {
		id: 'instance.files.studio.nbt-load-failed',
		defaultMessage: 'Could not parse NBT file',
	},
	notTextFile: {
		id: 'instance.files.studio.not-text-file',
		defaultMessage: 'This file is not a text file and cannot be opened in Studio.',
	},
	saveFailed: {
		id: 'instance.files.studio.save-failed',
		defaultMessage: 'Could not save file',
	},
	loadingFile: {
		id: 'instance.files.studio.loading-file',
		defaultMessage: 'Loading file...',
	},
	copy: { id: 'instance.files.studio.copy', defaultMessage: 'Copy' },
	cut: { id: 'instance.files.studio.cut', defaultMessage: 'Cut' },
	paste: { id: 'instance.files.studio.paste', defaultMessage: 'Paste' },
	newFile: { id: 'instance.files.studio.new-file', defaultMessage: 'New file' },
	newFolder: { id: 'instance.files.studio.new-folder', defaultMessage: 'New folder' },
	delete: { id: 'instance.files.studio.delete', defaultMessage: 'Move to trash' },
	createItem: { id: 'instance.files.studio.create-item', defaultMessage: 'Create item' },
	itemName: { id: 'instance.files.studio.item-name', defaultMessage: 'Name' },
	create: { id: 'instance.files.studio.create', defaultMessage: 'Create' },
	operationFailed: {
		id: 'instance.files.studio.operation-failed',
		defaultMessage: 'File operation failed',
	},
	nonTextFile: {
		id: 'instance.files.studio.non-text-file',
		defaultMessage: 'This file is not a text file and cannot be opened.',
	},
	openInSystem: {
		id: 'instance.files.studio.open-in-system',
		defaultMessage: 'Open in system',
	},
	openPath: {
		id: 'instance.files.studio.open-path',
		defaultMessage: 'Open path',
	},
	refresh: {
		id: 'instance.files.studio.refresh',
		defaultMessage: 'Refresh files',
	},
})

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const router = useRouter()
const instanceRoot = ref('')
const rootNodes = ref<StudioTreeNode[]>([])
const treeLoading = ref(true)
const fileLoading = ref(false)
const treeScrollElement = ref<HTMLElement | null>(null)
const contextMenu = ref<{ node: StudioTreeNode; x: number; y: number } | null>(null)
const contextMenuElement = ref<HTMLElement | null>(null)
const fileClipboard = ref<{ mode: 'copy' | 'cut'; node: StudioTreeNode } | null>(null)
const createModal = ref<InstanceType<typeof NewModal> | null>(null)
const createParentPath = ref('')
const createType = ref<'file' | 'directory'>('file')
const createName = ref('')
const studioEditor = ref<InstanceType<typeof StudioEditor> | InstanceType<typeof NbtEditor> | null>(
	null,
)
const nbtFiles = new Map<string, NbtFile>()
let watcherRegistrationId: string | null = null
let watcherInstanceId: string | null = null
let unlistenStudioFiles: (() => void) | null = null
let unwatchWorkspaceFiles: (() => void) | null = null
let watcherGeneration = 0
let changedPaths = new Set<string>()
let changeTimer: ReturnType<typeof setTimeout> | null = null

const {
	documents,
	activeDocument,
	activePath,
	hasAnyUnsavedChanges,
	activate: activateDocument,
	open: openDocument,
	close: closeDocument,
	saveActive: saveActiveDocument,
	saveAll: saveAllDocuments,
	updateActiveContent,
	reset: resetDocuments,
} = useStudioDocuments(
	(document, content) => {
		if (document.kind === 'nbt') {
			const nbtFile = nbtFiles.get(document.path)
			if (!nbtFile) throw new Error('NBT document metadata is unavailable')
			const root = NbtTag.fromString(content)
			if (!root.isCompound()) throw new Error('NBT root must be a compound')
			nbtFile.root = root
			return writeBinary(document.path, nbtFile.write())
		}
		return writeTextFile(resolvePath(document.path), content)
	},
	(error) => {
		addNotification({
			title: formatMessage(messages.saveFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	},
)

async function saveActiveFile() {
	const focusedElement = document.activeElement
	if (focusedElement instanceof HTMLElement) focusedElement.blur()
	return saveActiveDocument()
}

const selectedName = computed(() => activeDocument.value?.name ?? '')
const selectedFilePath = computed(() => activeDocument.value?.path ?? '')
const deleteLabel = computed(() =>
	formatMessage(props.instance ? messages.delete : commonMessages.deleteLabel),
)
const editorLanguage = computed(() => {
	const extension = selectedName.value.split('.').pop()?.toLowerCase()
	if (extension === 'dat' || extension === 'nbt') return 'json'
	if (extension === 'yml') return 'yaml'
	return extension ?? 'plaintext'
})
const breadcrumbSegments = computed(() => selectedFilePath.value.split('/').filter(Boolean))
const visibleBreadcrumbSegments = ref<string[]>([])
const breadcrumbOuter = ref<HTMLElement | null>(null)
const breadcrumbMeasure = ref<HTMLElement | null>(null)
let breadcrumbObserver: ResizeObserver | null = null

function previewKind(name: string): StudioDocument['kind'] | null {
	if (/\.(png|jpe?g|gif|webp|svg|bmp|ico|avif)$/i.test(name)) return 'image'
	if (/\.(mp4|webm|ogv|mov|m4v)$/i.test(name)) return 'video'
	return null
}

const previewUrl = computed(() =>
	activeDocument.value ? convertFileSrc(resolvePath(activeDocument.value.path)) : '',
)

function resolvePath(relativePath: string): string {
	return relativePath ? `${instanceRoot.value}/${relativePath}` : instanceRoot.value
}

async function readText(path: string): Promise<string> {
	if (props.instance) return readStudioText(props.instance.id, path)
	return readTextFile(resolvePath(path))
}

async function readBinary(path: string): Promise<Uint8Array> {
	if (props.instance) return readStudioBinary(props.instance.id, path)
	return readFile(resolvePath(path))
}

async function writeBinary(path: string, bytes: Uint8Array): Promise<void> {
	if (props.instance) return writeStudioBinary(props.instance.id, path, bytes)
	return writeFile(resolvePath(path), bytes)
}

async function deleteStudioFile(path: string): Promise<void> {
	if (props.instance) return trashStudioFile(props.instance.id, path)
	return remove(resolvePath(path), { recursive: true })
}

function workspaceRelativePath(path: string): string | null {
	const root = instanceRoot.value.replaceAll('\\', '/').replace(/\/+$/, '')
	const normalized = path.replaceAll('\\', '/')
	if (!root || normalized === root) return null
	if (normalized.startsWith(`${root}/`)) return normalized.slice(root.length + 1)
	return /^(?:[A-Za-z]:)?\//.test(normalized) ? null : normalized
}

function exitStudio() {
	if (props.server) {
		void router.push({
			name: 'MultiplayerServerDetail',
			params: { id: props.server.id },
			query: { tab: 'files' },
		})
		return
	}
	if (props.instance) {
		void router.push({ name: 'Files', params: { id: props.instance.id } })
	}
}

function readNbtContent(bytes: Uint8Array, path: string): string {
	const nbtFile = NbtFile.read(bytes)
	nbtFiles.set(path, nbtFile)
	return nbtFile.root.toPrettyString()
}

function joinPath(parent: string, name: string): string {
	return parent ? `${parent}/${name}` : name
}

function contextDestination(node: StudioTreeNode): string {
	return node.type === 'directory' ? node.path : node.path.split('/').slice(0, -1).join('/')
}

async function showContextMenu(event: MouseEvent, node: StudioTreeNode) {
	contextMenu.value = { node, x: event.clientX, y: event.clientY }
	await nextTick()
	if (!contextMenu.value || !contextMenuElement.value) return

	const padding = 10
	const rect = contextMenuElement.value.getBoundingClientRect()
	contextMenu.value.x = Math.min(contextMenu.value.x, window.innerWidth - rect.width - padding)
	if (rect.bottom > window.innerHeight - padding) {
		contextMenu.value.y = Math.max(padding, event.clientY - rect.height)
	}
}

function hideContextMenu() {
	contextMenu.value = null
}

function handleContextMenuKeydown(event: KeyboardEvent) {
	if (event.key === 'Escape') hideContextMenu()
}

function copyToClipboard(mode: 'copy' | 'cut') {
	if (!contextMenu.value) return
	fileClipboard.value = { mode, node: contextMenu.value.node }
	hideContextMenu()
}

async function copyItem(source: StudioTreeNode, destination: string): Promise<void> {
	if (source.type === 'file') {
		await copyFile(resolvePath(source.path), resolvePath(destination))
		return
	}

	await mkdir(resolvePath(destination), { recursive: true })
	const children = await listDirectory(source.path, source.depth + 1)
	await Promise.all(children.map((child) => copyItem(child, joinPath(destination, child.name))))
}

async function pasteClipboard() {
	if (!contextMenu.value || !fileClipboard.value) return
	const destinationParent = contextDestination(contextMenu.value.node)
	const { mode, node } = fileClipboard.value
	const destination = joinPath(destinationParent, node.name)
	if (node.path === destination || destination.startsWith(`${node.path}/`)) return

	try {
		if (mode === 'copy') await copyItem(node, destination)
		else {
			await rename(resolvePath(node.path), resolvePath(destination))
			fileClipboard.value = null
		}
		await refreshTree()
		hideContextMenu()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.operationFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	}
}

async function deleteItem() {
	if (!contextMenu.value) return
	try {
		await deleteStudioFile(contextMenu.value.node.path)
		if (activePath.value === contextMenu.value.node.path) await closeDocument(activePath.value)
		await refreshTree()
		hideContextMenu()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.operationFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	}
}

function showCreateModal(type: 'file' | 'directory') {
	if (!contextMenu.value) return
	createParentPath.value = contextDestination(contextMenu.value.node)
	createType.value = type
	createName.value = ''
	hideContextMenu()
	createModal.value?.show()
}

async function createItem() {
	const name = createName.value.trim()
	if (!name || /[\\/]/.test(name)) return
	const path = joinPath(createParentPath.value, name)
	try {
		if (createType.value === 'directory') await mkdir(resolvePath(path))
		else await writeTextFile(resolvePath(path), '')
		await refreshTree()
		createModal.value?.hide()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.operationFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	}
}

function updateVisibleBreadcrumbs() {
	const segments = breadcrumbSegments.value
	if (!breadcrumbOuter.value || !breadcrumbMeasure.value) {
		visibleBreadcrumbSegments.value = segments
		return
	}

	let start = 0
	while (start < Math.max(segments.length - 2, 0)) {
		const visible = start === 0 ? segments : ['...', ...segments.slice(start)]
		breadcrumbMeasure.value.textContent = visible.join(' > ')
		if (breadcrumbMeasure.value.scrollWidth <= breadcrumbOuter.value.clientWidth) {
			visibleBreadcrumbSegments.value = visible
			return
		}
		start += 1
	}

	visibleBreadcrumbSegments.value = segments.length > 1 ? ['...', segments.at(-1)!] : segments
}

async function listDirectory(path: string, depth: number): Promise<StudioTreeNode[]> {
	const entries = await readDir(resolvePath(path))
	return entries
		.map((entry) => ({
			name: entry.name,
			path: path ? `${path}/${entry.name}` : entry.name,
			type: entry.isDirectory ? ('directory' as const) : ('file' as const),
			depth,
			expanded: false,
			loaded: false,
			loading: false,
			children: [],
		}))
		.sort((a, b) => {
			if (a.type !== b.type) return a.type === 'directory' ? -1 : 1
			return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
		})
}

function collectDirectoryState(
	nodes: StudioTreeNode[],
	state = new Map<string, Pick<StudioTreeNode, 'expanded' | 'loaded'>>(),
) {
	for (const node of nodes) {
		if (node.type !== 'directory') continue
		state.set(node.path, { expanded: node.expanded, loaded: node.loaded })
		collectDirectoryState(node.children, state)
	}
	return state
}

async function restoreLoadedDirectories(
	nodes: StudioTreeNode[],
	state: Map<string, Pick<StudioTreeNode, 'expanded' | 'loaded'>>,
) {
	await Promise.all(
		nodes.map(async (node) => {
			if (node.type !== 'directory') return
			const previous = state.get(node.path)
			if (!previous?.loaded) return
			node.children = await listDirectory(node.path, node.depth + 1)
			node.loaded = true
			node.expanded = previous.expanded
			await restoreLoadedDirectories(node.children, state)
		}),
	)
}

async function refreshTree() {
	const directoryState = collectDirectoryState(rootNodes.value)
	const nextRoot = await listDirectory('', 0)
	await restoreLoadedDirectories(nextRoot, directoryState)
	const scrollTop = treeScrollElement.value?.scrollTop ?? 0
	rootNodes.value = nextRoot
	await nextTick()
	if (treeScrollElement.value) treeScrollElement.value.scrollTop = scrollTop
}

function flattenTree(nodes: StudioTreeNode[]): StudioTreeNode[] {
	return nodes.flatMap((node) => [
		node,
		...(node.type === 'directory' && node.expanded ? flattenTree(node.children) : []),
	])
}

const visibleNodes = computed(() => flattenTree(rootNodes.value))

async function loadRoot() {
	treeLoading.value = true
	try {
		rootNodes.value = await listDirectory('', 0)
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadDirectoryFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	} finally {
		treeLoading.value = false
	}
}

async function reloadCleanDocument(document: StudioDocument) {
	if (
		(document.kind !== 'text' && document.kind !== 'nbt') ||
		document.content !== document.savedContent ||
		document.saving
	)
		return
	try {
		const nextContent =
			document.kind === 'nbt'
				? readNbtContent(await readBinary(document.path), document.path)
				: await readTextFile(resolvePath(document.path))
		if (document.content !== document.savedContent || document.saving) return
		document.content = nextContent
		document.savedContent = nextContent
	} catch {
		// The path may have been removed or be between the two sides of an atomic rename.
	}
}

async function processFileChanges() {
	const paths = changedPaths
	changedPaths = new Set<string>()
	try {
		await refreshTree()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadDirectoryFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	}

	await Promise.all(
		documents.value
			.filter((document) => paths.has(document.path))
			.map((document) => reloadCleanDocument(document)),
	)
}

function scheduleFileChanges(paths: string[]) {
	for (const path of paths) changedPaths.add(path)
	if (changeTimer) clearTimeout(changeTimer)
	changeTimer = setTimeout(() => {
		changeTimer = null
		void processFileChanges()
	}, 150)
}

async function stopStudioWatcher() {
	unwatchWorkspaceFiles?.()
	unwatchWorkspaceFiles = null
	const instanceId = watcherInstanceId
	const registrationId = watcherRegistrationId
	watcherInstanceId = null
	watcherRegistrationId = null
	if (instanceId && registrationId) {
		await unregisterStudioWatcher(instanceId, registrationId).catch(() => undefined)
	}
}

async function startStudioWatcher() {
	const generation = ++watcherGeneration
	await stopStudioWatcher()
	if (props.server) {
		try {
			unwatchWorkspaceFiles = await watchFiles(
				instanceRoot.value,
				(event) => {
					const paths = event.paths
						.map(workspaceRelativePath)
						.filter((path): path is string => path !== null)
					if (paths.length > 0) scheduleFileChanges(paths)
				},
				{ recursive: true, delayMs: 150 },
			)
		} catch (error) {
			console.warn('Failed to start server Studio file watcher', error)
			return
		}
		if (generation !== watcherGeneration) {
			unwatchWorkspaceFiles?.()
			unwatchWorkspaceFiles = null
		}
		return
	}
	if (!props.instance) return
	const instanceId = props.instance.id
	const registrationId = await registerStudioWatcher(instanceId)
	if (generation !== watcherGeneration) {
		await unregisterStudioWatcher(instanceId, registrationId).catch(() => undefined)
		return
	}
	watcherInstanceId = instanceId
	watcherRegistrationId = registrationId
}

async function toggleDirectory(node: StudioTreeNode) {
	if (node.loading) return
	if (node.loaded) {
		node.expanded = !node.expanded
		return
	}

	node.loading = true
	try {
		node.children = await listDirectory(node.path, node.depth + 1)
		node.loaded = true
		node.expanded = true
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadDirectoryFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	} finally {
		node.loading = false
	}
}

watch(activePath, async (path) => {
	if (!path) return
	await nextTick()
	const node = Array.from(
		treeScrollElement.value?.querySelectorAll<HTMLElement>('[data-studio-path]') ?? [],
	).find((element) => element.dataset.studioPath === path)
	node?.scrollIntoView({ block: 'nearest' })
})

watch(breadcrumbSegments, () => nextTick(updateVisibleBreadcrumbs), { immediate: true })

onMounted(() => {
	breadcrumbObserver = new ResizeObserver(updateVisibleBreadcrumbs)
	if (breadcrumbOuter.value) breadcrumbObserver.observe(breadcrumbOuter.value)
	document.addEventListener('mousedown', hideContextMenu)
	document.addEventListener('keydown', handleContextMenuKeydown)
})

onBeforeUnmount(() => {
	breadcrumbObserver?.disconnect()
	document.removeEventListener('mousedown', hideContextMenu)
	document.removeEventListener('keydown', handleContextMenuKeydown)
})

async function openFile(node: StudioTreeNode) {
	if (node.type !== 'file' || fileLoading.value) return

	const existingDocument = documents.value.find((document) => document.path === node.path)
	if (existingDocument) {
		if (existingDocument.path === activePath.value) return
		await activateDocument(existingDocument.path)
		return
	}

	const mediaKind = previewKind(node.name)
	if (mediaKind) {
		await openDocument({
			kind: mediaKind,
			path: node.path,
			name: node.name,
			content: '',
			savedContent: '',
			saving: false,
		})
		return
	}
	if (/\.jar$/i.test(node.name)) {
		await openDocument({
			kind: 'unsupported',
			path: node.path,
			name: node.name,
			content: '',
			savedContent: '',
			saving: false,
		})
		return
	}

	fileLoading.value = true
	try {
		if (/\.(dat|nbt)$/i.test(node.name)) {
			let nextContent: string
			try {
				nextContent = readNbtContent(await readBinary(node.path), node.path)
			} catch (error) {
				addNotification({
					title: formatMessage(messages.nbtLoadFailed),
					text: error instanceof Error ? error.message : String(error),
					type: 'error',
				})
				return
			}
			await openDocument({
				kind: 'nbt',
				path: node.path,
				name: node.name,
				content: nextContent,
				savedContent: nextContent,
				saving: false,
			})
			return
		}
		const nextContent = await readText(node.path)
		const document: StudioDocument = {
			kind: 'text',
			path: node.path,
			name: node.name,
			content: nextContent,
			savedContent: nextContent,
			saving: false,
		}
		await openDocument(document)
	} catch {
		await openDocument({
			kind: 'unsupported',
			path: node.path,
			name: node.name,
			content: '',
			savedContent: '',
			saving: false,
		})
	} finally {
		fileLoading.value = false
	}
}

async function activateTab(path: string) {
	await activateDocument(path)
}

async function formatActiveDocument() {
	await studioEditor.value?.formatDocument()
}

async function openInSystem(path: string) {
	await openPath(resolvePath(path))
}

async function revealInSystem(path: string) {
	await highlightInFolder(resolvePath(path))
}

async function revealContextMenuItem() {
	if (!contextMenu.value) return
	await revealInSystem(contextMenu.value.node.path)
	hideContextMenu()
}

async function initialize() {
	instanceRoot.value =
		props.server?.path ?? (props.instance ? await get_full_path(props.instance.id) : '')
	resetDocuments()
	await loadRoot()
}

await initialize()

if (props.instance) {
	unlistenStudioFiles = await listenStudioFilesChanged((event) => {
		if (event.instanceId !== watcherInstanceId || event.registrationId !== watcherRegistrationId) {
			return
		}
		scheduleFileChanges(event.paths)
	})
}
await startStudioWatcher()

watch(
	() => props.instance?.id ?? props.server?.id,
	async () => {
		await initialize()
		await startStudioWatcher()
	},
)

function handleBeforeUnload(event: BeforeUnloadEvent) {
	if (!hasAnyUnsavedChanges.value) return
	event.preventDefault()
}

window.addEventListener('beforeunload', handleBeforeUnload)
onBeforeUnmount(() => {
	watcherGeneration++
	window.removeEventListener('beforeunload', handleBeforeUnload)
	unlistenStudioFiles?.()
	unlistenStudioFiles = null
	if (changeTimer) clearTimeout(changeTimer)
	void stopStudioWatcher()
})

onBeforeRouteLeave(() => {
	return saveAllDocuments()
})
</script>

<template>
	<div class="flex h-full min-h-0 flex-col">
		<section
			class="grid min-h-0 min-w-0 flex-1 grid-cols-[minmax(13rem,22rem)_minmax(0,1fr)] overflow-hidden rounded-[20px] border border-solid border-surface-4 bg-surface-1 shadow-sm"
		>
			<NewModal ref="createModal" :header="formatMessage(messages.createItem)" max-width="420px">
				<label class="flex flex-col gap-2 text-sm font-semibold text-contrast">
					{{ formatMessage(messages.itemName) }}
					<StyledInput v-model="createName" :input-attrs="{ autofocus: true }" />
				</label>
				<template #actions>
					<ButtonStyled color="brand">
						<button type="button" @click="createItem">
							{{ formatMessage(messages.create) }}
						</button>
					</ButtonStyled>
				</template>
			</NewModal>
			<aside
				class="flex min-h-0 min-w-0 flex-col border-0 border-r border-solid border-surface-4 bg-surface-2"
			>
				<header
					class="flex h-12 shrink-0 items-center gap-2 border-0 border-b border-solid border-surface-4 bg-surface-3 px-3"
				>
					<CodeIcon class="size-5 text-brand" />
					<h1 class="m-0 min-w-0 truncate text-sm font-bold text-contrast">
						{{ formatMessage(messages.title) }}
					</h1>
					<div class="ml-auto flex shrink-0 items-center gap-1">
						<ButtonStyled size="small" type="transparent" circular>
							<button
								v-tooltip="formatMessage(messages.refresh)"
								type="button"
								:aria-label="formatMessage(messages.refresh)"
								@click="refreshTree"
							>
								<RefreshCwIcon class="size-4" />
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="activeDocument" size="small" color="brand">
							<button type="button" :disabled="activeDocument.saving" @click="saveActiveFile">
								<SaveIcon class="size-4" />
								{{ formatMessage(commonMessages.saveButton) }}
							</button>
						</ButtonStyled>
						<ButtonStyled
							v-if="activeDocument?.kind === 'text' || activeDocument?.kind === 'nbt'"
							size="small"
							type="outlined"
						>
							<button type="button" @click="formatActiveDocument">
								{{ formatMessage(messages.format) }}
							</button>
						</ButtonStyled>
						<ButtonStyled size="small" type="outlined">
							<button type="button" @click="exitStudio">
								{{ formatMessage(messages.backToFiles) }}
							</button>
						</ButtonStyled>
					</div>
				</header>
				<div
					class="flex h-9 shrink-0 items-center px-3 text-xs font-bold uppercase tracking-wide text-secondary"
				>
					{{ formatMessage(messages.files) }}
				</div>
				<div
					ref="treeScrollElement"
					class="min-h-0 flex-1 pb-3"
					:class="contextMenu ? 'overflow-y-hidden' : 'overflow-y-auto'"
					role="tree"
				>
					<div v-if="treeLoading" class="px-4 py-3 text-sm text-secondary">
						{{ formatMessage(messages.loadingFile) }}
					</div>
					<button
						v-for="node in visibleNodes"
						:key="node.path"
						type="button"
						role="treeitem"
						:data-studio-path="node.path"
						:aria-expanded="node.type === 'directory' ? node.expanded : undefined"
						class="flex h-8 w-full cursor-pointer items-center gap-1 border-0 bg-transparent pr-3 text-left text-sm text-primary hover:bg-surface-3"
						:class="{
							'bg-brand-highlight !text-contrast': node.path === selectedFilePath,
							'!bg-surface-3':
								node.path === contextMenu?.node.path && node.path !== selectedFilePath,
						}"
						:style="{ paddingLeft: `${node.depth * 16 + 8}px` }"
						@click="node.type === 'directory' ? toggleDirectory(node) : openFile(node)"
						@contextmenu.prevent.stop="showContextMenu($event, node)"
					>
						<template v-if="node.type === 'directory'">
							<ChevronDownIcon v-if="node.expanded" class="size-4 shrink-0" />
							<ChevronRightIcon v-else class="size-4 shrink-0" />
						</template>
						<span v-else class="size-4 shrink-0" />
						<FolderIcon v-if="node.type === 'directory'" class="size-4 shrink-0 text-secondary" />
						<FileCodeIcon v-else class="size-4 shrink-0 text-secondary" />
						<span class="truncate">{{ node.name }}</span>
					</button>
				</div>
			</aside>

			<div class="flex min-h-0 min-w-0 flex-col bg-surface-2">
				<header
					class="flex h-12 shrink-0 items-center overflow-hidden border-0 border-b border-solid border-surface-4 bg-surface-3"
				>
					<StudioEditorTabs
						:documents="documents"
						:active-path="activePath"
						@activate="activateTab"
						@close="closeDocument"
					/>
				</header>
				<nav
					v-if="selectedFilePath"
					class="flex h-8 shrink-0 items-center gap-2 border-0 border-b border-solid border-surface-4 bg-surface-2 px-3 text-xs text-secondary"
					:aria-label="selectedFilePath"
				>
					<div ref="breadcrumbOuter" class="flex min-w-0 flex-1 items-center gap-1 overflow-hidden">
						<template
							v-for="(segment, index) in visibleBreadcrumbSegments"
							:key="`${segment}-${index}`"
						>
							<ChevronRightIcon v-if="index > 0" class="size-3.5 shrink-0" />
							<FolderIcon
								v-if="segment !== '...' && index < visibleBreadcrumbSegments.length - 1"
								class="size-3.5 shrink-0"
							/>
							<FileCodeIcon v-else class="size-3.5 shrink-0" />
							<span
								class="min-w-0 truncate whitespace-nowrap"
								:class="{ 'text-contrast': index === visibleBreadcrumbSegments.length - 1 }"
							>
								{{ segment }}
							</span>
						</template>
					</div>
					<ButtonStyled size="small" type="transparent">
						<button type="button" @click="revealInSystem(selectedFilePath)">
							{{ formatMessage(messages.openPath) }}
						</button>
					</ButtonStyled>
					<span
						ref="breadcrumbMeasure"
						class="pointer-events-none absolute whitespace-nowrap opacity-0"
						aria-hidden="true"
					/>
				</nav>

				<div class="relative min-h-0 min-w-0 flex-1">
					<div
						v-if="fileLoading"
						class="absolute inset-0 z-[2] flex items-center justify-center bg-surface-2 text-sm text-secondary"
					>
						{{ formatMessage(messages.loadingFile) }}
					</div>
					<div
						v-if="activeDocument?.kind === 'image'"
						class="flex size-full items-center justify-center overflow-auto bg-surface-1 p-6"
					>
						<img
							:src="previewUrl"
							:alt="activeDocument.name"
							class="max-h-full max-w-full object-contain"
						/>
					</div>
					<div
						v-else-if="activeDocument?.kind === 'video'"
						class="flex size-full items-center justify-center bg-surface-1 p-6"
					>
						<video :src="previewUrl" controls class="max-h-full max-w-full" />
					</div>
					<div
						v-else-if="activeDocument?.kind === 'unsupported'"
						class="flex size-full items-center justify-center p-8 text-center"
					>
						<div class="flex flex-col items-center gap-3">
							<p class="m-0 text-sm text-secondary">
								{{ formatMessage(messages.nonTextFile) }}
							</p>
							<ButtonStyled type="outlined">
								<button type="button" @click="openInSystem(activeDocument.path)">
									{{ formatMessage(messages.openInSystem) }}
								</button>
							</ButtonStyled>
						</div>
					</div>
					<StudioEditor
						v-if="activeDocument?.kind === 'text'"
						ref="studioEditor"
						:key="activeDocument.path"
						:file-path="activeDocument.path"
						:content="activeDocument.content"
						:language="editorLanguage"
						:read-only="activeDocument.saving"
						@update:content="updateActiveContent"
						@save="saveActiveFile"
						@blur="saveActiveFile"
					/>
					<NbtEditor
						v-else-if="activeDocument?.kind === 'nbt'"
						ref="studioEditor"
						:key="activeDocument.path"
						:file-path="activeDocument.path"
						:content="activeDocument.content"
						:read-only="activeDocument.saving"
						@update:content="updateActiveContent"
						@save="saveActiveFile"
					/>
					<div v-else class="flex size-full items-center justify-center p-8 text-center">
						<div class="flex max-w-md flex-col items-center gap-3">
							<CodeIcon class="size-14 text-secondary" />
							<h2 class="m-0 text-xl font-bold text-contrast">
								{{ formatMessage(messages.emptyTitle) }}
							</h2>
							<p class="m-0 text-sm leading-6 text-secondary">
								{{ formatMessage(messages.emptyDescription) }}
							</p>
						</div>
					</div>
				</div>
			</div>
			<Teleport to="#teleports">
				<div
					v-if="contextMenu"
					ref="contextMenuElement"
					class="fixed z-[9999] flex min-w-48 flex-col gap-1 rounded-xl border border-solid border-surface-5 bg-surface-3 p-1.5 shadow-lg"
					:style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
					role="menu"
					@mousedown.stop
				>
					<ButtonStyled size="small" type="transparent">
						<button
							type="button"
							class="w-full !justify-start"
							role="menuitem"
							@click="copyToClipboard('copy')"
						>
							<CopyIcon class="size-4" /> {{ formatMessage(messages.copy) }}
						</button>
					</ButtonStyled>
					<ButtonStyled size="small" type="transparent">
						<button
							type="button"
							class="w-full !justify-start"
							role="menuitem"
							@click="copyToClipboard('cut')"
						>
							<CopyIcon class="size-4" /> {{ formatMessage(messages.cut) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="fileClipboard" size="small" type="transparent">
						<button
							type="button"
							class="w-full !justify-start"
							role="menuitem"
							@click="pasteClipboard"
						>
							<CopyIcon class="size-4" /> {{ formatMessage(messages.paste) }}
						</button>
					</ButtonStyled>
					<div class="my-1 h-px bg-surface-5" />
					<ButtonStyled size="small" type="transparent">
						<button
							type="button"
							class="w-full !justify-start"
							role="menuitem"
							@click="showCreateModal('file')"
						>
							<FilePlusIcon class="size-4" /> {{ formatMessage(messages.newFile) }}
						</button>
					</ButtonStyled>
					<ButtonStyled size="small" type="transparent">
						<button
							type="button"
							class="w-full !justify-start"
							role="menuitem"
							@click="showCreateModal('directory')"
						>
							<FolderOpenIcon class="size-4" /> {{ formatMessage(messages.newFolder) }}
						</button>
					</ButtonStyled>
					<div class="my-1 h-px bg-surface-5" />
					<ButtonStyled size="small" type="transparent">
						<button
							type="button"
							class="w-full !justify-start"
							role="menuitem"
							@click="revealContextMenuItem"
						>
							<FolderOpenIcon class="size-4" /> {{ formatMessage(messages.openPath) }}
						</button>
					</ButtonStyled>
					<ButtonStyled size="small" color="red" type="transparent">
						<button type="button" class="w-full !justify-start" role="menuitem" @click="deleteItem">
							<TrashIcon class="size-4" /> {{ deleteLabel }}
						</button>
					</ButtonStyled>
				</div>
			</Teleport>
		</section>
	</div>
</template>
