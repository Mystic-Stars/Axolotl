import { setEulaAccepted } from '@modrinth/server'
import { ref, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import type EulaModal from '@/components/multiplayer/servers/EulaModal.vue'
import { type ServerView, useServers } from '@/composables/useServers'
import { servers as serversApi } from '@/helpers/servers'

/**
 * Shared "start with EULA gate" flow used by the servers overview and the
 * server detail page. Bind the returned `eulaModal` ref to an `<EulaModal>`
 * rendered with `ref="eulaModal"` in the component template.
 */
export function useServerLifecycle() {
	const { startServer } = useServers()

	const eulaModal = useTemplateRef<ComponentExposed<typeof EulaModal>>('eulaModal')
	const eulaText = ref('')
	let pendingId = ''

	/** Starts the server; if the EULA is unaccepted, shows the EULA modal first. */
	async function tryStartServer(server: ServerView) {
		if (!server.eulaAccepted && server.eulaExists) {
			try {
				eulaText.value = await serversApi.readFile(server.id, 'eula.txt')
				pendingId = server.id
				eulaModal.value?.show()
				return
			} catch {
				// No eula.txt: a fresh start will generate it
			}
		}
		await startServer(server.id)
	}

	async function acceptEula() {
		const id = pendingId
		if (!id) return
		try {
			const updated = setEulaAccepted(eulaText.value, true)
			await serversApi.writeFile(id, 'eula.txt', updated)
			pendingId = ''
			eulaModal.value?.hide()
			await startServer(id)
		} catch (error) {
			console.error(error)
		}
	}

	function declineEula() {
		pendingId = ''
		eulaModal.value?.hide()
	}

	return { eulaModal, eulaText, tryStartServer, acceptEula, declineEula }
}
