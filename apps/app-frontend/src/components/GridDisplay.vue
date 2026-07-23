<script setup>
import {
	ClipboardCopyIcon,
	EyeIcon,
	FolderOpenIcon,
	PlayIcon,
	PlusIcon,
	SearchIcon,
	StopCircleIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	Accordion,
	commonMessages,
	defineMessages,
	DropdownSelect,
	formatLoader,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { useStorage } from '@vueuse/core'
import dayjs from 'dayjs'
import { computed, ref } from 'vue'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import Instance from '@/components/ui/Instance.vue'
import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'
import { install_duplicate_instance } from '@/helpers/install'
import { remove } from '@/helpers/instance'

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
	name: { id: 'app.instances.sort.name', defaultMessage: 'Name' },
	lastPlayed: { id: 'app.instances.sort.last-played', defaultMessage: 'Last played' },
	dateCreated: { id: 'app.instances.sort.date-created', defaultMessage: 'Date created' },
	dateModified: { id: 'app.instances.sort.date-modified', defaultMessage: 'Date modified' },
	gameVersion: { id: 'app.instances.group.game-version', defaultMessage: 'Game version' },
	group: { id: 'app.instances.group.group', defaultMessage: 'Group' },
	loader: { id: 'app.instances.group.loader', defaultMessage: 'Loader' },
	none: { id: 'app.instances.group.none', defaultMessage: 'None' },
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
const confirmModal = ref(null)

async function deleteInstance() {
	if (currentDeleteInstance.value) {
		instanceComponents.value = instanceComponents.value.filter(
			(x) => x.instance.id !== currentDeleteInstance.value.id,
		)
		await remove(currentDeleteInstance.value.id).catch(handleError)
	}
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

const state = useStorage(
	`${props.label}-grid-display-state`,
	{
		group: 'Group',
		sortBy: 'Name',
		collapsedGroups: [],
	},
	localStorage,
	{ mergeDefaults: true },
)

const search = ref('')
const collapsedSectionKeys = computed(() => new Set(state.value.collapsedGroups ?? []))

const getSectionKey = (sectionName) => `${state.value.group}:${sectionName}`

const isSectionCollapsed = (sectionName) => {
	return collapsedSectionKeys.value.has(getSectionKey(sectionName))
}

const setSectionCollapsed = (sectionName, collapsed) => {
	const sectionKey = getSectionKey(sectionName)
	const collapsedSections = new Set(state.value.collapsedGroups ?? [])

	if (collapsed) {
		collapsedSections.add(sectionKey)
	} else {
		collapsedSections.delete(sectionKey)
	}

	state.value.collapsedGroups = [...collapsedSections]
}

const filteredResults = computed(() => {
	const { group = 'Group', sortBy = 'Name' } = state.value

	const instances = props.instances.filter((instance) => {
		return instance.name.toLowerCase().includes(search.value.toLowerCase())
	})

	if (sortBy === 'Name') {
		instances.sort((a, b) => {
			return a.name.localeCompare(b.name)
		})
	}

	if (sortBy === 'Game version') {
		instances.sort((a, b) => {
			return a.game_version.localeCompare(b.game_version, undefined, { numeric: true })
		})
	}

	if (sortBy === 'Last played') {
		instances.sort((a, b) => {
			return dayjs(b.last_played ?? 0).diff(dayjs(a.last_played ?? 0))
		})
	}

	if (sortBy === 'Date created') {
		instances.sort((a, b) => {
			return dayjs(b.date_created).diff(dayjs(a.date_created))
		})
	}

	if (sortBy === 'Date modified') {
		instances.sort((a, b) => {
			return dayjs(b.date_modified).diff(dayjs(a.date_modified))
		})
	}

	const instanceMap = new Map()

	if (group === 'Loader') {
		instances.forEach((instance) => {
			const loader = formatLoader(formatMessage, instance.loader)
			if (!instanceMap.has(loader)) {
				instanceMap.set(loader, [])
			}

			instanceMap.get(loader).push(instance)
		})
	} else if (group === 'Game version') {
		instances.forEach((instance) => {
			if (!instanceMap.has(instance.game_version)) {
				instanceMap.set(instance.game_version, [])
			}

			instanceMap.get(instance.game_version).push(instance)
		})
	} else if (group === 'Group') {
		instances.forEach((instance) => {
			if (instance.groups.length === 0) {
				instance.groups.push('None')
			}

			for (const category of instance.groups) {
				if (!instanceMap.has(category)) {
					instanceMap.set(category, [])
				}

				instanceMap.get(category).push(instance)
			}
		})
	} else {
		return instanceMap.set('None', instances)
	}

	// For 'name', we intuitively expect the sorting to apply to the name of the group first, not just the name of the instance
	// ie: Category A should come before B, even if the first instance in B comes before the first instance in A
	if (sortBy === 'Name') {
		const sortedEntries = [...instanceMap.entries()].sort((a, b) => {
			// None should always be first
			if (a[0] === 'None' && b[0] !== 'None') {
				return -1
			}
			if (a[0] !== 'None' && b[0] === 'None') {
				return 1
			}
			return a[0].localeCompare(b[0])
		})
		instanceMap.clear()
		sortedEntries.forEach((entry) => {
			instanceMap.set(entry[0], entry[1])
		})
	}
	// default sorting would do 1.20.4 < 1.8.9 because 2 < 8
	// localeCompare with numeric=true puts 1.8.9 < 1.20.4 because 8 < 20
	if (group === 'Game version') {
		const sortedEntries = [...instanceMap.entries()].sort((a, b) => {
			return a[0].localeCompare(b[0], undefined, { numeric: true })
		})
		instanceMap.clear()
		sortedEntries.forEach((entry) => {
			instanceMap.set(entry[0], entry[1])
		})
	}

	return instanceMap
})
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
		:divider="instanceSection.key !== 'None'"
		:open-by-default="!isSectionCollapsed(instanceSection.key)"
		class="row"
		@on-open="setSectionCollapsed(instanceSection.key, false)"
		@on-close="setSectionCollapsed(instanceSection.key, true)"
	>
		<template v-if="instanceSection.key !== 'None'" #title>
			<span class="text-base">{{ instanceSection.key }}</span>
		</template>
		<section class="instances">
			<Instance
				v-for="instance in instanceSection.value"
				ref="instanceComponents"
				:key="instance.id + instance.install_stage"
				:instance="instance"
				@contextmenu.prevent.stop="(event) => handleRightClick(event, instance.id)"
			/>
		</section>
	</Accordion>
	<ConfirmDeleteInstanceModal
		ref="confirmModal"
		:symlink-target="currentDeleteInstance?.symlink_target"
		@delete="deleteInstance"
	/>
	<ContextMenu ref="instanceOptions" @option-clicked="handleOptionsClick">
		<template #play> <PlayIcon /> {{ formatMessage(commonMessages.playButton) }} </template>
		<template #stop> <StopCircleIcon /> {{ formatMessage(commonMessages.stopButton) }} </template>
		<template #add_content> <PlusIcon /> {{ formatMessage(messages.addContent) }} </template>
		<template #edit> <EyeIcon /> {{ formatMessage(messages.viewInstance) }} </template>
		<template #duplicate>
			<ClipboardCopyIcon /> {{ formatMessage(messages.duplicateInstance) }}
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
