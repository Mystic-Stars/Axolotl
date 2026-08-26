import { defineMessage, type MessageDescriptor } from '@modrinth/ui'

import { createGcPresets } from '@/helpers/gc/gc-presets'
import type { GcContext, JavaArgumentPreset } from '@/helpers/gc/types'
import {
	FALLEN_AUTH_PROXY_BLOG_URL,
	FALLEN_AUTH_PROXY_JAVA_ARGS_STRING,
} from '@/helpers/java-arguments'

export type { JavaArgumentPreset }

export const JAVA_ARGUMENT_PRESET_GROUP_TITLES: Record<string, MessageDescriptor> = {
	gc: defineMessage({
		id: 'app.java-arguments.presets.gc-group-title',
		defaultMessage: 'Memory recycling strategy (GC)',
	}),
	auth: defineMessage({
		id: 'app.java-arguments.presets.auth-group-title',
		defaultMessage: 'Authentication service',
	}),
}

export const JAVA_ARGUMENT_PRESETS: JavaArgumentPreset[] = [
	{
		id: 'mojang-auth-mirror',
		group: 'auth',
		title: defineMessage({
			id: 'app.java-arguments.presets.auth-mirror.title',
			defaultMessage: 'Authentication service mirror',
		}),
		description: defineMessage({
			id: 'app.java-arguments.presets.auth-mirror.description',
			defaultMessage:
				'HTTP forwarding for the Mojang authentication servers hosted by Fallen-Breath.',
		}),
		args: FALLEN_AUTH_PROXY_JAVA_ARGS_STRING,
		link: FALLEN_AUTH_PROXY_BLOG_URL,
	},
]

export function getJavaArgumentPresets(gcContext?: GcContext): JavaArgumentPreset[] {
	return [...JAVA_ARGUMENT_PRESETS, ...createGcPresets(gcContext)]
}

export interface JavaArgumentPresetGroup {
	group: string
	title: MessageDescriptor
	presets: JavaArgumentPreset[]
}

export function getPresetsByGroup(presets: JavaArgumentPreset[]): JavaArgumentPresetGroup[] {
	const groups = new Map<string, JavaArgumentPreset[]>()
	for (const preset of presets) {
		const group = preset.group
		if (!groups.has(group)) {
			groups.set(group, [])
		}
		groups.get(group)!.push(preset)
	}
	return Array.from(groups, ([group, groupPresets]) => ({
		group,
		title: JAVA_ARGUMENT_PRESET_GROUP_TITLES[group] ?? JAVA_ARGUMENT_PRESET_GROUP_TITLES.auth,
		presets: groupPresets,
	}))
}
