<script setup lang="ts">
import {
	ClipboardCopyIcon,
	ContractIcon,
	DownloadIcon,
	EyeIcon,
	ExpandIcon,
	FolderOpenIcon,
	LeftArrowIcon,
	RefreshCwIcon,
	RightArrowIcon,
	SearchIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	EmptyState,
	injectNotificationManager,
	NewModal,
	ReadyTransition,
	useFormatDateTime,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { exists, mkdir, readDir, readFile, remove, stat } from '@tauri-apps/plugin-fs'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import { get_full_path } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { highlightInFolder, openPath } from '@/helpers/utils'

interface Screenshot {
	name: string
	path: string
	url: string
	objectUrl?: string
	modified: Date
	size: number
}

const props = defineProps<{
	instance: GameInstance
}>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const formatDate = useFormatDateTime({
	year: 'numeric',
	month: 'short',
	day: 'numeric',
	hour: 'numeric',
	minute: '2-digit',
})

const messages = defineMessages({
	searchPlaceholder: {
		id: 'app.instance.screenshots.search-placeholder',
		defaultMessage: 'Search {count} screenshots...',
	},
	noScreenshots: {
		id: 'app.instance.screenshots.empty-title',
		defaultMessage: 'No screenshots yet',
	},
	noScreenshotsDescription: {
		id: 'app.instance.screenshots.empty-description',
		defaultMessage: 'Screenshots taken in Minecraft will appear here automatically.',
	},
	noSearchResults: {
		id: 'app.instance.screenshots.no-search-results',
		defaultMessage: 'No screenshots match your search.',
	},
	viewScreenshot: {
		id: 'app.instance.screenshots.view',
		defaultMessage: 'View screenshot',
	},
	copyScreenshot: {
		id: 'app.instance.screenshots.copy',
		defaultMessage: 'Copy image',
	},
	copiedScreenshot: {
		id: 'app.instance.screenshots.copied',
		defaultMessage: 'Screenshot copied',
	},
	copyFailed: {
		id: 'app.instance.screenshots.copy-failed',
		defaultMessage: 'Could not copy screenshot',
	},
	saveAs: {
		id: 'app.instance.screenshots.save-as',
		defaultMessage: 'Save as...',
	},
	deleteScreenshot: {
		id: 'app.instance.screenshots.delete',
		defaultMessage: 'Delete screenshot',
	},
	deleteDescription: {
		id: 'app.instance.screenshots.delete-description',
		defaultMessage: 'Are you sure you want to permanently delete {name}?',
	},
	openScreenshotsFolder: {
		id: 'app.instance.screenshots.open-folder',
		defaultMessage: 'Open screenshots folder',
	},
	loadingFailed: {
		id: 'app.instance.screenshots.loading-failed',
		defaultMessage: 'Could not load screenshots',
	},
	deleteFailed: {
		id: 'app.instance.screenshots.delete-failed',
		defaultMessage: 'Could not delete screenshot',
	},
	zoomIn: {
		id: 'app.instance.screenshots.zoom-in',
		defaultMessage: 'View at full size',
	},
	zoomOut: {
		id: 'app.instance.screenshots.zoom-out',
		defaultMessage: 'Fit to window',
	},
	actionFailed: {
		id: 'app.instance.screenshots.action-failed',
		defaultMessage: 'Screenshot action failed',
	},
})

const IMAGE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'webp'])
const MIME_TYPES: Record<string, string> = {
	png: 'image/png',
	jpg: 'image/jpeg',
	jpeg: 'image/jpeg',
	webp: 'image/webp',
}

const instanceRoot = ref('')
const screenshots = ref<Screenshot[]>([])
const loading = ref(true)
const firstPaintPending = ref(true)
const searchQuery = ref('')
const selectedScreenshot = ref<Screenshot | null>(null)
const pendingDeletion = ref<Screenshot | null>(null)
const zoomedIn = ref(false)
const viewerModal = ref<InstanceType<typeof NewModal>>()
const deleteModal = ref<InstanceType<typeof NewModal>>()
const screenshotContextMenu = ref<InstanceType<typeof ContextMenu>>()

const screenshotContextMenuOptions = [
	{ name: 'view_screenshot' },
	{ name: 'copy_screenshot' },
	{ name: 'save_screenshot' },
	{ type: 'divider' },
	{ name: 'open_screenshot_folder' },
	{ name: 'copy_screenshot_filename' },
	{ name: 'copy_screenshot_path' },
	{ type: 'divider' },
	{ name: 'delete_screenshot', color: 'danger' },
]

const screenshotsPath = computed(() => `${instanceRoot.value}/screenshots`)
const filteredScreenshots = computed(() => {
	const query = searchQuery.value.trim().toLocaleLowerCase()
	if (!query) return screenshots.value
	return screenshots.value.filter((screenshot) =>
		screenshot.name.toLocaleLowerCase().includes(query),
	)
})

function extensionOf(fileName: string): string {
	return fileName.split('.').pop()?.toLocaleLowerCase() ?? ''
}

function formatFileSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function showError(title: string, error: unknown) {
	addNotification({
		type: 'error',
		title,
		text: error instanceof Error ? error.message : String(error),
	})
}

function revokePreviewUrls(items = screenshots.value) {
	for (const screenshot of items) {
		if (screenshot.objectUrl) {
			URL.revokeObjectURL(screenshot.objectUrl)
			screenshot.objectUrl = undefined
		}
	}
}

async function loadScreenshotPreview(screenshot: Screenshot) {
	if (screenshot.objectUrl) return

	try {
		const bytes = await readFile(screenshot.path)
		const extension = extensionOf(screenshot.name)
		const objectUrl = URL.createObjectURL(
			new Blob([bytes], { type: MIME_TYPES[extension] ?? 'image/png' }),
		)
		screenshot.objectUrl = objectUrl
		screenshot.url = objectUrl
	} catch (error) {
		showError(formatMessage(messages.loadingFailed), error)
	}
}

async function refresh() {
	loading.value = true
	try {
		if (!(await exists(screenshotsPath.value))) {
			revokePreviewUrls()
			screenshots.value = []
			return
		}

		const entries = await readDir(screenshotsPath.value)
		const nextScreenshots = await Promise.all(
			entries
				.filter((entry) => !entry.isDirectory && IMAGE_EXTENSIONS.has(extensionOf(entry.name)))
				.map(async (entry): Promise<Screenshot | null> => {
					const path = `${screenshotsPath.value}/${entry.name}`
					try {
						const metadata = await stat(path)
						return {
							name: entry.name,
							path,
							url: convertFileSrc(path),
							modified: metadata.mtime ?? new Date(0),
							size: metadata.size,
						}
					} catch {
						return null
					}
				}),
		)

		revokePreviewUrls()
		screenshots.value = nextScreenshots
			.filter((screenshot): screenshot is Screenshot => screenshot !== null)
			.sort((a, b) => b.modified.getTime() - a.modified.getTime())
	} catch (error) {
		revokePreviewUrls()
		screenshots.value = []
		showError(formatMessage(messages.loadingFailed), error)
	} finally {
		loading.value = false
		firstPaintPending.value = false
	}
}

function viewScreenshot(screenshot: Screenshot) {
	selectedScreenshot.value = screenshot
	zoomedIn.value = false
	viewerModal.value?.show()
}

function changeScreenshot(offset: number) {
	if (!selectedScreenshot.value || screenshots.value.length < 2) return
	const currentIndex = screenshots.value.findIndex(
		(screenshot) => screenshot.path === selectedScreenshot.value?.path,
	)
	const nextIndex = (currentIndex + offset + screenshots.value.length) % screenshots.value.length
	selectedScreenshot.value = screenshots.value[nextIndex]
	zoomedIn.value = false
}

async function imageToPng(blob: Blob): Promise<Blob> {
	if (blob.type === 'image/png') return blob

	const objectUrl = URL.createObjectURL(blob)
	try {
		const image = new Image()
		image.src = objectUrl
		await image.decode()
		const canvas = document.createElement('canvas')
		canvas.width = image.naturalWidth
		canvas.height = image.naturalHeight
		canvas.getContext('2d')?.drawImage(image, 0, 0)
		return await new Promise<Blob>((resolve, reject) => {
			canvas.toBlob(
				(result) => (result ? resolve(result) : reject(new Error('Image conversion failed'))),
				'image/png',
			)
		})
	} finally {
		URL.revokeObjectURL(objectUrl)
	}
}

async function copyScreenshot(screenshot: Screenshot) {
	try {
		const bytes = await readFile(screenshot.path)
		const extension = extensionOf(screenshot.name)
		const blob = new Blob([bytes], { type: MIME_TYPES[extension] ?? 'image/png' })
		const png = await imageToPng(blob)
		await navigator.clipboard.write([new ClipboardItem({ 'image/png': png })])
		addNotification({
			type: 'success',
			title: formatMessage(messages.copiedScreenshot),
		})
	} catch (error) {
		showError(formatMessage(messages.copyFailed), error)
	}
}

async function saveScreenshot(screenshot: Screenshot) {
	try {
		await invoke('plugin:files|file_save_as', {
			instanceId: props.instance.id,
			filePath: `screenshots/${screenshot.name}`,
		})
	} catch (error) {
		showError(formatMessage(commonMessages.downloadFailedLabel), error)
	}
}

async function openScreenshotsFolder() {
	try {
		await mkdir(screenshotsPath.value, { recursive: true })
		await openPath(screenshotsPath.value)
	} catch (error) {
		showError(formatMessage(messages.loadingFailed), error)
	}
}

async function showScreenshotInFolder(screenshot: Screenshot) {
	try {
		await highlightInFolder(screenshot.path)
	} catch (error) {
		showError(formatMessage(messages.actionFailed), error)
	}
}

async function copyScreenshotText(value: string, successTitle: string) {
	try {
		await navigator.clipboard.writeText(value)
		addNotification({
			type: 'success',
			title: successTitle,
		})
	} catch (error) {
		showError(formatMessage(messages.copyFailed), error)
	}
}

function showScreenshotContextMenu(event: MouseEvent, screenshot: Screenshot) {
	screenshotContextMenu.value?.showMenu(event, screenshot, screenshotContextMenuOptions)
}

async function handleScreenshotContextMenu({
	item,
	option,
}: {
	item: Screenshot
	option: string
}) {
	switch (option) {
		case 'view_screenshot':
			viewScreenshot(item)
			break
		case 'copy_screenshot':
			await copyScreenshot(item)
			break
		case 'save_screenshot':
			await saveScreenshot(item)
			break
		case 'open_screenshot_folder':
			await showScreenshotInFolder(item)
			break
		case 'copy_screenshot_filename':
			await copyScreenshotText(item.name, formatMessage(commonMessages.copiedFilenameLabel))
			break
		case 'copy_screenshot_path':
			await copyScreenshotText(item.path, formatMessage(commonMessages.copiedPathLabel))
			break
		case 'delete_screenshot':
			promptDelete(item)
			break
	}
}

function promptDelete(screenshot: Screenshot) {
	pendingDeletion.value = screenshot
	deleteModal.value?.show()
}

async function confirmDelete() {
	const screenshot = pendingDeletion.value
	if (!screenshot) return

	const deletedIndex = screenshots.value.findIndex((item) => item.path === screenshot.path)
	try {
		await remove(screenshot.path)
		deleteModal.value?.hide()
		pendingDeletion.value = null
		await refresh()

		if (selectedScreenshot.value?.path === screenshot.path) {
			if (screenshots.value.length === 0) {
				viewerModal.value?.hide()
				selectedScreenshot.value = null
			} else {
				selectedScreenshot.value =
					screenshots.value[Math.min(deletedIndex, screenshots.value.length - 1)]
			}
		}
	} catch (error) {
		showError(formatMessage(messages.deleteFailed), error)
	}
}

function handleKeydown(event: KeyboardEvent) {
	if (!selectedScreenshot.value) return
	if (event.key === 'ArrowLeft') {
		event.preventDefault()
		changeScreenshot(-1)
	} else if (event.key === 'ArrowRight') {
		event.preventDefault()
		changeScreenshot(1)
	}
}

async function initialize(instanceId: string) {
	firstPaintPending.value = true
	instanceRoot.value = await get_full_path(instanceId)
	searchQuery.value = ''
	selectedScreenshot.value = null
	await refresh()
}

onMounted(() => {
	window.addEventListener('keydown', handleKeydown)
	window.addEventListener('focus', refresh)
})

onUnmounted(() => {
	window.removeEventListener('keydown', handleKeydown)
	window.removeEventListener('focus', refresh)
	revokePreviewUrls()
})

watch(
	() => props.instance.id,
	(instanceId) => initialize(instanceId),
)

await initialize(props.instance.id)
</script>

<template>
	<ReadyTransition :pending="firstPaintPending">
		<div class="flex flex-col gap-4">
			<div class="flex flex-wrap items-center justify-between gap-3">
				<div v-if="screenshots.length > 0" class="relative min-w-64 flex-1 sm:max-w-md">
					<SearchIcon
						class="pointer-events-none absolute left-3 top-1/2 size-5 -translate-y-1/2 text-secondary"
					/>
					<input
						v-model="searchQuery"
						type="search"
						class="h-10 w-full rounded-xl border border-solid border-surface-5 bg-surface-2 pl-10 pr-3 text-primary outline-none transition-colors focus:border-brand"
						:placeholder="formatMessage(messages.searchPlaceholder, { count: screenshots.length })"
					/>
				</div>
				<div v-else />

				<div class="flex items-center gap-2">
					<ButtonStyled>
						<button @click="openScreenshotsFolder">
							<FolderOpenIcon />
							{{ formatMessage(messages.openScreenshotsFolder) }}
						</button>
					</ButtonStyled>
					<ButtonStyled circular>
						<button
							v-tooltip="formatMessage(commonMessages.refreshButton)"
							:disabled="loading"
							:aria-label="formatMessage(commonMessages.refreshButton)"
							@click="refresh"
						>
							<RefreshCwIcon :class="{ 'animate-spin': loading }" />
						</button>
					</ButtonStyled>
				</div>
			</div>

			<div
				v-if="filteredScreenshots.length > 0"
				class="grid grid-cols-[repeat(auto-fill,minmax(17rem,1fr))] gap-4"
			>
				<article
					v-for="screenshot in filteredScreenshots"
					:key="screenshot.path"
					class="group overflow-hidden rounded-2xl border border-solid border-surface-5 bg-surface-2 transition-colors hover:border-brand"
					@contextmenu.prevent.stop="
						(event) => showScreenshotContextMenu(event, screenshot)
					"
				>
					<button
						class="relative block aspect-video w-full cursor-zoom-in overflow-hidden border-0 bg-surface-1 p-0"
						:aria-label="formatMessage(messages.viewScreenshot)"
						@click="viewScreenshot(screenshot)"
					>
						<img
							:src="screenshot.url"
							:alt="screenshot.name"
							class="size-full object-cover transition-transform duration-200 group-hover:scale-[1.02]"
							@error="loadScreenshotPreview(screenshot)"
						/>
					</button>
					<div class="flex items-center gap-2 p-3">
						<div class="min-w-0 flex-1">
							<div class="truncate font-semibold text-contrast" :title="screenshot.name">
								{{ screenshot.name }}
							</div>
							<div class="mt-0.5 truncate text-sm text-secondary">
								{{ formatDate(screenshot.modified) }} · {{ formatFileSize(screenshot.size) }}
							</div>
						</div>
						<ButtonStyled circular type="transparent">
							<button
								v-tooltip="formatMessage(messages.copyScreenshot)"
								:aria-label="formatMessage(messages.copyScreenshot)"
								@click="copyScreenshot(screenshot)"
							>
								<ClipboardCopyIcon />
							</button>
						</ButtonStyled>
						<ButtonStyled circular type="transparent">
							<button
								v-tooltip="formatMessage(commonMessages.openInFolderButton)"
								:aria-label="formatMessage(commonMessages.openInFolderButton)"
								@click="showScreenshotInFolder(screenshot)"
							>
								<FolderOpenIcon />
							</button>
						</ButtonStyled>
						<ButtonStyled circular type="transparent" color="red" color-fill="text">
							<button
								v-tooltip="formatMessage(messages.deleteScreenshot)"
								:aria-label="formatMessage(messages.deleteScreenshot)"
								@click="promptDelete(screenshot)"
							>
								<TrashIcon />
							</button>
						</ButtonStyled>
					</div>
				</article>
			</div>

			<div
				v-else-if="screenshots.length > 0"
				class="rounded-2xl border border-solid border-surface-5 bg-surface-2 p-8 text-center text-secondary"
			>
				{{ formatMessage(messages.noSearchResults) }}
			</div>

			<EmptyState
				v-else
				type="no-images"
				:heading="formatMessage(messages.noScreenshots)"
				:description="formatMessage(messages.noScreenshotsDescription)"
			>
				<template #actions>
					<ButtonStyled>
						<button @click="openScreenshotsFolder">
							<FolderOpenIcon />
							{{ formatMessage(messages.openScreenshotsFolder) }}
						</button>
					</ButtonStyled>
				</template>
			</EmptyState>
		</div>
	</ReadyTransition>

	<NewModal
		ref="viewerModal"
		:max-width="'92rem'"
		:width="'calc(100vw - 4rem)'"
		:no-padding="true"
		:header="selectedScreenshot?.name"
		:on-hide="() => (selectedScreenshot = null)"
	>
		<div
			v-if="selectedScreenshot"
			class="relative flex min-h-64 max-h-[calc(100vh-13rem)] items-center justify-center overflow-auto bg-surface-1 p-4"
		>
			<img
				:src="selectedScreenshot.url"
				:alt="selectedScreenshot.name"
				:class="zoomedIn ? 'max-w-none cursor-zoom-out' : 'max-h-[calc(100vh-15rem)] max-w-full cursor-zoom-in'"
				@click="zoomedIn = !zoomedIn"
				@error="loadScreenshotPreview(selectedScreenshot)"
				@contextmenu.prevent.stop="
					(event) => showScreenshotContextMenu(event, selectedScreenshot)
				"
			/>
			<ButtonStyled v-if="screenshots.length > 1" class="absolute left-4" circular>
				<button
					:aria-label="formatMessage(commonMessages.backButton)"
					@click="changeScreenshot(-1)"
				>
					<LeftArrowIcon />
				</button>
			</ButtonStyled>
			<ButtonStyled v-if="screenshots.length > 1" class="absolute right-4" circular>
				<button
					:aria-label="formatMessage(commonMessages.nextButton)"
					@click="changeScreenshot(1)"
				>
					<RightArrowIcon />
				</button>
			</ButtonStyled>
		</div>
		<template #actions>
			<div v-if="selectedScreenshot" class="flex flex-wrap items-center justify-between gap-2">
				<div class="text-sm text-secondary">
					{{ formatDate(selectedScreenshot.modified) }} ·
					{{ formatFileSize(selectedScreenshot.size) }}
				</div>
				<div class="flex flex-wrap items-center gap-2">
					<ButtonStyled>
						<button @click="zoomedIn = !zoomedIn">
							<ContractIcon v-if="zoomedIn" />
							<ExpandIcon v-else />
							{{ formatMessage(zoomedIn ? messages.zoomOut : messages.zoomIn) }}
						</button>
					</ButtonStyled>
					<ButtonStyled>
						<button @click="showScreenshotInFolder(selectedScreenshot)">
							<FolderOpenIcon />
							{{ formatMessage(commonMessages.openInFolderButton) }}
						</button>
					</ButtonStyled>
					<ButtonStyled>
						<button @click="saveScreenshot(selectedScreenshot)">
							<DownloadIcon />
							{{ formatMessage(messages.saveAs) }}
						</button>
					</ButtonStyled>
					<ButtonStyled color="brand">
						<button @click="copyScreenshot(selectedScreenshot)">
							<ClipboardCopyIcon />
							{{ formatMessage(messages.copyScreenshot) }}
						</button>
					</ButtonStyled>
					<ButtonStyled color="red" color-fill="text">
						<button @click="promptDelete(selectedScreenshot)">
							<TrashIcon />
							{{ formatMessage(commonMessages.deleteLabel) }}
						</button>
					</ButtonStyled>
				</div>
			</div>
		</template>
	</NewModal>

	<ContextMenu ref="screenshotContextMenu" @option-clicked="handleScreenshotContextMenu">
		<template #view_screenshot>
			<EyeIcon />
			{{ formatMessage(messages.viewScreenshot) }}
		</template>
		<template #copy_screenshot>
			<ClipboardCopyIcon />
			{{ formatMessage(messages.copyScreenshot) }}
		</template>
		<template #save_screenshot>
			<DownloadIcon />
			{{ formatMessage(messages.saveAs) }}
		</template>
		<template #open_screenshot_folder>
			<FolderOpenIcon />
			{{ formatMessage(commonMessages.openInFolderButton) }}
		</template>
		<template #copy_screenshot_filename>
			<ClipboardCopyIcon />
			{{ formatMessage(commonMessages.copyFilenameButton) }}
		</template>
		<template #copy_screenshot_path>
			<ClipboardCopyIcon />
			{{ formatMessage(commonMessages.copyFullPathButton) }}
		</template>
		<template #delete_screenshot>
			<TrashIcon />
			{{ formatMessage(messages.deleteScreenshot) }}
		</template>
	</ContextMenu>

	<NewModal
		ref="deleteModal"
		fade="danger"
		:header="formatMessage(messages.deleteScreenshot)"
		:on-hide="() => (pendingDeletion = null)"
		max-width="32rem"
	>
		<p v-if="pendingDeletion" class="m-0 text-primary">
			{{ formatMessage(messages.deleteDescription, { name: pendingDeletion.name }) }}
		</p>
		<template #actions>
			<div class="flex justify-end gap-2">
				<ButtonStyled>
					<button @click="deleteModal?.hide()">
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="red">
					<button @click="confirmDelete">
						<TrashIcon />
						{{ formatMessage(commonMessages.deleteLabel) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
