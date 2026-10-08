import { defineMessages } from '@modrinth/ui'

import { GC_STRATEGY_DEFINITIONS } from './strategies.ts'
import type { GcContext, GcReason, GcResolution, ResolvedGcStrategyId } from './types'

export const gcReasonMessages = defineMessages({
    javaVersionUnknown: {
        id: 'app.java-arguments.gc.reason.java-version-unknown',
        defaultMessage: 'Java version unknown',
    },
    javaTooOld: {
        id: 'app.java-arguments.gc.reason.java-too-old',
        defaultMessage: 'Java too old; Shenandoah/ZGC unreliable',
    },
    insufficientMemory: {
        id: 'app.java-arguments.gc.reason.insufficient-memory',
        defaultMessage: 'Insufficient memory ({allocated}GB < 4GB)',
    },
    insufficientCpu: {
        id: 'app.java-arguments.gc.reason.insufficient-cpu',
        defaultMessage: 'Insufficient CPU ({cores} cores / {threads} threads)',
    },
    largeModpackLowMemory: {
        id: 'app.java-arguments.gc.reason.large-modpack-low-memory',
        defaultMessage: 'Large modpack ({mods} mods) but insufficient memory ({allocated}GB < 8GB)',
    },
    lightweightInstance: {
        id: 'app.java-arguments.gc.reason.lightweight-instance',
        defaultMessage: 'Lightweight instance ({loader}, {mods} mods)',
    },
    heavyInstance: {
        id: 'app.java-arguments.gc.reason.heavy-instance',
        defaultMessage: 'Heavy instance ({loader}, {mods} mods)',
    },
    lowResources: {
        id: 'app.java-arguments.gc.reason.low-resources',
        defaultMessage: 'Low resources ({memory}GB, {cores} cores)',
    },
    mediumResources: {
        id: 'app.java-arguments.gc.reason.medium-resources',
        defaultMessage: 'Medium resources ({memory}GB, {cores} cores)',
    },
    highResources: {
        id: 'app.java-arguments.gc.reason.high-resources',
        defaultMessage: 'High resources ({memory}GB, {cores} cores)',
    },
    zgcNonGenerational: {
        id: 'app.java-arguments.gc.reason.zgc-non-generational',
        defaultMessage: 'Java < 21; non-generational ZGC performs poorly',
    },
    ampleMemoryHighCores: {
        id: 'app.java-arguments.gc.reason.ample-memory-high-cores',
        defaultMessage: 'Ample memory and high CPU core count',
    },
    belowZgcRecommendation: {
        id: 'app.java-arguments.gc.reason.below-zgc-recommendation',
        defaultMessage: 'Resources below ZGC recommendation',
    },
})

function reason(
    descriptor: (typeof gcReasonMessages)[keyof typeof gcReasonMessages],
    values?: Record<string, unknown>,
): GcReason {
    return values ? { ...descriptor, values } : descriptor
}

export function resolveAutoGcArgs(context: GcContext): string {
    const resolution = resolveAutoGcStrategy(context)
    return GC_STRATEGY_DEFINITIONS[resolution.resolvedStrategy].buildArgs(context)
}

export function resolveAutoGcStrategy(context: GcContext): GcResolution {
    const reasonChain: (string | GcReason)[] = []

    const javaVersion = context.javaMajorVersion
    if (javaVersion !== null) {
        reasonChain.push(`Java ${javaVersion}`)
    } else {
        reasonChain.push(reason(gcReasonMessages.javaVersionUnknown))
    }

    if (javaVersion === null || javaVersion < 15) {
        reasonChain.push(reason(gcReasonMessages.javaTooOld))
        return { resolvedStrategy: 'g1gc-mojang', reasonChain }
    }

    if (context.allocatedMemoryMb < 4096) {
        reasonChain.push(
            reason(gcReasonMessages.insufficientMemory, {
                allocated: Math.round(context.allocatedMemoryMb / 1024),
            }),
        )
        return { resolvedStrategy: 'g1gc-mojang', reasonChain }
    }

    if (context.systemCpuCores <= 4 && context.systemLogicalProcessors <= 8) {
        reasonChain.push(
            reason(gcReasonMessages.insufficientCpu, {
                cores: context.systemCpuCores,
                threads: context.systemLogicalProcessors,
            }),
        )
        return { resolvedStrategy: 'g1gc-mojang', reasonChain }
    }

    if (context.modCount >= 200 && context.allocatedMemoryMb < 8192) {
        reasonChain.push(
            reason(gcReasonMessages.largeModpackLowMemory, {
                mods: context.modCount,
                allocated: Math.round(context.allocatedMemoryMb / 1024),
            }),
        )
        return { resolvedStrategy: 'g1gc-mojang', reasonChain }
    }

    const isLightweight =
        (context.loader === 'vanilla' ||
            context.loader === 'fabric' ||
            context.loader === 'quilt') &&
        context.modCount < 30

    if (isLightweight) {
        reasonChain.push(
            reason(gcReasonMessages.lightweightInstance, {
                loader: context.loader,
                mods: context.modCount,
            }),
        )
        return { resolvedStrategy: 'g1gc-mojang', reasonChain }
    }

    reasonChain.push(
        reason(gcReasonMessages.heavyInstance, {
            loader: context.loader,
            mods: context.modCount,
        }),
    )

    const memoryGb = context.allocatedMemoryMb / 1024
    const isResourceLow = context.allocatedMemoryMb < 6144 || context.systemCpuCores <= 6
    const isResourceMedium =
        !isResourceLow && context.allocatedMemoryMb < 10240 && context.systemCpuCores <= 12

    if (isResourceLow) {
        reasonChain.push(
            reason(gcReasonMessages.lowResources, {
                memory: Math.round(memoryGb),
                cores: context.systemCpuCores,
            }),
        )
        return { resolvedStrategy: 'g1gc-mojang', reasonChain }
    }

    if (isResourceMedium) {
        reasonChain.push(
            reason(gcReasonMessages.mediumResources, {
                memory: Math.round(memoryGb),
                cores: context.systemCpuCores,
            }),
        )
        reasonChain.push('→ Shenandoah')
        return { resolvedStrategy: 'shenandoah', reasonChain }
    }

    reasonChain.push(
        reason(gcReasonMessages.highResources, {
            memory: Math.round(memoryGb),
            cores: context.systemCpuCores,
        }),
    )

    if (javaVersion < 21) {
        reasonChain.push(reason(gcReasonMessages.zgcNonGenerational))
        reasonChain.push('→ Shenandoah')
        return { resolvedStrategy: 'shenandoah', reasonChain }
    }

    if (context.allocatedMemoryMb >= 10240 && context.systemCpuCores > 12) {
        reasonChain.push(reason(gcReasonMessages.ampleMemoryHighCores))
        reasonChain.push('→ ZGC')
        return { resolvedStrategy: 'zgc', reasonChain }
    }

    reasonChain.push(reason(gcReasonMessages.belowZgcRecommendation))
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
export { buildGcCandidateChain } from './strategies.ts'
