import { GC_STRATEGY_DEFINITIONS } from './strategies'
import type { GcContext, GcResolution, ResolvedGcStrategyId } from './types'

export function resolveAutoGcArgs(context: GcContext): string {
	const resolution = resolveAutoGcStrategy(context)
	return GC_STRATEGY_DEFINITIONS[resolution.resolvedStrategy].buildArgs(context)
}

export function resolveAutoGcStrategy(context: GcContext): GcResolution {
	const reasonChain: string[] = []

	const javaVersion = context.javaMajorVersion
	if (javaVersion !== null) {
		reasonChain.push(`Java ${javaVersion}`)
	} else {
		reasonChain.push('Java 版本未知')
	}

	if (javaVersion === null || javaVersion < 15) {
		reasonChain.push('Java 太旧，Shenandoah/ZGC 不可靠')
		return { resolvedStrategy: 'g1gc-mojang', reasonChain }
	}

	if (context.allocatedMemoryMb < 4096) {
		reasonChain.push(`内存不足 (${Math.round(context.allocatedMemoryMb / 1024)}GB < 4GB)`)
		return { resolvedStrategy: 'g1gc-mojang', reasonChain }
	}

	if (context.systemCpuCores <= 4 && context.systemLogicalProcessors <= 8) {
		reasonChain.push(
			`CPU 资源不足 (${context.systemCpuCores}核/${context.systemLogicalProcessors}线程)`,
		)
		return { resolvedStrategy: 'g1gc-mojang', reasonChain }
	}

	if (context.modCount >= 200 && context.allocatedMemoryMb < 8192) {
		reasonChain.push(
			`大型 ModPack (${context.modCount} mods) 但资源不足 (${Math.round(context.allocatedMemoryMb / 1024)}GB < 8GB)`,
		)
		return { resolvedStrategy: 'g1gc-mojang', reasonChain }
	}

	const isLightweight =
		(context.loader === 'vanilla' || context.loader === 'fabric' || context.loader === 'quilt') &&
		context.modCount < 30

	if (isLightweight) {
		reasonChain.push(`轻量实例 (${context.loader}, ${context.modCount} mods)`)
		return { resolvedStrategy: 'g1gc-mojang', reasonChain }
	}

	reasonChain.push(`重型实例 (${context.loader}, ${context.modCount} mods)`)

	const memoryGb = context.allocatedMemoryMb / 1024
	const isResourceLow = context.allocatedMemoryMb < 6144 || context.systemCpuCores <= 6
	const isResourceMedium =
		!isResourceLow && context.allocatedMemoryMb < 10240 && context.systemCpuCores <= 12

	if (isResourceLow) {
		reasonChain.push(`资源低 (${Math.round(memoryGb)}GB, ${context.systemCpuCores}核)`)
		return { resolvedStrategy: 'g1gc-mojang', reasonChain }
	}

	if (isResourceMedium) {
		reasonChain.push(`资源中 (${Math.round(memoryGb)}GB, ${context.systemCpuCores}核)`)
		reasonChain.push('→ Shenandoah')
		return { resolvedStrategy: 'shenandoah', reasonChain }
	}

	reasonChain.push(`资源高 (${Math.round(memoryGb)}GB, ${context.systemCpuCores}核)`)

	if (javaVersion < 21) {
		reasonChain.push('Java < 21，ZGC 非分代模式性能不佳')
		reasonChain.push('→ Shenandoah')
		return { resolvedStrategy: 'shenandoah', reasonChain }
	}

	if (context.allocatedMemoryMb >= 10240 && context.systemCpuCores > 12) {
		reasonChain.push('内存充足且 CPU 核心数高')
		reasonChain.push('→ ZGC')
		return { resolvedStrategy: 'zgc', reasonChain }
	}

	reasonChain.push('资源未达到 ZGC 推荐配置')
	reasonChain.push('→ Shenandoah')
	return { resolvedStrategy: 'shenandoah', reasonChain }
}

export function getResolvedStrategyName(strategyId: ResolvedGcStrategyId): string {
	const names: Record<ResolvedGcStrategyId, string> = {
		'g1gc-mojang': 'Mojang G1GC',
		pcl: 'PCL',
		shenandoah: 'Shenandoah',
		zgc: 'ZGC',
	}
	return names[strategyId]
}

// Re-exported from strategies so callers can keep importing from the
// auto-selector module while the chain stays testable via `node --test`.
export { buildGcCandidateChain } from './strategies'
