<script setup lang="ts">
import {
	MoreVerticalIcon,
	NoSignalIcon,
	PinIcon,
	PlayIcon,
	ServerIcon,
	SignalIcon,
	SpinnerIcon,
	StopCircleIcon,
} from '@modrinth/assets'
import {
	Avatar,
	Button,
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	OverflowMenu,
	SmartClickable,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import type { HomeWidgetSize } from '@/components/home/home-dashboard'
import { useHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import ManagedServerIcon from '@/components/multiplayer/servers/ServerIcon.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { useServers } from '@/composables/useServers'
import { trackEvent } from '@/helpers/analytics'
import { kill } from '@/helpers/instance'
import { servers as serversApi } from '@/helpers/servers'
import type { GameInstance } from '@/helpers/types'
import {
	getWorldIdentifier,
	type ServerWorld,
	set_world_display_status,
	start_join_server,
	type WorldWithInstance,
} from '@/helpers/worlds'
import { handleSevereError } from '@/store/error'

const props = defineProps<{
	instances: GameInstance[]
	dashboard?: boolean
	dashboardSize?: HomeWidgetSize | null
}>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const handleMinecraftLaunchError = useMinecraftLaunchError()
const runtime = useHomeDashboardRuntime()
const { favoriteWorlds, runningInstanceIds } = runtime
const { startServer, stopServer } = useServers()

const messages = defineMessages({
	pinnedServers: {
		id: 'app.home.servers.pinned',
		defaultMessage: 'Pinned servers',
	},
	emptyServers: {
		id: 'app.home.servers.empty',
		defaultMessage: 'Favorite a server and it will be pinned here.',
	},
	playersOnline: {
		id: 'app.home.servers.players-online',
		defaultMessage: '{online}/{max} online',
	},
	offline: {
		id: 'app.home.servers.offline',
		defaultMessage: 'Offline',
	},
	join: {
		id: 'app.home.servers.join',
		defaultMessage: 'Join server',
	},
	stop: {
		id: 'app.home.servers.stop',
		defaultMessage: 'Stop',
	},
	unpin: {
		id: 'app.home.servers.unpin',
		defaultMessage: 'Unpin from Home',
	},
	moreOptions: {
		id: 'app.home.servers.more-options',
		defaultMessage: 'More options',
	},
	localServer: {
		id: 'app.home.servers.local',
		defaultMessage: 'Local',
	},
	start: { id: 'app.servers.action.start', defaultMessage: 'Start' },
	stop: { id: 'app.servers.action.stop', defaultMessage: 'Stop' },
})

const startingServerKey = ref<string | null>(null)

const instanceById = computed(
	() => new Map(props.instances.map((instance) => [instance.id, instance])),
)
const servers = computed(() =>
	favoriteWorlds.value.flatMap((world) => {
		if (world.type !== 'server') return []
		const instance = instanceById.value.get(world.instance_id)
		return instance ? [{ instance, world: world as ServerWorld & WorldWithInstance }] : []
	}),
)
const localServers = computed(() => runtime.pinnedLocalServers.value)
const hasServers = computed(() => servers.value.length > 0 || localServers.value.length > 0)

function serverKey(world: ServerWorld & WorldWithInstance): string {
	return `${world.instance_id}:${world.index}:${world.address}`
}

function dataFor(world: ServerWorld & WorldWithInstance) {
	return runtime.getServerData(world.instance_id, world.address)
}

async function joinServer(world: ServerWorld & WorldWithInstance, instance: GameInstance) {
	const key = serverKey(world)
	startingServerKey.value = key

	try {
		await start_join_server(world.instance_id, world.address)
		trackEvent('InstanceStart', {
			loader: instance.loader,
			game_version: instance.game_version,
			source: 'HomePinnedServer',
		})
	} catch (error) {
		const handled = await handleMinecraftLaunchError(error, {
			instance_id: instance.id,
			instance_name: instance.name,
		})
		if (!handled) handleSevereError(error, { instanceId: instance.id })
	} finally {
		startingServerKey.value = null
	}
}

async function stopInstance(instance: GameInstance) {
	await kill(instance.id).catch(handleError)
	trackEvent('InstanceStop', {
		loader: instance.loader,
		game_version: instance.game_version,
		source: 'HomePinnedServer',
	})
}

async function unpinServer(world: ServerWorld & WorldWithInstance) {
	await set_world_display_status(world.instance_id, 'server', world.address, 'normal').catch(
		handleError,
	)
	await runtime.refreshFavorites()
}

async function startLocalServer(serverId: string) {
	await startServer(serverId)
	await runtime.refreshPinnedLocalServers()
}

async function stopLocalServer(serverId: string) {
	await stopServer(serverId)
	await runtime.refreshPinnedLocalServers()
}

async function unpinLocalServer(serverId: string) {
	await serversApi.updateSettings(serverId, { homePinned: false }).catch(handleError)
	await runtime.refreshPinnedLocalServers()
}
</script>

<template>
	<section
		class="home-pinned-servers flex min-w-0 min-h-0 h-full flex-col gap-3"
		:data-size="dashboardSize"
	>
		<div class="home-widget-heading flex min-w-0 h-8 flex-none items-center gap-2">
			<ServerIcon class="size-5 shrink-0 text-brand" aria-hidden="true" />
			<h2>{{ formatMessage(messages.pinnedServers) }}</h2>
		</div>
		<div v-if="!hasServers" class="home-widget-empty">
			<ServerIcon aria-hidden="true" />
			<span>{{ formatMessage(messages.emptyServers) }}</span>
		</div>
		<ul
			v-else
			class="home-server-list grid min-w-0 min-h-0 flex-1 grid-auto-rows-max content-start gap-1 m-0 list-none overflow-x-hidden overflow-y-auto p-0 pr-1"
		>
			<li v-for="server in servers" :key="serverKey(server.world)">
				<SmartClickable>
					<template #clickable>
						<router-link
							:aria-label="server.world.name"
							:to="`/instance/${encodeURIComponent(server.instance.id)}/worlds?highlight=${encodeURIComponent(getWorldIdentifier(server.world))}`"
						/>
					</template>
					<div class="home-server-row group smart-clickable:highlight-on-hover">
						<div class="relative shrink-0">
							<Avatar
								:src="dataFor(server.world).status?.favicon ?? (server.world.icon || undefined)"
								:tint-by="server.world.address"
								size="36px"
							/>
							<span
								class="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full border-2 border-solid border-bg-raised"
								:class="
									dataFor(server.world).refreshing
										? 'animate-pulse bg-secondary'
										: dataFor(server.world).status
											? 'bg-brand-green'
											: 'bg-red'
								"
								aria-hidden="true"
							/>
						</div>
						<div class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="truncate text-sm font-semibold text-contrast">
								{{ server.world.name }}
							</span>
							<span
								v-if="dataFor(server.world).status"
								class="flex min-w-0 items-center gap-1 text-xs text-secondary"
							>
								<SignalIcon class="size-3 shrink-0" aria-hidden="true" />
								<span class="truncate">
									{{
										formatMessage(messages.playersOnline, {
											online: dataFor(server.world).status?.players?.online ?? 0,
											max: dataFor(server.world).status?.players?.max ?? 0,
										})
									}}
								</span>
							</span>
							<span
								v-else-if="dataFor(server.world).refreshing"
								class="truncate text-xs text-secondary"
							>
								{{ server.world.address }}
							</span>
							<span v-else class="flex min-w-0 items-center gap-1 text-xs text-secondary">
								<NoSignalIcon class="size-3 shrink-0" aria-hidden="true" />
								<span class="truncate">{{ formatMessage(messages.offline) }}</span>
							</span>
						</div>
						<div
							class="ml-auto flex shrink-0 items-center gap-0.5 smart-clickable:allow-pointer-events"
						>
							<Button
								v-if="runningInstanceIds.includes(server.instance.id)"
								v-tooltip="formatMessage(messages.stop)"
								type="quiet"
								size="2xs"
								circular
								icon-only
								class="!text-red"
								@click="stopInstance(server.instance)"
								><StopCircleIcon />
							</Button>
							<Button
								v-else
								v-tooltip="formatMessage(messages.join)"
								type="quiet"
								size="2xs"
								circular
								icon-only
								class="!text-brand opacity-60 transition-opacity group-hover:opacity-100"
								:disabled="startingServerKey === serverKey(server.world)"
								@click="joinServer(server.world, server.instance)"
								><SpinnerIcon
									v-if="startingServerKey === serverKey(server.world)"
									class="animate-spin"
								/>
								<PlayIcon v-else />
							</Button>
							<ButtonStyled circular size="small" type="transparent" class="home-server-menu">
								<OverflowMenu
									:options="[
										{
											id: 'unpin',
											action: () => unpinServer(server.world),
										},
									]"
									:tooltip="formatMessage(messages.moreOptions)"
								>
									<MoreVerticalIcon />
									<template #unpin>
										<PinIcon class="rotate-45" aria-hidden="true" />
										{{ formatMessage(messages.unpin) }}
									</template>
								</OverflowMenu>
							</ButtonStyled>
						</div>
					</div>
				</SmartClickable>
			</li>
			<li v-for="server in localServers" :key="'local-' + server.id">
				<SmartClickable>
					<template #clickable>
						<router-link
							:aria-label="server.name"
							:to="`/multiplayer/servers/${encodeURIComponent(server.id)}`"
						/>
					</template>
					<div class="home-server-row group smart-clickable:highlight-on-hover">
						<div class="relative shrink-0">
							<ManagedServerIcon
								:icon-path="server.iconPath"
								:server-type="server.serverType"
								:server-id="server.id"
								size="36px"
							/>
							<span
								class="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full border-2 border-solid border-bg-raised"
								:class="server.running ? 'bg-brand-green' : 'bg-red'"
								aria-hidden="true"
							/>
						</div>
						<div class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="truncate text-sm font-semibold text-contrast">{{ server.name }}</span>
							<span class="flex min-w-0 items-center gap-1 text-xs text-secondary">
								<span class="shrink-0 rounded bg-button-bg px-1 text-[10px] font-semibold">
									{{ formatMessage(messages.localServer) }}
								</span>
								<span class="truncate">
									{{
										server.port
											? `localhost:${server.port}`
											: `${server.serverType} ${server.gameVersion}`
									}}
								</span>
							</span>
						</div>
						<div
							class="ml-auto flex shrink-0 items-center gap-0.5 smart-clickable:allow-pointer-events"
						>
							<Button
								v-tooltip="formatMessage(server.running ? messages.stop : messages.start)"
								type="quiet"
								size="2xs"
								circular
								icon-only
								:class="server.running ? '!text-red' : '!text-brand'"
								@click="server.running ? stopLocalServer(server.id) : startLocalServer(server.id)"
								><StopCircleIcon v-if="server.running" />
								<PlayIcon v-else />
							</Button>
							<ButtonStyled circular size="small" type="transparent" class="home-server-menu">
								<OverflowMenu
									:options="[{ id: 'unpin', action: () => unpinLocalServer(server.id) }]"
									:tooltip="formatMessage(messages.moreOptions)"
								>
									<MoreVerticalIcon />
									<template #unpin>
										<PinIcon class="rotate-45" aria-hidden="true" />
										{{ formatMessage(messages.unpin) }}
									</template>
								</OverflowMenu>
							</ButtonStyled>
						</div>
					</div>
				</SmartClickable>
			</li>
		</ul>
	</section>
</template>

<style scoped>
.home-widget-heading h2 {
	min-width: 0;
	overflow: hidden;
	margin: 0;
	color: var(--color-contrast);
	font-size: 1rem;
	font-weight: 700;
	letter-spacing: 0;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-server-row {
	display: flex;
	min-width: 0;
	align-items: center;
	gap: 0.625rem;
	padding: 0.5rem;
	border-radius: 6px;
	transition: background-color 120ms ease;
}

.home-pinned-servers[data-size='2x1'] .home-server-list,
.home-pinned-servers[data-size='2x2'] .home-server-list {
	grid-template-columns: repeat(2, minmax(0, 1fr));
	column-gap: 0.5rem;
}

.home-pinned-servers[data-size='1x1'] {
	gap: 0.375rem;
}

.home-pinned-servers[data-size='1x1'] .home-widget-heading {
	height: 1.5rem;
}

.home-pinned-servers[data-size='1x1'] .home-server-row {
	gap: 0.5rem;
	padding: 0.375rem;
}

.home-pinned-servers[data-size='1x1'] .home-server-menu {
	display: none;
}

.home-widget-empty {
	display: flex;
	max-width: 20rem;
	margin: auto;
	flex-direction: column;
	align-items: center;
	gap: 0.5rem;
	color: var(--color-secondary);
	font-size: 0.8125rem;
	line-height: 1.4;
	text-align: center;
}

.home-widget-empty svg {
	width: 1.5rem;
	height: 1.5rem;
	opacity: 0.7;
}
</style>
