import type { ContentItem, ContentModpackCardCategory, ContentModpackCardVersion, ContentModpackCardProject, ContentOwner } from '@modrinth/ui'

export interface InstanceContentModpackCache {
	project: ContentModpackCardProject
	version: ContentModpackCardVersion
	owner: ContentOwner | null
	categories: ContentModpackCardCategory[]
	hasUpdate: boolean
	updateVersionId: string | null
}

export interface InstanceContentCache {
	instanceId: string
	updatedAt: number

	// 内容数据（大，变动频率低）
	contentItems: ContentItem[] | null
	modpack: InstanceContentModpackCache | null
	linkedContentItems: ContentItem[]

	// UI 偏好（关闭软件后保留）
	modpackHintDismissed: boolean
}

// ---- 内部实现：数据拆分存储，避免单 key 过大导致 localStorage 静默写入失败 ----

interface CacheDataSlice {
	contentItems: ContentItem[] | null
	modpack: InstanceContentModpackCache | null
	linkedContentItems: ContentItem[]
}

interface CacheUiSlice {
	modpackHintDismissed: boolean
}

function dataKey(instanceId: string): string {
	return `instance:${instanceId}:data`
}

function uiKey(instanceId: string): string {
	return `instance:${instanceId}:ui`
}

function safeGetItem(key: string): string | null {
	try {
		return localStorage.getItem(key)
	} catch (err) {
		console.error(`[InstanceCache] Failed to read localStorage key "${key}"`, err)
		return null
	}
}

function safeSetItem(key: string, value: string): boolean {
	try {
		localStorage.setItem(key, value)
		return true
	} catch (err) {
		console.error(`[InstanceCache] Failed to write localStorage key "${key}" (size: ${value.length} bytes)`, err)
		return false
	}
}

function safeRemoveItem(key: string): void {
	try {
		localStorage.removeItem(key)
	} catch (err) {
		console.error(`[InstanceCache] Failed to remove localStorage key "${key}"`, err)
	}
}

function defaultData(): CacheDataSlice {
	return {
		contentItems: null,
		modpack: null,
		linkedContentItems: [],
	}
}

function defaultUi(): CacheUiSlice {
	return {
		modpackHintDismissed: false,
	}
}

function readDataSlice(instanceId: string): CacheDataSlice | null {
	try {
		const raw = safeGetItem(dataKey(instanceId))
		if (!raw) return null
		return JSON.parse(raw) as CacheDataSlice
	} catch (err) {
		console.error(`[InstanceCache] Failed to parse data slice for "${instanceId}"`, err)
		return null
	}
}

function writeDataSlice(instanceId: string, patch: Partial<CacheDataSlice>): boolean {
	const existing = readDataSlice(instanceId)
	const base = existing ?? defaultData()
	const merged = { ...base, ...patch }
	return safeSetItem(dataKey(instanceId), JSON.stringify(merged))
}

function readUiSlice(instanceId: string): CacheUiSlice | null {
	try {
		const raw = safeGetItem(uiKey(instanceId))
		if (!raw) return null
		return JSON.parse(raw) as CacheUiSlice
	} catch (err) {
		console.error(`[InstanceCache] Failed to parse UI slice for "${instanceId}"`, err)
		return null
	}
}

function writeUiSlice(instanceId: string, patch: Partial<CacheUiSlice>): boolean {
	const existing = readUiSlice(instanceId)
	const base = existing ?? defaultUi()
	const merged = { ...base, ...patch }
	return safeSetItem(uiKey(instanceId), JSON.stringify(merged))
}

// ---- 旧缓存迁移 ----

/**
 * 旧的分散缓存 key 列表。在首次写入新缓存后清理，释放 localStorage 空间。
 */
const LEGACY_KEYS = [
	'instance-content-cache',
	'instance-linked-content-cache',
	'content-ui-state',
	'content-tab-modpack-hint-dismissed',
]

/** 匹配旧的 content-filters-* key */
const LEGACY_FILTER_KEY_PREFIX = 'content-filters-'

let migrationDone = false

/**
 * 清理旧版缓存系统遗留的 localStorage key。
 * 只执行一次（per session），释放被旧数据占用的配额。
 */
function migrateFromLegacyCache(): void {
	if (migrationDone) return
	migrationDone = true

	// 删除固定名称的旧 key
	for (const key of LEGACY_KEYS) {
		safeRemoveItem(key)
	}

	// 删除所有 content-filters-type:* 和 content-filters-status:* 旧 key
	try {
		const keysToRemove: string[] = []
		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i)
			if (key?.startsWith(LEGACY_FILTER_KEY_PREFIX)) {
				keysToRemove.push(key)
			}
		}
		for (const key of keysToRemove) {
			safeRemoveItem(key)
		}
	} catch (err) {
		console.error('[InstanceCache] Failed to enumerate localStorage keys during migration', err)
	}
}

/**
 * 读取实例的统一内容缓存。
 * 返回 null 表示该实例没有任何数据缓存（data slice 为空）。
 */
export function readInstanceCache(instanceId: string): InstanceContentCache | null {
	const data = readDataSlice(instanceId)
	if (!data?.contentItems) return null

	const ui = readUiSlice(instanceId)

	return {
		instanceId,
		updatedAt: 0,
		contentItems: data.contentItems,
		modpack: data.modpack,
		linkedContentItems: data.linkedContentItems,
		modpackHintDismissed: ui?.modpackHintDismissed ?? false,
	}
}

/**
 * 写入实例的统一内容缓存（merge 模式）。
 * 大小数据自动分流：内容数据写入 data key，UI 状态写入 ui key。
 */
export function writeInstanceCache(
	instanceId: string,
	patch: Partial<Omit<InstanceContentCache, 'instanceId' | 'updatedAt'>>,
): void {
	migrateFromLegacyCache()

	const dataPatch: Partial<CacheDataSlice> = {}
	const uiPatch: Partial<CacheUiSlice> = {}

	if ('contentItems' in patch) dataPatch.contentItems = patch.contentItems
	if ('modpack' in patch) dataPatch.modpack = patch.modpack
	if ('linkedContentItems' in patch) dataPatch.linkedContentItems = patch.linkedContentItems
	if ('modpackHintDismissed' in patch) uiPatch.modpackHintDismissed = patch.modpackHintDismissed

	if (Object.keys(dataPatch).length > 0) {
		writeDataSlice(instanceId, dataPatch)
	}
	if (Object.keys(uiPatch).length > 0) {
		writeUiSlice(instanceId, uiPatch)
	}
}

/**
 * 删除实例的统一内容缓存。
 * 在实例被删除时调用。
 */
export function removeInstanceCache(instanceId: string): void {
	safeRemoveItem(dataKey(instanceId))
	safeRemoveItem(uiKey(instanceId))
}
