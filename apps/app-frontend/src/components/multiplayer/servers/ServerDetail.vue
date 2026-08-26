<script setup lang="ts">
import {
	ArrowLeftIcon,
	FolderOpenIcon,
	GlobeIcon,
	LoaderCircleIcon,
	MoreVerticalIcon,
	PencilIcon,
	PlayIcon,
	RefreshCwIcon,
	ShieldIcon,
	StopCircleIcon,
	TerminalSquareIcon,
	WrenchIcon,
} from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	defineMessages,
	injectFilePicker,
	injectNotificationManager,
	NavTabs,
	OverflowMenu,
	TagItem,
	useVIntl,
} from '@modrinth/ui'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import EulaModal from '@/components/multiplayer/servers/EulaModal.vue'
import {
	isServerStatusVisible,
	SERVER_STATUS_META,
} from '@/components/multiplayer/servers/server-status'
import ServerConsole from '@/components/multiplayer/servers/ServerConsole.vue'
import ServerFilesPanel from '@/components/multiplayer/servers/ServerFilesPanel.vue'
import ServerIcon from '@/components/multiplayer/servers/ServerIcon.vue'
import ServerSettingsPanel from '@/components/multiplayer/servers/ServerSettingsPanel.vue'
import { useMultiplayerSession } from '@/composables/useMultiplayerSession'
import { useServerLifecycle } from '@/composables/useServerLifecycle'
import { useServers } from '@/composables/useServers'
import { type PortProcessInfoData, servers as serversApi } from '@/helpers/servers'
import { openPath } from '@/helpers/utils'

const route = useRoute()
const router = useRouter()
const serverId = route.params.id as string

const { servers, refresh, stopServer } = useServers()
const { eulaModal, eulaText, tryStartServer, acceptEula, declineEula } = useServerLifecycle()
const filePicker = injectFilePicker()
const multiplayerSession = useMultiplayerSession()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	console: { id: 'app.servers.detail.console', defaultMessage: 'Console' },
	files: { id: 'app.servers.detail.files', defaultMessage: 'Files' },
	settings: { id: 'app.servers.detail.settings', defaultMessage: 'Settings' },
	back: { id: 'app.servers.detail.back', defaultMessage: 'Servers' },
	start: { id: 'app.servers.action.start', defaultMessage: 'Start' },
	stop: { id: 'app.servers.action.stop', defaultMessage: 'Stop' },
	openFolder: { id: 'app.servers.action.open-folder', defaultMessage: 'Open folder' },
	share: { id: 'app.servers.action.share', defaultMessage: 'Share online' },
	notFound: {
		id: 'app.servers.detail.not-found',
		defaultMessage: 'This server no longer exists.',
	},
	typeLabel: {
		id: 'app.servers.card.type',
		defaultMessage: '{type} · {version}',
	},
	port: { id: 'app.servers.card.port', defaultMessage: 'Port {port}' },
	editIcon: { id: 'app.servers.icon.edit', defaultMessage: 'Edit icon' },
	removeIcon: { id: 'app.servers.icon.remove', defaultMessage: 'Remove icon' },
	portConflictTitle: {
		id: 'app.servers.port.conflict-title',
		defaultMessage: 'Port {port} is already in use',
	},
	portConflictDescription: {
		id: 'app.servers.port.conflict-description',
		defaultMessage:
			'{process} is currently occupying this port, so the server cannot start. Change the server port, or force quit the process below.',
	},
	portUnknownProcess: {
		id: 'app.servers.port.unknown-process',
		defaultMessage: 'Unknown process (PID {pid})',
	},
	portForceQuit: {
		id: 'app.servers.port.force-quit',
		defaultMessage: 'Force quit process',
	},
	portChange: {
		id: 'app.servers.port.change',
		defaultMessage: 'Change port',
	},
	portRecheck: {
		id: 'app.servers.port.recheck',
		defaultMessage: 'Recheck',
	},
	portForceQuitFailed: {
		id: 'app.servers.port.force-quit-failed',
		defaultMessage: 'Failed to quit the process occupying the port',
	},
})

const server = computed(() => servers.value.find((entry) => entry.id === serverId))
const statusMeta = computed(() => (server.value ? SERVER_STATUS_META[server.value.status] : null))
const showStatus = computed(() =>
	server.value ? isServerStatusVisible(server.value.status) : false,
)

const isLoaded = ref(false)
const hasSeenServer = ref(false)

const DEFAULT_SERVER_PORT = 25565
const PORT_CHECK_INTERVAL_MS = 10_000

const { addNotification } = injectNotificationManager()

const portProcess = ref<PortProcessInfoData | null>(null)
const checkingPort = ref(false)
const killingPortProcess = ref(false)
let portCheckToken = 0

const effectivePort = computed(() =>
	server.value ? (server.value.port ?? DEFAULT_SERVER_PORT) : null,
)
const portConflict = computed(
	() => !!server.value && server.value.status !== 'running' && !!portProcess.value,
)
const occupyingProcessLabel = computed(() => {
	const info = portProcess.value
	if (!info) return ''
	return info.name
		? `${info.name} (PID ${info.pid})`
		: formatMessage(messages.portUnknownProcess, { pid: info.pid })
})

/** Polls whether something else is listening on the server's port. */
async function checkPortOccupation(silent = true) {
	const port = effectivePort.value
	if (port == null || server.value?.status === 'running') {
		portCheckToken++
		portProcess.value = null
		return
	}
	if (!silent) checkingPort.value = true
	const token = ++portCheckToken
	try {
		const info = await serversApi.portProcess(port)
		if (token === portCheckToken) portProcess.value = info
	} catch {
		if (token === portCheckToken) portProcess.value = null
	} finally {
		if (!silent) checkingPort.value = false
	}
}

function recheckPort() {
	void checkPortOccupation(false)
}

async function forceQuitPortProcess() {
	const port = effectivePort.value
	if (port == null || killingPortProcess.value) return
	killingPortProcess.value = true
	try {
		await serversApi.killPortProcess(port)
		await checkPortOccupation()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.portForceQuitFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	} finally {
		killingPortProcess.value = false
	}
}

/** Opens the settings tab and focuses the port field once the properties editor has loaded. */
async function goToPortSetting() {
	tabIndex.value = 2
	let portField: HTMLElement | null = null
	for (let i = 0; i < 20 && !portField; i++) {
		await nextTick()
		portField = document.getElementById('server-prop-server-port')
		if (!portField) await new Promise((resolve) => setTimeout(resolve, 100))
	}
	if (portField) {
		portField.scrollIntoView({ behavior: 'smooth', block: 'center' })
		portField.focus({ preventScroll: true })
	}
}

watch([() => server.value?.status, effectivePort], () => void checkPortOccupation(), {
	immediate: true,
})

const portCheckTimer = setInterval(() => void checkPortOccupation(), PORT_CHECK_INTERVAL_MS)
onUnmounted(() => clearInterval(portCheckTimer))

onMounted(async () => {
	if (servers.value.length === 0) await refresh().catch(() => {})
	isLoaded.value = true
})

// A server disappearing after it was loaded means it was deleted: go back to the list
// instead of showing a "no longer exists" dead end.
watch([server, isLoaded], ([value, loaded]) => {
	if (value) {
		hasSeenServer.value = true
		return
	}
	if (loaded && hasSeenServer.value) void router.replace('/multiplayer/servers')
})

const tabIndex = ref(route.query.tab === 'files' ? 1 : route.query.tab === 'settings' ? 2 : 0)
const tabLinks = computed(() => [
	{ label: formatMessage(messages.console), href: 'console', icon: TerminalSquareIcon },
	{ label: formatMessage(messages.files), href: 'files', icon: FolderOpenIcon },
	{ label: formatMessage(messages.settings), href: 'settings', icon: WrenchIcon },
])

async function toggleRunning() {
	if (!server.value) return
	if (server.value.status === 'running') {
		await stopServer(server.value.id)
	} else {
		await tryStartServer(server.value)
	}
}

async function setServerIcon() {
	if (!server.value) return
	try {
		const picked = await (filePicker.pickInstanceIcon?.() ?? filePicker.pickImage())
		if (!picked?.path) return
		await serversApi.setIcon(server.value.id, picked.path)
		await refresh()
	} catch (error) {
		console.error(error)
	}
}

async function resetServerIcon() {
	if (!server.value?.iconPath) return
	try {
		await serversApi.setIcon(server.value.id, null)
		await refresh()
	} catch (error) {
		console.error(error)
	}
}

async function shareOnline() {
	if (!server.value?.port) return
	await router.push({ path: '/multiplayer/rooms' })
	void multiplayerSession.hostHongshi(server.value.port, null, null)
}
</script>

<template>
	<div class="multiplayer-fixed-render flex min-h-0 w-full flex-1 flex-col gap-3">
		<div v-if="!server && isLoaded && !hasSeenServer" class="text-secondary">
			{{ formatMessage(messages.notFound) }}
		</div>

		<template v-else-if="server">
			<div class="flex min-w-0 shrink-0 flex-wrap items-center justify-between gap-3">
				<div class="flex min-w-0 items-center gap-3">
					<ButtonStyled type="transparent" circular>
						<button
							type="button"
							:aria-label="formatMessage(messages.back)"
							@click="router.push('/multiplayer/servers')"
						>
							<ArrowLeftIcon />
						</button>
					</ButtonStyled>
					<div class="group relative shrink-0">
						<button
							v-tooltip="formatMessage(messages.editIcon)"
							type="button"
							class="cursor-pointer rounded-xl transition-transform group-active:scale-95"
							:aria-label="formatMessage(messages.editIcon)"
							@click="setServerIcon"
						>
							<ServerIcon
								:icon-path="server.iconPath"
								:server-type="server.serverType"
								:server-id="server.id"
								size="44px"
							/>
						</button>
						<OverflowMenu
							v-if="server.iconPath"
							class="absolute -right-1 -top-1 flex size-5 items-center justify-center rounded-full bg-surface-4 text-secondary shadow-md transition-colors hover:text-contrast"
							:options="[
								{
									id: 'remove',
									color: 'danger',
									action: () => resetServerIcon(),
								},
							]"
						>
							<MoreVerticalIcon class="size-3.5" />
						</OverflowMenu>
					</div>
					<div class="min-w-0">
						<div class="flex min-w-0 items-center gap-2">
							<h2 class="m-0 truncate text-xl font-semibold text-contrast">
								{{ server.name }}
							</h2>
							<TagItem v-if="showStatus && statusMeta" class="shrink-0">
								<span :class="`font-semibold ${statusMeta.color}`">
									{{ formatMessage(statusMeta.label) }}
								</span>
							</TagItem>
						</div>
						<div class="mt-0.5 flex min-w-0 items-center gap-2 text-sm text-secondary">
							<span class="truncate">
								{{
									formatMessage(messages.typeLabel, {
										type: server.serverType,
										version: server.gameVersion,
									})
								}}
							</span>
							<span v-if="server.port" class="shrink-0">
								{{ formatMessage(messages.port, { port: server.port }) }}
							</span>
						</div>
					</div>
				</div>

				<div class="flex flex-wrap gap-2">
					<ButtonStyled v-if="server.status !== 'running' && !portConflict" color="brand">
						<button type="button" @click="toggleRunning">
							<PlayIcon />
							{{ formatMessage(messages.start) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-else-if="server.status === 'running'" color="red" type="outlined">
						<button type="button" @click="toggleRunning">
							<StopCircleIcon />
							{{ formatMessage(messages.stop) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="server.status === 'running' && server.port" type="outlined">
						<button type="button" @click="shareOnline">
							<GlobeIcon />
							{{ formatMessage(messages.share) }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="outlined">
						<button type="button" @click="openPath(server.path)">
							<FolderOpenIcon />
							{{ formatMessage(messages.openFolder) }}
						</button>
					</ButtonStyled>
				</div>
			</div>

			<Admonition
				v-if="portConflict && portProcess"
				type="warning"
				:header="formatMessage(messages.portConflictTitle, { port: effectivePort })"
			>
				{{
					formatMessage(messages.portConflictDescription, {
						process: occupyingProcessLabel,
					})
				}}
				<template #actions>
					<div class="flex flex-wrap items-center gap-2">
						<ButtonStyled color="brand">
							<button type="button" :disabled="killingPortProcess" @click="goToPortSetting">
								<PencilIcon />
								{{ formatMessage(messages.portChange) }}
							</button>
						</ButtonStyled>
						<ButtonStyled color="red">
							<button type="button" :disabled="killingPortProcess" @click="forceQuitPortProcess">
								<LoaderCircleIcon v-if="killingPortProcess" class="animate-spin" />
								<ShieldIcon v-else />
								{{ formatMessage(messages.portForceQuit) }}
							</button>
						</ButtonStyled>
						<ButtonStyled type="transparent">
							<button type="button" :disabled="checkingPort" @click="recheckPort">
								<LoaderCircleIcon v-if="checkingPort" class="animate-spin" />
								<RefreshCwIcon v-else />
								{{ formatMessage(messages.portRecheck) }}
							</button>
						</ButtonStyled>
					</div>
				</template>
			</Admonition>

			<NavTabs
				mode="local"
				:active-index="tabIndex"
				:links="tabLinks"
				@tab-click="tabIndex = $event"
			/>

			<div v-if="tabIndex === 0" class="max-h-[calc(100dvh-var(--top-bar-height))] min-h-0 flex-1">
				<ServerConsole :server="server" />
			</div>
			<div v-else-if="tabIndex === 1" class="min-h-0 flex-1 overflow-y-auto pr-1">
				<ServerFilesPanel :server="server" />
			</div>
			<div v-else class="min-h-0 flex-1 overflow-y-auto pr-1">
				<ServerSettingsPanel :server="server" @deleted="router.push('/multiplayer/servers')" />
			</div>

			<EulaModal ref="eulaModal" :text="eulaText" @accept="acceptEula" @decline="declineEula" />
		</template>
	</div>
</template>

<style>
/*
 * fixed 渲染模式（服务器详情页）：页面自身不滚动，控制台/设置区内部滚动，
 * 命令输入框始终停留在可视区域内。
 * 显式把 page-transition-grid 定高（100%），让整条 h-full 百分比高度链有确定参照，
 * 避免网格行高被日志内容撑开导致终端无限增高。
 * 只去掉 .app-viewport 的 scrollbar-gutter（避免多余的空滚动条轨道），
 * 保留 overflow: auto 作为兜底——内容万一超出视口仍可滚动，不会被裁切。
 */
.app-viewport:has(.multiplayer-fixed-render) .page-transition-grid {
	height: 100%;
}
.app-viewport:has(.multiplayer-fixed-render) {
	scrollbar-gutter: auto;
}
</style>
