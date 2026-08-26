<script setup lang="ts">
import { CodeIcon, FileArchiveIcon } from '@modrinth/assets'
import type { EditingFile, FileContextMenuOption, FileItem } from '@modrinth/ui'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	FilePageLayout,
	injectNotificationManager,
	provideFileManager,
	ReadyTransition,
	useDebugLogger,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import {
	mkdir,
	readDir,
	readFile as readFileBytes,
	readTextFile,
	remove,
	rename,
	stat,
	writeTextFile,
} from '@tauri-apps/plugin-fs'
import { computed, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import { instance_listener } from '@/helpers/events'
import { get_full_path } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { highlightInFolder } from '@/helpers/utils'

const props = defineProps<{
	instance: GameInstance
	options: unknown
	offline: boolean
	playing: boolean
	installed: boolean
	isServerInstance: boolean
}>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const debug = useDebugLogger('Files')
const router = useRouter()

const messages = defineMessages({
	saveAs: {
		id: 'instance.files.save-as',
		defaultMessage: 'Save as...',
	},
	addingFiles: {
		id: 'instance.files.adding-files',
		defaultMessage: 'Adding files ({completed}/{total})',
	},
	openInSchematicWorkshop: {
		id: 'instance.files.open-in-schematic-workshop',
		defaultMessage: 'Open in schematic workshop',
	},
	openStudio: {
		id: 'instance.files.open-studio',
		defaultMessage: 'Open Studio',
	},
})

const instanceRoot = ref('')
const items = ref<FileItem[]>([])
/** True until the first directory read for the current instance path finishes (initial load only). */
const firstPaintPending = ref(true)
const loading = ref(true)
const error = ref<Error | null>(null)
const currentPath = ref('')
const editingFile = ref<EditingFile | null>(null)

debug('setup: start, instance.id =', props.instance.id)

instanceRoot.value = await get_full_path(props.instance.id)
debug('setup: instanceRoot =', instanceRoot.value)
await refresh()
debug('setup: refresh complete, items =', items.value.length, 'error =', error.value)

function resolvePath(relativePath: string): string {
	return relativePath ? `${instanceRoot.value}/${relativePath}` : instanceRoot.value
}

async function listDirectory(dirPath: string): Promise<FileItem[]> {
	const absPath = resolvePath(dirPath)
	debug('listDirectory: dirPath =', dirPath, 'absPath =', absPath)
	const entries = await readDir(absPath)
	debug('listDirectory: got', entries.length, 'entries')

	const results = await Promise.all(
		entries.map(async (entry) => {
			const entryAbsPath = `${absPath}/${entry.name}`
			let metadata
			try {
				metadata = await stat(entryAbsPath)
			} catch {
				debug('listDirectory: stat failed for', entry.name, '- skipping')
				return null
			}
			const item: FileItem = {
				name: entry.name,
				type: entry.isDirectory ? 'directory' : 'file',
				path: dirPath ? `${dirPath}/${entry.name}` : entry.name,
				modified: metadata.mtime ? Math.floor(metadata.mtime.getTime() / 1000) : 0,
				created: metadata.birthtime ? Math.floor(metadata.birthtime.getTime() / 1000) : 0,
			}
			if (!entry.isDirectory) {
				item.size = metadata.size
			}
			if (entry.isDirectory) {
				try {
					const children = await readDir(entryAbsPath)
					item.count = children.length
				} catch {
					item.count = 0
				}
			}
			return item
		}),
	)
	return results.filter((item): item is FileItem => item !== null)
}

async function refresh() {
	debug('refresh: called, currentPath =', currentPath.value, 'instanceRoot =', instanceRoot.value)
	loading.value = true
	error.value = null
	try {
		items.value = await listDirectory(currentPath.value)
		debug('refresh: success, items =', items.value.length)
	} catch (e) {
		debug('refresh: error =', e)
		error.value = e instanceof Error ? e : new Error(String(e))
		items.value = []
	} finally {
		loading.value = false
		firstPaintPending.value = false
	}
}

function navigateTo(path: string) {
	debug('navigateTo:', path)
	currentPath.value = path.startsWith('/') ? path.slice(1) : path
	refresh()
}

function startEditing(file: EditingFile) {
	editingFile.value = file
}

function stopEditing() {
	editingFile.value = null
}

async function handleCreateItem(name: string, type: 'file' | 'directory') {
	const targetPath = currentPath.value ? `${currentPath.value}/${name}` : name
	const absPath = resolvePath(targetPath)
	try {
		if (type === 'directory') {
			await mkdir(absPath)
		} else {
			await writeTextFile(absPath, '')
		}
		await refresh()
	} catch (e) {
		addNotification({
			title: formatMessage(commonMessages.createFailedLabel),
			text: e instanceof Error ? e.message : '',
			type: 'error',
		})
	}
}

async function handleRenameItem(path: string, newName: string) {
	const oldAbs = resolvePath(path)
	const parentDir = path.includes('/') ? path.substring(0, path.lastIndexOf('/')) : ''
	const newPath = parentDir ? `${parentDir}/${newName}` : newName
	const newAbs = resolvePath(newPath)
	try {
		await rename(oldAbs, newAbs)
		await refresh()
	} catch (e) {
		addNotification({
			title: formatMessage(commonMessages.renameFailedLabel),
			text: e instanceof Error ? e.message : '',
			type: 'error',
		})
	}
}

async function handleMoveItem(source: string, destination: string) {
	try {
		await rename(resolvePath(source), resolvePath(destination))
		await refresh()
	} catch (e) {
		addNotification({
			title: formatMessage(commonMessages.moveFailedLabel),
			text: e instanceof Error ? e.message : '',
			type: 'error',
		})
	}
}

async function handleDeleteItem(path: string, recursive: boolean) {
	try {
		await remove(resolvePath(path), { recursive })
		await refresh()
	} catch (e) {
		addNotification({
			title: formatMessage(commonMessages.deleteFailedLabel),
			text: e instanceof Error ? e.message : '',
			type: 'error',
		})
	}
}

async function handleReadFile(path: string): Promise<string> {
	return await readTextFile(resolvePath(path))
}

async function handleReadFileAsBlob(path: string): Promise<Blob> {
	const bytes = await readFileBytes(resolvePath(path))
	return new Blob([bytes])
}

async function handleWriteFile(path: string, content: string) {
	await writeTextFile(resolvePath(path), content)
}

async function handleDownloadFile(path: string, _fileName: string) {
	await invoke('plugin:files|file_save_as', {
		instanceId: props.instance.id,
		filePath: path,
	})
}

async function handleExtractFile(path: string, override: boolean, dry: boolean) {
	try {
		return await invoke('plugin:files|file_extract_zip', {
			instanceId: props.instance.id,
			filePath: path,
			overrideConflicts: override,
			dryRun: dry,
		})
	} catch (e) {
		addNotification({
			title: formatMessage(commonMessages.extractFailedLabel),
			text: e instanceof Error ? e.message : '',
			type: 'error',
		})
	}
}

function getAdditionalMenuOptions(
	item: Pick<FileItem, 'name' | 'type' | 'path'>,
): FileContextMenuOption[] {
	if (item.type !== 'file' || !/\.(litematic|schem)$/i.test(item.name)) return []

	return [
		{
			id: 'open-in-schematic-workshop',
			label: formatMessage(messages.openInSchematicWorkshop),
			icon: FileArchiveIcon,
			action: () => {
				void router.push({
					name: 'Schematic workshop',
					query: { instance: props.instance.id, path: item.path },
				})
			},
		},
	]
}

debug('setup: registering instance_listener')
const unlistenInstances = await instance_listener(
	async (event: { event: string; instance_id: string }) => {
		debug('instance_listener: event =', event.event, 'path =', event.instance_id)
		if (event.instance_id === props.instance.id && event.event === 'synced') {
			debug('instance_listener: synced event matched, calling refresh')
			await refresh()
		}
	},
)
debug('setup: instance_listener registered')

onUnmounted(() => {
	unlistenInstances()
})

watch(
	() => props.instance.id,
	async () => {
		debug('watch instance.id: changed to', props.instance.id)
		firstPaintPending.value = true
		instanceRoot.value = await get_full_path(props.instance.id)
		currentPath.value = ''
		await refresh()
	},
)

provideFileManager({
	items,
	loading,
	error,
	currentPath,
	navigateTo,
	editingFile,
	startEditing,
	stopEditing,
	createItem: handleCreateItem,
	renameItem: handleRenameItem,
	moveItem: handleMoveItem,
	deleteItem: handleDeleteItem,
	readFile: handleReadFile,
	readFileAsBlob: handleReadFileAsBlob,
	writeFile: handleWriteFile,
	downloadFile: handleDownloadFile,
	extractFile: handleExtractFile,
	refresh,
	basePath: instanceRoot,
	openInFolder: (path: string) => highlightInFolder(path),
	getAdditionalMenuOptions,
	downloadButtonLabel: formatMessage(messages.saveAs),
	uploadingLabel: (completed: number, total: number) =>
		formatMessage(messages.addingFiles, { completed, total }),
	symlinkTarget: computed(() => props.instance.symlink_target),
})
</script>

<template>
	<ReadyTransition :pending="firstPaintPending">
		<div class="flex flex-col gap-4">
			<FilePageLayout :show-refresh-button="true">
				<template #before-refresh>
					<ButtonStyled color="brand">
						<button
							type="button"
							class="!h-10"
							@click="router.push({ name: 'FileStudio', params: { id: instance.id } })"
						>
							<CodeIcon class="size-5" />
							<span class="inline-flex items-center gap-1">
								{{ formatMessage(messages.openStudio) }}
								<span
									class="rounded bg-orange px-1.5 py-0.5 text-[10px] font-bold uppercase leading-none text-contrast"
								>
									Beta
								</span>
							</span>
						</button>
					</ButtonStyled>
				</template>
			</FilePageLayout>
		</div>
	</ReadyTransition>
</template>
