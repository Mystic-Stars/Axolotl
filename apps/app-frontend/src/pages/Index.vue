<script setup lang="ts">
import { LayoutTemplateIcon, MinimizeIcon } from '@modrinth/assets'
import {
	defineMessages,
	injectNotificationManager,
	injectPageContext,
	useVIntl,
} from '@modrinth/ui'
import { computed, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { getActivePlayerName } from '@/components/home/home-utils'
import HomeCalendar from '@/components/home/HomeCalendar.vue'
import HomeDailyChallenge from '@/components/home/HomeDailyChallenge.vue'
import HomeGreeting from '@/components/home/HomeGreeting.vue'
import HomeInstancePickerModal from '@/components/home/HomeInstancePickerModal.vue'
import HomeMinecraftNews from '@/components/home/HomeMinecraftNews.vue'
import HomeMinimal from '@/components/home/HomeMinimal.vue'
import HomePinnedInstances from '@/components/home/HomePinnedInstances.vue'
import HomePinnedServers from '@/components/home/HomePinnedServers.vue'
import HomePinnedWorlds from '@/components/home/HomePinnedWorlds.vue'
import HomePlayInsights from '@/components/home/HomePlayInsights.vue'
import HomeRecentWorlds from '@/components/home/HomeRecentWorlds.vue'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { get_default_user, users } from '@/helpers/auth'
import { instance_listener } from '@/helpers/events'
import { list } from '@/helpers/instance'
import { get as getSettings, set as setSettings } from '@/helpers/settings'
import type { GameInstance } from '@/helpers/types'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { useTheming } from '@/store/state'
import type { FeatureFlag, HomeLayout } from '@/store/theme'

const { handleError } = injectNotificationManager()
const route = useRoute()
const router = useRouter()
const breadcrumbs = useBreadcrumbs()
const { formatMessage } = useVIntl()
const { offline } = useNetworkStatus()
const themeStore = useTheming()
const pageContext = injectPageContext()

const messages = defineMessages({
	home: { id: 'app.home.breadcrumb', defaultMessage: 'Home' },
	switchToMinimal: {
		id: 'app.home.layout.switch-to-minimal',
		defaultMessage: 'Switch to Minimal Home',
	},
	switchToInformation: {
		id: 'app.home.layout.switch-to-information',
		defaultMessage: 'Switch to Information Home',
	},
	homeLayoutToggle: {
		id: 'app.home.layout.toggle',
		defaultMessage: 'Minimal Home',
	},
})

const recentProjectsInHomeFlag: FeatureFlag = 'worlds_in_home'

breadcrumbs.setRootContext({ name: formatMessage(messages.home), link: route.path })

const instances = ref<GameInstance[]>([])
const playerName = ref<string | null>(null)
const instancePicker = ref<InstanceType<typeof HomeInstancePickerModal>>()
const isMinimal = computed(() => themeStore.homeLayout === 'minimal')
const showRecentProjects = computed(() => themeStore.getFeatureFlag(recentProjectsInHomeFlag))
const switchingLayout = ref(false)
const layoutSwitchStyle = computed(() => ({
	bottom: themeStore.getFeatureFlag('page_path') ? '3.5rem' : '1rem',
	right: `calc(${pageContext.floatingActionBarOffsets?.right.value ?? '0px'} + 1rem)`,
}))

async function clearMissingMinimalInstance() {
	const selectedId = themeStore.minimalHomeInstanceId
	if (!selectedId || instances.value.some((instance) => instance.id === selectedId)) return

	themeStore.minimalHomeInstanceId = null
	try {
		const settings = await getSettings()
		if (settings.minimal_home_instance_id === null) return
		settings.minimal_home_instance_id = null
		await setSettings(settings)
	} catch (error) {
		handleError(error)
	}
}

async function fetchInstances() {
	try {
		instances.value = await list()
		await clearMissingMinimalInstance()
		return true
	} catch (error) {
		handleError(error)
		return false
	}
}

async function fetchPlayerName() {
	const selectedUser = await get_default_user(offline.value).catch(() => undefined)
	if (!selectedUser) return

	const accounts = await users(offline.value).catch(() => [])
	playerName.value = getActivePlayerName(selectedUser, accounts)
}

async function selectMinimalInstance(instance: GameInstance) {
	try {
		const settings = await getSettings()
		settings.minimal_home_instance_id = instance.id
		await setSettings(settings)
		themeStore.minimalHomeInstanceId = instance.id
	} catch (error) {
		handleError(error)
	}
}

function createInstance() {
	void router.push('/create')
}

async function redirectToCreateIfEmpty() {
	if (instances.value.length > 0 || route.path !== '/') return false
	await router.replace('/create')
	return true
}

async function toggleHomeLayout() {
	if (switchingLayout.value) return

	const previousLayout = themeStore.homeLayout
	const nextLayout: HomeLayout = previousLayout === 'minimal' ? 'standard' : 'minimal'
	switchingLayout.value = true
	themeStore.homeLayout = nextLayout

	try {
		const settings = await getSettings()
		settings.home_layout = nextLayout
		await setSettings(settings)
	} catch (error) {
		themeStore.homeLayout = previousLayout
		handleError(error)
	} finally {
		switchingLayout.value = false
	}
}

const instancesLoaded = await fetchInstances()
if (!instancesLoaded || instances.value.length > 0) await fetchPlayerName()

const unlistenInstance = await instance_listener(async () => {
	if (await fetchInstances()) await redirectToCreateIfEmpty()
})

onUnmounted(() => {
	unlistenInstance()
})

if (instancesLoaded) await redirectToCreateIfEmpty()
</script>

<template>
	<HomeInstancePickerModal
		ref="instancePicker"
		:instances="instances"
		:selected-instance-id="themeStore.minimalHomeInstanceId"
		@select="selectMinimalInstance"
	/>
	<div class="min-h-full">
		<div v-if="!isMinimal" class="home-layout p-6 pb-20">
			<div class="flex min-w-0 flex-col gap-6">
				<HomeGreeting :player-name="playerName" />
				<section data-onboarding-id="home-instances" class="flex flex-col gap-6">
					<HomeRecentWorlds v-if="showRecentProjects" :instances="instances" />
					<HomePinnedInstances :instances="instances" />
					<HomePinnedWorlds :instances="instances" />
				</section>
			</div>
			<aside class="flex min-w-0 flex-col gap-4">
				<HomeCalendar :instances="instances" />
				<HomePinnedServers :instances="instances" />
			</aside>
		</div>

		<HomeMinimal
			v-else
			:instances="instances"
			:player-name="playerName"
			:selected-instance-id="themeStore.minimalHomeInstanceId"
			@choose="instancePicker?.show()"
			@create="createInstance"
		/>
	</div>
	<button
		v-tooltip="formatMessage(isMinimal ? messages.switchToInformation : messages.switchToMinimal)"
		data-onboarding-id="home-layout-switch"
		type="button"
		role="switch"
		class="home-layout-switch"
		:class="{ 'is-minimal': isMinimal }"
		:style="layoutSwitchStyle"
		:disabled="switchingLayout"
		:aria-checked="isMinimal"
		:aria-label="formatMessage(messages.homeLayoutToggle)"
		@click="toggleHomeLayout"
	>
		<span class="home-layout-switch-option home-layout-switch-information" aria-hidden="true">
			<LayoutTemplateIcon />
		</span>
		<span class="home-layout-switch-thumb" aria-hidden="true" />
		<span class="home-layout-switch-option home-layout-switch-minimal" aria-hidden="true">
			<MinimizeIcon />
		</span>
	</button>
	<Teleport v-if="!isMinimal" to="#sidebar-default-teleport-target">
		<div class="flex min-w-0 flex-col">
			<HomePlayInsights />
			<HomeDailyChallenge />
			<HomeMinecraftNews />
		</div>
	</Teleport>
</template>

<style scoped>
.home-layout {
	display: grid;
	grid-template-columns: minmax(0, 1fr);
	gap: 1.5rem;
	align-items: start;
}

.home-layout-switch {
	position: fixed;
	z-index: 40;
	display: grid;
	grid-template-columns: repeat(2, 2rem);
	align-items: center;
	width: 4.5rem;
	height: 2.5rem;
	margin: 0;
	padding: 0.25rem;
	border: 0;
	border-radius: 9999px;
	background: var(--color-button-bg);
	box-shadow:
		inset 0 0 0 1px var(--color-divider),
		var(--shadow-button),
		0 0.25rem 0.75rem rgb(0 0 0 / 20%);
	cursor: pointer;
	isolation: isolate;
	transition:
		filter 150ms ease,
		transform 150ms ease;
}

.home-layout-switch:hover:not(:disabled) {
	filter: brightness(var(--hover-brightness));
}

.home-layout-switch:active:not(:disabled) {
	transform: scale(0.96);
}

.home-layout-switch:focus-visible {
	outline: none;
	box-shadow:
		0 0 0 4px var(--color-brand-shadow),
		inset 0 0 0 1px var(--color-divider),
		var(--shadow-button),
		0 0.25rem 0.75rem rgb(0 0 0 / 20%);
}

.home-layout-switch:disabled {
	cursor: not-allowed;
	opacity: 0.6;
}

.home-layout-switch-thumb {
	position: absolute;
	top: 0.25rem;
	left: 0.25rem;
	z-index: 0;
	width: 2rem;
	height: 2rem;
	border-radius: 9999px;
	background: var(--color-brand);
	transition: transform 180ms ease;
}

.home-layout-switch.is-minimal .home-layout-switch-thumb {
	transform: translateX(2rem);
}

.home-layout-switch-option {
	position: relative;
	z-index: 1;
	display: flex;
	width: 2rem;
	height: 2rem;
	align-items: center;
	justify-content: center;
	color: var(--color-secondary);
	transition: color 180ms ease;
}

.home-layout-switch-option :deep(svg) {
	width: 1rem;
	height: 1rem;
}

.home-layout-switch:not(.is-minimal) .home-layout-switch-information,
.home-layout-switch.is-minimal .home-layout-switch-minimal {
	color: var(--color-accent-contrast);
}

@media (min-width: 64rem) {
	.home-layout {
		grid-template-columns: minmax(0, 1fr) 20rem;
	}
}
</style>
