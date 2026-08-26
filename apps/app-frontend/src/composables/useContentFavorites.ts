import { computed, ref } from 'vue'

import {
	addContentFavorite,
	type ContentFavorite,
	type ContentFavoriteInput,
	contentFavoriteKey,
	type FavoriteProvider,
	listContentFavorites,
	removeContentFavorite,
} from '../helpers/content-favorites.ts'

export interface ContentFavoritesApi {
	list: () => Promise<ContentFavorite[]>
	add: (input: ContentFavoriteInput) => Promise<ContentFavorite>
	remove: (provider: FavoriteProvider, projectId: string) => Promise<void>
}

function sortFavorites(values: ContentFavorite[]) {
	return [...values].sort(
		(left, right) =>
			right.saved_at - left.saved_at ||
			left.provider.localeCompare(right.provider) ||
			left.project_id.localeCompare(right.project_id),
	)
}

function sameFavorites(left: ContentFavorite[], right: ContentFavorite[]) {
	return (
		left.length === right.length &&
		left.every(
			(favorite, index) =>
				favorite.provider === right[index]?.provider &&
				favorite.project_id === right[index]?.project_id &&
				favorite.content_type === right[index]?.content_type &&
				favorite.saved_at === right[index]?.saved_at,
		)
	)
}

export function createContentFavoritesStore(api: ContentFavoritesApi) {
	const favorites = ref<ContentFavorite[]>([])
	const loaded = ref(false)
	const loading = ref(false)
	const pendingKeys = ref(new Set<string>())
	let loadPromise: Promise<ContentFavorite[]> | null = null

	function setFavorites(values: ContentFavorite[]) {
		const sorted = sortFavorites(values)
		if (!sameFavorites(favorites.value, sorted)) favorites.value = sorted
		return favorites.value
	}

	function setPending(key: string, pending: boolean) {
		const next = new Set(pendingKeys.value)
		if (pending) next.add(key)
		else next.delete(key)
		pendingKeys.value = next
	}

	async function load(force = false): Promise<ContentFavorite[]> {
		if (loaded.value && !force) return favorites.value
		if (loadPromise) return await loadPromise

		loading.value = true
		loadPromise = api
			.list()
			.then((values) => {
				setFavorites(values)
				loaded.value = true
				return favorites.value
			})
			.finally(() => {
				loading.value = false
				loadPromise = null
			})

		return await loadPromise
	}

	const favoriteKeys = computed(
		() =>
			new Set(
				favorites.value.map((favorite) =>
					contentFavoriteKey(favorite.provider, favorite.project_id),
				),
			),
	)

	function isFavorite(provider: FavoriteProvider, projectId: string) {
		return favoriteKeys.value.has(contentFavoriteKey(provider, projectId))
	}

	function isPending(provider: FavoriteProvider, projectId: string) {
		return pendingKeys.value.has(contentFavoriteKey(provider, projectId))
	}

	async function add(input: ContentFavoriteInput) {
		const key = contentFavoriteKey(input.provider, input.project_id)
		if (pendingKeys.value.has(key)) return
		setPending(key, true)
		try {
			const saved = await api.add(input)
			setFavorites([
				...favorites.value.filter(
					(favorite) => contentFavoriteKey(favorite.provider, favorite.project_id) !== key,
				),
				saved,
			])
			loaded.value = true
		} finally {
			setPending(key, false)
		}
	}

	async function remove(provider: FavoriteProvider, projectId: string) {
		const key = contentFavoriteKey(provider, projectId)
		if (pendingKeys.value.has(key)) return
		setPending(key, true)
		try {
			await api.remove(provider, projectId)
			setFavorites(
				favorites.value.filter(
					(favorite) => contentFavoriteKey(favorite.provider, favorite.project_id) !== key,
				),
			)
			loaded.value = true
		} finally {
			setPending(key, false)
		}
	}

	async function toggle(input: ContentFavoriteInput) {
		if (isFavorite(input.provider, input.project_id)) {
			await remove(input.provider, input.project_id)
		} else {
			await add(input)
		}
	}

	return {
		favorites,
		loaded,
		loading,
		load,
		isFavorite,
		isPending,
		add,
		remove,
		toggle,
	}
}

const sharedStore = createContentFavoritesStore({
	list: listContentFavorites,
	add: addContentFavorite,
	remove: removeContentFavorite,
})

export function useContentFavorites() {
	return sharedStore
}
