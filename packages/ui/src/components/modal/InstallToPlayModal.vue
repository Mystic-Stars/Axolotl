<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)" :closable="true">
		<div class="flex flex-col gap-4 max-w-[500px]">
			<Admonition type="info" :header="formatMessage(messages.admonitionHeader)">
				{{ formatMessage(messages.admonitionBody) }}
			</Admonition>

			<div
				v-if="sharedBy?.name"
				class="flex items-center gap-2 text-sm text-[var(--color-text-tertiary)]"
			>
				<Avatar
					v-if="sharedBy?.icon_url"
					:src="sharedBy.icon_url"
					:alt="sharedBy.name"
					size="24px"
				/>
				<span>
					<IntlFormatted :message-id="messages.sharedBy" :values="{ name: sharedBy.name }">
						<template #name="{ children }">
							<span class="font-semibold text-[var(--color-text-primary)]"
								><component :is="() => children"
							/></span>
						</template>
					</IntlFormatted>
				</span>
			</div>

			<div class="flex flex-col gap-2">
				<span class="text-sm font-semibold text-[var(--color-text-tertiary)]">{{
					formatMessage(messages.sharedInstanceLabel)
				}}</span>
				<div class="flex items-center gap-3 rounded-xl bg-surface-4 p-3">
					<Avatar :src="project.icon_url" :alt="project.title" size="48px" />
					<div class="flex flex-col gap-0.5">
						<span class="font-semibold text-[var(--color-text-primary)]">{{ project.title }}</span>
						<span class="text-sm text-[var(--color-text-tertiary)]">
							{{ loaderDisplay }} {{ project.game_versions?.[0] }}
							<template v-if="modCount">
								· {{ formatProjectTypeSentence(formatMessage, 'mod', modCount) }}
							</template>
						</span>
					</div>
				</div>
			</div>
		</div>

		<template #actions>
			<div class="flex justify-end gap-2">
				<Button @click="handleDecline"
					><XIcon />
					{{ formatMessage(commonMessages.declineButton) }}
				</Button>
				<Button type="colored" color="brand" @click="handleAccept"
					><CheckIcon />
					{{ formatMessage(commonMessages.acceptButton) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, XIcon } from '@modrinth/assets'
import type { Project } from '@modrinth/utils'
import { computed, ref } from 'vue'

import { defineMessages, useVIntl } from '../../composables/i18n'
import { formatLoader } from '../../utils'
import { commonMessages, formatProjectTypeSentence } from '../../utils/common-messages'
import Admonition from '../base/Admonition.vue'
import Avatar from '../base/Avatar.vue'
import Button from '../base/buttons/Button.vue'
import IntlFormatted from '../base/IntlFormatted.vue'
import NewModal from './NewModal.vue'

const props = defineProps<{
	project: Project
	sharedBy?: {
		name: string
		icon_url?: string
	}
	modCount?: number
}>()

const emit = defineEmits<{
	accept: []
	decline: []
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'modal.install-to-play.title', defaultMessage: 'Install to play' },
	admonitionHeader: {
		id: 'modal.install-to-play.admonition-header',
		defaultMessage: 'Shared server instance',
	},
	admonitionBody: {
		id: 'modal.install-to-play.admonition-body',
		defaultMessage:
			'This server requires modded content to play. Accept to install the needed files from Modrinth.',
	},
	sharedBy: {
		id: 'modal.install-to-play.shared-by',
		defaultMessage: '<name>{name}</name> shared this instance with you today.',
	},
	sharedInstanceLabel: {
		id: 'modal.install-to-play.shared-instance-label',
		defaultMessage: 'Shared instance',
	},
})
const modal = ref<InstanceType<typeof NewModal>>()

const loaderDisplay = computed(() => {
	const loader = props.project.loaders?.[0]
	if (!loader) return ''
	return formatLoader(formatMessage, loader)
})

function handleAccept() {
	// TODO: Implement accept logic
	emit('accept')
	modal.value?.hide()
}

function handleDecline() {
	emit('decline')
	modal.value?.hide()
}

function show(e?: MouseEvent) {
	modal.value?.show(e)
}

function hide() {
	modal.value?.hide()
}

defineExpose({ show, hide })
</script>
