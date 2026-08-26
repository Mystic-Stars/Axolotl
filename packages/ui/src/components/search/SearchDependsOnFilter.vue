<template>
	<div :class="innerPanelClass" class="flex flex-col gap-3">
		<ProjectCombobox
			v-if="isModpack"
			v-model="pendingProjectId"
			:project-types="selectableProjectTypes"
			:exclude-project-ids="dependencyProjectIds"
			:search-placeholder="formatMessage(messages.searchContentPlaceholder)"
			:show-chevron="false"
			:sync-with-selection="false"
			:show-search-icon="true"
			@update:model-value="addIncludedProject"
		/>
		<ProjectCombobox
			v-else-if="!showSelectedProject"
			ref="projectCombobox"
			:model-value="selectedProjectId"
			:project-types="selectableProjectTypes"
			:search-placeholder="formatMessage(messages.searchProjectPlaceholder)"
			:show-chevron="false"
			:sync-with-selection="false"
			:show-search-icon="true"
			@update:model-value="setSelectedProjectId"
		/>
		<div v-if="isModpack" class="flex flex-col gap-1">
			<div
				v-for="{ projectId, project } in loadedDependentProjects"
				:key="projectId"
				class="group flex min-w-0 items-center gap-2 rounded-xl px-1 py-1"
			>
				<img
					v-if="project.icon_url"
					:src="project.icon_url"
					:alt="project.title"
					class="size-6 shrink-0 rounded-md object-cover"
				/>
				<PackageIcon v-else class="size-6 shrink-0 text-secondary" />
				<span class="min-w-0 flex-1 truncate font-medium text-contrast">{{ project.title }}</span>
				<button
					type="button"
					class="rounded border-0 bg-transparent p-1 text-secondary hover:text-red"
					:aria-label="formatMessage(messages.removeIncludedProjectTooltip)"
					@click="removeIncludedProject(projectId)"
				>
					<XIcon class="size-4" />
				</button>
			</div>
		</div>
		<div
			v-else-if="showSelectedProject"
			:class="selectedProjectClass ?? 'bg-surface-2'"
			class="flex items-center gap-2 rounded-2xl p-2.5"
		>
			<img
				v-if="selectedProject?.icon_url"
				:src="selectedProject.icon_url"
				:alt="selectedProject.title"
				class="size-12 shrink-0 rounded-xl object-cover"
			/>
			<PackageIcon v-else class="size-12 shrink-0 text-secondary" />
			<div class="min-w-0 flex-1">
				<div class="truncate text-base font-bold text-contrast">
					{{ selectedProject?.title ?? selectedProjectId }}
				</div>
				<MultiSelect
					:model-value="draftDependencyTypes"
					:options="dependencyTypeOptions"
					:clearable="false"
					fit-content
					@open="resetDraftDependencyTypes"
					@close="commitDependencyTypes"
					@update:model-value="setDraftDependencyTypes"
				/>
			</div>
			<button
				type="button"
				class="rounded border-0 bg-transparent p-1 text-secondary hover:text-contrast"
				@click="setSelectedProjectId(undefined)"
			>
				<XIcon class="size-5" />
			</button>
		</div>
	</div>
</template>

<script setup lang="ts">
import { PackageIcon, XIcon } from '@modrinth/assets'
import { useQuery } from '@tanstack/vue-query'
import { computed, ref, watch } from 'vue'

import { defineMessages, useVIntl } from '../../composables/i18n'
import { injectModrinthClient } from '../../providers'
import {
	type DependencyType,
	type FilterValue,
	formatDependencyProjectFilterOption,
	parseDependencyProjectFilterOption,
} from '../../utils/search'
import MultiSelect, { type MultiSelectOption } from '../base/MultiSelect.vue'
import ProjectCombobox, { type ProjectType, type SearchHit } from '../project/ProjectCombobox.vue'

const FILTER_TYPE_ID = 'compatible_dependency_project_ids'
const props = defineProps<{
	projectType: string
	innerPanelClass?: string
	selectedProjectClass?: string
}>()
const selectedFilters = defineModel<FilterValue[]>('selectedFilters', { required: true })
const { formatMessage } = useVIntl()
const { labrinth } = injectModrinthClient()
const selectableProjectTypes: ProjectType[] = [
	'mod',
	'resourcepack',
	'shader',
	'datapack',
	'plugin',
]
const isModpack = computed(() => props.projectType === 'modpack')
const dependencyProjectIds = computed(() => [
	...new Set(
		selectedFilters.value
			.filter((f) => f.type === FILTER_TYPE_ID)
			.map((f) => parseDependencyProjectFilterOption(f.option).projectId),
	),
])
const pendingProjectId = ref<string>()
const selectedDependencyFilter = computed(() =>
	selectedFilters.value.find((f) => f.type === FILTER_TYPE_ID),
)
const selectedProjectId = computed(() =>
	selectedDependencyFilter.value
		? parseDependencyProjectFilterOption(selectedDependencyFilter.value.option).projectId
		: undefined,
)
const dependencyTypes = computed<DependencyType[]>(() =>
	selectedDependencyFilter.value
		? parseDependencyProjectFilterOption(selectedDependencyFilter.value.option).dependencyTypes
		: ['required'],
)
const draftDependencyTypes = ref<DependencyType[]>([...dependencyTypes.value])
const projectCombobox = ref<{ selectedProject: SearchHit | null } | null>(null)
const selectedProject = computed(() => projectCombobox.value?.selectedProject ?? null)
const showSelectedProject = computed(() =>
	Boolean(selectedProjectId.value && selectedProject.value),
)
watch(dependencyTypes, (types) => (draftDependencyTypes.value = [...types]), { immediate: true })
const { data: dependentProjects } = useQuery({
	queryKey: computed(() => ['search-depends-on-filter', dependencyProjectIds.value]),
	queryFn: () => labrinth.projects_v2.getMultiple(dependencyProjectIds.value),
	enabled: computed(() => isModpack.value && dependencyProjectIds.value.length > 0),
	placeholderData: (previous) => previous,
	refetchOnWindowFocus: false,
})
const dependentProjectMap = computed(
	() => new Map((dependentProjects.value ?? []).map((p) => [p.id, p])),
)
const loadedDependentProjects = computed(() =>
	dependencyProjectIds.value.flatMap((projectId) => {
		const project = dependentProjectMap.value.get(projectId)
		return project ? [{ projectId, project }] : []
	}),
)
const dependencyTypeOptions = computed<MultiSelectOption<DependencyType>[]>(() => [
	{ value: 'required', label: formatMessage(messages.required) },
	{ value: 'optional', label: formatMessage(messages.optional) },
	{ value: 'embedded', label: formatMessage(messages.embedded) },
])
function setSelectedProjectId(projectId: string | undefined) {
	const other = selectedFilters.value.filter((f) => f.type !== FILTER_TYPE_ID)
	selectedFilters.value = projectId
		? [
				...other,
				{
					type: FILTER_TYPE_ID,
					option: formatDependencyProjectFilterOption(projectId, ['required']),
				},
			]
		: other
}
function addIncludedProject(projectId: string | undefined) {
	if (projectId && !dependencyProjectIds.value.includes(projectId))
		selectedFilters.value = [...selectedFilters.value, { type: FILTER_TYPE_ID, option: projectId }]
	pendingProjectId.value = undefined
}
function removeIncludedProject(projectId: string) {
	selectedFilters.value = selectedFilters.value.filter(
		(f) =>
			f.type !== FILTER_TYPE_ID ||
			parseDependencyProjectFilterOption(f.option).projectId !== projectId,
	)
}
function resetDraftDependencyTypes() {
	draftDependencyTypes.value = [...dependencyTypes.value]
}
function setDraftDependencyTypes(types: DependencyType[]) {
	draftDependencyTypes.value = types
}
function commitDependencyTypes() {
	if (!selectedProjectId.value) return
	const types =
		draftDependencyTypes.value.length > 0
			? draftDependencyTypes.value
			: (['required'] as DependencyType[])
	selectedFilters.value = selectedFilters.value.map((f) =>
		f.type === FILTER_TYPE_ID
			? { ...f, option: formatDependencyProjectFilterOption(selectedProjectId.value!, types) }
			: f,
	)
}
const messages = defineMessages({
	searchContentPlaceholder: {
		id: 'search.filter.included_content.search_placeholder',
		defaultMessage: 'Search content...',
	},
	searchProjectPlaceholder: {
		id: 'search.filter.dependent_project.search_placeholder',
		defaultMessage: 'Search for a project...',
	},
	removeIncludedProjectTooltip: {
		id: 'search.filter.included_content.remove_project.tooltip',
		defaultMessage: 'Remove project',
	},
	required: { id: 'search.filter.dependency_type.required', defaultMessage: 'Required' },
	optional: { id: 'search.filter.dependency_type.optional', defaultMessage: 'Optional' },
	embedded: { id: 'search.filter.dependency_type.embedded', defaultMessage: 'Embedded' },
})
</script>
