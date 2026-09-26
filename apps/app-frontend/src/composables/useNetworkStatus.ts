import { computed, readonly, ref } from 'vue'

export function createNetworkStatus(initialBrowserOffline = false) {
	const browserOffline = ref(initialBrowserOffline)
	const networkReachable = ref<boolean | undefined>()
	const offline = computed(() => browserOffline.value || networkReachable.value === false)

	return {
		offline: readonly(offline),
		browserOffline: readonly(browserOffline),
		setNetworkReachable(reachable: boolean) {
			networkReachable.value = reachable
		},
		setBrowserOffline(value: boolean) {
			browserOffline.value = value
		},
		markBrowserOnline() {
			browserOffline.value = false
			networkReachable.value = undefined
		},
		refreshBrowserOffline() {
			browserOffline.value = typeof navigator !== 'undefined' && !navigator.onLine
		},
	}
}

const sharedNetworkStatus = createNetworkStatus(
	typeof navigator !== 'undefined' && !navigator.onLine,
)

if (typeof window !== 'undefined') {
	window.addEventListener('offline', () => {
		sharedNetworkStatus.setBrowserOffline(true)
	})
	window.addEventListener('online', () => {
		sharedNetworkStatus.markBrowserOnline()
	})
}

export function useNetworkStatus() {
	return sharedNetworkStatus
}

export function isOfflineMode() {
	return sharedNetworkStatus.offline.value
}
