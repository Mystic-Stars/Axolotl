import { LeftArrowIcon } from '@modrinth/assets'
import { markRaw } from 'vue'

import { commonMessages } from '#ui/utils/common-messages'

import type { StageConfigInput } from '../../../base'
import ImportInstanceStage from '../components/ImportInstanceStage.vue'
import { type CreationFlowContextValue, creationFlowMessages } from '../creation-flow-context'

export const stageConfig: StageConfigInput<CreationFlowContextValue> = {
    id: 'import-instance',
    title: (ctx) =>
        ctx.isImportMode.value
            ? ctx.formatMessage(creationFlowMessages.importInstanceTitle)
            : ctx.formatMessage(creationFlowMessages.chooseModpackTitle),
    stageContent: markRaw(ImportInstanceStage),
    skip: (ctx) => !ctx.isImportMode.value && ctx.setupType.value !== 'modpack',
    leftButtonConfig: (ctx) => ({
        label: ctx.formatMessage(commonMessages.backButton),
        icon: LeftArrowIcon,
        onClick: () => {
            if (ctx.skipSetupType.value) {
                ctx.modal.value?.hide()
                ctx.onBack?.()
            } else {
                ctx.isImportMode.value = false
                ctx.setupType.value = null
                ctx.modal.value?.setStage('setup-type')
            }
        },
    }),
    rightButtonConfig: null,
    maxWidth: '520px',
}
