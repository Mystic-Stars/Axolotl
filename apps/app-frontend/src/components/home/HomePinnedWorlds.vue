<script setup lang="ts">
import { GameIcon } from '@modrinth/assets'
import { defineMessages, GAME_MODES, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { getHomeWidgetCardDensity, type HomeWidgetSize } from '@/components/home/home-dashboard'
import { useHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import WorldItem from '@/components/ui/world/WorldItem.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { kill, run } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import {
	getWorldIdentifier,
	hasWorldQuickPlaySupport,
	start_join_singleplayer_world,
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
const { favoriteWorlds, gameVersions, runningInstanceIds } = runtime
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

const startingWorldKey = ref<string | null>(null)
const playingWorldKey = ref<string | null>(null)

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
const worldDensity = computed(() => getHomeWidgetCardDensity(props.dashboardSize))

function favoriteKey(world: WorldWithInstance): string {
	return `${world.instance_id}:${world.type}:${getWorldIdentifier(world)}`
}

watch(runningInstanceIds, (instanceIds) => {
	if (playingWorldKey.value && !instanceIds.includes(playingWorldKey.value.split(':', 1)[0])) {
		playingWorldKey.value = null
	}
})

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
</script>

<template>
	<section class="home-pinned-worlds flex min-w-0 min-h-0 h-full flex-col gap-3" :data-size="dashboardSize">
		<div class="home-widget-heading flex min-w-0 h-8 flex-none items-center gap-2">
			<h2>{{ formatMessage(messages.pinnedWorlds) }}</h2>
		</div>
		<div v-if="favorites.length > 0" class="home-world-list flex min-w-0 min-h-0 flex-1 flex-col gap-1 overflow-x-hidden overflow-y-auto pr-1">
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
				:instance-loader="favorite.instance.loader"
				:shortcut-instance-id="favorite.instance.id"
				:flat="dashboard"
				:dashboard-density="worldDensity"
				@play="joinWorld(favorite.world, favorite.instance)"
				@play-instance="playInstance(favorite.instance)"
				@stop="stopInstance(favorite.instance)"
				@update="runtime.refreshFavorites"
			/>
		</div>
		<div v-else class="home-widget-empty">
			<GameIcon aria-hidden="true" />
			<span>{{ formatMessage(messages.emptyWorlds) }}</span>
		</div>
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

.home-pinned-worlds[data-size='1x1'] {
	gap: 0.375rem;
}

.home-pinned-worlds[data-size='1x1'] .home-widget-heading {
	height: 1.5rem;
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
