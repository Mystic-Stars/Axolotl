<template>
	<Transition name="splash-fade" @after-leave="onAfterLeave">
		<div v-if="!doneLoading" class="fixed inset-0 z-[10000] dark">
			<div class="absolute h-screen w-full flex flex-col justify-center items-center gap-4 z-[9998]" data-tauri-drag-region>
				<img class="app-logo" src="@/assets/axolotl.png" alt="Axolotl Launcher" />
				<ProgressBar class="max-w-xs" :progress="Math.min(loadingProgress, 100)" />
				<span v-if="message">{{ message }}</span>
			</div>
			<div class="gradient-bg" data-tauri-drag-region></div>
			<div class="cube-bg"></div>
			<div class="absolute top-0 left-0 w-full h-full bg-bg z-[9995]"></div>
		</div>
	</Transition>
</template>

<script setup>
import { defineMessages, injectLoadingState, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import ProgressBar from '@/components/ui/ProgressBar.vue'
import { loading_listener } from '@/helpers/events.js'

const doneLoading = ref(false)
const loadingProgress = ref(0)
const message = ref()

const MIN_DISPLAY_MS = 500
const mountedAt = Date.now()

const loading = injectLoadingState()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	updatingAppDirectory: {
		id: 'app.splash.updating-app-directory',
		defaultMessage: 'Updating app directory...',
	},
	checkingForUpdates: {
		id: 'app.splash.checking-for-updates',
		defaultMessage: 'Checking for updates...',
	},
})

function onAfterLeave() {
	loading.setEnabled(true)
}

watch(
	[loading.barEnabled, loading.pending],
	([barEnabled, pending]) => {
		if (barEnabled) {
			return
		}

		if (pending) {
			loadingProgress.value = 0
			fakeLoadingIncrease()
			return
		}

		const elapsed = Date.now() - mountedAt
		const delay = Math.max(0, MIN_DISPLAY_MS - elapsed)

		setTimeout(() => {
			if (loading.pending.value) {
				return
			}
			doneLoading.value = true
		}, delay)
	},
	{ immediate: true },
)

function fakeLoadingIncrease() {
	if (loadingProgress.value < 95) {
		setTimeout(() => {
			loadingProgress.value += 2
			fakeLoadingIncrease()
		}, 5)
	}
}

loading_listener(async (e) => {
	if (e.event.type === 'directory_move') {
		loadingProgress.value = 100 * (e.fraction ?? 1)
		message.value = formatMessage(messages.updatingAppDirectory)
	} else if (e.event.type === 'checking_for_updates') {
		loadingProgress.value = 100 * (e.fraction ?? 1)
		message.value = formatMessage(messages.checkingForUpdates)
	}
})
</script>

<style scoped lang="scss">
.splash-fade-leave-active {
	transition: opacity 0.3s ease-in-out;
}

.splash-fade-leave-to {
	opacity: 0;
}

.app-logo {
	height: min(18rem, 45vh);
	width: min(18rem, 45vw);
	object-fit: contain;
	filter: drop-shadow(0 0 2rem rgba(255, 77, 157, 0.35));
}

.gradient-bg {
	position: absolute;
	height: 100vh;
	width: 100vw;
	background:
		linear-gradient(180deg, rgba(255, 77, 157, 0.24) 0%, rgba(48, 16, 40, 0.56) 97.29%),
		linear-gradient(0deg, rgba(22, 18, 28, 0.68), rgba(22, 18, 28, 0.68));
	z-index: 9997;
}

.cube-bg {
	position: absolute;

	left: 50%;
	top: 50%;
	transform: translate(-50%, -50%);

	width: 180vw;
	height: 180vh;
	opacity: 0.8;
	background: #16181c url('@/assets/loading/cube.png') center no-repeat;
	background-size: contain;

	z-index: 9996;
}

</style>
