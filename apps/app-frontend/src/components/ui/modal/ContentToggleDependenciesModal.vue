<script setup lang="ts">
import {
	Avatar,
	ButtonStyled,
	commonMessages,
	defineMessages,
	IntlFormatted,
	MinecraftFormattedText,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { autoCleanToText } from '@sfirew/minecraft-motd-parser'
import { ref } from 'vue'

export interface ContentToggleDependencyItem {
	title: string
	iconUrl?: string | null
	versionNumber?: string
}

export interface ContentToggleDependenciesData {
	enabling: boolean
	bulk: boolean
	primaryTitle: string
	related: ContentToggleDependencyItem[]
}

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: {
		id: 'app.instance.mods.toggle-dependencies.title',
		defaultMessage: 'Confirm toggle',
	},
	singleBody: {
		id: 'app.instance.mods.toggle-dependencies.single-body',
		defaultMessage:
			'Toggling {project} will affect {count, plural, one {# other item} other {# other items}}.',
	},
	bulkBody: {
		id: 'app.instance.mods.toggle-dependencies.bulk-body',
		defaultMessage:
			'Toggling the selected content will affect {count, plural, one {# other item} other {# other items}}.',
	},
	willEnable: {
		id: 'app.instance.mods.toggle-dependencies.will-enable',
		defaultMessage: 'The following content will be enabled:',
	},
	willDisable: {
		id: 'app.instance.mods.toggle-dependencies.will-disable',
		defaultMessage: 'The following content will be disabled:',
	},
	warning: {
		id: 'app.instance.mods.toggle-dependencies.warning',
		defaultMessage:
			'Do you want to apply these related changes automatically? Ignoring them may break the game.',
	},
	apply: {
		id: 'app.instance.mods.toggle-dependencies.apply',
		defaultMessage: 'Toggle related content',
	},
	selectedOnly: {
		id: 'app.instance.mods.toggle-dependencies.selected-only',
		defaultMessage: 'Only toggle selected',
	},
})

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const data = ref<ContentToggleDependenciesData | null>(null)
let settled = false
let resolveShow: ((choice: 'apply' | 'selected' | 'cancel') => void) | null = null

function finish(choice: 'apply' | 'selected' | 'cancel') {
	if (settled) return
	settled = true
	const resolve = resolveShow
	resolveShow = null
	if (resolve) resolve(choice)
	modal.value?.hide()
}

function show(value: ContentToggleDependenciesData): Promise<'apply' | 'selected' | 'cancel'> {
	data.value = value
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
		max-width="36rem"
		:on-hide="() => finish('cancel')"
	>
		<div v-if="data" class="flex flex-col gap-4">
			<p class="m-0 text-primary">
				<IntlFormatted
					:message-id="data.bulk ? messages.bulkBody : messages.singleBody"
					:values="{ count: data.related.length }"
				>
					<template #project>
						<MinecraftFormattedText :text="data.primaryTitle" />
					</template>
				</IntlFormatted>
			</p>

			<div v-if="data.related.length > 0" class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">
					{{ formatMessage(data.enabling ? messages.willEnable : messages.willDisable) }}
				</span>
				<div
					v-for="item in data.related"
					:key="item.title"
					class="flex items-center gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-3"
				>
					<Avatar
						:src="item.iconUrl"
						:alt="autoCleanToText(item.title)"
						size="2.5rem"
						:tint-by="item.title"
						no-shadow
					/>
					<div class="flex min-w-0 flex-col gap-0.5">
						<MinecraftFormattedText
							:text="item.title"
							class="truncate font-semibold text-contrast"
						/>
						<span v-if="item.versionNumber" class="truncate text-sm text-secondary">
							{{ item.versionNumber }}
						</span>
					</div>
				</div>
			</div>

			<p class="m-0 text-secondary">{{ formatMessage(messages.warning) }}</p>
		</div>

		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<ButtonStyled type="outlined">
					<button @click="finish('cancel')">
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled type="outlined">
					<button @click="finish('apply')">{{ formatMessage(messages.apply) }}</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button @click="finish('selected')">{{ formatMessage(messages.selectedOnly) }}</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
