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
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	OverflowMenu,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { instance_listener, process_listener } from '@/helpers/events'
import { kill } from '@/helpers/instance'
import { get_all } from '@/helpers/process'
import type { GameInstance } from '@/helpers/types'
import {
	get_favorite_worlds,
	get_instance_protocol_version,
	type ProtocolVersion,
	refreshServerData,
	type ServerData,
	type ServerWorld,
	set_world_display_status,
	start_join_server,
	type WorldWithInstance,
} from '@/helpers/worlds'
import { handleSevereError } from '@/store/error'

const props = defineProps<{
	instances: GameInstance[]
}>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const handleMinecraftLaunchError = useMinecraftLaunchError()

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
})

const favoriteWorlds = ref<WorldWithInstance[]>([])
const serverData = ref<Record<string, ServerData>>({})
const protocolVersions = ref<Record<string, ProtocolVersion | null>>({})
const runningInstanceIds = ref<string[]>([])
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

function serverKey(world: ServerWorld & WorldWithInstance): string {
	return `${world.instance_id}:${world.address}`
}

async function refreshServer(address: string, instanceId: string) {
	serverData.value[address] ??= { refreshing: true }
	await refreshServerData(
		serverData.value[address],
		protocolVersions.value[instanceId] ?? null,
		address,
	)
}

async function refreshFavorites() {
	favoriteWorlds.value = await get_favorite_worlds().catch((error): WorldWithInstance[] => {
		handleError(error)
		return []
	})

	const serverInstanceIds = new Set(
		favoriteWorlds.value
			.filter((world) => world.type === 'server')
			.map((world) => world.instance_id),
	)
	await Promise.all(
		[...serverInstanceIds].map(async (instanceId) => {
			protocolVersions.value[instanceId] = await get_instance_protocol_version(instanceId).catch(
				() => null,
			)
		}),
	)

	for (const world of favoriteWorlds.value) {
		if (world.type === 'server') {
			void refreshServer(world.address, world.instance_id)
		}
	}
}

async function checkProcesses() {
	const processes = await get_all().catch(() => [])
	runningInstanceIds.value = processes.map((process) => process.instance_id)
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
	await refreshFavorites()
}

const unlistenProcesses = await process_listener(checkProcesses)
const unlistenInstances = await instance_listener(refreshFavorites)

await refreshFavorites()

onMounted(() => {
	void checkProcesses()
})

onUnmounted(() => {
	unlistenProcesses()
	unlistenInstances()
})
</script>

<template>
	<section class="card-shadow flex min-w-0 flex-col gap-3 rounded-2xl bg-bg-raised p-4">
		<div class="flex items-center gap-2">
			<ServerIcon class="size-5 shrink-0 text-brand" aria-hidden="true" />
			<h2 class="m-0 truncate text-lg font-bold text-contrast">
				{{ formatMessage(messages.pinnedServers) }}
			</h2>
		</div>
		<p v-if="servers.length === 0" class="m-0 text-sm text-secondary">
			{{ formatMessage(messages.emptyServers) }}
		</p>
		<ul v-else class="m-0 flex list-none flex-col p-0">
			<li
				v-for="server in servers"
				:key="serverKey(server.world)"
				class="group flex min-w-0 items-center gap-2.5 rounded-lg px-1.5 py-1.5 transition-colors hover:bg-button-bg"
			>
				<div class="relative shrink-0">
					<Avatar
						:src="
							serverData[server.world.address]?.status?.favicon ??
							(server.world.icon || undefined)
						"
						:tint-by="server.world.address"
						size="36px"
					/>
					<span
						class="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full border-2 border-solid border-bg-raised"
						:class="
							serverData[server.world.address]?.refreshing
								? 'animate-pulse bg-secondary'
								: serverData[server.world.address]?.status
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
						v-if="serverData[server.world.address]?.status"
						class="flex min-w-0 items-center gap-1 text-xs text-secondary"
					>
						<SignalIcon class="size-3 shrink-0" aria-hidden="true" />
						<span class="truncate">
							{{
								formatMessage(messages.playersOnline, {
									online: serverData[server.world.address]?.status?.players?.online ?? 0,
									max: serverData[server.world.address]?.status?.players?.max ?? 0,
								})
							}}
						</span>
					</span>
					<span
						v-else-if="serverData[server.world.address]?.refreshing"
						class="truncate text-xs text-secondary"
					>
						{{ server.world.address }}
					</span>
					<span v-else class="flex min-w-0 items-center gap-1 text-xs text-secondary">
						<NoSignalIcon class="size-3 shrink-0" aria-hidden="true" />
						<span class="truncate">{{ formatMessage(messages.offline) }}</span>
					</span>
				</div>
				<div class="ml-auto flex shrink-0 items-center gap-0.5">
					<ButtonStyled
						v-if="runningInstanceIds.includes(server.instance.id)"
						circular
						size="small"
						type="transparent"
					>
						<button
							v-tooltip="formatMessage(messages.stop)"
							class="!text-red"
							@click="stopInstance(server.instance)"
						>
							<StopCircleIcon />
						</button>
					</ButtonStyled>
					<ButtonStyled v-else circular size="small" type="transparent">
						<button
							v-tooltip="formatMessage(messages.join)"
							class="!text-brand opacity-60 transition-opacity group-hover:opacity-100"
							:disabled="startingServerKey === serverKey(server.world)"
							@click="joinServer(server.world, server.instance)"
						>
							<SpinnerIcon
								v-if="startingServerKey === serverKey(server.world)"
								class="animate-spin"
							/>
							<PlayIcon v-else />
						</button>
					</ButtonStyled>
					<ButtonStyled circular size="small" type="transparent">
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
			</li>
		</ul>
	</section>
</template>
