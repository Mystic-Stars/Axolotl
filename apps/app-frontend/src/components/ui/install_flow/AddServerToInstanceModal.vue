<script setup>
import { CheckIcon, PlusIcon, SearchIcon } from '@modrinth/assets'
import {
	Admonition,
	Avatar,
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import { trackEvent } from '@/helpers/analytics'
import { list } from '@/helpers/instance'
import { add_server_to_instance, get_instance_worlds } from '@/helpers/worlds.ts'

const { handleError } = injectNotificationManager()
const queryClient = useQueryClient()
const { formatMessage } = useVIntl()
const messages = defineMessages({
	addServer: { id: 'app.server.add-to-instance', defaultMessage: 'Add server to instance' },
	compatibilityWarning: {
		id: 'app.server.compatibility-warning',
		defaultMessage: 'This server may not be compatible with all instances.',
	},
	searchInstance: {
		id: 'app.server.search-instance',
		defaultMessage: 'Search for an instance',
	},
	adding: { id: 'app.server.adding', defaultMessage: 'Adding...' },
	added: { id: 'app.server.added', defaultMessage: 'Added' },
	add: { id: 'app.server.add', defaultMessage: 'Add' },
	symlinkWarningHeader: { id: 'app.symlink-warning.write.header' },
	symlinkWarningBody: { id: 'app.symlink-warning.write.body' },
})

defineProps({
	symlinkTarget: {
		type: String,
		default: null,
	},
})

const modal = ref()
const searchFilter = ref('')
const instances = ref([])

const serverName = ref('')
const serverAddress = ref('')

const shownInstances = computed(() =>
	instances.value.filter((instance) => {
		return instance.name.toLowerCase().includes(searchFilter.value.toLowerCase())
	}),
)

defineExpose({
	show: async (name, address) => {
		serverName.value = name
		serverAddress.value = address
		searchFilter.value = ''

		const instanceValues = await list().catch(handleError)
		await Promise.allSettled(
			instanceValues.map(async (instance) => {
				instance.adding = false
				instance.added = false

				try {
					const worlds = await get_instance_worlds(instance.id)
					instance.added = worlds.some(
						(w) => w.type === 'server' && w.address === serverAddress.value,
					)
				} catch {
					// Ignore - will show as not added
				}
			}),
		)

		instances.value = instanceValues
		modal.value.show()

		trackEvent('AddServerToInstanceStart', { source: 'AddServerToInstanceModal' })
	},
})

async function addServer(instance) {
	instance.adding = true
	try {
		await add_server_to_instance(instance.id, serverName.value, serverAddress.value, 'prompt')
		instance.added = true
		await queryClient.invalidateQueries({ queryKey: ['worlds', instance.id] })

		trackEvent('AddServerToInstance', {
			server_name: serverName.value,
			instance_name: instance.name,
			source: 'AddServerToInstanceModal',
		})
	} catch (err) {
		handleError(err)
	}
	instance.adding = false
}
</script>

<template>
	<ModalWrapper ref="modal" :header="formatMessage(messages.addServer)">
		<div class="flex flex-col gap-4 min-w-[350px]">
			<Admonition
				v-if="symlinkTarget"
				type="warning"
				:header="formatMessage(messages.symlinkWarningHeader)"
			>
				{{ formatMessage(messages.symlinkWarningBody, { path: symlinkTarget }) }}
			</Admonition>
			<Admonition type="warning" :body="formatMessage(messages.compatibilityWarning)" />
			<StyledInput
				v-model="searchFilter"
				:icon="SearchIcon"
				type="search"
				:placeholder="formatMessage(messages.searchInstance)"
				autocomplete="off"
			/>
			<div class="max-h-[21rem] overflow-y-auto">
				<div
					v-for="instance in shownInstances"
					:key="instance.id"
					class="flex w-full items-center justify-between gap-2 bg-bg-raised text-icon shadow-none"
				>
					<router-link
						class="btn btn-transparent p-2 text-left"
						:to="`/instance/${encodeURIComponent(instance.id)}`"
						@click="modal.hide()"
					>
						<Avatar
							:src="instance.icon_path ? convertFileSrc(instance.icon_path) : null"
							class="mr-2 [--size:2rem]"
						/>
						{{ instance.name }}
					</router-link>
					<ButtonStyled>
						<button :disabled="instance.added || instance.adding" @click="addServer(instance)">
							<PlusIcon v-if="!instance.added && !instance.adding" />
							<CheckIcon v-else-if="instance.added" />
							{{
								instance.adding
									? formatMessage(messages.adding)
									: instance.added
										? formatMessage(messages.added)
										: formatMessage(messages.add)
							}}
						</button>
					</ButtonStyled>
				</div>
			</div>
			<div class="input-group push-right">
				<ButtonStyled>
					<button @click="modal.hide()">{{ formatMessage(commonMessages.cancelButton) }}</button>
				</ButtonStyled>
			</div>
		</div>
	</ModalWrapper>
</template>
