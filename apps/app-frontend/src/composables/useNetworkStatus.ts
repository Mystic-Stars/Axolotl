import { computed, readonly, ref } from 'vue'

const browserOffline = ref(typeof navigator !== 'undefined' && !navigator.onLine)
const networkReachable = ref<boolean | undefined>()
const offline = computed(() => browserOffline.value || networkReachable.value === false)

if (typeof window !== 'undefined') {
	window.addEventListener('offline', () => {
		browserOffline.value = true
	})
	window.addEventListener('online', () => {
		browserOffline.value = false
		networkReachable.value = undefined
	})
}

export function useNetworkStatus() {
	return {
		offline: readonly(offline),
		browserOffline: readonly(browserOffline),
		setNetworkReachable(reachable: boolean) {
			networkReachable.value = reachable
		},
	}
}

export function isOfflineMode() {
	return offline.value
}
