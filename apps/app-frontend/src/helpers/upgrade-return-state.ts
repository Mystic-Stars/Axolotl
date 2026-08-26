import { isRef, toRaw, unref } from 'vue'

import type { UpgradeFlowSnapshot } from '@/pages/instance/upgrade/flow'

let parked: UpgradeFlowSnapshot | null = null

export function parkUpgradeFlow(snapshot: UpgradeFlowSnapshot) {
	parked = cloneUpgradeFlowSnapshot(snapshot)
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

export function cloneUpgradeFlowSnapshot(snapshot: UpgradeFlowSnapshot): UpgradeFlowSnapshot {
	return structuredClone(toPlainUpgradeDto(snapshot)) as UpgradeFlowSnapshot
}

export function peekUpgradeFlow(instanceId?: string): UpgradeFlowSnapshot | null {
	if (!parked || (instanceId && parked.instanceId !== instanceId)) return null
	return cloneUpgradeFlowSnapshot(parked)
}

export function consumeUpgradeFlow(
	instanceId: string,
	returnFullPath: string,
): UpgradeFlowSnapshot | null {
	if (!parked || parked.instanceId !== instanceId || parked.returnFullPath !== returnFullPath)
		return null
	const snapshot = cloneUpgradeFlowSnapshot(parked)
	parked = null
	return snapshot
}

export function restoreUpgradeFlow(
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

export function clearUpgradeFlow() {
	parked = null
}

export function upgradeProjectPath(
	provider: string | null,
	projectId: string | null,
): string | null {
	if (!projectId) return null
	if (provider === 'modrinth') return `/project/${encodeURIComponent(projectId)}`
	if (provider === 'curseforge') return `/project/curseforge/${encodeURIComponent(projectId)}`
	return null
}
