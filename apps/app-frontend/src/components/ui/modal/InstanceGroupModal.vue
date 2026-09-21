<template>
	<NewModal ref="modal" no-padding scrollable actions-divider max-width="560px" width="560px">
		<template #title>
			<span class="text-2xl font-semibold text-contrast">
				{{
					existingGroupName
						? formatMessage(messages.addToGroupTitle, { groupName: existingGroupName })
						: formatMessage(messages.header)
				}}
			</span>
		</template>

		<template v-if="!existingGroupName">
			<div class="flex flex-col gap-2.5 p-6">
				<label for="new-group-name" class="font-semibold text-contrast">
					{{ formatMessage(messages.groupName) }}
				</label>
				<StyledInput
					id="new-group-name"
					ref="groupNameInput"
					v-model="newGroupName"
					:placeholder="formatMessage(messages.groupNamePlaceholder)"
					:maxlength="32"
					@click="groupNameInput?.select()"
				/>
			</div>

			<div class="h-px bg-divider" />
		</template>

		<div class="flex h-[400px] flex-col gap-3 overflow-y-auto bg-surface-2 py-4">
			<div class="px-6">
				<StyledInput
					v-model="newGroupSearch"
					:icon="SearchIcon"
					:placeholder="formatMessage(messages.searchInstance)"
					class="w-full"
				/>
			</div>

			<div
				v-if="newGroupInstances.length === 0"
				class="flex items-center justify-center py-12 text-secondary"
			>
				{{ formatMessage(messages.noInstancesFound) }}
			</div>
			<div v-else class="flex flex-col gap-1">
				<div
					v-for="instance in newGroupInstances"
					:key="instance.id"
					class="flex items-center justify-between gap-4 px-6 py-1.5 hover:bg-surface-3"
					:class="{ 'opacity-60': selectedNewGroupInstanceIds.has(instance.id) }"
				>
					<div class="flex min-w-0 items-center gap-2.5">
						<InstanceIcon
							:icon-path="instance.icon_path"
							:instance-id="instance.id"
							:loader="instance.loader"
							class="!size-[2rem] !rounded-md"
						/>
						<div class="flex min-w-0 items-center gap-2">
							<span class="truncate font-semibold text-contrast">{{ instance.name }}</span>
							<TagItem v-if="instance.groups && instance.groups.length > 0" class="shrink-0">
								{{ getGroupName(instance.groups[0]) }}
							</TagItem>
						</div>
					</div>
					<Button
						:type="selectedNewGroupInstanceIds.has(instance.id) ? 'outlined' : 'base'"
						@click="toggleNewGroupInstance(instance.id)"
					>
						<CheckIcon v-if="selectedNewGroupInstanceIds.has(instance.id)" />
						{{
							formatMessage(
								selectedNewGroupInstanceIds.has(instance.id) ? messages.added : messages.add,
							)
						}}
					</Button>
				</div>
			</div>
		</div>

		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<Button type="outlined" @click="modal?.hide()">
					<XIcon />
					{{ formatMessage(messages.cancel) }}
				</Button>
				<Button
					type="colored"
					color="brand"
					:disabled="existingGroupName ? !canSaveGroups : !canCreateGroup"
					@click="existingGroupName ? handleSaveGroups() : handleCreateGroup()"
				>
					<SpinnerIcon v-if="creatingGroup" class="animate-spin" />
					<CheckIcon v-else-if="existingGroupName" />
					<PlusIcon v-else />
					{{ formatMessage(existingGroupName ? messages.saveChanges : messages.createGroup) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, PlusIcon, SearchIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	defineMessages,
	NewButton as Button,
	NewModal,
	StyledInput,
	TagItem,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import { create_group, set_group_memberships, list_groups } from '@/helpers/instance-groups'
import { list } from '@/helpers/instance'
import { FAVORITES_GROUP_ID } from '@/composables/useInstanceGroups'
import type { GameInstance } from '@/helpers/types'

const { formatMessage } = useVIntl()

const props = defineProps<{
	instanceIds: string[]
	existingGroupName?: string
	existingGroupId?: string
}>()

const emit = defineEmits<{
	(e: 'applied'): void
}>()

const messages = defineMessages({
	header: {
		id: 'app.instances.batch-edit-groups.header',
		defaultMessage: 'Create group',
	},
	addToGroupTitle: {
		id: 'app.instances.batch-edit-groups.add-to-group-title',
		defaultMessage: 'Add instances to "{groupName}"',
	},
	groupName: {
		id: 'app.instances.batch-edit-groups.group-name',
		defaultMessage: 'Group name',
	},
	groupNamePlaceholder: {
		id: 'app.instances.batch-edit-groups.group-name-placeholder',
		defaultMessage: 'Enter group name',
	},
	searchInstance: {
		id: 'app.instances.batch-edit-groups.search-instance',
		defaultMessage: 'Search instances',
	},
	noInstancesFound: {
		id: 'app.instances.batch-edit-groups.no-instances-found',
		defaultMessage: 'No instances found',
	},
	add: {
		id: 'app.instances.batch-edit-groups.add',
		defaultMessage: 'Add',
	},
	added: {
		id: 'app.instances.batch-edit-groups.added',
		defaultMessage: 'Added',
	},
	cancel: {
		id: 'app.instances.batch-edit-groups.cancel',
		defaultMessage: 'Cancel',
	},
	createGroup: {
		id: 'app.instances.batch-edit-groups.create',
		defaultMessage: 'Create group',
	},
	pinnedGroupName: {
		id: 'app.instances.group.pinned',
		defaultMessage: 'Pinned',
	},
	saveChanges: {
		id: 'app.instances.batch-edit-groups.save-changes',
		defaultMessage: 'Save',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const groupNameInput = ref<InstanceType<typeof StyledInput>>()
const newGroupName = ref('')
const newGroupSearch = ref('')
const creatingGroup = ref(false)
const allInstances = ref<GameInstance[]>([])
const groupNameMap = ref(new Map<string, string>())
const selectedNewGroupInstanceIds = ref(new Set<string>())

const canCreateGroup = computed(() => newGroupName.value.trim().length > 0)

const canSaveGroups = computed(() => {
	if (!props.existingGroupName) return false
	const groupId = props.existingGroupId
	if (!groupId) return false
	return allInstances.value.some((instance) => {
		const isSelected = selectedNewGroupInstanceIds.value.has(instance.id)
		const hasGroup = (instance.groups || []).includes(groupId)
		return isSelected !== hasGroup
	})
})

const newGroupInstances = computed(() => {
	const search = newGroupSearch.value.toLowerCase()
	return allInstances.value.filter((instance) => {
		if (!search) return true
		return instance.name.toLowerCase().includes(search)
	})
})

function getGroupName(groupId: string) {
	if (groupId === FAVORITES_GROUP_ID) {
		return formatMessage(messages.pinnedGroupName)
	}
	return groupNameMap.value.get(groupId) || groupId
}

function show(ids?: string[]) {
	newGroupSearch.value = ''
	creatingGroup.value = false
	Promise.all([list(), list_groups()]).then(([instances, groups]) => {
		allInstances.value = instances as GameInstance[]
		const map = new Map<string, string>()
		for (const g of groups) {
			map.set(g.id, g.name)
		}
		groupNameMap.value = map

		if (!props.existingGroupName) {
			const customGroups = groups.filter((g: { id: string }) => g.id !== 'group:favorites')
			let groupNumber = customGroups.length + 1
			const existingNames = new Set(groups.map((g: { name: string }) => g.name.toLowerCase()))
			while (existingNames.has(`group ${groupNumber}`)) {
				groupNumber++
			}
			newGroupName.value = `Group ${groupNumber}`
		}

		if (props.existingGroupName) {
			const groupId = props.existingGroupId
			selectedNewGroupInstanceIds.value = new Set(
				allInstances.value.filter((i) => (i.groups || []).includes(groupId || '')).map((i) => i.id),
			)
		} else {
			selectedNewGroupInstanceIds.value = new Set(ids ?? props.instanceIds)
		}
	})
	modal.value?.show()
}

function toggleNewGroupInstance(instanceId: string) {
	const next = new Set(selectedNewGroupInstanceIds.value)
	if (next.has(instanceId)) {
		next.delete(instanceId)
	} else {
		next.add(instanceId)
	}
	selectedNewGroupInstanceIds.value = next
}

async function handleCreateGroup() {
	const name = newGroupName.value.trim()
	if (!name || selectedNewGroupInstanceIds.value.size === 0) return

	creatingGroup.value = true

	const group = await create_group(name).catch(() => null)
	if (!group) {
		creatingGroup.value = false
		return
	}

	if (selectedNewGroupInstanceIds.value.size > 0) {
		const updates = [...selectedNewGroupInstanceIds.value].map((instanceId) => ({
			instance_id: instanceId,
			group_ids: [group.id],
		}))
		await set_group_memberships(updates).catch(() => {})
	}

	creatingGroup.value = false
	modal.value?.hide()
	emit('applied')
}

async function handleSaveGroups() {
	if (!props.existingGroupName) return

	creatingGroup.value = true
	const groupId = props.existingGroupId

	const changedInstances = allInstances.value.filter((instance) => {
		const isSelected = selectedNewGroupInstanceIds.value.has(instance.id)
		const hasGroup = (instance.groups || []).includes(groupId || '')
		return isSelected !== hasGroup
	})

	const updates = changedInstances.map((instance) => {
		const isSelected = selectedNewGroupInstanceIds.value.has(instance.id)
		return {
			instance_id: instance.id,
			group_ids: isSelected ? [groupId!] : [],
		}
	})
	await set_group_memberships(updates).catch(() => {})

	creatingGroup.value = false
	modal.value?.hide()
	emit('applied')
}

defineExpose({ show })
</script>
