import type { StageConfigInput } from '../../../base'
import type { CreationFlowContextValue } from '../creation-flow-context'
import { stageConfig as customSetupStageConfig } from './custom-setup-stage'
import { stageConfig as importInstanceStageConfig } from './import-instance-stage'
import { stageConfig as setupTypeStageConfig } from './setup-type-stage'

export const stageConfigs: StageConfigInput<CreationFlowContextValue>[] = [
    setupTypeStageConfig,
    importInstanceStageConfig,
    customSetupStageConfig,
]
