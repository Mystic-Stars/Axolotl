<script setup>
import {
	ClipboardCopyIcon,
	EyeIcon,
	FolderOpenIcon,
	MoreVerticalIcon,
	PinIcon,
	PlayIcon,
	PlusIcon,
	SearchIcon,
	StopCircleIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Accordion,
	ButtonStyled,
	Checkbox,
	commonMessages,
	defineMessages,
	DropdownSelect,
	FloatingActionBar,
	formatLoader,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import Instance from '@/components/ui/Instance.vue'
import BatchEditGroupsModal from '@/components/ui/modal/BatchEditGroupsModal.vue'
import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'
import { UNGROUPED_GROUP_KEY, useGridGrouping } from '@/composables/useGridGrouping'
import { install_duplicate_instance } from '@/helpers/install'
import { edit, remove, set_pinned } from '@/helpers/instance'

const { handleError } = injectNotificationManager()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	search: { id: 'app.instances.search', defaultMessage: 'Search' },
	select: { id: 'app.instances.select', defaultMessage: 'Select...' },
	groupBy: { id: 'app.instances.group-by', defaultMessage: 'Group by:' },
	addContent: { id: 'app.instances.add-content', defaultMessage: 'Add content' },
	viewInstance: { id: 'app.instances.view-instance', defaultMessage: 'View instance' },
	duplicateInstance: {
		id: 'app.instances.duplicate-instance',
		defaultMessage: 'Duplicate instance',
	},
	copyPath: { id: 'app.instances.copy-path', defaultMessage: 'Copy path' },
	pinToHome: { id: 'app.instances.pin-to-home', defaultMessage: 'Pin to Home' },
	unpinFromHome: { id: 'app.instances.unpin-from-home', defaultMessage: 'Unpin from Home' },
	name: { id: 'app.instances.sort.name', defaultMessage: 'Name' },
	lastPlayed: { id: 'app.instances.sort.last-played', defaultMessage: 'Last played' },
	dateCreated: { id: 'app.instances.sort.date-created', defaultMessage: 'Date created' },
	dateModified: { id: 'app.instances.sort.date-modified', defaultMessage: 'Date modified' },
	gameVersion: { id: 'app.instances.group.game-version', defaultMessage: 'Game version' },
	group: { id: 'app.instances.group.group', defaultMessage: 'Group' },
	loader: { id: 'app.instances.group.loader', defaultMessage: 'Loader' },
	none: { id: 'app.instances.group.none', defaultMessage: 'None' },
	ungrouped: { id: 'app.instances.group.ungrouped', defaultMessage: 'No group' },
	editGroups: { id: 'app.instances.edit-groups', defaultMessage: 'Edit groups' },
	selectAll: { id: 'app.instances.select-all', defaultMessage: 'Select all' },
	deselectAll: { id: 'app.instances.deselect-all', defaultMessage: 'Deselect all' },
	selectedCount: {
		id: 'app.instances.selected-count',
		defaultMessage: '{count, plural, one {# selected} other {# selected}}',
	},
})

const optionMessages = {
	Name: messages.name,
	'Last played': messages.lastPlayed,
	'Date created': messages.dateCreated,
	'Date modified': messages.dateModified,
	'Game version': messages.gameVersion,
	Group: messages.group,
	Loader: messages.loader,
	None: messages.none,
}

const formatOption = (option) =>
	optionMessages[option] ? formatMessage(optionMessages[option]) : option

const props = defineProps({
	instances: {
		type: Array,
		default() {
			return []
		},
	},
	label: {
		type: String,
		default: '',
	},
})

const instanceOptions = ref(null)
const instanceComponents = ref(null)
const currentDeleteInstance = ref(null)
const batchDeleteCount = ref(0)
const confirmModal = ref(null)
const search = ref('')

const filteredInstances = computed(() =>
	props.instances.filter((instance) =>
		instance.name.toLowerCase().includes(search.value.toLowerCase()),
	),
)

const { state, grouping, filteredResults, isSectionCollapsed, setSectionCollapsed } =
	useGridGrouping(props.label, filteredInstances, {
		formatLoader: (loader) => formatLoader(formatMessage, loader),
	})

async function deleteInstance() {
	if (currentDeleteInstance.value) {
		instanceComponents.value = instanceComponents.value.filter(
			(x) => x.instance.id !== currentDeleteInstance.value.id,
		)
		await remove(currentDeleteInstance.value.id).catch(handleError)
	}
	batchDeleteCount.value = 0
}

async function duplicateInstance(p) {
	await install_duplicate_instance(p).catch(handleError)
}

const handleRightClick = (event, instanceId) => {
	const item = instanceComponents.value.find((x) => x.instance.id === instanceId)
	const baseOptions = [
		{ name: 'add_content' },
		{ type: 'divider' },
		{ name: 'edit' },
		{ name: 'duplicate' },
		{ name: item.instance.pinned_at ? 'unpin' : 'pin' },
		{ name: 'open' },
		{ name: 'copy' },
		{ type: 'divider' },
		{
			name: 'delete',
			color: 'danger',
		},
	]

	instanceOptions.value.showMenu(
		event,
		item,
		item.playing
			? [
					{
						name: 'stop',
						color: 'danger',
					},
					...baseOptions,
				]
			: [
					{
						name: 'play',
						color: 'primary',
					},
					...baseOptions,
				],
	)
}

const handleOptionsClick = async (args) => {
	switch (args.option) {
		case 'play':
			args.item.play(null, 'InstanceGridContextMenu')
			break
		case 'stop':
			args.item.stop(null, 'InstanceGridContextMenu')
			break
		case 'add_content':
			await args.item.addContent()
			break
		case 'edit':
			await args.item.seeInstance()
			break
		case 'duplicate':
			if (args.item.instance.install_stage == 'installed')
				await duplicateInstance(args.item.instance.id)
			break
		case 'pin':
			await set_pinned(args.item.instance.id, true).catch(handleError)
			break
		case 'unpin':
			await set_pinned(args.item.instance.id, false).catch(handleError)
			break
		case 'open':
			await args.item.openFolder()
			break
		case 'copy':
			await navigator.clipboard.writeText(args.item.instance.id)
			break
		case 'delete':
			currentDeleteInstance.value = args.item.instance
			confirmModal.value.show()
			break
	}
}

// Selection mode
const selectMode = ref(false)
const selectedInstanceIds = ref(new Set())
const batchEditModal = ref(null)

let longPressTimer = null
let longPressTriggered = false

function startLongPress(instanceId) {
	longPressTriggered = false
	longPressTimer = setTimeout(() => {
		longPressTriggered = true
		if (!selectMode.value) {
			selectMode.value = true
		}
		toggleInstanceSelection(instanceId)
	}, 500)
}

function cancelLongPress() {
	if (longPressTimer) {
		clearTimeout(longPressTimer)
		longPressTimer = null
	}
}

function handleCardClick(instanceId, event) {
	if (longPressTriggered) {
		longPressTriggered = false
		return
	}
	if (selectMode.value) {
		toggleInstanceSelection(instanceId)
	}
}

function toggleSelectMode() {
	selectMode.value = !selectMode.value
	if (!selectMode.value) {
		selectedInstanceIds.value.clear()
	}
}

function toggleInstanceSelection(instanceId) {
	const newSet = new Set(selectedInstanceIds.value)
	if (newSet.has(instanceId)) {
		newSet.delete(instanceId)
	} else {
		newSet.add(instanceId)
	}
	selectedInstanceIds.value = newSet
	if (newSet.size === 0) {
		selectMode.value = false
	}
}

function handleCheckboxClick(instanceId) {
	if (!selectMode.value) {
		selectMode.value = true
	}
	toggleInstanceSelection(instanceId)
}

function openBatchEdit() {
	batchEditModal.value?.show()
}

const batchDeleteConfirmModal = ref(null)

function openBatchDelete() {
	batchDeleteCount.value = selectedInstanceIds.value.size
	batchDeleteConfirmModal.value?.show()
}

async function batchDeleteInstances() {
	for (const id of selectedInstanceIds.value) {
		instanceComponents.value = instanceComponents.value.filter((x) => x.instance.id !== id)
		await remove(id).catch(handleError)
	}
	selectedInstanceIds.value.clear()
	selectMode.value = false
}

const visibleInstanceIds = computed(() => {
	const ids = []
	for (const section of Array.from(filteredResults.value, ([, value]) => value)) {
		for (const instance of section) {
			ids.push(instance.id)
		}
	}
	return ids
})

const isAllSelected = computed(() => {
	const visibleIds = visibleInstanceIds.value
	return visibleIds.length > 0 && visibleIds.every((id) => selectedInstanceIds.value.has(id))
})

function toggleSelectAll() {
	const visibleIds = visibleInstanceIds.value
	if (isAllSelected.value) {
		const newSet = new Set(selectedInstanceIds.value)
		for (const id of visibleIds) {
			newSet.delete(id)
		}
		selectedInstanceIds.value = newSet
	} else {
		const newSet = new Set(selectedInstanceIds.value)
		for (const id of visibleIds) {
			newSet.add(id)
		}
		selectedInstanceIds.value = newSet
	}
}

function onBatchEditApplied() {
	selectedInstanceIds.value.clear()
	selectMode.value = false
}
</script>
<template>
	<div class="flex gap-2">
		<StyledInput
			v-model="search"
			:icon="SearchIcon"
			type="text"
			:placeholder="formatMessage(messages.search)"
			clearable
			wrapper-class="flex-1"
		/>
		<DropdownSelect
			v-slot="{ selected }"
			v-model="state.sortBy"
			name="Sort Dropdown"
			class="max-w-[16rem]"
			:options="['Name', 'Last played', 'Date created', 'Date modified', 'Game version']"
			:display-name="formatOption"
			:placeholder="formatMessage(messages.select)"
		>
			<span class="font-semibold text-primary">{{
				formatMessage(commonMessages.sortByLabel)
			}}</span>
			<span class="font-semibold text-secondary">{{ selected }}</span>
		</DropdownSelect>
		<DropdownSelect
			v-slot="{ selected }"
			v-model="state.group"
			class="max-w-[16rem]"
			name="Group Dropdown"
			:options="['Group', 'Loader', 'Game version', 'None']"
			:display-name="formatOption"
			:placeholder="formatMessage(messages.select)"
		>
			<span class="font-semibold text-primary">{{ formatMessage(messages.groupBy) }} </span>
			<span class="font-semibold text-secondary">{{ selected }}</span>
		</DropdownSelect>
	</div>
	<Accordion
		v-for="instanceSection in Array.from(filteredResults, ([key, value]) => ({
			key,
			value,
		}))"
		:key="instanceSection.key"
		:divider="grouping === 'Group' || instanceSection.key !== UNGROUPED_GROUP_KEY"
		:open-by-default="!isSectionCollapsed(instanceSection.key)"
		class="row"
		@on-open="setSectionCollapsed(instanceSection.key, false)"
		@on-close="setSectionCollapsed(instanceSection.key, true)"
	>
		<template v-if="grouping === 'Group' || instanceSection.key !== UNGROUPED_GROUP_KEY" #title>
			<span class="text-base">{{
				instanceSection.key === UNGROUPED_GROUP_KEY
					? formatMessage(messages.ungrouped)
					: instanceSection.key
			}}</span>
		</template>
		<section class="instances">
			<div
				v-for="instance in instanceSection.value"
				:key="instance.id + instance.install_stage"
				class="group relative"
			>
				<div
					class="relative cursor-pointer select-none rounded-lg transition-all hover:brightness-90 active:scale-[0.98]"
					@click="handleCardClick(instance.id, $event)"
					@mousedown="!selectMode && startLongPress(instance.id)"
					@mouseup="cancelLongPress"
					@mouseleave="cancelLongPress"
					@touchstart="!selectMode && startLongPress(instance.id)"
					@touchend="cancelLongPress"
					@touchcancel="cancelLongPress"
				>
					<div :class="{ 'pointer-events-none': selectMode }">
						<Instance
							ref="instanceComponents"
							:instance="instance"
							:disabled="selectMode"
							:class="{ 'opacity-50': selectMode && !selectedInstanceIds.has(instance.id) }"
							@contextmenu.prevent.stop="(event) => handleRightClick(event, instance.id)"
						/>
					</div>
				</div>
				<div
					class="absolute right-2 bottom-2 z-10 transition-opacity"
					:class="
						selectMode && selectedInstanceIds.has(instance.id)
							? ''
							: 'opacity-0 group-hover:opacity-100'
					"
					@click.stop="handleCheckboxClick(instance.id)"
				>
					<Checkbox :model-value="selectedInstanceIds.has(instance.id)" />
				</div>
				<div
					v-if="!selectMode"
					class="absolute right-2 top-2 opacity-0 group-hover:opacity-100 transition-opacity"
					@click.stop="(event) => handleRightClick(event, instance.id)"
				>
					<ButtonStyled circular size="small" type="transparent">
						<button type="button">
							<MoreVerticalIcon />
						</button>
					</ButtonStyled>
				</div>
			</div>
		</section>
	</Accordion>
	<ConfirmDeleteInstanceModal
		ref="confirmModal"
		:symlink-target="currentDeleteInstance?.symlink_target"
		:count="batchDeleteCount"
		@delete="batchDeleteCount > 0 ? batchDeleteInstances() : deleteInstance()"
	/>
	<ConfirmDeleteInstanceModal
		ref="batchDeleteConfirmModal"
		:count="selectedInstanceIds.size"
		@delete="batchDeleteInstances"
	/>
	<BatchEditGroupsModal
		ref="batchEditModal"
		:instance-ids="[...selectedInstanceIds]"
		@applied="onBatchEditApplied"
	/>
	<FloatingActionBar :shown="selectMode" position="top" aria-label="Instance selection">
		<span class="px-3 py-2 text-base font-semibold text-contrast tabular-nums">
			{{ formatMessage(messages.selectedCount, { count: selectedInstanceIds.size }) }}
		</span>
		<div class="mx-0.5 h-6 w-px bg-surface-5" />
		<ButtonStyled type="transparent">
			<button type="button" @click="toggleSelectAll">
				<span>{{
					isAllSelected ? formatMessage(messages.deselectAll) : formatMessage(messages.selectAll)
				}}</span>
			</button>
		</ButtonStyled>
		<ButtonStyled type="transparent">
			<button type="button" @click="openBatchEdit">
				<span>{{ formatMessage(messages.editGroups) }}</span>
			</button>
		</ButtonStyled>
		<ButtonStyled color="red" type="transparent">
			<button type="button" @click="openBatchDelete">
				<TrashIcon />
				<span class="bar-label">{{ formatMessage(commonMessages.deleteLabel) }}</span>
			</button>
		</ButtonStyled>
		<div class="ml-auto" />
		<ButtonStyled type="transparent">
			<button class="!text-primary" type="button" @click="toggleSelectMode">
				<XIcon class="hidden cq-show-icon" />
				<span class="bar-label">{{ formatMessage(commonMessages.clearButton) }}</span>
			</button>
		</ButtonStyled>
	</FloatingActionBar>
	<ContextMenu ref="instanceOptions" @option-clicked="handleOptionsClick">
		<template #play> <PlayIcon /> {{ formatMessage(commonMessages.playButton) }} </template>
		<template #stop> <StopCircleIcon /> {{ formatMessage(commonMessages.stopButton) }} </template>
		<template #add_content> <PlusIcon /> {{ formatMessage(messages.addContent) }} </template>
		<template #edit> <EyeIcon /> {{ formatMessage(messages.viewInstance) }} </template>
		<template #duplicate>
			<ClipboardCopyIcon /> {{ formatMessage(messages.duplicateInstance) }}
		</template>
		<template #pin> <PinIcon /> {{ formatMessage(messages.pinToHome) }} </template>
		<template #unpin>
			<PinIcon class="rotate-45" /> {{ formatMessage(messages.unpinFromHome) }}
		</template>
		<template #delete> <TrashIcon /> {{ formatMessage(commonMessages.deleteLabel) }} </template>
		<template #open>
			<FolderOpenIcon /> {{ formatMessage(commonMessages.openFolderButton) }}
		</template>
		<template #copy> <ClipboardCopyIcon /> {{ formatMessage(messages.copyPath) }} </template>
	</ContextMenu>
</template>
<style lang="scss" scoped>
.row {
	width: 100%;
}

.instances {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
	width: 100%;
	gap: 0.75rem;
	margin-right: auto;
	scroll-behavior: smooth;
	overflow-y: auto;
}
</style>
