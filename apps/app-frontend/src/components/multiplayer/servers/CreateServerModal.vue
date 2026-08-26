<script setup lang="ts">
import { commonMessages, MultiStageModal } from '@modrinth/ui'
import { computed, useTemplateRef, watch } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import {
	createCreateServerFlowContext,
	provideCreateServerFlow,
} from '@/components/multiplayer/servers/create-server-flow'
import EulaModal from '@/components/multiplayer/servers/EulaModal.vue'

const emit = defineEmits<{
	created: [serverId: string]
}>()

const modal = useTemplateRef<ComponentExposed<typeof MultiStageModal>>('modal')
const eulaModal = useTemplateRef<ComponentExposed<typeof EulaModal>>('eulaModal')

const ctx = createCreateServerFlowContext(modal)
provideCreateServerFlow(ctx)

const cancelButton = computed(() => ({
	label: ctx.formatMessage(commonMessages.cancelButton),
	disabled: ctx.installPhase.value === 'downloading' || ctx.installPhase.value === 'first-run',
	onClick: () => modal.value?.hide(),
}))

watch(ctx.showEulaModal, (visible) => {
	if (visible) eulaModal.value?.show()
	else eulaModal.value?.hide()
})

function show() {
	ctx.reset()
	modal.value?.setStage(0)
	modal.value?.show()
}

function handleHide() {
	if (ctx.createdServer.value) emit('created', ctx.createdServer.value.id)
}

defineExpose({ show, hide: () => modal.value?.hide() })
</script>

<template>
	<MultiStageModal
		ref="modal"
		:stages="ctx.stageConfigs"
		:context="ctx"
		:back-button-enabled="
			(flowCtx) =>
				flowCtx.installPhase.value !== 'downloading' && flowCtx.installPhase.value !== 'first-run'
		"
		:cancel-button="cancelButton"
		@hide="handleHide"
	/>
	<EulaModal
		ref="eulaModal"
		:text="ctx.eulaText.value"
		@accept="ctx.acceptEula"
		@decline="ctx.declineEula"
	/>
</template>
