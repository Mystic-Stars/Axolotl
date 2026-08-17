<script setup lang="ts">
import { Avatar, ButtonStyled, Checkbox, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

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
}

export interface ContentInstallPreviewSkipped {
	id: string
	title: string
	reason: string
}

export interface ContentInstallPreviewData {
	primary: {
		title: string
		iconUrl?: string | null
		versionNumber?: string
	}
	instanceName: string
	installDependencies: boolean
	dependencies: ContentInstallPreviewDependency[]
	skipped: ContentInstallPreviewSkipped[]
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
})

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const data = ref<ContentInstallPreviewData | null>(null)
const selectedIds = ref<Set<string>>(new Set())
let settled = false
let resolveShow: ((approvedIds: string[] | null) => void) | null = null

const installableDependencies = computed(
	() =>
		data.value?.dependencies.filter(
			(dependency) => !dependency.alreadyInstalled && !dependency.required,
		) ?? [],
)
const selectedInstallableCount = computed(
	() =>
		installableDependencies.value.filter((dependency) => selectedIds.value.has(dependency.id))
			.length,
)
const hasUnresolvedDependencies = computed(() => (data.value?.skipped.length ?? 0) > 0)

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

function finish(approvedIds: string[] | null) {
	if (settled) return
	settled = true
	const resolve = resolveShow
	resolveShow = null
	if (resolve) resolve(approvedIds)
	modal.value?.hide()
}

function confirm() {
	finish(
		installableDependencies.value
			.filter((dependency) => selectedIds.value.has(dependency.id))
			.map((dependency) => dependency.id),
	)
}

function hide() {
	finish(null)
}

function show(value: ContentInstallPreviewData): Promise<string[] | null> {
	data.value = value
	selectedIds.value = new Set(
		value.installDependencies
			? value.dependencies
					.filter((dependency) => !dependency.alreadyInstalled)
					.map((dependency) => dependency.id)
			: [],
	)
	settled = false
	modal.value?.show()
	return new Promise((resolve) => {
		resolveShow = resolve
	})
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		scrollable
		max-content-height="70vh"
		max-width="40rem"
		:on-hide="hide"
	>
		<div v-if="data" class="flex flex-col gap-4">
			<div
				class="flex items-center gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-3"
			>
				<Avatar
					:src="data.primary.iconUrl"
					:alt="data.primary.title"
					size="2.5rem"
					:tint-by="data.primary.title"
					no-shadow
				/>
				<div class="flex min-w-0 flex-col gap-0.5">
					<span class="truncate font-semibold text-contrast">{{ data.primary.title }}</span>
					<span v-if="data.primary.versionNumber" class="truncate text-sm text-secondary">
						{{ data.primary.versionNumber }}
					</span>
				</div>
			</div>

			<p class="m-0 text-primary">
				{{
					formatMessage(messages.description, {
						count: data.dependencies.length,
						project: data.primary.title,
						instance: data.instanceName,
					})
				}}
			</p>

			<div v-if="data.dependencies.length > 0" class="flex flex-col gap-2">
				<div class="flex items-center justify-between">
					<span class="flex items-center gap-2 font-semibold text-contrast">
						{{ formatMessage(messages.dependenciesHeader) }}
						<span
							class="rounded-full bg-surface-4 px-2 py-0.5 text-xs font-medium tabular-nums text-secondary"
						>
							{{ formatMessage(messages.dependenciesCount, { count: data.dependencies.length }) }}
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
					v-for="dependency in data.dependencies"
					:key="dependency.id"
					class="flex items-center gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-3"
					:class="{ 'opacity-60': dependency.alreadyInstalled }"
				>
					<Checkbox
						:model-value="selectedIds.has(dependency.id)"
						:disabled="dependency.alreadyInstalled || dependency.required"
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

			<div v-if="data.skipped.length > 0" class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.skippedHeader) }}</span>
				<div
					v-for="skipped in data.skipped"
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
					<button @click="confirm">
						{{
							hasUnresolvedDependencies
								? formatMessage(messages.installResolved)
								: formatMessage(messages.install)
						}}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
