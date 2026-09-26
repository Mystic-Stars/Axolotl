<template>
	<NewModal
		ref="modal"
		max-width="560px"
		:closable="true"
		:close-on-click-outside="true"
		:on-hide="() => emit('cancel')"
	>
		<template #title>
			<span class="text-[var(--color-text-primary)]">{{
				formatMessage(messages.title, { type: contentTypeLabel })
			}}</span>
		</template>

		<div class="flex flex-col gap-4">
			<!-- File name display -->
			<span class="text-[var(--color-text-tertiary)] text-sm truncate">{{ internalFileName }}</span>

			<!-- Instance search -->
			<StyledInput
				v-model="searchQuery"
				:placeholder="formatMessage(messages.searchPlaceholder)"
				:icon="SearchIcon"
				class="w-full"
			/>

			<!-- Instance list -->
			<div
				v-if="filteredInstances.length === 0"
				class="flex flex-col items-center gap-2 py-8 text-[var(--color-text-tertiary)]"
			>
				<PackageOpenIcon class="size-8" />
				<span class="text-sm">{{ formatMessage(messages.noInstances) }}</span>
			</div>

			<div v-else class="flex flex-col gap-2">
				<InstanceRowCard
					v-for="inst in filteredInstances"
					:key="inst.id"
					:name="inst.name"
					:version="inst.gameVersion"
					:loader="inst.loader"
					@select="emit('install', inst.id)"
				>
					<template #prepend>
						<FolderOpenIcon
							v-if="!inst.iconUrl"
							class="size-6 text-[var(--color-text-tertiary)] shrink-0"
							stroke-width="1.5"
						/>
						<img v-else :src="inst.iconUrl" alt="" class="size-6 shrink-0 rounded object-cover" />
					</template>
					<template #append>
						<ButtonStyled type="standard" size="small" @click.stop="emit('install', inst.id)">
							<template #prefix>
								<DownloadIcon class="size-4" />
							</template>
							{{ formatMessage(messages.install) }}
						</ButtonStyled>
					</template>
				</InstanceRowCard>
			</div>

			<!-- Create new instance link -->
			<button
				class="flex items-center gap-2 text-sm text-brand transition-colors hover:text-brand-hover hover:cursor-pointer self-start"
				@click="emit('navigateCreate')"
			>
				<PlusIcon class="size-4" />
				{{ formatMessage(messages.createNew) }}
			</button>
		</div>

		<template #actions>
			<div class="flex w-full items-center justify-end">
				<ButtonStyled type="transparent" @click="emit('cancel')">
					{{ formatMessage(messages.cancel) }}
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import {
	DownloadIcon,
	FolderOpenIcon,
	PackageOpenIcon,
	PlusIcon,
	SearchIcon,
} from '@modrinth/assets'
import { computed, ref } from 'vue'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import InstanceRowCard from '#ui/components/base/InstanceRowCard.vue'
import StyledInput from '#ui/components/base/StyledInput.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'drop.generic_install.title',
		defaultMessage: 'Install {type}',
	},
	searchPlaceholder: {
		id: 'drop.generic_install.search',
		defaultMessage: 'Search instances...',
	},
	install: {
		id: 'drop.generic_install.install',
		defaultMessage: 'Install',
	},
	cancel: {
		id: 'drop.generic_install.cancel',
		defaultMessage: 'Cancel',
	},
	createNew: {
		id: 'drop.generic_install.create_new',
		defaultMessage: 'Create new instance...',
	},
	noInstances: {
		id: 'drop.generic_install.no_instances',
		defaultMessage: 'No instances found',
	},
})

export interface InstanceInfo {
	id: string
	name: string
	iconUrl?: string | null
	gameVersion?: string | null
	loader?: string | null
}

defineProps<{
	instances?: InstanceInfo[]
}>()

const emit = defineEmits<{
	(e: 'install', instanceId: string): void
	(e: 'cancel' | 'navigateCreate'): void
}>()

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const searchQuery = ref('')
const internalInstances = ref<InstanceInfo[]>([])
const internalFileName = ref('')
const contentTypeLabel = ref('')

const contentTypeMap: Record<string, string> = {
	mod: 'Mod',
	resource_pack: 'Resource Pack',
	data_pack: 'Data Pack',
	shader_pack: 'Shader Pack',
	world_save: 'World',
	litematic: 'Schematic',
	schematic: 'Schematic',
}

const filteredInstances = computed(() => {
	const query = searchQuery.value.toLowerCase().trim()
	if (!query) return internalInstances.value
	return internalInstances.value.filter((inst) => inst.name.toLowerCase().includes(query))
})

async function show(options: {
	contentType: string
	fileName: string
	instances?: InstanceInfo[]
}) {
	contentTypeLabel.value = contentTypeMap[options.contentType] ?? options.contentType
	internalFileName.value = options.fileName
	searchQuery.value = ''

	if (options.instances) {
		internalInstances.value = options.instances
	} else {
		// Default: try to load instances from the injected list provider
		// When instances are not provided via props, the parent should populate them
		internalInstances.value = []
	}

	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

defineExpose({ show, hide })
</script>
