import type { MessageDescriptor } from '@modrinth/ui'

import type { InstanceLoader } from '@/helpers/types'

export type GcStrategyId = 'g1gc-mojang' | 'pcl' | 'shenandoah' | 'zgc' | 'auto'

export type ResolvedGcStrategyId = Exclude<GcStrategyId, 'auto'>

export interface GcContext {
	javaMajorVersion: number | null
	allocatedMemoryMb: number
	systemCpuCores: number
	systemLogicalProcessors: number
	modCount: number
	loader: InstanceLoader
}

export interface GcResolution {
	resolvedStrategy: ResolvedGcStrategyId
	reasonChain: string[]
}

export interface GcStrategyDefinition {
	id: GcStrategyId
	baseArgs: string
	detect: (currentArgs: string) => boolean
	buildArgs: (context?: GcContext) => string
}

export interface JavaArgumentPreset {
	id: string
	title: MessageDescriptor
	description: MessageDescriptor
	args: string
	link: string
	group: string
	resolveArgs?: (context?: GcContext) => string
	detect?: (currentArgs: string) => boolean
	autoResolvedName?: string
	autoReasonChain?: string[]
}
