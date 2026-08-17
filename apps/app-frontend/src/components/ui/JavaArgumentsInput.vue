<script setup lang="ts">
import {
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	ExternalIcon,
	GlobeIcon,
	SparklesIcon,
	XIcon,
} from '@modrinth/assets'
import {
	AutoLink,
	ButtonStyled,
	Collapsible,
	defineMessages,
	NewModal,
	TagItem,
	useVIntl,
} from '@modrinth/ui'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'

import type { GcContext } from '@/helpers/gc/types'
import { resolveAutoGcArgs } from '@/helpers/gc/auto-selector'
import {
	getJavaArgumentPresets,
	getPresetsByGroup,
	type JavaArgumentPreset,
} from '@/helpers/java-argument-presets'

const model = defineModel<string>({ required: true })

const props = withDefaults(
	defineProps<{
		id?: string
		placeholder?: string
		disabled?: boolean
		gcContext?: GcContext
		showAutoDetails?: boolean
	}>(),
	{
		id: undefined,
		placeholder: undefined,
		disabled: false,
		gcContext: undefined,
		showAutoDetails: false,
	},
)

const { formatMessage } = useVIntl()

const messages = defineMessages({
	presetsButton: {
		id: 'app.java-arguments.presets.button',
		defaultMessage: 'Argument presets',
	},
	presetsModalTitle: {
		id: 'app.java-arguments.presets.modal-title',
		defaultMessage: 'Java argument presets',
	},
	usePreset: {
		id: 'app.java-arguments.presets.use',
		defaultMessage: 'Use preset',
	},
	presetApplied: {
		id: 'app.java-arguments.presets.applied',
		defaultMessage: 'Applied',
	},
	removePreset: {
		id: 'app.java-arguments.presets.remove',
		defaultMessage: 'Remove preset',
	},
	presetArguments: {
		id: 'app.java-arguments.presets.arguments',
		defaultMessage: 'Arguments',
	},
	collapseGroup: {
		id: 'app.java-arguments.presets.collapse-group',
		defaultMessage: 'Collapse group',
	},
	expandGroup: {
		id: 'app.java-arguments.presets.expand-group',
		defaultMessage: 'Expand group',
	},
	autoResolved: {
		id: 'app.java-arguments.presets.gc.auto.resolved',
		defaultMessage: 'Resolved → {strategy}',
	},
	autoReasonChain: {
		id: 'app.java-arguments.presets.gc.auto.reason-chain',
		defaultMessage: 'Decision path: {chain}',
	},
})

const presets = computed(() => getJavaArgumentPresets(props.gcContext))
const groupedPresets = computed(() => getPresetsByGroup(presets.value))

const modal = ref<InstanceType<typeof NewModal>>()
const expandedPresetIds = ref(new Set<string>())
const expandedGroupIds = ref(new Set<string>())
const copiedPresetId = ref<string | null>(null)

function getPresetArgs(preset: JavaArgumentPreset): string {
	return preset.resolveArgs ? preset.resolveArgs(props.gcContext) : preset.args
}

function getDisplayArgs(preset: JavaArgumentPreset): string {
	if (preset.id === 'gc-auto' && props.showAutoDetails && props.gcContext) {
		return resolveAutoGcArgs(props.gcContext)
	}
	return getPresetArgs(preset)
}

function getActivePresets(value: string): JavaArgumentPreset[] {
	const trimmed = value.trimStart()
	const matches = presets.value
		.filter((preset) =>
			preset.detect ? preset.detect(trimmed) : trimmed.startsWith(preset.args),
		)
		.sort((a, b) => {
			const aIsAuto = a.id === 'gc-auto' ? 1 : 0
			const bIsAuto = b.id === 'gc-auto' ? 1 : 0
			return aIsAuto - bIsAuto || presets.value.indexOf(a) - presets.value.indexOf(b)
		})
	const result: JavaArgumentPreset[] = []
	const seenGroups = new Set<string>()
	for (const preset of matches) {
		if (seenGroups.has(preset.group)) continue
		seenGroups.add(preset.group)
		result.push(preset)
	}
	return result
}

function removePresetArgs(preset: JavaArgumentPreset, args: string): string {
	const presetArgs = getPresetArgs(preset)
	if (args.startsWith(presetArgs)) {
		return args.slice(presetArgs.length).trimStart()
	}
	if (preset.detect && preset.detect(args)) {
		return args.replace(presetArgs, '').trimStart()
	}
	return args
}

const split = computed(() => {
	const trimmed = model.value.trimStart()
	const active = getActivePresets(trimmed)
	let rest = trimmed
	for (const preset of active) {
		rest = removePresetArgs(preset, rest)
	}
	return { active, rest }
})

const activePresets = computed(() => split.value.active)

const rest = computed<string>({
	get: () => split.value.rest,
	set: (value) => {
		const argsToJoin = activePresets.value.map(getPresetArgs)
		model.value = argsToJoin.length ? argsToJoin.join(' ') + (value ? ` ${value}` : '') : value
	},
})

function onInput(event: Event) {
	rest.value = (event.target as HTMLInputElement).value
}

function removeOtherGroupPresets(preset: JavaArgumentPreset, currentArgs: string): string {
	const groupPresets = presets.value.filter((p) => p.group === preset.group && p.id !== preset.id)
	let result = currentArgs
	for (const other of groupPresets) {
		result = removePresetArgs(other, result)
	}
	return result
}

function applyPreset(preset: JavaArgumentPreset) {
	const argsToApply = getPresetArgs(preset)
	const cleanedRest = removeOtherGroupPresets(preset, split.value.rest)
	model.value = argsToApply + (cleanedRest ? ` ${cleanedRest}` : '')
}

function removePreset(preset: JavaArgumentPreset) {
	model.value = removePresetArgs(preset, model.value.trimStart()).trimStart()
}

function showPresets() {
	modal.value?.show()
}

async function copyPresetArgs(preset: JavaArgumentPreset) {
	const argsToCopy = getDisplayArgs(preset)
	await navigator.clipboard.writeText(argsToCopy)
	copiedPresetId.value = preset.id
	setTimeout(() => {
		if (copiedPresetId.value === preset.id) {
			copiedPresetId.value = null
		}
	}, 1500)
}

function isPresetCollapsed(preset: JavaArgumentPreset) {
	return !expandedPresetIds.value.has(preset.id)
}

function togglePresetCollapsed(preset: JavaArgumentPreset) {
	const next = new Set(expandedPresetIds.value)
	if (next.has(preset.id)) {
		next.delete(preset.id)
	} else {
		next.add(preset.id)
	}
	expandedPresetIds.value = next
}

function isPresetActive(preset: JavaArgumentPreset) {
	return activePresets.value.some((p) => p.id === preset.id)
}

function isGroupCollapsed(group: string) {
	return !expandedGroupIds.value.has(group)
}

function toggleGroupCollapsed(group: string) {
	const next = new Set(expandedGroupIds.value)
	if (next.has(group)) {
		next.delete(group)
	} else {
		next.add(group)
	}
	expandedGroupIds.value = next
}

function getAutoResolvedLabel(preset: JavaArgumentPreset): string | null {
	if (preset.id !== 'gc-auto' || !preset.autoResolvedName) return null
	return formatMessage(messages.autoResolved, { strategy: preset.autoResolvedName })
}

function getAutoReasonChainText(preset: JavaArgumentPreset): string | null {
	if (preset.id !== 'gc-auto' || !preset.autoReasonChain) return null
	return formatMessage(messages.autoReasonChain, { chain: preset.autoReasonChain.join(' → ') })
}

const tagsScrollRef = ref<HTMLElement | null>(null)
const showTagsFade = ref(false)
let tagsResizeObserver: ResizeObserver | null = null

function updateTagsFade() {
	const el = tagsScrollRef.value
	if (!el) return
	showTagsFade.value = el.scrollWidth > el.clientWidth + 1
}

watch(tagsScrollRef, (el) => {
	if (el) {
		updateTagsFade()
		tagsResizeObserver?.disconnect()
		tagsResizeObserver = new ResizeObserver(updateTagsFade)
		tagsResizeObserver.observe(el)
	} else {
		tagsResizeObserver?.disconnect()
		tagsResizeObserver = null
	}
})

watch(activePresets, () => {
	nextTick(updateTagsFade)
})

onBeforeUnmount(() => {
	tagsResizeObserver?.disconnect()
	tagsResizeObserver = null
})
</script>

<template>
	<div class="flex flex-col gap-2">
		<div class="flex items-center gap-2">
			<div
				class="flex min-w-0 flex-1 items-center gap-2 rounded-xl bg-surface-4 px-3 transition-[box-shadow,color] focus-within:ring-4 focus-within:ring-brand-shadow"
				:class="props.disabled ? 'cursor-not-allowed opacity-50' : ''"
			>
				<div
					v-if="activePresets.length"
					ref="tagsScrollRef"
					class="tags-scroll flex min-w-0 max-w-[50%] shrink-0 items-center gap-1 overflow-x-auto"
					:class="{ 'tags-fade-right': showTagsFade }"
					@scroll="updateTagsFade"
				>
					<TagItem
						v-for="preset in activePresets"
						:key="preset.id"
						class="shrink-0"
						:action="props.disabled ? undefined : () => removePreset(preset)"
						:aria-label="formatMessage(messages.removePreset)"
					>
						{{ formatMessage(preset.title) }}
						<XIcon aria-hidden="true" />
					</TagItem>
				</div>
				<input
					:id="props.id"
					:value="rest"
					:disabled="props.disabled"
					:placeholder="props.placeholder"
					class="h-9 min-w-0 flex-1 bg-transparent px-0 py-2 text-base font-medium text-primary placeholder:text-secondary focus:text-contrast focus:shadow-none focus:outline-none"
					autocomplete="off"
					type="text"
					@input="onInput"
				/>
			</div>
			<ButtonStyled type="outlined" class="shrink-0">
				<button type="button" :disabled="props.disabled" @click="showPresets">
					<SparklesIcon aria-hidden="true" />
					{{ formatMessage(messages.presetsButton) }}
				</button>
			</ButtonStyled>
		</div>

		<NewModal
			ref="modal"
			:header="formatMessage(messages.presetsModalTitle)"
			width="min(640px, calc(100vw - 2rem))"
			max-width="640px"
		>
			<div class="flex flex-col gap-6">
				<div
					v-for="groupEntry in groupedPresets"
					:key="groupEntry.group"
					class="flex flex-col gap-3"
				>
					<div
						role="button"
						tabindex="0"
						class="flex w-full cursor-pointer select-none items-center justify-between gap-2"
						:aria-expanded="!isGroupCollapsed(groupEntry.group)"
						:aria-label="
							formatMessage(
								isGroupCollapsed(groupEntry.group)
									? messages.expandGroup
									: messages.collapseGroup,
							)
						"
						@click="toggleGroupCollapsed(groupEntry.group)"
						@keydown.enter="toggleGroupCollapsed(groupEntry.group)"
						@keydown.space.prevent="toggleGroupCollapsed(groupEntry.group)"
					>
						<h3 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(groupEntry.title) }}
						</h3>
						<DropdownIcon
							class="size-4 shrink-0 text-secondary transition-transform"
							:class="{ 'rotate-180': !isGroupCollapsed(groupEntry.group) }"
							aria-hidden="true"
						/>
					</div>

					<Collapsible :collapsed="isGroupCollapsed(groupEntry.group)">
						<div class="flex flex-col gap-3">
							<div
								v-for="preset in groupEntry.presets"
								:key="preset.id"
								class="flex flex-col gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-4"
							>
								<div class="flex items-start gap-3 text-left">
									<GlobeIcon class="mt-0.5 size-6 shrink-0 text-secondary" aria-hidden="true" />
									<div class="min-w-0 flex-1">
										<p class="m-0 text-base font-semibold text-contrast">
											{{ formatMessage(preset.title) }}
										</p>
										<AutoLink
											:to="preset.link"
											target="_blank"
											rel="noreferrer"
											class="inline-flex items-start gap-1 text-sm text-secondary hover:text-brand hover:underline"
										>
											<span class="min-w-0">{{ formatMessage(preset.description) }}</span>
											<ExternalIcon class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
										</AutoLink>
										<p
											v-if="showAutoDetails && getAutoResolvedLabel(preset)"
											class="m-0 mt-2 text-sm font-medium text-brand"
										>
											{{ getAutoResolvedLabel(preset) }}
										</p>
										<p
											v-if="showAutoDetails && getAutoReasonChainText(preset)"
											class="m-0 mt-1 text-xs text-secondary"
										>
											{{ getAutoReasonChainText(preset) }}
										</p>
									</div>
									<ButtonStyled :type="isPresetActive(preset) ? 'standard' : 'outlined'" color="brand">
										<button
											type="button"
											:disabled="isPresetActive(preset)"
											@click="applyPreset(preset)"
										>
											<CheckIcon v-if="isPresetActive(preset)" aria-hidden="true" />
											{{
												formatMessage(
													isPresetActive(preset) ? messages.presetApplied : messages.usePreset,
												)
											}}
										</button>
									</ButtonStyled>
								</div>
								<template v-if="preset.id !== 'gc-auto' || showAutoDetails">
									<div class="flex items-center gap-2">
									<div class="h-px min-w-0 flex-1 bg-surface-4" />
									<button
										v-tooltip="formatMessage(messages.presetArguments)"
										type="button"
										:aria-label="formatMessage(messages.presetArguments)"
										class="flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-full border-none bg-transparent text-secondary transition-colors hover:bg-surface-5 hover:text-contrast"
										@click="togglePresetCollapsed(preset)"
									>
										<DropdownIcon
											class="size-4 transition-transform"
											:class="{ 'rotate-180': !isPresetCollapsed(preset) }"
											aria-hidden="true"
										/>
									</button>
								</div>
								<Collapsible :collapsed="isPresetCollapsed(preset)">
									<div class="flex items-start gap-2">
										<code
											class="min-w-0 flex-1 overflow-x-auto whitespace-pre-wrap break-all text-left font-mono text-xs leading-relaxed text-primary"
										>
											{{ getDisplayArgs(preset) }}
										</code>
										<button
											type="button"
											:aria-label="formatMessage(messages.presetArguments)"
											class="flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-full border-none bg-transparent text-secondary transition-colors hover:bg-surface-5 hover:text-contrast"
											@click="copyPresetArgs(preset)"
										>
											<CheckIcon
												v-if="copiedPresetId === preset.id"
												class="size-4 text-green"
												aria-hidden="true"
											/>
											<CopyIcon v-else class="size-4" aria-hidden="true" />
										</button>
									</div>
								</Collapsible>
								</template>
							</div>
						</div>
					</Collapsible>
				</div>
			</div>
		</NewModal>
	</div>
</template>

<style scoped>
.tags-scroll {
	scrollbar-width: none;
}

.tags-scroll::-webkit-scrollbar {
	display: none;
}

.tags-fade-right {
	mask-image: linear-gradient(
		to right,
		black 0%,
		black calc(100% - 1.25rem),
		transparent 100%
	);
	-webkit-mask-image: linear-gradient(
		to right,
		black 0%,
		black calc(100% - 1.25rem),
		transparent 100%
	);
}
</style>