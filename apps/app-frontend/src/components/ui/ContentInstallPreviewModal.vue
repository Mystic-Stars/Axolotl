<script setup lang="ts">
import { XIcon } from '@modrinth/assets'
import { Avatar, ButtonStyled, Checkbox, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import { getActiveDependencyConflictIdentities } from '@/providers/content-selection-logic'

export interface ContentInstallPreviewDependency {
	id: string
	title: string
	iconUrl?: string | null
	versionNumber?: string
	fileName?: string
	requiredBy: string[]
	alreadyInstalled: boolean
	versionMismatch?: boolean
	selectionReason?: string
	required?: boolean
	requiredByKeys?: string[]
	requiredForKeys?: string[]
}

export interface ContentInstallPreviewSkipped {
	id: string
	title: string
	reason: string
	requiredByKeys?: string[]
}

export interface ContentInstallPreviewData {
	primary?: ContentInstallPreviewPrimary
	primaries?: ContentInstallPreviewPrimary[]
	instanceName: string
	installDependencies: boolean
	dependencies: ContentInstallPreviewDependency[]
	skipped: ContentInstallPreviewSkipped[]
}

export interface ContentInstallPreviewPrimary {
	key?: string
	title: string
	iconUrl?: string | null
	versionNumber?: string
	provider?: string
	contentType?: string
	error?: string
	conflictIdentities?: string[]
	removable?: boolean
}

export interface ContentInstallBatchPreviewResult {
	approvedIds: string[]
	primaryKeys: string[]
}

export interface ContentInstallConflictPrompt {
	candidate: {
		title: string
		provider: string
		contentType: string
		iconUrl?: string | null
	}
	existing: Array<{
		title: string
		provider: string
		fileName?: string
	}>
	source: 'heuristic'
	confidence: 'high' | 'possible'
}

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: {
		id: 'app.content-install.preview.header',
		defaultMessage: 'Confirm installation',
	},
	description: {
		id: 'app.content-install.preview.description',
		defaultMessage:
			'{count, plural, one {# dependency will be installed automatically} other {# dependencies will be installed automatically}} for {project} in {instance}.',
	},
	batchDescription: {
		id: 'app.content-install.preview.batch-description',
		defaultMessage:
			'Review {projectCount, plural, one {# project} other {# projects}} and {dependencyCount, plural, one {# dependency} other {# dependencies}} for {instance}.',
	},
	selectedContentHeader: {
		id: 'app.content-install.preview.selected-content-header',
		defaultMessage: 'Selected content',
	},
	removeProject: {
		id: 'app.content-install.preview.remove-project',
		defaultMessage: 'Remove {project} from this installation',
	},
	dependenciesHeader: {
		id: 'app.content-install.preview.dependencies-header',
		defaultMessage: 'Dependencies',
	},
	dependenciesCount: {
		id: 'app.content-install.preview.dependencies-count',
		defaultMessage: '{count, plural, one {# dependency} other {# dependencies}}',
	},
	requiredBy: {
		id: 'app.content-install.preview.required-by',
		defaultMessage: 'Required by {projects}',
	},
	alreadyInstalled: {
		id: 'app.content-install.preview.already-installed',
		defaultMessage: 'Already installed',
	},
	versionMismatch: {
		id: 'app.content-install.preview.version-mismatch',
		defaultMessage: 'Version may not match this instance',
	},
	skippedHeader: {
		id: 'app.content-install.preview.skipped-header',
		defaultMessage: 'Skipped',
	},
	onlyChecked: {
		id: 'app.content-install.preview.only-checked',
		defaultMessage: 'Only checked dependencies will be installed.',
	},
	selectAll: {
		id: 'app.content-install.preview.select-all',
		defaultMessage: 'Select all',
	},
	clearAll: {
		id: 'app.content-install.preview.clear-all',
		defaultMessage: 'Clear all',
	},
	cancel: {
		id: 'app.content-install.preview.cancel',
		defaultMessage: 'Cancel',
	},
	install: {
		id: 'app.content-install.preview.install',
		defaultMessage: 'Install',
	},
	installResolved: {
		id: 'app.content-install.preview.install-resolved',
		defaultMessage: 'Install resolved content',
	},
	conflictHeader: {
		id: 'app.content-install.preview.conflict-header',
		defaultMessage: 'Possible duplicate content',
	},
	conflictDescription: {
		id: 'app.content-install.preview.conflict-description',
		defaultMessage:
			'{candidate} may be the same content as an installed or selected project. Continue anyway?',
	},
	continueAnyway: {
		id: 'app.content-install.preview.continue-anyway',
		defaultMessage: 'Install anyway',
	},
	existingContent: {
		id: 'app.content-install.preview.existing-content',
		defaultMessage: 'Existing content',
	},
})

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const data = ref<ContentInstallPreviewData | null>(null)
const selectedIds = ref<Set<string>>(new Set())
const removedPrimaryKeys = ref<Set<string>>(new Set())
let settled = false
let batchMode = false
let conflictMode = false
const conflictPrompt = ref<ContentInstallConflictPrompt | null>(null)
let resolveShow:
	| ((result: string[] | ContentInstallBatchPreviewResult | boolean | null) => void)
	| null = null

const primaries = computed(() => {
	if (!data.value) return []
	if (data.value.primaries?.length) return data.value.primaries
	return data.value.primary ? [data.value.primary] : []
})
const visiblePrimaries = computed(() =>
	primaries.value.filter((primary) => !primary.key || !removedPrimaryKeys.value.has(primary.key)),
)
const visiblePrimaryKeys = computed(() =>
	visiblePrimaries.value.map((primary) => primary.key).filter((key): key is string => !!key),
)
const visiblePrimaryKeySet = computed(() => new Set(visiblePrimaryKeys.value))
const visibleDependencies = computed(
	() =>
		data.value?.dependencies.filter(
			(dependency) =>
				!dependency.requiredByKeys?.length ||
				dependency.requiredByKeys.some((key) => visiblePrimaryKeySet.value.has(key)),
		) ?? [],
)
const activeConflictIdentities = computed(() =>
	getActiveDependencyConflictIdentities(data.value?.dependencies ?? [], visiblePrimaryKeySet.value),
)
function primaryError(primary: ContentInstallPreviewPrimary) {
	if (!primary.error) return null
	if (!primary.conflictIdentities?.length) return primary.error
	return primary.conflictIdentities.some((identity) => activeConflictIdentities.value.has(identity))
		? primary.error
		: null
}
function dependencyRequired(dependency: ContentInstallPreviewDependency) {
	if (!dependency.required) return false
	return (
		!dependency.requiredForKeys?.length ||
		dependency.requiredForKeys.some((key) => visiblePrimaryKeySet.value.has(key))
	)
}
const visibleSkipped = computed(
	() =>
		data.value?.skipped.filter(
			(skipped) =>
				!skipped.requiredByKeys?.length ||
				skipped.requiredByKeys.some((key) => visiblePrimaryKeySet.value.has(key)),
		) ?? [],
)
const hasBlockingPrimary = computed(() =>
	visiblePrimaries.value.some((primary) => !!primaryError(primary)),
)

const installableDependencies = computed(
	() =>
		visibleDependencies.value.filter(
			(dependency) => !dependency.alreadyInstalled && !dependencyRequired(dependency),
		) ?? [],
)
const selectedInstallableCount = computed(
	() =>
		installableDependencies.value.filter((dependency) => selectedIds.value.has(dependency.id))
			.length,
)
const hasUnresolvedDependencies = computed(() => visibleSkipped.value.length > 0)

function toggleDependency(id: string, value: boolean) {
	const next = new Set(selectedIds.value)
	if (value) next.add(id)
	else next.delete(id)
	selectedIds.value = next
}

function toggleAll(value: boolean) {
	const next = new Set(selectedIds.value)
	for (const dependency of installableDependencies.value) {
		if (value) next.add(dependency.id)
		else next.delete(dependency.id)
	}
	selectedIds.value = next
}

function finish(result: string[] | ContentInstallBatchPreviewResult | null) {
	if (settled) return
	settled = true
	const resolve = resolveShow
	resolveShow = null
	if (resolve) resolve(result)
	modal.value?.hide()
}

function confirm() {
	if (conflictMode) {
		finish(true)
		return
	}
	if (hasBlockingPrimary.value || visiblePrimaries.value.length === 0) return
	const approvedIds = visibleDependencies.value
		.filter(
			(dependency) =>
				!dependency.alreadyInstalled &&
				(dependencyRequired(dependency) || selectedIds.value.has(dependency.id)),
		)
		.map((dependency) => dependency.id)
	finish(batchMode ? { approvedIds, primaryKeys: visiblePrimaryKeys.value } : approvedIds)
}

function hide() {
	finish(null)
}

function show(value: ContentInstallPreviewData): Promise<string[] | null> {
	resolveShow?.(null)
	resolveShow = null
	data.value = value
	batchMode = false
	conflictMode = false
	conflictPrompt.value = null
	removedPrimaryKeys.value = new Set()
	selectedIds.value = new Set(
		value.installDependencies
			? value.dependencies
					.filter((dependency) => !dependency.alreadyInstalled)
					.map((dependency) => dependency.id)
			: [],
	)
	settled = false
	modal.value?.show()
	return new Promise<string[] | null>((resolve) => {
		resolveShow = (result) => resolve(Array.isArray(result) ? result : null)
	})
}

function showBatch(
	value: ContentInstallPreviewData,
): Promise<ContentInstallBatchPreviewResult | null> {
	resolveShow?.(null)
	resolveShow = null
	data.value = value
	batchMode = true
	conflictMode = false
	conflictPrompt.value = null
	removedPrimaryKeys.value = new Set()
	selectedIds.value = new Set(
		value.installDependencies
			? value.dependencies
					.filter((dependency) => !dependency.alreadyInstalled)
					.map((dependency) => dependency.id)
			: [],
	)
	settled = false
	modal.value?.show()
	return new Promise<ContentInstallBatchPreviewResult | null>((resolve) => {
		resolveShow = (result) => resolve(result && !Array.isArray(result) ? result : null)
	})
}

function showConflict(value: ContentInstallConflictPrompt): Promise<boolean> {
	resolveShow?.(null)
	resolveShow = null
	data.value = null
	batchMode = false
	conflictMode = true
	conflictPrompt.value = value
	settled = false
	modal.value?.show()
	return new Promise<boolean>((resolve) => {
		resolveShow = (result) => resolve(result === true)
	})
}

function removePrimary(key: string) {
	removedPrimaryKeys.value = new Set([...removedPrimaryKeys.value, key])
}

defineExpose({ show, showBatch, showConflict })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(conflictMode ? messages.conflictHeader : messages.header)"
		scrollable
		max-content-height="70vh"
		max-width="40rem"
		:on-hide="hide"
	>
		<div v-if="conflictPrompt" class="flex flex-col gap-4">
			<div
				class="flex items-center gap-3 rounded-lg border border-solid border-warning bg-warning-bg p-3"
			>
				<Avatar
					:src="conflictPrompt.candidate.iconUrl"
					:alt="conflictPrompt.candidate.title"
					size="2.5rem"
					:tint-by="conflictPrompt.candidate.title"
					no-shadow
				/>
				<div class="min-w-0 flex-1">
					<span class="block truncate font-semibold text-contrast">{{
						conflictPrompt.candidate.title
					}}</span>
					<span class="block truncate text-sm text-secondary">
						{{
							[conflictPrompt.candidate.provider, conflictPrompt.candidate.contentType].join(' · ')
						}}
					</span>
				</div>
			</div>
			<p class="m-0 text-primary">
				{{
					formatMessage(messages.conflictDescription, {
						candidate: conflictPrompt.candidate.title,
					})
				}}
			</p>
			<div class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">{{
					formatMessage(messages.existingContent)
				}}</span>
				<div
					v-for="item in conflictPrompt.existing"
					:key="`${item.provider}:${item.title}:${item.fileName ?? ''}`"
					class="flex items-center justify-between gap-3 rounded-lg border border-solid border-surface-4 bg-surface-2 px-3 py-2"
				>
					<span class="min-w-0 truncate font-medium text-contrast">{{ item.title }}</span>
					<span class="shrink-0 text-sm text-secondary">{{ item.provider }}</span>
				</div>
			</div>
		</div>
		<div v-else-if="data" class="flex flex-col gap-4">
			<div v-if="batchMode" class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">{{
					formatMessage(messages.selectedContentHeader)
				}}</span>
				<div
					v-for="primary in visiblePrimaries"
					:key="primary.key ?? primary.title"
					class="flex items-center gap-3 rounded-lg border border-solid border-surface-4 bg-surface-2 p-3"
				>
					<Avatar
						:src="primary.iconUrl"
						:alt="primary.title"
						size="2.5rem"
						:tint-by="primary.title"
						no-shadow
					/>
					<div class="flex min-w-0 flex-1 flex-col gap-0.5">
						<span class="truncate font-semibold text-contrast">{{ primary.title }}</span>
						<span class="truncate text-sm text-secondary">
							{{
								[primary.versionNumber, primary.provider, primary.contentType]
									.filter(Boolean)
									.join(' · ')
							}}
						</span>
						<span v-if="primaryError(primary)" class="text-sm text-red">
							{{ primaryError(primary) }}
						</span>
					</div>
					<ButtonStyled v-if="primary.removable && primary.key" circular type="transparent">
						<button
							type="button"
							:aria-label="formatMessage(messages.removeProject, { project: primary.title })"
							@click="removePrimary(primary.key)"
						>
							<XIcon />
						</button>
					</ButtonStyled>
				</div>
			</div>
			<div
				v-else-if="visiblePrimaries[0]"
				class="flex items-center gap-3 rounded-lg border border-solid border-surface-4 bg-surface-2 p-3"
			>
				<Avatar
					:src="visiblePrimaries[0].iconUrl"
					:alt="visiblePrimaries[0].title"
					size="2.5rem"
					:tint-by="visiblePrimaries[0].title"
					no-shadow
				/>
				<div class="flex min-w-0 flex-col gap-0.5">
					<span class="truncate font-semibold text-contrast">{{ visiblePrimaries[0].title }}</span>
					<span v-if="visiblePrimaries[0].versionNumber" class="truncate text-sm text-secondary">
						{{ visiblePrimaries[0].versionNumber }}
					</span>
				</div>
			</div>

			<p class="m-0 text-primary">
				{{
					batchMode
						? formatMessage(messages.batchDescription, {
								projectCount: visiblePrimaries.length,
								dependencyCount: visibleDependencies.length,
								instance: data.instanceName,
							})
						: formatMessage(messages.description, {
								count: visibleDependencies.length,
								project: visiblePrimaries[0]?.title ?? '',
								instance: data.instanceName,
							})
				}}
			</p>

			<div v-if="visibleDependencies.length > 0" class="flex flex-col gap-2">
				<div class="flex items-center justify-between">
					<span class="flex items-center gap-2 font-semibold text-contrast">
						{{ formatMessage(messages.dependenciesHeader) }}
						<span
							class="rounded-full bg-surface-4 px-2 py-0.5 text-xs font-medium tabular-nums text-secondary"
						>
							{{ formatMessage(messages.dependenciesCount, { count: visibleDependencies.length }) }}
						</span>
					</span>
					<ButtonStyled v-if="installableDependencies.length > 1" size="small" type="transparent">
						<button @click="toggleAll(selectedInstallableCount !== installableDependencies.length)">
							{{
								selectedInstallableCount === installableDependencies.length
									? formatMessage(messages.clearAll)
									: formatMessage(messages.selectAll)
							}}
						</button>
					</ButtonStyled>
				</div>
				<div
					v-for="dependency in visibleDependencies"
					:key="dependency.id"
					class="flex items-center gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-3"
					:class="{ 'opacity-60': dependency.alreadyInstalled }"
				>
					<Checkbox
						:model-value="dependencyRequired(dependency) || selectedIds.has(dependency.id)"
						:disabled="dependency.alreadyInstalled || dependencyRequired(dependency)"
						class="shrink-0"
						@update:model-value="(value) => toggleDependency(dependency.id, value)"
					/>
					<Avatar
						:src="dependency.iconUrl"
						:alt="dependency.title"
						size="2.5rem"
						:tint-by="dependency.title"
						no-shadow
					/>
					<div class="flex min-w-0 flex-1 flex-col gap-0.5">
						<span class="truncate font-semibold text-contrast">{{ dependency.title }}</span>
						<span v-if="dependency.versionNumber" class="truncate text-sm text-secondary">
							{{ dependency.versionNumber }}
						</span>
						<span v-if="dependency.requiredBy.length > 0" class="truncate text-sm text-secondary">
							{{
								formatMessage(messages.requiredBy, {
									projects: dependency.requiredBy.join(', '),
								})
							}}
						</span>
					</div>
					<span
						v-if="dependency.versionMismatch"
						class="shrink-0 rounded-full bg-warning-bg px-2 py-0.5 text-xs font-medium text-warning-text"
					>
						{{ formatMessage(messages.versionMismatch) }}
					</span>
					<span
						v-if="dependency.selectionReason"
						class="shrink-0 rounded-full bg-surface-4 px-2 py-0.5 text-xs font-medium text-secondary"
					>
						{{ dependency.selectionReason }}
					</span>
					<span
						v-if="dependency.alreadyInstalled"
						class="shrink-0 rounded-full bg-surface-4 px-2 py-0.5 text-xs font-medium text-secondary"
					>
						{{ formatMessage(messages.alreadyInstalled) }}
					</span>
				</div>
				<span class="text-sm text-secondary">{{ formatMessage(messages.onlyChecked) }}</span>
			</div>

			<div v-if="visibleSkipped.length > 0" class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.skippedHeader) }}</span>
				<div
					v-for="skipped in visibleSkipped"
					:key="skipped.id"
					class="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-lg border border-solid border-surface-4 bg-surface-2 px-3 py-2"
				>
					<span class="min-w-0 flex-1 truncate font-medium text-contrast">
						{{ skipped.title }}
					</span>
					<span class="shrink-0 text-sm text-secondary">{{ skipped.reason }}</span>
				</div>
			</div>
		</div>

		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<ButtonStyled type="outlined">
					<button @click="hide">{{ formatMessage(messages.cancel) }}</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button
						:disabled="!conflictMode && (hasBlockingPrimary || visiblePrimaries.length === 0)"
						@click="confirm"
					>
						{{
							conflictMode
								? formatMessage(messages.continueAnyway)
								: hasUnresolvedDependencies
									? formatMessage(messages.installResolved)
									: formatMessage(messages.install)
						}}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
