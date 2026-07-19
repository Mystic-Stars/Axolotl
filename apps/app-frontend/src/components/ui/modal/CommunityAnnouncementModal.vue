<script setup lang="ts">
import { ButtonStyled, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import { AxolotlBrandConfig } from '@/config'

const DISMISSAL_KEY = 'axolotl-community-announcement-video-promotion-dismissed'

const { formatMessage } = useVIntl()
const modal = ref<InstanceType<typeof NewModal>>()

const messages = defineMessages({
	title: {
		id: 'app.community-announcement.title',
		defaultMessage: 'To all our users',
	},
	response: {
		id: 'app.community-announcement.response',
		defaultMessage:
			'Since Axolotl Launcher was promoted on video platforms, we have received far more love and attention than we expected, along with many thoughtful suggestions and high-quality reports.',
	},
	thanks: {
		id: 'app.community-announcement.thanks',
		defaultMessage:
			'I have done my best to personally reply to every comment on the videos. Thank you sincerely for all your support!',
	},
	feedbackPrefix: {
		id: 'app.community-announcement.feedback-prefix',
		defaultMessage: 'If you have more ideas or suggestions, please join the player QQ group: ',
	},
	feedbackMiddle: {
		id: 'app.community-announcement.feedback-middle',
		defaultMessage: ', or share them with us through ',
	},
	feedbackSuffix: {
		id: 'app.community-announcement.feedback-suffix',
		defaultMessage: '. We look forward to improving Axolotl Launcher together!',
	},
})

function dismiss() {
	localStorage.setItem(DISMISSAL_KEY, 'true')
}

function close() {
	modal.value?.hide()
}

function showIfNeeded() {
	if (localStorage.getItem(DISMISSAL_KEY) !== 'true') {
		modal.value?.show()
	}
}

defineExpose({ showIfNeeded })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		:on-hide="dismiss"
		max-width="640px"
	>
		<div class="flex flex-col gap-4 text-primary">
			<p class="m-0 leading-relaxed">
				{{ formatMessage(messages.response) }}
			</p>
			<p class="m-0 leading-relaxed">
				{{ formatMessage(messages.thanks) }}
			</p>
			<p class="m-0 leading-relaxed">
				{{ formatMessage(messages.feedbackPrefix)
				}}<span class="font-semibold text-contrast">{{ AxolotlBrandConfig.qqGroupNumber }}</span
				>{{ formatMessage(messages.feedbackMiddle)
				}}<a
					:href="AxolotlBrandConfig.supportUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="font-medium text-brand hover:underline"
					>GitHub Issues</a
				>{{ formatMessage(messages.feedbackSuffix) }}
			</p>
		</div>

		<template #actions>
			<div class="flex justify-end">
				<ButtonStyled color="brand">
					<button @click="close">
						{{ formatMessage(commonMessages.closeButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
