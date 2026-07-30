import Fuse from 'fuse.js'
import type { Ref } from 'vue'
import { ref, watch, watchSyncEffect } from 'vue'

// window 级内存（导航切换保留，关软件丢弃）
const memory: Record<string, Map<string, any>> = ((window as any).__ctMemory ??= {})
function getMap<K, V>(namespace: string): Map<K, V> {
	if (!memory[namespace]) memory[namespace] = new Map()
	return memory[namespace]
}

export function useContentSearch<T>(
	items: Ref<T[]>,
	keys: string[],
	options?: { threshold?: number; distance?: number; memoryKey?: string },
) {
	const searchMemory = getMap<string, string>('search')
	const initialQuery = options?.memoryKey ? searchMemory.get(options.memoryKey) ?? '' : ''
	const searchQuery = ref(initialQuery)
	const fuse = new Fuse<T>([], {
		keys,
		threshold: options?.threshold ?? 0.4,
		distance: options?.distance ?? 100,
	})
	watchSyncEffect(() => fuse.setCollection(items.value))

	// 搜索变化写入内存
	if (options?.memoryKey) {
		watch(searchQuery, (val) => {
			searchMemory.set(options.memoryKey!, val)
		})
	}

	function search(source: T[]): T[] {
		const query = searchQuery.value.trim()
		if (!query) return source
		return fuse.search(query).map(({ item }) => item)
	}

	return { searchQuery, search }
}
