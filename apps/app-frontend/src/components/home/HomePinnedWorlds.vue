<script setup lang="ts">
import {
	defineMessages,
	GAME_MODES,
	type GameVersion,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import WorldItem from '@/components/ui/world/WorldItem.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { instance_listener, process_listener } from '@/helpers/events'
import { kill, run } from '@/helpers/instance'
import { get_all } from '@/helpers/process'
import { get_game_versions } from '@/helpers/tags'
import type { GameInstance } from '@/helpers/types'
import {
	get_favorite_worlds,
	getWorldIdentifier,
	hasWorldQuickPlaySupport,
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
	pinnedWorlds: {
		id: 'app.home.worlds.pinned',
		defaultMessage: 'Pinned worlds',
	},
	emptyWorlds: {
		id: 'app.home.worlds.empty',
		defaultMessage: 'Favorite a world and it will be pinned here.',
	},
})

const favoriteWorlds = ref<WorldWithInstance[]>([])
const runningInstanceIds = ref<string[]>([])
const startingWorldKey = ref<string | null>(null)
const playingWorldKey = ref<string | null>(null)
const gameVersions = ref<GameVersion[]>(await get_game_versions().catch(() => []))

const instanceById = computed(
	() => new Map(props.instances.map((instance) => [instance.id, instance])),
)
const favorites = computed(() =>
	favoriteWorlds.value.flatMap((world) => {
		if (world.type !== 'singleplayer') return []
		const instance = instanceById.value.get(world.instance_id)
		return instance ? [{ instance, world }] : []
	}),
)

function favoriteKey(world: WorldWithInstance): string {
	return `${world.instance_id}:${world.type}:${getWorldIdentifier(world)}`
}

async function refreshFavorites() {
	favoriteWorlds.value = await get_favorite_worlds().catch((error): WorldWithInstance[] => {
		handleError(error)
		return []
	})
}

async function checkProcesses() {
	const processes = await get_all().catch((error) => {
		handleError(error)
		return []
	})
	runningInstanceIds.value = processes.map((process) => process.instance_id)
	if (
		playingWorldKey.value &&
		!runningInstanceIds.value.includes(playingWorldKey.value.split(':', 1)[0])
	) {
		playingWorldKey.value = null
	}
}

async function joinWorld(world: WorldWithInstance, instance: GameInstance) {
	if (world.type !== 'singleplayer') return
	const key = favoriteKey(world)
	startingWorldKey.value = key

	try {
		await start_join_singleplayer_world(world.instance_id, world.path)
		playingWorldKey.value = key
		trackEvent('InstanceStart', {
			loader: instance.loader,
			game_version: instance.game_version,
			source: 'HomePinnedWorld',
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
			source: 'HomePinnedWorld',
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
		source: 'HomePinnedWorld',
	})
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
	<section class="flex flex-col gap-3">
		<h2 class="m-0 text-lg font-bold text-contrast">
			{{ formatMessage(messages.pinnedWorlds) }}
		</h2>
		<div v-if="favorites.length > 0" class="flex flex-col gap-2">
			<WorldItem
				v-for="favorite in favorites"
				:key="favoriteKey(favorite.world)"
				:world="favorite.world"
				:playing-instance="runningInstanceIds.includes(favorite.instance.id)"
				:playing-world="playingWorldKey === favoriteKey(favorite.world)"
				:starting-instance="startingWorldKey === favoriteKey(favorite.world)"
				:supports-world-quick-play="
					hasWorldQuickPlaySupport(gameVersions, favorite.instance.game_version)
				"
				:game-mode="
					favorite.world.type === 'singleplayer' ? GAME_MODES[favorite.world.game_mode] : undefined
				"
				:instance-id="favorite.instance.id"
				:instance-name="favorite.instance.name"
				:instance-icon="favorite.instance.icon_path"
				:shortcut-instance-id="favorite.instance.id"
				@play="joinWorld(favorite.world, favorite.instance)"
				@play-instance="playInstance(favorite.instance)"
				@stop="stopInstance(favorite.instance)"
				@update="refreshFavorites"
			/>
		</div>
		<p v-else class="m-0 text-sm text-secondary">
			{{ formatMessage(messages.emptyWorlds) }}
		</p>
	</section>
</template>
