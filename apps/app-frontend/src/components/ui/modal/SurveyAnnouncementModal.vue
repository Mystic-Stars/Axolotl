<script setup lang="ts">
import { ButtonStyled, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ref } from 'vue'

import { AxolotlBrandConfig } from '@/config'

const DISMISSAL_KEY = 'axolotl-survey-promotion-dismissed'
const LAUNCHED_BEFORE_KEY = 'axolotl-launcher-opened-before'

const launchedBefore = localStorage.getItem(LAUNCHED_BEFORE_KEY) === 'true'
localStorage.setItem(LAUNCHED_BEFORE_KEY, 'true')

const { formatMessage } = useVIntl()
const modal = ref<InstanceType<typeof NewModal>>()

const messages = defineMessages({
	title: {
		id: 'app.survey-promotion.title',
		defaultMessage: 'Community survey',
	},
	intro: {
		id: 'app.survey-promotion.intro',
		defaultMessage:
			'To help us improve Axolotl Launcher, we have prepared a short survey and would love to hear your feedback.',
	},
	reward: {
		id: 'app.survey-promotion.reward',
		defaultMessage:
			'Lucky participants will be drawn to win a genuine Minecraft account (redeemable for cash).',
	},
	deferHint: {
		id: 'app.survey-promotion.defer-hint',
		defaultMessage:
			'If you close this dialog, you can still find the survey later in Settings > About.',
	},
	fillSurvey: {
		id: 'app.survey-promotion.fill-survey',
		defaultMessage: 'Fill out the survey',
	},
})

function dismiss() {
	localStorage.setItem(DISMISSAL_KEY, 'true')
}

function fillOutSurvey() {
	dismiss()
	modal.value?.hide()
	openUrl(AxolotlBrandConfig.surveyUrl)
}

function showIfNeeded() {
	if (!launchedBefore) return
	if (localStorage.getItem(DISMISSAL_KEY) === 'true') return
	modal.value?.show()
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
				{{ formatMessage(messages.intro) }}
			</p>
			<p class="m-0 leading-relaxed font-semibold text-contrast">
				{{ formatMessage(messages.reward) }}
			</p>
			<p class="m-0 text-sm leading-relaxed text-secondary">
				{{ formatMessage(messages.deferHint) }}
			</p>
		</div>

		<template #actions>
			<div class="flex justify-end gap-2">
				<ButtonStyled>
					<button @click="modal?.hide()">
						{{ formatMessage(commonMessages.closeButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button @click="fillOutSurvey">
						{{ formatMessage(messages.fillSurvey) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
