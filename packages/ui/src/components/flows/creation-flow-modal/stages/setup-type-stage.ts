import { markRaw } from 'vue'

import type { StageConfigInput } from '../../../base'
import SetupTypeStage from '../components/SetupTypeStage.vue'
import { type CreationFlowContextValue, flowTypeHeadingMessages } from '../creation-flow-context'

export const stageConfig: StageConfigInput<CreationFlowContextValue> = {
    id: 'setup-type',
    title: (ctx) => ctx.formatMessage(flowTypeHeadingMessages[ctx.flowType]),
    stageContent: markRaw(SetupTypeStage),
    skip: (ctx) => ctx.skipSetupType.value,
    leftButtonConfig: null,
    rightButtonConfig: null,
    maxWidth: '520px',
}
