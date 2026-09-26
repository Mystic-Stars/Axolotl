<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title, { version })"
		:on-hide="handleHide"
		:disable-close="responding"
		fade="warning"
		max-width="520px"
	>
		<div class="flex flex-col gap-3">
			<p class="m-0 leading-relaxed text-contrast">
				{{ formatMessage(messages.body, { version }) }}
			</p>
			<p class="m-0 leading-relaxed text-secondary">
				{{ formatMessage(messages.laterDescription, { version }) }}
			</p>
		</div>

		<template #actions>
			<div class="flex flex-wrap justify-end gap-2">
				<Button type="outlined" :disabled="responding" @click="modal?.hide()"
					><ClockIcon aria-hidden="true" />
					{{ formatMessage(messages.setUpLater) }}
				</Button>
				<Button :disabled="responding" @click="confirmDownload"
					><SpinnerIcon v-if="responding" class="animate-spin" aria-hidden="true" />
					<DownloadIcon v-else aria-hidden="true" />
					{{ formatMessage(messages.download) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { ClockIcon, DownloadIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, NewModal, useVIntl } from '@modrinth/ui'
import { onUnmounted, ref, useTemplateRef } from 'vue'
import { useRouter } from 'vue-router'

import { respond_to_java_download_confirmation } from '@/helpers/jre'

interface JavaDownloadConfirmationRequest {
	requestId: string
	version: number
}

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const router = useRouter()

const messages = defineMessages({
	title: {
		id: 'app.java-download-confirmation.title',
		defaultMessage: 'Download Java {version}?',
	},
	body: {
		id: 'app.java-download-confirmation.body',
		defaultMessage:
			'No compatible Java {version} installation was found. You can download it now or postpone Java setup while the remaining game resources continue downloading.',
	},
	laterDescription: {
		id: 'app.java-download-confirmation.later-description',
		defaultMessage:
			'The instance will finish installing, but Java {version} must be configured before you can play.',
	},
	setUpLater: {
		id: 'app.java-download-confirmation.set-up-later',
		defaultMessage: 'Set up later',
	},
	download: {
		id: 'app.java-download-confirmation.download',
		defaultMessage: 'Download Java',
	},
	postponedTitle: {
		id: 'app.java-download-confirmation.postponed-title',
		defaultMessage: 'Java setup postponed',
	},
	postponedBody: {
		id: 'app.java-download-confirmation.postponed-body',
		defaultMessage:
			'The instance will finish installing. Configure Java {version} before launching it.',
	},
})

const modal = useTemplateRef('modal')
const request = ref<JavaDownloadConfirmationRequest | null>(null)
const version = ref(0)
const responding = ref(false)
let decisionSent = false

function show(payload: JavaDownloadConfirmationRequest) {
	request.value = payload
	version.value = payload.version
	responding.value = false
	decisionSent = false
	modal.value?.show()
}

function handleHide() {
	const pendingRequest = request.value
	request.value = null
	responding.value = false

	if (!pendingRequest || decisionSent) return

	decisionSent = true
	void respond_to_java_download_confirmation(pendingRequest.requestId, false)
	addNotification({
		title: formatMessage(messages.postponedTitle),
		text: formatMessage(messages.postponedBody, { version: pendingRequest.version }),
		type: 'warning',
	})
}

async function confirmDownload() {
	const pendingRequest = request.value
	if (!pendingRequest || responding.value) return

	responding.value = true
	decisionSent = true
	const response = respond_to_java_download_confirmation(pendingRequest.requestId, true)
	modal.value?.hide()
	await router.push('/downloads')
	await response
}

onUnmounted(() => {
	const pendingRequest = request.value
	if (!pendingRequest || decisionSent) return

	decisionSent = true
	void respond_to_java_download_confirmation(pendingRequest.requestId, false)
})

defineExpose({ show })
</script>
