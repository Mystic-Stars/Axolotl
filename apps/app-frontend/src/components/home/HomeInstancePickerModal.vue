<script setup lang="ts">
import { CheckIcon } from '@modrinth/assets'
import { defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { nextTick, ref } from 'vue'

import InstancePickerList from '@/components/ui/instance/InstancePickerList.vue'
import type { GameInstance } from '@/helpers/types'

defineProps<{
	instances: GameInstance[]
	selectedInstanceId?: string | null
}>()

const emit = defineEmits<{
	select: [instance: GameInstance]
}>()

const { formatMessage } = useVIntl()
const modal = ref<InstanceType<typeof NewModal>>()
const instancePicker = ref<InstanceType<typeof InstancePickerList>>()

const messages = defineMessages({
	title: {
		id: 'app.home.minimal.picker.title',
		defaultMessage: 'Choose a Home instance',
	},
	search: {
		id: 'app.home.minimal.picker.search',
		defaultMessage: 'Search instances',
	},
	noInstances: {
		id: 'app.home.minimal.picker.no-instances',
		defaultMessage: 'No instances available',
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

function show() {
	instancePicker.value?.reset()
	modal.value?.show()
	void nextTick(() => instancePicker.value?.focus())
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
			<InstancePickerList
				ref="instancePicker"
				:instances="instances"
				:search-placeholder="formatMessage(messages.search)"
				:no-instances-message="formatMessage(messages.noInstances)"
				:no-matches-message="formatMessage(messages.noResults)"
				:select-label="(instance) => formatMessage(messages.select, { name: instance.name })"
				@select="selectInstance"
			>
				<template #action="{ instance }">
					<CheckIcon
						v-if="instance.id === selectedInstanceId"
						class="size-5 shrink-0 text-brand"
						aria-hidden="true"
					/>
				</template>
			</InstancePickerList>
		</div>
	</NewModal>
</template>
