import type { AbstractWebNotificationManager } from '@modrinth/ui'
import { provideTags } from '@modrinth/ui'
import { ref } from 'vue'

import { get_game_versions, get_loaders } from '@/helpers/tags'

export function setupTagsProvider(
	notificationManager: AbstractWebNotificationManager,
	stateInitialization: Promise<void>,
) {
	const { handleError } = notificationManager

	const gameVersions = ref([])
	const loaders = ref([])

	async function refreshGameVersions() {
		try {
			gameVersions.value = await get_game_versions()
		} catch (error) {
			handleError(error)
		}
	}

	stateInitialization
		.then(async () => {
			await Promise.all([
				refreshGameVersions(),
				get_loaders()
					.then((v) => {
						loaders.value = v
					})
					.catch(handleError),
			])
		})
		.catch(() => {})

	provideTags({ gameVersions, loaders, refreshGameVersions })
}
