import { defineMessage } from '@modrinth/ui'

import { getResolvedStrategyName, resolveAutoGcStrategy } from '@/helpers/gc/auto-selector'
import { GC_STRATEGY_DEFINITIONS } from '@/helpers/gc/strategies'
import type { GcContext, JavaArgumentPreset } from '@/helpers/gc/types'
import { AUTO_GC_PRESET_ARG } from '@/helpers/java-arguments'

const GC_WIKI_URL = 'https://docs.oracle.com/en/java/javase/21/gctuning/introduction.html'
const G1GC_DOCS_URL =
	'https://docs.oracle.com/en/java/javase/21/gctuning/garbage-collector-implementation.html'
const SHENANDOAH_DOCS_URL = 'https://wiki.openjdk.org/display/shenandoah/Main'
const ZGC_DOCS_URL = 'https://wiki.openjdk.org/display/zgc/Main'

export function createGcPresets(gcContext?: GcContext): JavaArgumentPreset[] {
	const autoResolution = gcContext ? resolveAutoGcStrategy(gcContext) : null
	const autoResolvedName = autoResolution
		? getResolvedStrategyName(autoResolution.resolvedStrategy)
		: 'Mojang G1GC'

	return [
		{
			id: 'gc-auto',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.auto.title',
				defaultMessage: 'Auto',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.auto.description',
				defaultMessage: 'Automatically select the best GC strategy for your system',
			}),
			args: AUTO_GC_PRESET_ARG,
			resolveArgs: () => AUTO_GC_PRESET_ARG,
			detect: (currentArgs) => currentArgs.includes(AUTO_GC_PRESET_ARG),
			link: GC_WIKI_URL,
			autoResolvedName,
			autoReasonChain: autoResolution?.reasonChain,
		},
		{
			id: 'gc-g1gc-mojang',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.g1gc-mojang.title',
				defaultMessage: 'Mojang G1GC',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.g1gc-mojang.description',
				defaultMessage: 'G1GC tuning from the official Minecraft launcher',
			}),
			args: GC_STRATEGY_DEFINITIONS['g1gc-mojang'].baseArgs,
			resolveArgs: () => GC_STRATEGY_DEFINITIONS['g1gc-mojang'].baseArgs,
			detect: GC_STRATEGY_DEFINITIONS['g1gc-mojang'].detect,
			link: G1GC_DOCS_URL,
		},
		{
			id: 'gc-pcl',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.g1gc-pcl.title',
				defaultMessage: 'PCL',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.g1gc-pcl.description',
				defaultMessage: 'Shenandoah (adaptive) tuning used by the PCL launcher',
			}),
			args: GC_STRATEGY_DEFINITIONS.pcl.baseArgs,
			resolveArgs: () => GC_STRATEGY_DEFINITIONS.pcl.baseArgs,
			detect: GC_STRATEGY_DEFINITIONS.pcl.detect,
			link: SHENANDOAH_DOCS_URL,
		},
		{
			id: 'gc-shenandoah',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.shenandoah.title',
				defaultMessage: 'Shenandoah',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.shenandoah.description',
				defaultMessage: 'Low-pause adaptive Shenandoah with large pages (if supported)',
			}),
			args: GC_STRATEGY_DEFINITIONS.shenandoah.baseArgs,
			resolveArgs: () => GC_STRATEGY_DEFINITIONS.shenandoah.baseArgs,
			detect: GC_STRATEGY_DEFINITIONS.shenandoah.detect,
			link: SHENANDOAH_DOCS_URL,
		},
		{
			id: 'gc-zgc',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.zgc.title',
				defaultMessage: 'ZGC',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.zgc.description',
				defaultMessage: 'Ultra-low latency GC for high-end systems (Java 15+)',
			}),
			args: GC_STRATEGY_DEFINITIONS.zgc.buildArgs(gcContext),
			resolveArgs: (context) => GC_STRATEGY_DEFINITIONS.zgc.buildArgs(context),
			detect: GC_STRATEGY_DEFINITIONS.zgc.detect,
			link: ZGC_DOCS_URL,
		},
	]
}
