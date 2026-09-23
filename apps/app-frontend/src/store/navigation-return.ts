import { createPinia, defineStore, getActivePinia, setActivePinia } from 'pinia'
import { isRef, ref, toRaw, unref } from 'vue'

import type { UpgradeFlowSnapshot } from '../pages/instance/upgrade/flow.ts'

export interface BrowseReturnSnapshot<T> {
	url: string
	scrollTop: number
	state: T
}

function toPlainUpgradeDto(value: unknown): unknown {
	const unwrapped = isRef(value) ? unref(value) : value
	if (Array.isArray(unwrapped)) return unwrapped.map(toPlainUpgradeDto)
	if (unwrapped && typeof unwrapped === 'object') {
		return Object.fromEntries(
			Object.entries(toRaw(unwrapped)).map(([key, entry]) => [key, toPlainUpgradeDto(entry)]),
		)
	}
	return unwrapped
}

function cloneUpgradeFlowSnapshot(snapshot: UpgradeFlowSnapshot): UpgradeFlowSnapshot {
	return structuredClone(toPlainUpgradeDto(snapshot)) as UpgradeFlowSnapshot
}

export function isBrowseReturnSourcePath(path: string): boolean {
	return path === '/downloads' || path.startsWith('/project/') || path.startsWith('/instance/')
}

/**
 * Owns browse/upgrade return navigation state that used to live in module-level
 * helpers. UI and router read this store instead of hidden module globals.
 */
export const useNavigationReturnStore = defineStore('navigationReturnStore', () => {
	const browseSnapshot = ref<BrowseReturnSnapshot<unknown> | null>(null)
	const browseReturnUrl = ref<string | null>(null)
	const parkedUpgrade = ref<UpgradeFlowSnapshot | null>(null)

	function saveBrowseReturnSnapshot<T>(snapshot: BrowseReturnSnapshot<T>): void {
		browseSnapshot.value = snapshot
	}

	function consumeBrowseReturnSnapshot<T>(url: string): BrowseReturnSnapshot<T> | null {
		if (browseSnapshot.value?.url !== url) return null
		const snapshot = browseSnapshot.value as BrowseReturnSnapshot<T>
		browseSnapshot.value = null
		return snapshot
	}

	function hasBrowseReturnSnapshot(url: string): boolean {
		return browseSnapshot.value?.url === url
	}

	function clearBrowseReturnSnapshot(): void {
		browseSnapshot.value = null
		browseReturnUrl.value = null
	}

	function prepareBrowseReturnNavigation(url: string, sourcePath: string): boolean {
		if (isBrowseReturnSourcePath(sourcePath) && hasBrowseReturnSnapshot(url)) {
			browseReturnUrl.value = url
			return true
		}
		clearBrowseReturnSnapshot()
		return false
	}

	function isBrowseReturnNavigation(url: string): boolean {
		return browseReturnUrl.value === url
	}

	function completeBrowseReturnNavigation(url: string): void {
		if (browseReturnUrl.value === url) browseReturnUrl.value = null
	}

	function parkUpgradeFlow(snapshot: UpgradeFlowSnapshot) {
		parkedUpgrade.value = cloneUpgradeFlowSnapshot(snapshot)
	}

	function peekUpgradeFlow(instanceId?: string): UpgradeFlowSnapshot | null {
		if (!parkedUpgrade.value || (instanceId && parkedUpgrade.value.instanceId !== instanceId))
			return null
		return cloneUpgradeFlowSnapshot(parkedUpgrade.value)
	}

	function consumeUpgradeFlow(
		instanceId: string,
		returnFullPath: string,
	): UpgradeFlowSnapshot | null {
		if (
			!parkedUpgrade.value ||
			parkedUpgrade.value.instanceId !== instanceId ||
			parkedUpgrade.value.returnFullPath !== returnFullPath
		) {
			return null
		}
		const snapshot = cloneUpgradeFlowSnapshot(parkedUpgrade.value)
		parkedUpgrade.value = null
		return snapshot
	}

	function restoreUpgradeFlow(
		instanceId: string,
		returnFullPath: string,
		hydrate: (snapshot: UpgradeFlowSnapshot) => void,
	): UpgradeFlowSnapshot | null {
		const snapshot = peekUpgradeFlow(instanceId)
		if (!snapshot || snapshot.returnFullPath !== returnFullPath) return null
		hydrate(snapshot)
		consumeUpgradeFlow(instanceId, returnFullPath)
		return snapshot
	}

	function clearUpgradeFlow() {
		parkedUpgrade.value = null
	}

	return {
		saveBrowseReturnSnapshot,
		consumeBrowseReturnSnapshot,
		hasBrowseReturnSnapshot,
		clearBrowseReturnSnapshot,
		prepareBrowseReturnNavigation,
		isBrowseReturnNavigation,
		completeBrowseReturnNavigation,
		parkUpgradeFlow,
		peekUpgradeFlow,
		consumeUpgradeFlow,
		restoreUpgradeFlow,
		clearUpgradeFlow,
	}
})

export function upgradeProjectPath(
	provider: string | null,
	projectId: string | null,
): string | null {
	if (!projectId) return null
	if (provider === 'modrinth') return `/project/${encodeURIComponent(projectId)}`
	if (provider === 'curseforge') return `/project/curseforge/${encodeURIComponent(projectId)}`
	return null
}

export { cloneUpgradeFlowSnapshot }

function ensureActivePinia() {
	if (!getActivePinia()) {
		setActivePinia(createPinia())
	}
	return useNavigationReturnStore()
}

export function parkUpgradeFlow(snapshot: UpgradeFlowSnapshot) {
	ensureActivePinia().parkUpgradeFlow(snapshot)
}

export function peekUpgradeFlow(instanceId?: string): UpgradeFlowSnapshot | null {
	return ensureActivePinia().peekUpgradeFlow(instanceId)
}

export function consumeUpgradeFlow(
	instanceId: string,
	returnFullPath: string,
): UpgradeFlowSnapshot | null {
	return ensureActivePinia().consumeUpgradeFlow(instanceId, returnFullPath)
}

export function restoreUpgradeFlow(
	instanceId: string,
	returnFullPath: string,
	hydrate: (snapshot: UpgradeFlowSnapshot) => void,
): UpgradeFlowSnapshot | null {
	return ensureActivePinia().restoreUpgradeFlow(instanceId, returnFullPath, hydrate)
}

export function clearUpgradeFlow() {
	ensureActivePinia().clearUpgradeFlow()
}

export function saveBrowseReturnSnapshot<T>(snapshot: BrowseReturnSnapshot<T>): void {
	ensureActivePinia().saveBrowseReturnSnapshot(snapshot)
}

export function consumeBrowseReturnSnapshot<T>(url: string): BrowseReturnSnapshot<T> | null {
	return ensureActivePinia().consumeBrowseReturnSnapshot<T>(url)
}

export function hasBrowseReturnSnapshot(url: string): boolean {
	return ensureActivePinia().hasBrowseReturnSnapshot(url)
}

export function clearBrowseReturnSnapshot(): void {
	ensureActivePinia().clearBrowseReturnSnapshot()
}

export function prepareBrowseReturnNavigation(url: string, sourcePath: string): boolean {
	return ensureActivePinia().prepareBrowseReturnNavigation(url, sourcePath)
}

export function isBrowseReturnNavigation(url: string): boolean {
	return ensureActivePinia().isBrowseReturnNavigation(url)
}

export function completeBrowseReturnNavigation(url: string): void {
	ensureActivePinia().completeBrowseReturnNavigation(url)
}
