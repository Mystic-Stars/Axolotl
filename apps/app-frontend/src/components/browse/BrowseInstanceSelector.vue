<script setup lang="ts">
import { CheckIcon, DownloadIcon, PlusIcon, TrashIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { nextTick, ref } from 'vue'
import { useRouter } from 'vue-router'

import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import InstancePickerList from '@/components/ui/instance/InstancePickerList.vue'
import type { GameInstance } from '@/helpers/types'

const props = defineProps<{
	instances: GameInstance[]
	selectedInstance: GameInstance | null
	selectedCount: number
	installCurrent: () => Promise<boolean>
	clearCurrent: () => void
}>()

const emit = defineEmits<{
	select: [instance: GameInstance]
	cancelSwitch: []
}>()

const { formatMessage } = useVIntl()
const router = useRouter()
const pickerModal = ref<InstanceType<typeof NewModal>>()
const switchModal = ref<InstanceType<typeof NewModal>>()
const instancePicker = ref<InstanceType<typeof InstancePickerList>>()
const pendingInstance = ref<GameInstance | null>(null)
const installingCurrent = ref(false)

const messages = defineMessages({
	title: {
		id: 'app.browse.instance-selector.title',
		defaultMessage: 'Choose an instance',
	},
	search: {
		id: 'app.browse.instance-selector.search',
		defaultMessage: 'Search instances',
	},
	noInstances: {
		id: 'app.browse.instance-selector.no-instances',
		defaultMessage: 'Create an instance before installing content',
	},
	noResults: {
		id: 'app.browse.instance-selector.no-results',
		defaultMessage: 'No matching instances',
	},
	select: {
		id: 'app.browse.instance-selector.select',
		defaultMessage: 'Choose {name}',
	},
	create: {
		id: 'app.browse.instance-selector.create',
		defaultMessage: 'Create instance',
	},
	switchTitle: {
		id: 'app.browse.instance-selector.switch-title',
		defaultMessage: 'Switch installation target?',
	},
	switchDescription: {
		id: 'app.browse.instance-selector.switch-description',
		defaultMessage:
			'{count, plural, one {# selected project is} other {# selected projects are}} resolved for the current target. Switching will discard the resolved versions.',
	},
	currentTarget: {
		id: 'app.browse.instance-selector.current-target',
		defaultMessage: 'Current target',
	},
	newTarget: {
		id: 'app.browse.instance-selector.new-target',
		defaultMessage: 'New target',
	},
	installCurrent: {
		id: 'app.browse.instance-selector.install-current',
		defaultMessage: 'Install to current instance',
	},
	installingCurrent: {
		id: 'app.browse.instance-selector.installing-current',
		defaultMessage: 'Preparing installation…',
	},
	clearAndSwitch: {
		id: 'app.browse.instance-selector.clear-and-switch',
		defaultMessage: 'Clear and switch',
	},
	cancel: {
		id: 'app.browse.instance-selector.cancel',
		defaultMessage: 'Cancel',
	},
})

function show() {
	instancePicker.value?.reset()
	pickerModal.value?.show()
	void nextTick(() => instancePicker.value?.focus())
}

function choose(instance: GameInstance) {
	if (instance.id === props.selectedInstance?.id) {
		pickerModal.value?.hide()
		return
	}
	if (props.selectedCount > 0 && props.selectedInstance) {
		pendingInstance.value = instance
		pickerModal.value?.hide()
		void nextTick(() => switchModal.value?.show())
		return
	}
	emit('select', instance)
	pickerModal.value?.hide()
}

function requestSwitch(instance: GameInstance) {
	choose(instance)
}

async function installAndSwitch() {
	if (!pendingInstance.value || installingCurrent.value) return
	installingCurrent.value = true
	try {
		if (!(await props.installCurrent())) return
		const next = pendingInstance.value
		pendingInstance.value = null
		installingCurrent.value = false
		switchModal.value?.hide()
		emit('select', next)
	} finally {
		installingCurrent.value = false
	}
}

function clearAndSwitch() {
	if (!pendingInstance.value) return
	props.clearCurrent()
	const next = pendingInstance.value
	pendingInstance.value = null
	switchModal.value?.hide()
	emit('select', next)
}

function createInstance() {
	pickerModal.value?.hide()
	void router.push('/create')
}

function cancelSwitch() {
	pendingInstance.value = null
	switchModal.value?.hide()
	emit('cancelSwitch')
}

defineExpose({ show, requestSwitch })
</script>

<template>
	<NewModal
		ref="pickerModal"
		:header="formatMessage(messages.title)"
		max-width="560px"
		width="min(560px, calc(100vw - 2rem))"
		scrollable
		max-content-height="min(28rem, calc(100dvh - 18rem))"
		actions-divider
	>
		<div class="flex min-w-0 flex-col">
			<InstancePickerList
				ref="instancePicker"
				:instances="instances"
				:search-placeholder="formatMessage(messages.search)"
				:no-instances-message="formatMessage(messages.noInstances)"
				:no-matches-message="formatMessage(messages.noResults)"
				:select-label="(instance) => formatMessage(messages.select, { name: instance.name })"
				@select="choose"
			>
				<template #action="{ instance }">
					<CheckIcon
						v-if="instance.id === selectedInstance?.id"
						class="size-5 shrink-0 text-brand"
						aria-hidden="true"
					/>
				</template>
			</InstancePickerList>
		</div>
		<template #actions>
			<div class="flex justify-start">
				<ButtonStyled type="transparent">
					<button type="button" @click="createInstance">
						<PlusIcon />
						{{ formatMessage(messages.create) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>

	<NewModal
		ref="switchModal"
		:header="formatMessage(messages.switchTitle)"
		width="min(600px, calc(100vw - 2rem))"
		max-width="600px"
		scrollable
		max-content-height="min(32rem, 70vh)"
		actions-divider
		:disable-close="installingCurrent"
	>
		<div v-if="pendingInstance && selectedInstance" class="flex min-w-0 flex-col gap-5">
			<div
				class="grid min-w-0 gap-3 sm:grid-cols-[minmax(0,1fr)_1px_minmax(0,1fr)] sm:items-stretch"
			>
				<div class="flex min-w-0 flex-col gap-2 px-1 py-1">
					<span class="text-xs font-semibold text-secondary">
						{{ formatMessage(messages.currentTarget) }}
					</span>
					<div class="flex min-w-0 items-center gap-3">
						<InstanceIcon
							class="size-10 shrink-0"
							:icon-path="selectedInstance.icon_path"
							:instance-id="selectedInstance.id"
							:loader="selectedInstance.loader"
						/>
						<span class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="truncate font-semibold text-contrast">{{ selectedInstance.name }}</span>
							<span class="truncate text-sm capitalize text-secondary">
								{{ selectedInstance.loader }} {{ selectedInstance.game_version }}
							</span>
						</span>
					</div>
				</div>

				<div aria-hidden="true" class="h-px w-full bg-surface-4 sm:h-auto sm:w-px" />

				<div class="flex min-w-0 flex-col gap-2 px-1 py-1">
					<span class="text-xs font-semibold text-secondary">
						{{ formatMessage(messages.newTarget) }}
					</span>
					<div class="flex min-w-0 items-center gap-3">
						<InstanceIcon
							class="size-10 shrink-0"
							:icon-path="pendingInstance.icon_path"
							:instance-id="pendingInstance.id"
							:loader="pendingInstance.loader"
						/>
						<span class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="truncate font-semibold text-contrast">{{ pendingInstance.name }}</span>
							<span class="truncate text-sm capitalize text-secondary">
								{{ pendingInstance.loader }} {{ pendingInstance.game_version }}
							</span>
						</span>
					</div>
				</div>
			</div>

			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.switchDescription, { count: selectedCount }) }}
			</p>
		</div>
		<template #actions>
			<div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-end">
				<ButtonStyled type="outlined">
					<button
						type="button"
						class="w-full sm:w-auto"
						:disabled="installingCurrent"
						@click="cancelSwitch"
					>
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="red" type="outlined">
					<button
						type="button"
						class="w-full sm:w-auto"
						:disabled="installingCurrent"
						@click="clearAndSwitch"
					>
						<TrashIcon />
						{{ formatMessage(messages.clearAndSwitch) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button
						type="button"
						class="w-full sm:w-auto"
						:disabled="installingCurrent"
						@click="installAndSwitch"
					>
						<DownloadIcon />
						{{
							formatMessage(
								installingCurrent ? messages.installingCurrent : messages.installCurrent,
							)
						}}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
