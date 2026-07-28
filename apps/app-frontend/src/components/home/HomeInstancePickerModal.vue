<script setup lang="ts">
import { CheckIcon, SearchIcon } from '@modrinth/assets'
import { defineMessages, NewModal, StyledInput, useVIntl } from '@modrinth/ui'
import { computed, nextTick, ref } from 'vue'

import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import type { GameInstance } from '@/helpers/types'

const props = defineProps<{
	instances: GameInstance[]
	selectedInstanceId?: string | null
}>()

const emit = defineEmits<{
	select: [instance: GameInstance]
}>()

const { formatMessage, locale } = useVIntl()
const modal = ref<InstanceType<typeof NewModal>>()
const searchInput = ref<InstanceType<typeof StyledInput>>()
const searchQuery = ref('')

const messages = defineMessages({
	title: {
		id: 'app.home.minimal.picker.title',
		defaultMessage: 'Choose a Home instance',
	},
	search: {
		id: 'app.home.minimal.picker.search',
		defaultMessage: 'Search instances',
	},
	noResults: {
		id: 'app.home.minimal.picker.no-results',
		defaultMessage: 'No matching instances',
	},
	select: {
		id: 'app.home.minimal.picker.select',
		defaultMessage: 'Choose {name}',
	},
})

const visibleInstances = computed(() => {
	const query = searchQuery.value.trim().toLocaleLowerCase(locale.value)
	return props.instances
		.filter((instance) => {
			if (!query) return true
			return [instance.name, instance.loader, instance.game_version].some((value) =>
				value.toLocaleLowerCase(locale.value).includes(query),
			)
		})
		.slice()
		.sort((a, b) => a.name.localeCompare(b.name, locale.value, { sensitivity: 'base' }))
})

function show() {
	searchQuery.value = ''
	modal.value?.show()
	void nextTick(() => searchInput.value?.focus())
}

function selectInstance(instance: GameInstance) {
	emit('select', instance)
	modal.value?.hide()
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		max-width="560px"
		width="min(560px, calc(100vw - 2rem))"
		scrollable
		max-content-height="min(36rem, 70vh)"
	>
		<div class="flex min-w-0 flex-col gap-4">
			<StyledInput
				ref="searchInput"
				v-model="searchQuery"
				type="search"
				:icon="SearchIcon"
				:placeholder="formatMessage(messages.search)"
				wrapper-class="w-full"
				clearable
			/>
			<ul v-if="visibleInstances.length > 0" class="m-0 flex list-none flex-col gap-1 p-0">
				<li v-for="instance in visibleInstances" :key="instance.id" class="min-w-0">
					<button
						type="button"
						class="flex min-h-16 w-full cursor-pointer items-center gap-3 rounded-lg border-0 bg-transparent px-3 py-2 text-left text-primary transition-colors hover:bg-button-bg focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow"
						:aria-label="formatMessage(messages.select, { name: instance.name })"
						@click="selectInstance(instance)"
					>
						<InstanceIcon
							class="size-10 shrink-0"
							:icon-path="instance.icon_path"
							:instance-id="instance.id"
						/>
						<span class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="truncate font-semibold text-contrast">{{ instance.name }}</span>
							<span class="truncate text-sm capitalize text-secondary">
								{{ instance.loader }} {{ instance.game_version }}
							</span>
						</span>
						<CheckIcon
							v-if="instance.id === selectedInstanceId"
							class="size-5 shrink-0 text-brand"
							aria-hidden="true"
						/>
					</button>
				</li>
			</ul>
			<p v-else class="m-0 py-8 text-center text-sm text-secondary">
				{{ formatMessage(messages.noResults) }}
			</p>
		</div>
	</NewModal>
</template>
