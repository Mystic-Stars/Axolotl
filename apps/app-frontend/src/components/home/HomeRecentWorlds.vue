<script setup lang="ts">
import type { GameVersion } from '@modrinth/ui'
import { defineMessages, GAME_MODES, injectNotificationManager, useVIntl } from '@modrinth/ui'
import type { Dayjs } from 'dayjs'
import dayjs from 'dayjs'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import InstanceItem from '@/components/ui/world/InstanceItem.vue'
import WorldItem from '@/components/ui/world/WorldItem.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { instance_listener, process_listener } from '@/helpers/events'
import { kill, run } from '@/helpers/instance'
import { get_all } from '@/helpers/process'
import { get_game_versions } from '@/helpers/tags'
import type { GameInstance } from '@/helpers/types'
import {
	get_instance_protocol_version,
	get_recent_worlds,
	getWorldIdentifier,
	hasServerQuickPlaySupport,
	hasWorldQuickPlaySupport,
	type ProtocolVersion,
	refreshServerData,
	type ServerData,
	type ServerWorld,
	start_join_server,
	start_join_singleplayer_world,
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
	recentTitle: {
		id: 'app.home.recent.title',
		defaultMessage: 'Start from your recent projects',
	},
})

const MAX_RECENT_ITEMS = 4

type RecentItem =
	| { type: 'world'; last_played: Dayjs; instance: GameInstance; world: WorldWithInstance }
	| { type: 'instance'; last_played: Dayjs; instance: GameInstance }

const recentItems = ref<RecentItem[]>([])
const serverData = ref<Record<string, ServerData>>({})
const protocolVersions = ref<Record<string, ProtocolVersion | null>>({})
const runningInstanceIds = ref<string[]>([])
const startingWorldKey = ref<string | null>(null)
const playingWorldKey = ref<string | null>(null)
const gameVersions = ref<GameVersion[]>(await get_game_versions().catch(() => []))

const instanceById = computed(
	() => new Map(props.instances.map((instance) => [instance.id, instance])),
)

function worldKey(world: WorldWithInstance): string {
	return `${world.instance_id}:${world.type}:${getWorldIdentifier(world)}`
}

async function populateRecentItems() {
	const worlds = await get_recent_worlds(MAX_RECENT_ITEMS, ['normal', 'favorite']).catch(
		(error): WorldWithInstance[] => {
			handleError(error)
			return []
		},
	)

	const worldItems: RecentItem[] = worlds.flatMap((world) => {
		const instance = instanceById.value.get(world.instance_id)
		if (!instance || !world.last_played) return []
		return [{ type: 'world' as const, last_played: dayjs(world.last_played), instance, world }]
	})

	const coveredInstanceIds = new Set(worldItems.map((item) => item.instance.id))
	const instanceItems: RecentItem[] = props.instances
		.filter((instance) => instance.last_played && !coveredInstanceIds.has(instance.id))
		.map((instance) => ({
			type: 'instance' as const,
			last_played: dayjs(instance.last_played),
			instance,
		}))

	recentItems.value = [...worldItems, ...instanceItems]
		.sort((a, b) => b.last_played.diff(a.last_played))
		.slice(0, MAX_RECENT_ITEMS)

	const servers = recentItems.value.flatMap((item) =>
		item.type === 'world' && item.world.type === 'server'
			? [{ instanceId: item.instance.id, address: (item.world as ServerWorld).address }]
			: [],
	)
	await Promise.all(
		[...new Set(servers.map((server) => server.instanceId))].map(async (instanceId) => {
			protocolVersions.value[instanceId] = await get_instance_protocol_version(instanceId).catch(
				() => null,
			)
		}),
	)
	for (const { instanceId, address } of servers) {
		void refreshServer(address, instanceId)
	}
}

async function refreshServer(address: string, instanceId: string) {
	serverData.value[address] ??= { refreshing: true }
	await refreshServerData(
		serverData.value[address],
		protocolVersions.value[instanceId] ?? null,
		address,
	)
}

async function checkProcesses() {
	const processes = await get_all().catch(() => [])
	runningInstanceIds.value = processes.map((process) => process.instance_id)
	if (
		playingWorldKey.value &&
		!runningInstanceIds.value.includes(playingWorldKey.value.split(':', 1)[0])
	) {
		playingWorldKey.value = null
	}
}

async function joinWorld(world: WorldWithInstance, instance: GameInstance) {
	const key = worldKey(world)
	startingWorldKey.value = key

	try {
		if (world.type === 'server') {
			await start_join_server(world.instance_id, world.address)
		} else {
			await start_join_singleplayer_world(world.instance_id, world.path)
		}
		playingWorldKey.value = key
		trackEvent('InstanceStart', {
			loader: instance.loader,
			game_version: instance.game_version,
			source: 'HomeRecentWorld',
		})
	} catch (error) {
		const handled = await handleMinecraftLaunchError(error, {
			instance_id: instance.id,
			instance_name: instance.name,
		})
		if (!handled) handleSevereError(error, { instanceId: instance.id })
	} finally {
		startingWorldKey.value = null
	}
}

async function playInstance(instance: GameInstance) {
	try {
		await run(instance.id)
		trackEvent('InstanceStart', {
			loader: instance.loader,
			game_version: instance.game_version,
			source: 'HomeRecentWorld',
		})
	} catch (error) {
		const handled = await handleMinecraftLaunchError(error, {
			instance_id: instance.id,
			instance_name: instance.name,
		})
		if (!handled) handleSevereError(error, { instanceId: instance.id })
	}
}

async function stopInstance(instance: GameInstance) {
	await kill(instance.id).catch(handleError)
	playingWorldKey.value = null
	trackEvent('InstanceStop', {
		loader: instance.loader,
		game_version: instance.game_version,
		source: 'HomeRecentWorld',
	})
}

watch(() => props.instances, populateRecentItems)

await populateRecentItems()

const unlistenProcesses = await process_listener(checkProcesses)
const unlistenInstances = await instance_listener(populateRecentItems)

onMounted(() => {
	void checkProcesses()
})

onUnmounted(() => {
	unlistenProcesses()
	unlistenInstances()
})
</script>

<template>
	<section v-if="recentItems.length > 0" class="flex flex-col gap-3">
		<h2 class="m-0 text-lg font-bold text-contrast">
			{{ formatMessage(messages.recentTitle) }}
		</h2>
		<div class="flex flex-col gap-2">
			<template
				v-for="item in recentItems"
				:key="item.type === 'world' ? worldKey(item.world) : `${item.instance.id}:instance`"
			>
				<WorldItem
					v-if="item.type === 'world'"
					:world="item.world"
					:playing-instance="runningInstanceIds.includes(item.instance.id)"
					:playing-world="playingWorldKey === worldKey(item.world)"
					:starting-instance="startingWorldKey === worldKey(item.world)"
					:supports-server-quick-play="
						item.world.type === 'server' &&
						hasServerQuickPlaySupport(gameVersions, item.instance.game_version)
					"
					:supports-world-quick-play="
						item.world.type === 'singleplayer' &&
						hasWorldQuickPlaySupport(gameVersions, item.instance.game_version)
					"
					:current-protocol="protocolVersions[item.instance.id]"
					:refreshing="
						item.world.type === 'server'
							? serverData[(item.world as ServerWorld).address]?.refreshing
							: undefined
					"
					:server-status="
						item.world.type === 'server'
							? serverData[(item.world as ServerWorld).address]?.status
							: undefined
					"
					:rendered-motd="
						item.world.type === 'server'
							? serverData[(item.world as ServerWorld).address]?.renderedMotd
							: undefined
					"
					:game-mode="
						item.world.type === 'singleplayer' ? GAME_MODES[item.world.game_mode] : undefined
					"
					:instance-id="item.instance.id"
					:instance-name="item.instance.name"
					:instance-icon="item.instance.icon_path"
					:shortcut-instance-id="item.instance.id"
					@play="joinWorld(item.world, item.instance)"
					@play-instance="playInstance(item.instance)"
					@stop="stopInstance(item.instance)"
					@refresh="
						item.world.type === 'server'
							? refreshServer((item.world as ServerWorld).address, item.instance.id)
							: undefined
					"
					@update="populateRecentItems"
				/>
				<InstanceItem v-else :instance="item.instance" :last_played="item.last_played" />
			</template>
		</div>
	</section>
</template>
