<script setup lang="ts">
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import type { SearchResult } from '@modrinth/utils'
import dayjs from 'dayjs'
import { computed, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import RowDisplay from '@/components/RowDisplay.vue'
import RecentWorldsList from '@/components/ui/world/RecentWorldsList.vue'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { get_search_results } from '@/helpers/cache.js'
import { instance_listener } from '@/helpers/events'
import { list } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { useBreadcrumbs } from '@/store/breadcrumbs'

const { handleError } = injectNotificationManager()
const route = useRoute()
const breadcrumbs = useBreadcrumbs()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	home: { id: 'app.home.breadcrumb', defaultMessage: 'Home' },
	welcomeBack: { id: 'app.home.welcome-back', defaultMessage: 'Welcome back!' },
	welcome: {
		id: 'app.home.welcome',
		defaultMessage: 'Welcome to Axolotl Launcher!',
	},
	discoverModpack: {
		id: 'app.home.discover-modpack',
		defaultMessage: 'Discover a modpack',
	},
	discoverMods: { id: 'app.home.discover-mods', defaultMessage: 'Discover mods' },
})

breadcrumbs.setRootContext({ name: formatMessage(messages.home), link: route.path })

const instances = ref<GameInstance[]>([])

const featuredModpacks = ref<SearchResult[]>([])
const featuredMods = ref<SearchResult[]>([])
const installedModpacksFilter = ref('')

const recentInstances = computed(() =>
	instances.value
		.filter((x) => x.last_played)
		.slice()
		.sort((a, b) => dayjs(b.last_played).diff(dayjs(a.last_played))),
)

const hasFeaturedProjects = computed(
	() => (featuredModpacks.value?.length ?? 0) + (featuredMods.value?.length ?? 0) > 0,
)

const { offline } = useNetworkStatus()

async function fetchInstances() {
	instances.value = await list().catch(handleError)

	const filters = []
	for (const instance of instances.value) {
		if (instance.link && instance.link.project_id) {
			filters.push(`NOT"project_id"="${instance.link.project_id}"`)
		}
	}
	installedModpacksFilter.value = filters.join(' AND ')
}

async function fetchFeaturedModpacks() {
	const response = await get_search_results(
		`?facets=[["project_type:modpack"]]&limit=10&index=follows&filters=${installedModpacksFilter.value}`,
	)

	if (response) {
		featuredModpacks.value = response.result.hits
	} else {
		featuredModpacks.value = []
	}
}

async function fetchFeaturedMods() {
	const response = await get_search_results('?facets=[["project_type:mod"]]&limit=10&index=follows')

	if (response) {
		featuredMods.value = response.result.hits
	} else {
		featuredModpacks.value = []
	}
}

async function refreshFeaturedProjects() {
	await Promise.all([fetchFeaturedModpacks(), fetchFeaturedMods()])
}

await fetchInstances()
if (!offline.value) await refreshFeaturedProjects()

watch(offline, (isOffline) => {
	if (isOffline) {
		featuredModpacks.value = []
		featuredMods.value = []
	} else {
		void refreshFeaturedProjects()
	}
})

const unlistenInstance = await instance_listener(
	async (e: { event: string; instance_id: string }) => {
		await fetchInstances()

		if (!offline.value && (e.event === 'added' || e.event === 'created' || e.event === 'removed')) {
			await refreshFeaturedProjects()
		}
	},
)

onUnmounted(() => {
	unlistenInstance()
})
</script>

<template>
	<div class="p-6 flex flex-col gap-2">
		<h1 v-if="recentInstances?.length > 0" class="m-0 text-2xl font-extrabold">
			{{ formatMessage(messages.welcomeBack) }}
		</h1>
		<h1 v-else class="m-0 text-2xl font-extrabold">
			{{ formatMessage(messages.welcome) }}
		</h1>
		<RecentWorldsList :recent-instances="recentInstances" />
		<RowDisplay
			v-if="hasFeaturedProjects"
			:instances="[
				{
					label: formatMessage(messages.discoverModpack),
					route: '/browse/modpack',
					instances: featuredModpacks,
					downloaded: false,
				},
				{
					label: formatMessage(messages.discoverMods),
					route: '/browse/mod',
					instances: featuredMods,
					downloaded: false,
				},
			]"
			:can-paginate="true"
		/>
	</div>
</template>
