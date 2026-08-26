<script setup lang="ts">
import type { ServerTypeId } from '@modrinth/server'
import { Avatar } from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed } from 'vue'

import { SERVER_TYPE_META } from '@/components/multiplayer/servers/server-type'
import { isBuiltInInstanceIcon } from '@/helpers/instance-icon-frame'

const props = withDefaults(
	defineProps<{
		iconPath?: string | null
		serverType: ServerTypeId
		serverId?: string | null
		size?: string
	}>(),
	{
		iconPath: null,
		serverId: null,
		size: '2rem',
	},
)

const iconUrl = computed(() => (props.iconPath ? convertFileSrc(props.iconPath) : null))

const frameless = computed(() => (props.iconPath ? isBuiltInInstanceIcon(props.iconPath) : false))

const typeMeta = computed(() => SERVER_TYPE_META[props.serverType])

// Inline styles instead of Tailwind arbitrary values: underscores inside
// `var(--_color)` are converted to spaces by Tailwind's arbitrary-value
// parsing, which generates invalid CSS and breaks the production build.
const monogramStyle = computed(() => ({
	color: typeMeta.value.colorVar,
	backgroundColor: `color-mix(in srgb, ${typeMeta.value.colorVar} 14%, transparent)`,
}))
</script>

<template>
	<Avatar
		v-if="iconUrl"
		:src="iconUrl"
		:size="size"
		:tint-by="serverId"
		:class="{ '!border-0 !rounded-none !bg-transparent !shadow-none': frameless }"
	/>
	<div
		v-else
		class="flex shrink-0 items-center justify-center rounded-lg text-xs font-bold"
		:style="{
			'--_size': size,
			width: 'var(--_size)',
			height: 'var(--_size)',
			fontSize: 'calc(var(--_size) * 0.375)',
			...monogramStyle,
		}"
	>
		{{ typeMeta.monogram }}
	</div>
</template>
