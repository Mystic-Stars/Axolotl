<script setup lang="ts">
import { CheckIcon, EditIcon, SearchIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, NewModal, StyledInput, useVIntl } from '@modrinth/ui'
import { computed, nextTick, ref, useTemplateRef } from 'vue'

import type { SchematicBlockState } from '@/lab/schematic-preview/backend'
import {
	type LoadedSchematicResources,
	resolveSchematicMaterialTexture,
} from '@/lab/schematic-preview/resources'

import SchematicMaterialSwatch from './SchematicMaterialSwatch.vue'

const props = defineProps<{
	blocks: SchematicBlockState[]
	resources?: LoadedSchematicResources
	selectedCount: number
	displayName: (name: string) => string
}>()

const emit = defineEmits<{
	replace: [state: SchematicBlockState]
}>()

const { formatMessage, locale } = useVIntl()
const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')
const searchInput = useTemplateRef<InstanceType<typeof StyledInput>>('searchInput')
const search = ref('')
const selectedName = ref('')

const messages = defineMessages({
	title: {
		id: 'app.lab.schematic-preview.block-picker.title',
		defaultMessage: 'Replace blocks',
	},
	description: {
		id: 'app.lab.schematic-preview.block-picker.description',
		defaultMessage: 'Choose the block that will replace the {count} selected blocks.',
	},
	search: {
		id: 'app.lab.schematic-preview.block-picker.search',
		defaultMessage: 'Search blocks by name or ID',
	},
	results: {
		id: 'app.lab.schematic-preview.block-picker.results',
		defaultMessage: '{count} blocks',
	},
	empty: {
		id: 'app.lab.schematic-preview.block-picker.empty',
		defaultMessage: 'No matching blocks',
	},
	cancel: { id: 'app.lab.schematic-preview.block-picker.cancel', defaultMessage: 'Cancel' },
	confirm: {
		id: 'app.lab.schematic-preview.block-picker.confirm',
		defaultMessage: 'Replace {count} blocks',
	},
})

const visibleBlocks = computed(() => {
	const query = search.value.trim().toLocaleLowerCase(locale.value)
	return props.blocks
		.filter((block) => {
			if (!query) return true
			return [props.displayName(block.name), block.name].some((value) =>
				value.toLocaleLowerCase(locale.value).includes(query),
			)
		})
		.sort((left, right) =>
			props
				.displayName(left.name)
				.localeCompare(props.displayName(right.name), locale.value, { sensitivity: 'base' }),
		)
})

const selectedBlock = computed(() =>
	props.blocks.find((block) => block.name === selectedName.value),
)

function textureUv(name: string) {
	return props.resources
		? resolveSchematicMaterialTexture(name, props.resources.previewResources)
		: undefined
}

function fallbackColor(name: string) {
	let hash = 0
	for (const character of name) hash = (hash * 31 + character.charCodeAt(0)) | 0
	return `hsl(${Math.abs(hash) % 360} 42% 48%)`
}

async function show(preferredName?: string) {
	search.value = ''
	selectedName.value = props.blocks.some((block) => block.name === preferredName)
		? (preferredName ?? '')
		: ''
	modal.value?.show()
	await nextTick()
	searchInput.value?.focus()
}

function confirm() {
	if (!selectedBlock.value) return
	emit('replace', selectedBlock.value)
	modal.value?.hide()
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		width="min(760px, calc(100vw - 2rem))"
		max-width="760px"
		scrollable
		max-content-height="min(44rem, 76vh)"
	>
		<div class="flex min-h-[28rem] min-w-0 flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.description, { count: selectedCount }) }}
			</p>

			<div class="flex items-center gap-3">
				<StyledInput
					ref="searchInput"
					v-model="search"
					class="min-w-0 flex-1"
					:icon="SearchIcon"
					type="search"
					:placeholder="formatMessage(messages.search)"
					clearable
				/>
				<span class="shrink-0 text-xs tabular-nums text-secondary">
					{{ formatMessage(messages.results, { count: visibleBlocks.length }) }}
				</span>
			</div>

			<p
				v-if="visibleBlocks.length === 0"
				class="m-0 flex flex-1 items-center justify-center text-sm text-secondary"
			>
				{{ formatMessage(messages.empty) }}
			</p>
			<div v-else class="block-picker-grid grid grid-cols-4 gap-2" role="listbox">
				<button
					v-for="block in visibleBlocks"
					:key="block.name"
					type="button"
					class="block-picker-option"
					:class="{ 'block-picker-option-selected': selectedName === block.name }"
					:aria-selected="selectedName === block.name"
					:title="`${displayName(block.name)}\n${block.name}`"
					role="option"
					@click="selectedName = block.name"
					@dblclick="confirm"
				>
					<SchematicMaterialSwatch
						v-if="resources"
						class="block-picker-swatch"
						:atlas="resources.atlas"
						:uv="textureUv(block.name)"
						:fallback-color="fallbackColor(block.name)"
						:state="block"
						:resources="resources"
					/>
					<span v-else class="block-picker-swatch bg-surface-4"></span>
					<span class="block-picker-copy">
						<strong>{{ displayName(block.name) }}</strong>
						<small>{{ block.name }}</small>
					</span>
					<span v-if="selectedName === block.name" class="block-picker-check">
						<CheckIcon />
					</span>
				</button>
			</div>
		</div>

		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<ButtonStyled type="transparent">
					<button type="button" @click="modal?.hide()">
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button type="button" :disabled="!selectedBlock" @click="confirm">
						<EditIcon />{{ formatMessage(messages.confirm, { count: selectedCount }) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<style scoped>
.block-picker-option {
	position: relative;
	display: grid;
	min-width: 0;
	min-height: 4.25rem;
	cursor: pointer;
	grid-template-columns: 2.5rem minmax(0, 1fr);
	align-items: center;
	gap: 0.625rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-md);
	padding: 0.625rem;
	background: var(--surface-2);
	color: var(--color-text-dark);
	font: inherit;
	text-align: left;
	transition:
		border-color 120ms ease,
		background-color 120ms ease;
	content-visibility: auto;
	contain-intrinsic-size: 4.25rem;
}

.block-picker-option:hover {
	border-color: var(--surface-5);
	background: var(--color-button-bg);
}

.block-picker-option-selected {
	border-color: var(--color-brand);
	background: color-mix(in srgb, var(--color-brand) 10%, var(--surface-2));
}

.block-picker-swatch {
	width: 2.5rem;
	height: 2.5rem;
	border: 1px solid rgb(255 255 255 / 14%);
	border-radius: var(--radius-sm);
	image-rendering: pixelated;
}

.block-picker-copy {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: 0.15rem;
}

.block-picker-copy strong,
.block-picker-copy small {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.block-picker-copy strong {
	font-size: 0.78rem;
}

.block-picker-copy small {
	color: var(--color-text-secondary);
	font-size: 0.62rem;
}

.block-picker-check {
	position: absolute;
	top: 0.35rem;
	right: 0.35rem;
	display: grid;
	width: 1.1rem;
	height: 1.1rem;
	place-items: center;
	border-radius: 50%;
	background: var(--color-brand);
	color: var(--color-brand-inverted);
}

.block-picker-check svg {
	width: 0.75rem;
	height: 0.75rem;
}

@media (max-width: 680px) {
	.block-picker-grid {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}
}

@media (max-width: 420px) {
	.block-picker-grid {
		grid-template-columns: minmax(0, 1fr);
	}
}
</style>
