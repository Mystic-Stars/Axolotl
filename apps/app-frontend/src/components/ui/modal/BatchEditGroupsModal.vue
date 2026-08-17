<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" fade="standard" max-width="500px">
		<p class="m-0 text-secondary">
			{{ formatMessage(messages.description, { count: instanceIds.length }) }}
		</p>

		<div class="flex flex-col gap-3 mt-4">
			<RadioButtons v-model="selectedGroup" :items="groupOptions">
				<template #default="{ item }">
					<span class="flex items-center justify-between flex-1 leading-none">
						<span>{{ item || formatMessage(messages.noGroup) }}</span>
						<button
							v-if="item"
							class="bg-transparent border-none cursor-pointer text-secondary hover:text-red rounded flex items-center justify-center w-6 h-6"
							@click.stop="deleteGroup(item)"
						>
							<TrashIcon class="w-4 h-4" />
						</button>
						<span v-else class="w-6 h-6" />
					</span>
				</template>
			</RadioButtons>

			<div class="flex gap-2 items-center">
				<StyledInput
					v-model="newGroupInput"
					:placeholder="formatMessage(messages.enterGroupName)"
					class="w-full max-w-[300px]"
					@submit="addNewGroup"
				/>
				<ButtonStyled>
					<button class="w-fit !shadow-none" @click="addNewGroup">
						<PlusIcon /> {{ formatMessage(messages.createGroup) }}
					</button>
				</ButtonStyled>
			</div>
		</div>

		<template #actions>
			<div class="flex gap-2 justify-end">
				<ButtonStyled type="outlined">
					<button @click="modal?.hide()">
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button @click="confirm">
						<CheckIcon />
						{{ formatMessage(messages.applyButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, PlusIcon, TrashIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	NewModal,
	RadioButtons,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { edit, list } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

const { formatMessage } = useVIntl()

const props = defineProps<{
	instanceIds: string[]
}>()

const emit = defineEmits<{
	(e: 'applied'): void
}>()

const messages = defineMessages({
	header: {
		id: 'app.instances.batch-edit-groups.header',
		defaultMessage: 'Edit groups',
	},
	description: {
		id: 'app.instances.batch-edit-groups.description',
		defaultMessage: 'Select groups to apply to {count} instance(s).',
	},
	enterGroupName: {
		id: 'app.instances.batch-edit-groups.enter-group-name',
		defaultMessage: 'Enter group name',
	},
	createGroup: {
		id: 'app.instances.batch-edit-groups.create-group',
		defaultMessage: 'Create new group',
	},
	applyButton: {
		id: 'app.instances.batch-edit-groups.apply',
		defaultMessage: 'Apply',
	},
	noGroup: {
		id: 'app.instances.group.ungrouped',
		defaultMessage: 'No group',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const selectedGroup = ref('')
const newGroupInput = ref('')
const allInstances = ref<GameInstance[]>([])

const availableGroups = computed(() => {
	const groups = new Set<string>()
	for (const instance of allInstances.value) {
		for (const group of instance.groups) {
			groups.add(group)
		}
	}
	return [...groups]
})

const groupOptions = computed(() => ['', ...availableGroups.value])

function show() {
	selectedGroup.value = ''
	newGroupInput.value = ''
	list().then((instances) => {
		allInstances.value = instances as GameInstance[]
	})
	modal.value?.show()
}

function addNewGroup() {
	const text = newGroupInput.value.trim()
	if (text.length > 0) {
		const groupName = text.substring(0, 32)
		allInstances.value.push({ groups: [groupName] } as GameInstance)
		selectedGroup.value = groupName
		newGroupInput.value = ''
	}
}

async function deleteGroup(group: string) {
	for (const instance of allInstances.value) {
		if (instance.groups.includes(group)) {
			const newGroups = instance.groups.filter((g) => g !== group)
			await edit(instance.id, { groups: newGroups }).catch(() => {})
			instance.groups = newGroups
		}
	}
	if (selectedGroup.value === group) {
		selectedGroup.value = ''
	}
}

async function confirm() {
	if (newGroupInput.value.trim().length > 0) {
		addNewGroup()
	}

	modal.value?.hide()

	const groups = selectedGroup.value ? [selectedGroup.value.trim().substring(0, 32)] : []

	for (const instanceId of props.instanceIds) {
		await edit(instanceId, { groups }).catch(() => {})
	}

	emit('applied')
}

defineExpose({
	show,
})
</script>
