<template>
	<div
		ref="treeRef"
		role="tree"
		:aria-label="formatMessage(messages.treeLabel)"
		class="max-h-[18rem] overflow-y-auto rounded-lg border border-solid border-surface-4 bg-surface-2 p-1"
		@keydown="handleKeydown"
	>
		<div
			v-for="(row, index) in visibleRows"
			:key="row.id"
			:ref="(element) => setRowRef(row.id, element)"
			role="treeitem"
			:tabindex="index === activeIndex ? 0 : -1"
			:aria-level="row.level + 1"
			:aria-expanded="row.expandable ? row.expanded : undefined"
			:aria-selected="row.kind === 'item' ? row.selected : undefined"
			:aria-disabled="row.disabled ? true : undefined"
			class="flex cursor-pointer items-center gap-2 rounded-md py-2 pr-2 text-sm transition-colors duration-150"
			:class="[
				row.disabled ? 'cursor-not-allowed opacity-50' : 'hover:bg-surface-3',
				row.selected
					? 'bg-brand-highlight text-[var(--color-text-primary)]'
					: 'text-[var(--color-text-default)]',
				row.kind === 'item' ? '' : 'font-semibold text-[var(--color-text-primary)]',
			]"
			:style="{ paddingLeft: `${row.level * 1.25 + 0.5}rem` }"
			@click="activateRow(row)"
			@focus="activeIndex = index"
		>
			<ChevronRightIcon
				v-if="row.expandable"
				class="h-4 w-4 shrink-0 text-[var(--color-text-tertiary)] transition-transform duration-150"
				:class="{ 'rotate-90': row.expanded }"
				aria-hidden="true"
			/>
			<span v-else class="w-4 shrink-0" aria-hidden="true" />
			<span class="min-w-0 flex-1 truncate">{{ row.label }}</span>
			<span v-if="row.disabledReason" class="shrink-0 text-xs text-[var(--color-text-tertiary)]">
				{{ row.disabledReason }}
			</span>
		</div>
	</div>
</template>

<script setup lang="ts">
import { ChevronRightIcon } from '@modrinth/assets'
import { computed, nextTick, ref } from 'vue'

import { defineMessages, useVIntl } from '../../composables/i18n'
import { type KeyBinding, type KeyBindingDevice, keyCodeLabel } from '../../utils/keybinding'
import { KEYBINDING_CATALOG, type KeybindingCatalogItem } from '../../utils/keybinding-catalog'

const props = withDefaults(
	defineProps<{
		/** The combination being assembled, used to mark the chosen input. */
		binding: KeyBinding | null
		/** Reasons an input may not be used, keyed by code. */
		disabledCodes?: Map<string, string>
	}>(),
	{ disabledCodes: () => new Map() },
)

const emit = defineEmits<{
	select: [{ device: KeyBindingDevice; code: string }]
	escape: []
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	treeLabel: {
		id: 'ui.keybinding-modal.tree-label',
		defaultMessage: 'Inputs a shortcut can use',
	},
})

interface TreeRow {
	id: string
	kind: 'device' | 'group' | 'item'
	level: number
	label: string
	expandable: boolean
	expanded: boolean
	selected: boolean
	disabled: boolean
	disabledReason?: string
	item?: KeybindingCatalogItem
}

const expanded = ref(new Set<string>(['keyboard']))
const activeIndex = ref(0)
const rowRefs = new Map<string, HTMLElement>()

function groupRowId(deviceId: string, groupId: string) {
	return `${deviceId}:${groupId}`
}

function itemRowId(deviceId: string, groupId: string, code: string) {
	return `${deviceId}:${groupId}:${code}`
}

function itemLabel(item: KeybindingCatalogItem) {
	return item.label ? formatMessage(item.label) : keyCodeLabel(item.code)
}

function itemState(item: KeybindingCatalogItem) {
	const reason = props.disabledCodes.get(item.code)
	return { disabled: Boolean(reason), disabledReason: reason }
}

const visibleRows = computed<TreeRow[]>(() => {
	const rows: TreeRow[] = []

	for (const device of KEYBINDING_CATALOG) {
		const deviceExpanded = expanded.value.has(device.id)
		rows.push({
			id: device.id,
			kind: 'device',
			level: 0,
			label: formatMessage(device.label),
			expandable: true,
			expanded: deviceExpanded,
			selected: false,
			disabled: false,
		})
		if (!deviceExpanded) continue

		for (const group of device.groups) {
			const rowId = groupRowId(device.id, group.id)
			const groupExpanded = expanded.value.has(rowId)
			rows.push({
				id: rowId,
				kind: 'group',
				level: 1,
				label: formatMessage(group.label),
				expandable: true,
				expanded: groupExpanded,
				selected: false,
				disabled: false,
			})
			if (!groupExpanded) continue

			for (const item of group.items) {
				const state = itemState(item)
				rows.push({
					id: itemRowId(device.id, group.id, item.code),
					kind: 'item',
					level: 2,
					label: itemLabel(item),
					expandable: false,
					expanded: false,
					selected: props.binding?.device === item.device && props.binding?.code === item.code,
					item,
					...state,
				})
			}
		}
	}

	return rows
})

function setRowRef(id: string, element: unknown) {
	if (element instanceof HTMLElement) rowRefs.set(id, element)
	else rowRefs.delete(id)
}

function focusRow(index: number) {
	const row = visibleRows.value[index]
	if (!row) return
	activeIndex.value = index
	void nextTick(() => rowRefs.get(row.id)?.focus())
}

function toggle(row: TreeRow) {
	if (!row.expandable) return
	const next = new Set(expanded.value)
	if (next.has(row.id)) next.delete(row.id)
	else next.add(row.id)
	expanded.value = next

	// Collapsing can leave the active row past the end, which would stop the
	// keyboard from moving anywhere.
	const lastIndex = visibleRows.value.length - 1
	if (activeIndex.value > lastIndex) focusRow(lastIndex)
}

function parentIndexOf(row: TreeRow) {
	for (let index = visibleRows.value.indexOf(row) - 1; index >= 0; index -= 1) {
		if (visibleRows.value[index].level < row.level) return index
	}
	return -1
}

function activateRow(row: TreeRow) {
	if (row.kind === 'item') {
		if (row.disabled || !row.item) return
		emit('select', { device: row.item.device, code: row.item.code })
		return
	}
	toggle(row)
}

function handleKeydown(event: KeyboardEvent) {
	const row = visibleRows.value[activeIndex.value]
	if (!row) return

	switch (event.key) {
		case 'ArrowDown':
			event.preventDefault()
			focusRow(Math.min(activeIndex.value + 1, visibleRows.value.length - 1))
			break
		case 'ArrowUp':
			event.preventDefault()
			focusRow(Math.max(activeIndex.value - 1, 0))
			break
		case 'ArrowRight':
			event.preventDefault()
			if (row.expandable && !row.expanded) toggle(row)
			else if (row.expandable) focusRow(activeIndex.value + 1)
			break
		case 'ArrowLeft': {
			event.preventDefault()
			if (row.expandable && row.expanded) {
				toggle(row)
				break
			}
			const parent = parentIndexOf(row)
			if (parent !== -1) focusRow(parent)
			break
		}
		case 'Home':
			event.preventDefault()
			focusRow(0)
			break
		case 'End':
			event.preventDefault()
			focusRow(visibleRows.value.length - 1)
			break
		case 'Enter':
		case ' ':
			event.preventDefault()
			activateRow(row)
			break
		case 'Escape':
			// Handled here so escape leaves the picker instead of the whole dialog.
			event.preventDefault()
			event.stopPropagation()
			emit('escape')
			break
		default:
			break
	}
}
</script>
