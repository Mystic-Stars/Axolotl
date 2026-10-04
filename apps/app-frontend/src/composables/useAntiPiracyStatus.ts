import { listen } from '@tauri-apps/api/event'
import { readonly, ref } from 'vue'

import { clear_official_login_marker, get_anti_piracy_status } from '../helpers/auth.js'

export type AntiPiracyStatus = {
	region: 'checking' | 'cn' | 'non_cn' | 'unavailable'
	restricted: boolean
	revision: number
}

export function createAntiPiracyStatus() {
	const status = ref<AntiPiracyStatus>({ region: 'checking', restricted: false, revision: 0 })

	function setStatus(next: AntiPiracyStatus) {
		if (next.revision < status.value.revision) return
		status.value = next
	}

	return { status: readonly(status), setStatus }
}

const { status, setStatus } = createAntiPiracyStatus()
let initialization: Promise<void> | undefined

export function isOfflineAccountRestrictedError(error: unknown): boolean {
	const message =
		error instanceof Error
			? error.message
			: typeof error === 'string'
				? error
				: JSON.stringify(error)
	return message?.includes('OFFLINE_ACCOUNT_RESTRICTED') ?? false
}

export async function refreshAntiPiracyStatus(): Promise<void> {
	const next = (await get_anti_piracy_status()) as AntiPiracyStatus
	setStatus(next)
}

export function useAntiPiracyStatus() {
	initialization ??= (async () => {
		try {
			await listen<AntiPiracyStatus>('anti-piracy-status-changed', ({ payload }) => {
				setStatus(payload)
			})
		} catch (error) {
			console.warn('Could not subscribe to offline account eligibility changes', error)
		}
		await refreshAntiPiracyStatus()
	})().catch((error) => {
		console.warn('Could not initialize offline account eligibility status', error)
	})
	return {
		status,
		refresh: refreshAntiPiracyStatus,
		clear: async () => {
			setStatus((await clear_official_login_marker()) as AntiPiracyStatus)
		},
	}
}
