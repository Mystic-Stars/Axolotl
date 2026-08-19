<script setup lang="ts">
import { SaveIcon, SpinnerIcon } from '@modrinth/assets'
import {
	defineMessages,
	injectNotificationManager,
	NewButton as Button,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings'
import { parseTerracottaPublicNodes } from '@/helpers/terracotta'

const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()

const messages = defineMessages({
	publicNodes: {
		id: 'app.multiplayer.terracotta.public-nodes',
		defaultMessage: 'Terracotta public nodes',
	},
	publicNodesDescription: {
		id: 'app.multiplayer.terracotta.public-nodes-description',
		defaultMessage:
			'Enter one node URI per line. Changes apply the next time you host or join a room. Leave empty to use only Terracotta defaults.',
	},
	publicNodesPlaceholder: {
		id: 'app.multiplayer.terracotta.public-nodes-placeholder',
		defaultMessage: 'wss://center.node.1tmc.top',
	},
	publicNodeInvalid: {
		id: 'app.multiplayer.terracotta.public-node-invalid',
		defaultMessage: 'Invalid node URI: {node}. Use http, https, tcp, tls, udp, ws, or wss.',
	},
	savePublicNodes: {
		id: 'app.multiplayer.terracotta.save-public-nodes',
		defaultMessage: 'Save nodes',
	},
	publicNodesSaved: {
		id: 'app.multiplayer.terracotta.public-nodes-saved',
		defaultMessage: 'Terracotta public nodes saved',
	},
})

const initialSettings = await getSettings()
const savedPublicNodes = ref(initialSettings.terracotta_public_nodes)
const publicNodesInput = ref(savedPublicNodes.value.join('\n'))
const publicNodesTouched = ref(false)
const isSavingPublicNodes = ref(false)

const parsedPublicNodes = computed(() => parseTerracottaPublicNodes(publicNodesInput.value))
const publicNodesChanged = computed(
	() => publicNodesInput.value !== savedPublicNodes.value.join('\n'),
)
const showPublicNodesError = computed(
	() => publicNodesTouched.value && parsedPublicNodes.value.invalidNode !== null,
)

async function savePublicNodes() {
	publicNodesTouched.value = true
	if (parsedPublicNodes.value.invalidNode || isSavingPublicNodes.value) return

	isSavingPublicNodes.value = true
	try {
		const settings = await getSettings()
		settings.terracotta_public_nodes = parsedPublicNodes.value.nodes
		await setSettings(settings)
		savedPublicNodes.value = [...parsedPublicNodes.value.nodes]
		publicNodesInput.value = savedPublicNodes.value.join('\n')
		addNotification({
			type: 'success',
			title: formatMessage(messages.publicNodesSaved),
		})
	} catch (error) {
		handleError(error)
	} finally {
		isSavingPublicNodes.value = false
	}
}
</script>

<template>
	<section class="flex max-w-3xl flex-col gap-4">
		<div>
			<h2 id="terracotta-public-nodes-title" class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.publicNodes) }}
			</h2>
			<p id="terracotta-public-nodes-description" class="mb-0 mt-1 leading-tight text-secondary">
				{{ formatMessage(messages.publicNodesDescription) }}
			</p>
		</div>

		<textarea
			id="terracotta-public-nodes"
			v-model="publicNodesInput"
			class="box-border min-h-32 w-full resize-y rounded-lg border border-solid bg-surface-3 px-3 py-2 font-mono text-sm text-contrast outline-none transition-colors"
			:class="showPublicNodesError ? 'border-red' : 'border-surface-4 focus:border-brand'"
			:placeholder="formatMessage(messages.publicNodesPlaceholder)"
			aria-labelledby="terracotta-public-nodes-title"
			:aria-invalid="showPublicNodesError"
			:aria-describedby="
				showPublicNodesError
					? 'terracotta-public-nodes-description terracotta-public-nodes-error'
					: 'terracotta-public-nodes-description'
			"
			:spellcheck="false"
			@input="publicNodesTouched = true"
		></textarea>

		<p v-if="showPublicNodesError" id="terracotta-public-nodes-error" class="m-0 text-sm text-red">
			{{ formatMessage(messages.publicNodeInvalid, { node: parsedPublicNodes.invalidNode }) }}
		</p>

		<div class="flex justify-end border-0 border-t border-solid border-surface-4 pt-4">
			<Button
				type="colored"
				color="brand"
				native-type="button"
				:disabled="!publicNodesChanged || !!parsedPublicNodes.invalidNode || isSavingPublicNodes"
				@click="savePublicNodes"
			>
				<SpinnerIcon v-if="isSavingPublicNodes" class="animate-spin" />
				<SaveIcon v-else />
				{{ formatMessage(messages.savePublicNodes) }}
			</Button>
		</div>
	</section>
</template>
