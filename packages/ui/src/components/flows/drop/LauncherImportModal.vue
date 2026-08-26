<template>
	<NewModal ref="modal" max-width="560px" :closable="true" @hide="onHide">
		<template #title>
			<span class="text-contrast">{{
				formatMessage(messages.title, { launcherName: primaryLauncherName })
			}}</span>
		</template>

		<div class="flex flex-col gap-4">
			<!-- Subtitle -->
			<span class="text-secondary text-sm">
				{{
					formatMessage(messages.subtitle, {
						basePath: primaryBasePath,
						count: allInstances.length,
					})
				}}
			</span>

			<!-- Select / Deselect all -->
			<div class="flex items-center gap-4 text-sm">
				<button
					class="text-brand hover:text-brand-hover cursor-pointer transition-colors"
					@click="selectAll"
				>
					{{ formatMessage(messages.selectAll) }}
				</button>
				<button
					class="text-brand hover:text-brand-hover cursor-pointer transition-colors"
					@click="deselectAll"
				>
					{{ formatMessage(messages.deselectAll) }}
				</button>
				<span class="text-secondary ml-auto">{{
					formatMessage(messages.selected, { n: selectedCount })
				}}</span>
			</div>

			<!-- Instance list grouped by launcher -->
			<div class="flex flex-col gap-3 max-h-[360px] overflow-y-auto">
				<div v-for="group in internalResults" :key="group.launcherType" class="flex flex-col gap-1">
					<span class="text-xs font-semibold text-secondary uppercase tracking-wide px-1">
						{{ group.launcherName }}
					</span>
					<div class="flex flex-col gap-1">
						<InstanceRowCard
							v-for="inst in group.instances"
							:key="`${group.launcherType}:${inst.name}`"
							:name="inst.name"
							:version="inst.version"
							:loader="inst.loader"
							@select="toggleInstance(group.launcherType, group.launcherName, inst)"
						>
							<template #prepend>
								<Checkbox
									:model-value="isSelected(group.launcherType, inst.name)"
									@update:model-value="toggleInstance(group.launcherType, group.launcherName, inst)"
									@click.stop
								/>
							</template>
						</InstanceRowCard>
					</div>
				</div>
			</div>

			<div
				v-if="allInstances.length === 0"
				class="flex items-center justify-center py-8 text-secondary"
			>
				<span class="text-sm">{{ formatMessage(messages.noInstances) }}</span>
			</div>
		</div>

		<template #actions>
			<div class="flex w-full items-center justify-between">
				<ButtonStyled type="transparent">
					<button type="button" @click="emit('cancel')">
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
				<ButtonStyled :disabled="selectedCount === 0">
					<button class="flex items-center gap-2" @click="handleConfirm">
						<DownloadIcon class="size-4" />
						{{ formatMessage(messages.importAction, { n: selectedCount }) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { DownloadIcon } from '@modrinth/assets'
import { computed, ref } from 'vue'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import Checkbox from '#ui/components/base/Checkbox.vue'
import InstanceRowCard from '#ui/components/base/InstanceRowCard.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'drop.launcher_import.title',
		defaultMessage: 'Import instances from {launcherName}',
	},
	subtitle: {
		id: 'drop.launcher_import.subtitle',
		defaultMessage: 'Scanned {count} instances in {basePath}',
	},
	selectAll: {
		id: 'drop.launcher_import.select_all',
		defaultMessage: 'Select All',
	},
	deselectAll: {
		id: 'drop.launcher_import.deselect_all',
		defaultMessage: 'Deselect All',
	},
	selected: {
		id: 'drop.launcher_import.selected',
		defaultMessage: '{n} selected',
	},
	cancel: {
		id: 'drop.launcher_import.cancel',
		defaultMessage: 'Cancel',
	},
	importAction: {
		id: 'drop.launcher_import.import',
		defaultMessage: 'Import ({n})',
	},
	noInstances: {
		id: 'drop.launcher_import.no_instances',
		defaultMessage: 'No instances found',
	},
})

export interface InstanceInfo {
	name: string
	path: string
	version: string
	loader: string
	compatibleMode?: boolean
	versionPath?: string
}

export interface LauncherInfo {
	launcherName: string
	launcherType: string
	instances: InstanceInfo[]
}

export interface SelectionEntry {
	launcherType: string
	launcherName: string
	instances: Array<{ name: string; path: string; compatibleMode?: boolean; versionPath?: string }>
}

defineProps<{
	results?: LauncherInfo[]
}>()

const emit = defineEmits<{
	(e: 'confirm', selections: SelectionEntry[]): void
	(e: 'cancel'): void
}>()

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const internalResults = ref<LauncherInfo[]>([])
const selectedMap = ref<Map<string, Set<string>>>(new Map())
const isConfirming = ref(false)

const allInstances = computed(() => {
	const items: Array<{
		launcherType: string
		launcherName: string
		name: string
		path: string
		compatibleMode?: boolean
		versionPath?: string
	}> = []
	for (const group of internalResults.value) {
		for (const inst of group.instances) {
			items.push({
				launcherType: group.launcherType,
				launcherName: group.launcherName,
				name: inst.name,
				path: inst.path,
				compatibleMode: inst.compatibleMode,
				versionPath: inst.versionPath,
			})
		}
	}
	return items
})

const primaryLauncherName = computed(() => {
	return internalResults.value[0]?.launcherName ?? ''
})

const primaryBasePath = computed(() => {
	// Cannot know the base path from the LauncherInfo structure,
	// but we show the launcher name which is sufficient
	return internalResults.value[0]?.launcherName ?? ''
})

const selectedCount = computed(() => {
	let count = 0
	for (const [, names] of selectedMap.value) {
		count += names.size
	}
	return count
})

function isSelected(launcherType: string, name: string): boolean {
	return selectedMap.value.get(launcherType)?.has(name) ?? false
}

function onHide() {
	if (isConfirming.value) {
		isConfirming.value = false
		return
	}
	emit('cancel')
}

function toggleInstance(launcherType: string, launcherName: string, inst: InstanceInfo) {
	const set = selectedMap.value.get(launcherType)
	if (!set) {
		selectedMap.value.set(launcherType, new Set([inst.name]))
	} else if (set.has(inst.name)) {
		set.delete(inst.name)
		if (set.size === 0) selectedMap.value.delete(launcherType)
	} else {
		set.add(inst.name)
	}
	selectedMap.value = new Map(selectedMap.value)
}

function selectAll() {
	const map = new Map<string, Set<string>>()
	for (const group of internalResults.value) {
		const names = new Set(group.instances.map((i) => i.name))
		map.set(group.launcherType, names)
	}
	selectedMap.value = map
}

function deselectAll() {
	selectedMap.value = new Map()
}

function handleConfirm() {
	if (selectedCount.value === 0) return
	isConfirming.value = true
	const selections: SelectionEntry[] = []
	for (const [launcherType, names] of selectedMap.value) {
		const group = internalResults.value.find((g) => g.launcherType === launcherType)
		if (!group) continue
		const instances = group.instances
			.filter((i) => names.has(i.name))
			.map((i) => ({
				name: i.name,
				path: i.path,
				compatibleMode: i.compatibleMode,
				versionPath: i.versionPath,
			}))
		if (instances.length > 0) {
			selections.push({ launcherType, launcherName: group.launcherName, instances })
		}
	}
	modal.value?.hide()
	setTimeout(() => {
		emit('confirm', selections)
	}, 0)
}

function show(results: LauncherInfo[]) {
	internalResults.value = results
	deselectAll()

	// If only one instance total, auto-confirm
	const allItems = allInstances.value
	if (allItems.length === 1) {
		const item = allItems[0]
		emit('confirm', [
			{
				launcherType: item.launcherType,
				launcherName: item.launcherName,
				instances: [
					{
						name: item.name,
						path: item.path,
						compatibleMode: item.compatibleMode,
						versionPath: item.versionPath,
					},
				],
			},
		])
		return
	}

	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

defineExpose({ show, hide })
</script>
