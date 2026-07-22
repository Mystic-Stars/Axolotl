<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		:on-hide="handleHide"
		max-width="640px"
		scrollable
	>
		<p class="m-0 mb-4 text-secondary">
			{{ formatMessage(messages.description) }}
		</p>
		<div class="grid grid-cols-4 gap-3 sm:grid-cols-5">
			<button
				v-for="icon in builtInInstanceIcons"
				:key="icon.id"
				type="button"
				class="group flex min-w-0 cursor-pointer flex-col items-center gap-2 rounded-xl border border-solid bg-surface-2 p-3 text-secondary transition-all hover:border-brand hover:bg-brand-highlight hover:text-contrast focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand disabled:cursor-wait disabled:opacity-60"
				:class="
					loadingIconId === icon.id
						? 'border-brand bg-brand-highlight text-contrast'
						: 'border-surface-5'
				"
				:aria-label="formatMessage(icon.name)"
				:aria-pressed="loadingIconId === icon.id"
				:disabled="loadingIconId !== null"
				@click="selectBuiltInIcon(icon)"
			>
				<img :src="icon.url" alt="" class="aspect-square w-full object-contain" />
				<span class="w-full truncate text-center text-xs font-semibold">{{
					formatMessage(icon.name)
				}}</span>
			</button>
		</div>

		<template #actions>
			<ButtonStyled type="outlined">
				<button :disabled="loadingIconId !== null" @click="selectUploadedIcon">
					<UploadIcon />
					{{ formatMessage(messages.upload) }}
				</button>
			</ButtonStyled>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { UploadIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NewModal,
	type PickedFile,
	useVIntl,
} from '@modrinth/ui'
import { ref, useTemplateRef } from 'vue'

import { cache_icon } from '@/helpers/instance'
import { type BuiltInInstanceIcon, builtInInstanceIcons } from '@/helpers/instance-icons'
import { pickImage } from '@/providers/setup/file-picker'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')
const loadingIconId = ref<string | null>(null)
let resolveSelection: ((selection: PickedFile | null) => void) | null = null

const messages = defineMessages({
	title: {
		id: 'app.instance.icon-picker.title',
		defaultMessage: 'Choose an instance icon',
	},
	description: {
		id: 'app.instance.icon-picker.description',
		defaultMessage: 'Choose a built-in Minecraft icon or upload your own image.',
	},
	upload: {
		id: 'app.instance.icon-picker.upload',
		defaultMessage: 'Upload image',
	},
	loadError: {
		id: 'app.instance.icon-picker.load-error',
		defaultMessage: 'Failed to load the bundled icon.',
	},
})

function finish(selection: PickedFile | null) {
	const resolve = resolveSelection
	resolveSelection = null
	modal.value?.hide()
	resolve?.(selection)
}

function handleHide() {
	loadingIconId.value = null
	if (resolveSelection) {
		resolveSelection(null)
		resolveSelection = null
	}
}

async function selectBuiltInIcon(icon: BuiltInInstanceIcon) {
	loadingIconId.value = icon.id
	try {
		const response = await fetch(icon.url)
		if (!response.ok) throw new Error(formatMessage(messages.loadError))
		const blob = await response.blob()
		const file = new File([blob], `${icon.id}.png`, { type: blob.type || 'image/png' })
		const path = await cache_icon(
			icon.id + '.png',
			Array.from(new Uint8Array(await blob.arrayBuffer())),
		)
		finish({ file, path, previewUrl: icon.url, frameless: true })
	} catch (error) {
		handleError(error)
	} finally {
		loadingIconId.value = null
	}
}

async function selectUploadedIcon() {
	try {
		const selection = await pickImage()
		if (selection) finish(selection)
	} catch (error) {
		handleError(error)
	}
}

function show(): Promise<PickedFile | null> {
	resolveSelection?.(null)
	const modalInstance = modal.value
	if (!modalInstance) return Promise.resolve(null)

	return new Promise((resolve) => {
		resolveSelection = resolve
		modalInstance.show()
	})
}

defineExpose({ show })
</script>
