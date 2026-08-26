export {
	buildGcCandidateChain,
	getResolvedStrategyName,
	resolveAutoGcStrategy,
} from './auto-selector'
export { collectGcContext } from './context'
export { createGcPresets, getAutoResolution, getResolvedStrategyDisplayName } from './gc-presets'
export { detectGcStrategy, GC_STRATEGY_DEFINITIONS, getStrategyBaseArgs } from './strategies'
export type {
	GcContext,
	GcResolution,
	GcStrategyDefinition,
	GcStrategyId,
	ResolvedGcStrategyId,
} from './types'
