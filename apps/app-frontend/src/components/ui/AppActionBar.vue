<template>
	<div class="flex gap-2 items-center">
		<Dropdown
			v-model:shown="notificationCenterShown"
			placement="bottom-end"
			:triggers="['click']"
			:hide-triggers="['click']"
		>
			<ButtonStyled type="transparent" circular>
				<button
					v-tooltip="formatMessage(messages.notifications)"
					:aria-label="formatMessage(messages.notifications)"
					class="relative"
				>
					<BellIcon />
					<span
						v-if="hasUnreadNotifications"
						class="absolute right-0 top-0 size-2 rounded-full bg-red ring-2 ring-bg-raised"
					/>
				</button>
			</ButtonStyled>
			<template #popper>
				<div class="w-[22rem] max-w-[calc(100vw-2rem)] p-2">
					<div class="mb-2 flex items-center justify-between px-2">
						<span class="font-semibold text-contrast">{{
							formatMessage(messages.notifications)
						}}</span>
						<button
							v-if="notificationHistory.length"
							class="text-xs text-secondary hover:text-contrast"
							@click="clearNotificationHistory"
						>
							{{ formatMessage(messages.clearNotifications) }}
						</button>
					</div>
					<div
						v-if="!notificationHistory.length"
						class="px-2 py-4 text-center text-sm text-secondary"
					>
						{{ formatMessage(messages.noNotifications) }}
					</div>
					<div v-else class="flex max-h-[22rem] flex-col gap-1 overflow-auto">
						<div
							v-for="item in notificationHistory"
							:key="item.key"
							class="flex items-start gap-2 rounded-lg p-2 hover:bg-button-bg"
						>
							<div
								class="mt-1 size-2 shrink-0 rounded-full"
								:class="notificationDotClass(item.type)"
							/>
							<button class="min-w-0 flex-1 text-left" @click="openNotification(item)">
								<div class="truncate text-sm font-medium text-contrast">{{ item.title }}</div>
								<div v-if="item.text" class="line-clamp-2 text-xs text-secondary">
									{{ item.text }}
								</div>
							</button>
							<button
								v-tooltip="formatMessage(messages.dismissNotification)"
								class="shrink-0 text-secondary hover:text-contrast"
								@click="dismissNotification(item)"
							>
								<XIcon class="size-4" />
							</button>
						</div>
					</div>
				</div>
			</template>
		</Dropdown>
		<ButtonStyled
			v-if="!isDownloadsPage && hasActiveDownloads && !hasVisibleActiveDownloadToasts"
			color="brand"
			type="transparent"
			circular
		>
			<button v-tooltip="formatMessage(messages.viewActiveDownloads)" @click="goToDownloads">
				<DownloadIcon />
			</button>
		</ButtonStyled>
		<div v-if="offline" class="flex items-center gap-1">
			<UnplugIcon class="text-secondary" />
			<span class="text-sm text-contrast"> {{ formatMessage(messages.offline) }} </span>
		</div>
		<AppUpdateButton />
		<div
			class="flex border-solid border-surface-5 text-sm items-center gap-2 py-1.5 px-3 rounded-xl border"
		>
			<template v-if="selectedProcess">
				<OnlineIndicatorIcon />
				<div class="text-contrast flex items-center gap-2">
					<router-link
						v-tooltip="formatMessage(messages.viewInstance)"
						:to="`/instance/${encodeURIComponent(selectedProcess.instance.id)}`"
						class="hover:underline"
					>
						{{ selectedProcess.instance.name }}
					</router-link>
					<Dropdown
						v-if="currentProcesses.length > 1"
						placement="bottom"
						:triggers="['click']"
						:hide-triggers="['click']"
						@show="showInstances = true"
						@hide="showInstances = false"
					>
						<ButtonStyled type="transparent" circular size="small">
							<button
								v-tooltip="
									showInstances
										? formatMessage(messages.hideMoreRunningInstances)
										: formatMessage(messages.showMoreRunningInstances)
								"
							>
								<DropdownIcon :class="{ 'rotate-180': !!showInstances }" />
							</button>
						</ButtonStyled>
						<template #popper>
							<div class="flex w-[20rem] max-h-[24rem] flex-col gap-2 overflow-auto">
								<div
									v-for="process in currentProcesses"
									:key="process.uuid"
									class="flex w-full items-center gap-2 rounded-xl bg-surface-4 p-2 text-sm"
								>
									<button
										v-tooltip.left="
											process.uuid === selectedProcess.uuid
												? formatMessage(messages.primaryInstance)
												: formatMessage(messages.makePrimaryInstance)
										"
										class="flex flex-grow items-center gap-2"
										:class="{
											'active:scale-95 transition-transform': process.uuid !== selectedProcess.uuid,
										}"
										:disabled="process.uuid === selectedProcess.uuid"
										@click="selectProcess(process)"
									>
										<OnlineIndicatorIcon />
										<span class="mr-auto text-contrast flex items-center gap-2">
											{{ process.instance.name }}
											<StarIcon v-if="process.uuid === selectedProcess.uuid" class="text-orange" />
										</span>
									</button>
									<button
										v-tooltip="formatMessage(messages.stopInstance)"
										class="active:scale-95 flex"
										@click.stop="stop(process)"
									>
										<StopCircleIcon class="text-red size-5" />
									</button>
									<button
										v-tooltip="formatMessage(messages.viewLogs)"
										class="active:scale-95 flex"
										@click.stop="goToTerminal(process.instance.id)"
									>
										<TerminalSquareIcon class="text-secondary size-5" />
									</button>
								</div>
							</div>
						</template>
					</Dropdown>
				</div>
				<button
					v-tooltip="formatMessage(messages.stopInstance)"
					class="active:scale-95 flex"
					@click="stop(selectedProcess)"
				>
					<StopCircleIcon class="text-red size-5" />
				</button>
				<button
					v-tooltip="formatMessage(messages.viewLogs)"
					class="active:scale-95 flex"
					@click="goToTerminal()"
				>
					<TerminalSquareIcon class="text-secondary size-5" />
				</button>
			</template>
			<template v-else>
				<span class="size-2 rounded-full bg-secondary" />
				<span class="text-secondary"> {{ formatMessage(messages.noInstancesRunning) }} </span>
			</template>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	BellIcon,
	DownloadIcon,
	DropdownIcon,
	OnlineIndicatorIcon,
	StarIcon,
	StopCircleIcon,
	TerminalSquareIcon,
	UnplugIcon,
	XIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	injectPopupNotificationManager,
	type PopupNotification,
	type PopupNotificationProgressItem,
	useVIntl,
	type WebNotification,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { Dropdown } from 'floating-vue'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import AppUpdateButton from '@/components/ui/app-update-button/index.vue'
import { useInstallJobNotifications } from '@/composables/browse/install-job-notifications'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { trackEvent } from '@/helpers/analytics'
import { loading_listener, process_listener } from '@/helpers/events'
import { get_many as getInstances } from '@/helpers/instance'
import { get_all as getRunningProcesses, kill as killProcess } from '@/helpers/process'
import type { LoadingBar } from '@/helpers/state'
import { progress_bars_list } from '@/helpers/state'
import type { GameInstance } from '@/helpers/types'
import { downloadBarTypes, injectDownloadManager } from '@/providers/download-manager'

const notificationManager = injectNotificationManager()
const { handleError } = notificationManager
const popupNotificationManager = injectPopupNotificationManager()
const downloadManager = injectDownloadManager()
const { formatMessage } = useVIntl()

type NotificationHistoryItem = {
	key: string
	createdAt?: number
	title: string
	text?: string
	type?: 'error' | 'warning' | 'success' | 'info' | 'download'
	collapsed?: boolean
	expand: () => void
	dismiss: () => void
}

const notificationHistory = computed<NotificationHistoryItem[]>(() =>
	[
		...notificationManager.getNotifications().map((item: WebNotification) => ({
			key: `web-${item.id}`,
			createdAt: item.createdAt,
			title: item.title ?? formatMessage(messages.notifications),
			text: item.text,
			type: item.type,
			collapsed: item.collapsed,
			expand: () => notificationManager.expandNotification(item.id),
			dismiss: () => notificationManager.removeNotification(item.id),
		})),
		...popupNotificationManager.getNotifications().map((item: PopupNotification) => ({
			key: `popup-${item.id}`,
			createdAt: item.createdAt,
			title: item.title,
			text:
				item.text ??
				(item.progressItems
					?.filter((progressItem) => progressItem.text)
					.map((progressItem) => `${progressItem.title}: ${progressItem.text}`)
					.join('\n') ||
					undefined),
			type: item.type,
			collapsed: item.collapsed,
			expand: () => popupNotificationManager.expandNotification(item.id),
			dismiss: () => popupNotificationManager.removeNotification(item.id),
		})),
	].sort((a, b) => (b.createdAt ?? 0) - (a.createdAt ?? 0)),
)

const hasUnreadNotifications = computed(() =>
	notificationHistory.value.some(
		(item) => !item.collapsed && ['error', 'warning'].includes(item.type ?? ''),
	),
)

function notificationDotClass(type?: NotificationHistoryItem['type']): string {
	if (type === 'error') return 'bg-red'
	if (type === 'warning') return 'bg-orange'
	if (type === 'success') return 'bg-green'
	if (type === 'download') return 'bg-green'
	return 'bg-blue'
}

function dismissNotification(item: NotificationHistoryItem) {
	item.dismiss()
}

async function openNotification(item: NotificationHistoryItem) {
	item.expand()
	notificationCenterShown.value = false
}

function clearNotificationHistory() {
	for (const item of notificationManager.getNotifications()) {
		notificationManager.collapseNotification(item.id)
	}
	for (const item of popupNotificationManager.getNotifications()) {
		popupNotificationManager.collapseNotification(item.id)
	}
}

const router = useRouter()
const route = useRoute()
const isDownloadsPage = computed(
	() => route.path === '/downloads' || route.path.startsWith('/downloads/'),
)

const showInstances = ref(false)
const notificationCenterShown = ref(false)

interface RunningProcess {
	uuid: string
	instance_id: string
	instance: GameInstance
}

interface LoadingEventPayload {
	event: LoadingBar['bar_type']
	loader_uuid: string
	fraction: number | null
	message: string
}

const messages = defineMessages({
	offline: {
		id: 'app.action-bar.offline',
		defaultMessage: 'Offline',
	},
	viewInstance: {
		id: 'app.action-bar.view-instance',
		defaultMessage: 'View instance',
	},
	showMoreRunningInstances: {
		id: 'app.action-bar.show-more-running-instances',
		defaultMessage: 'Show more running instances',
	},
	hideMoreRunningInstances: {
		id: 'app.action-bar.hide-more-running-instances',
		defaultMessage: 'Hide more running instances',
	},
	primaryInstance: {
		id: 'app.action-bar.primary-instance',
		defaultMessage: 'Primary instance',
	},
	makePrimaryInstance: {
		id: 'app.action-bar.make-primary-instance',
		defaultMessage: 'Make primary instance',
	},
	stopInstance: {
		id: 'app.action-bar.stop-instance',
		defaultMessage: 'Stop instance',
	},
	viewLogs: {
		id: 'app.action-bar.view-logs',
		defaultMessage: 'View logs',
	},
	noInstancesRunning: {
		id: 'app.action-bar.no-instances-running',
		defaultMessage: 'No instances running',
	},
	notifications: {
		id: 'app.action-bar.notifications',
		defaultMessage: 'Notifications',
	},
	clearNotifications: {
		id: 'app.action-bar.notifications.clear',
		defaultMessage: 'Clear all',
	},
	noNotifications: {
		id: 'app.action-bar.notifications.empty',
		defaultMessage: 'No notifications',
	},
	dismissNotification: {
		id: 'app.action-bar.notifications.dismiss',
		defaultMessage: 'Dismiss notification',
	},
	downloadingJava: {
		id: 'app.action-bar.downloading-java',
		defaultMessage: 'Downloading Java {version}',
	},
	downloadingModpack: {
		id: 'app.downloads.phase.downloading-pack-file',
		defaultMessage: 'Downloading modpack',
	},
	downloads: {
		id: 'app.action-bar.downloads',
		defaultMessage: 'Downloads',
	},
	viewActiveDownloads: {
		id: 'app.action-bar.view-active-downloads',
		defaultMessage: 'View active downloads',
	},
	exportingModpack: {
		id: 'app.action-bar.exporting-modpack',
		defaultMessage: 'Exporting modpack',
	},
})

const currentProcesses = ref<RunningProcess[]>([])
const selectedProcess = ref<RunningProcess | undefined>()

const refresh = async () => {
	const processes = ((await getRunningProcesses().catch((error) => {
		handleError(error)
		return []
	})) ?? []) as Array<{ uuid: string; instance_id: string }>
	const instanceIds = processes.map((process) => process.instance_id)
	const instances: GameInstance[] = await getInstances(instanceIds).catch((error) => {
		handleError(error)
		return []
	})

	currentProcesses.value = processes
		.map((process) => {
			const instance = instances.find((item) => process.instance_id === item.id)
			if (!instance) {
				return null
			}
			return {
				...process,
				instance,
			}
		})
		.filter((process): process is RunningProcess => process !== null)
	if (!selectedProcess.value || !currentProcesses.value.includes(selectedProcess.value)) {
		selectedProcess.value = currentProcesses.value[0]
	}
}

await refresh()

const { offline } = useNetworkStatus()

const unlistenProcess = await process_listener(async () => {
	await refresh()
})

const stop = async (process: RunningProcess) => {
	try {
		await killProcess(process.uuid).catch(handleError)

		trackEvent('InstanceStop', {
			loader: process.instance.loader,
			game_version: process.instance.game_version,
			source: 'AppBar',
		})
	} catch (e) {
		console.error(e)
	}
	await refresh()
}

function goToTerminal(instanceId?: string) {
	const selectedInstanceId = instanceId ?? selectedProcess.value?.instance.id
	if (!selectedInstanceId) {
		return
	}
	router.push(`/instance/${encodeURIComponent(selectedInstanceId)}/logs`)
}

const currentLoadingBars = ref<LoadingBar[]>([])
const currentLoadingBarIconUrls = ref<Record<string, string | null>>({})
const notificationId = ref<string | number | null>(null)
const dismissed = ref(false)

function getLoadingBarKey(loadingBar: LoadingBar): string {
	return `${loadingBar.loading_bar_uuid ?? loadingBar.id}`
}

function getLoadingProgress(loadingBar: LoadingBar): number {
	if (!loadingBar.total || loadingBar.total <= 0) {
		return 0
	}
	return Math.max(0, Math.min(1, (loadingBar.current ?? 0) / (loadingBar.total ?? 0)))
}

function getLoadingText(loadingBar: LoadingBar): string {
	return loadingBar.message ?? ''
}

function getDisplayIconUrl(icon: string | null | undefined): string | null {
	if (!icon) {
		return null
	}
	if (/^(https?:|data:|blob:|asset:|tauri:)/.test(icon)) {
		return icon
	}
	return convertFileSrc(icon)
}

function getNotification(): PopupNotification | null {
	if (!notificationId.value) {
		return null
	}
	const notification = popupNotificationManager
		.getNotifications()
		.find((notification) => notification.id === notificationId.value)
	return notification ?? null
}

function collapseNotification(): void {
	if (!notificationId.value) {
		return
	}
	popupNotificationManager.collapseNotification(notificationId.value)
}

function removeNotification(): void {
	if (!notificationId.value) {
		return
	}
	popupNotificationManager.removeNotification(notificationId.value)
	notificationId.value = null
}

function buildDownloadItems(): PopupNotificationProgressItem[] {
	return [
		...installJobNotifications.progressItems.value,
		...currentLoadingBars.value.map((bar) => {
			const isPackDownload = bar.bar_type?.type === 'pack_download'
			return {
				id: getLoadingBarKey(bar),
				title: bar.title ?? '',
				text: getLoadingText(bar),
				iconUrl: currentLoadingBarIconUrls.value[getLoadingBarKey(bar)] ?? null,
				progress: getLoadingProgress(bar),
				waiting: !bar.total || bar.total <= 0,
				// Pack downloads report file counts, so prefer count UI over raw percentage.
				progressType: isPackDownload ? 'count' : 'percentage',
				progressCurrent: bar.current,
				progressTotal: bar.total,
			}
		}),
	]
}

const hasVisibleActiveDownloadToasts = computed(() => {
	const notification = getNotification()
	return !!notification && !notification.collapsed
})
const hasActiveDownloads = computed(
	() =>
		installJobNotifications.active.value ||
		currentLoadingBars.value.some((bar) => downloadBarTypes.has(bar.bar_type?.type ?? '')),
)
const hasDownloadNotificationItems = computed(
	() => installJobNotifications.hasItems.value || currentLoadingBars.value.length > 0,
)

function updateNotification(resummon = false): void {
	const shouldResummon = resummon && !isDownloadsPage.value
	if (shouldResummon) {
		dismissed.value = false
	}

	if (!hasDownloadNotificationItems.value) {
		removeNotification()
		dismissed.value = false
		return
	}

	if (notificationId.value && !getNotification()) {
		notificationId.value = null
		dismissed.value = true
	}

	if (dismissed.value && !shouldResummon) {
		return
	}

	let notif = getNotification()
	if (notif?.collapsed && shouldResummon) {
		notif.collapsed = false
	}
	const progressItems = buildDownloadItems()

	if (notif) {
		notif.title = installJobNotifications.hasItems.value
			? installJobNotifications.title.value
			: formatMessage(messages.downloads)
		notif.text = undefined
		notif.progressItems = progressItems
		notif.buttons = installJobNotifications.buttons.value
		notif.onClick = hasDownloadNotificationItems.value ? goToDownloads : undefined
		notif.progress = undefined
		notif.waiting = undefined
		notif.autoCloseMs =
			progressItems.length > 0 && progressItems.every((item) => item.showProgress === false)
				? 30 * 1000
				: null
		if (!notif.collapsed) popupNotificationManager.setNotificationTimer(notif)
	} else {
		notif = popupNotificationManager.addPopupNotification({
			title: installJobNotifications.hasItems.value
				? installJobNotifications.title.value
				: formatMessage(messages.downloads),
			type: 'download',
			autoCloseMs: null,
			progressItems,
			buttons: installJobNotifications.buttons.value,
			onClick: hasDownloadNotificationItems.value ? goToDownloads : undefined,
		})
		notificationId.value = notif.id
		if (isDownloadsPage.value) {
			popupNotificationManager.collapseNotification(notif.id)
		}
		if (progressItems.length > 0 && progressItems.every((item) => item.showProgress === false)) {
			notif.autoCloseMs = 30 * 1000
			popupNotificationManager.setNotificationTimer(notif)
		}
	}
}

function formatLoadingBars(loadingBar: LoadingBar): LoadingBar {
	const formatted = { ...loadingBar }
	if (formatted.bar_type?.type === 'java_download') {
		formatted.title = formatMessage(messages.downloadingJava, {
			version: formatted.bar_type.version,
		})
	}
	if (formatted.bar_type?.type === 'pack_file_download') {
		formatted.message = formatMessage(messages.downloadingModpack)
	}
	if (formatted.bar_type?.instance_id) {
		formatted.title = formatted.bar_type.instance_name ?? formatted.bar_type.instance_id
	}
	if (formatted.bar_type?.type === 'zip_extract') {
		formatted.title = formatMessage(messages.exportingModpack)
	}
	if (formatted.bar_type?.pack_name) {
		formatted.title = formatted.bar_type.pack_name
	}
	return formatted
}

function isVisibleLoadingBar(loadingBar: LoadingBar): boolean {
	return (
		loadingBar.bar_type?.type !== 'launcher_update' &&
		[
			'java_download',
			'pack_file_download',
			'pack_download',
			'minecraft_download',
			'copy_instance',
			'zip_extract',
		].includes(loadingBar.bar_type?.type ?? '')
	)
}

function applyLoadingEvent(payload: LoadingEventPayload): boolean {
	const key = payload.loader_uuid
	const index = currentLoadingBars.value.findIndex((bar) => getLoadingBarKey(bar) === key)

	if (payload.fraction === null) {
		if (index >= 0) {
			currentLoadingBars.value.splice(index, 1)
			const { [key]: _removedIcon, ...remainingIcons } = currentLoadingBarIconUrls.value
			currentLoadingBarIconUrls.value = remainingIcons
		}
		return false
	}

	const loadingBar = formatLoadingBars({
		loading_bar_uuid: payload.loader_uuid,
		message: payload.message,
		current: payload.fraction,
		total: 1,
		bar_type: payload.event,
	})
	if (!isVisibleLoadingBar(loadingBar)) return false

	if (index >= 0) {
		currentLoadingBars.value.splice(index, 1, loadingBar)
	} else {
		currentLoadingBars.value.push(loadingBar)
	}
	currentLoadingBarIconUrls.value[key] = getDisplayIconUrl(payload.event?.icon)
	return index < 0
}

async function refreshLoadingBars() {
	const bars: Record<string, LoadingBar> = await progress_bars_list().catch((error) => {
		handleError(error)
		return {}
	})

	currentLoadingBars.value = Object.values(bars).map(formatLoadingBars).filter(isVisibleLoadingBar)

	const instanceIds = Array.from(
		new Set(
			currentLoadingBars.value
				.map((bar) => bar.bar_type?.instance_id)
				.filter((instanceId): instanceId is string => !!instanceId),
		),
	)
	const instances = instanceIds.length
		? await getInstances(instanceIds).catch((error) => {
				handleError(error)
				return []
			})
		: []
	const instanceIconUrls = new Map(
		instances.map((instance) => [instance.id, getDisplayIconUrl(instance.icon_path)]),
	)
	currentLoadingBarIconUrls.value = Object.fromEntries(
		currentLoadingBars.value.map((bar) => {
			const barIconUrl = getDisplayIconUrl(bar.bar_type?.icon)
			const instanceIconUrl = bar.bar_type?.instance_id
				? instanceIconUrls.get(bar.bar_type.instance_id)
				: null
			return [getLoadingBarKey(bar), barIconUrl ?? instanceIconUrl ?? null]
		}),
	)

	currentLoadingBars.value.sort((a, b) => {
		const aKey = `${a.loading_bar_uuid ?? a.id ?? ''}`
		const bKey = `${b.loading_bar_uuid ?? b.id ?? ''}`
		return aKey.localeCompare(bKey)
	})

	updateNotification()
}

const installJobNotifications = await useInstallJobNotifications({
	router,
	manager: downloadManager,
	handleError,
	onChange: updateNotification,
})

await refreshLoadingBars()

let newBarDuringWindow = false
let loadingNotificationTimer: ReturnType<typeof setTimeout> | null = null

const unlistenLoading = await loading_listener((payload: LoadingEventPayload) => {
	const isNewBar = applyLoadingEvent(payload)
	if (isNewBar) {
		newBarDuringWindow = true
	}
	if (loadingNotificationTimer !== null) {
		return
	}
	loadingNotificationTimer = setTimeout(() => {
		loadingNotificationTimer = null
		if (newBarDuringWindow) {
			newBarDuringWindow = false
			if (isDownloadsPage.value) {
				updateNotification()
			} else {
				removeNotification()
				updateNotification(true)
			}
		} else {
			updateNotification()
		}
	}, 250)
})

function goToDownloads() {
	router.push('/downloads')
}

watch(
	() => route.path,
	() => {
		if (isDownloadsPage.value) {
			collapseNotification()
		}
		updateNotification()
	},
)

function selectProcess(process: RunningProcess) {
	selectedProcess.value = process
}

onBeforeUnmount(() => {
	if (loadingNotificationTimer !== null) {
		clearTimeout(loadingNotificationTimer)
		loadingNotificationTimer = null
	}
	removeNotification()
	dismissed.value = false
	unlistenProcess()
	unlistenLoading()
	installJobNotifications.dispose()
})
</script>
