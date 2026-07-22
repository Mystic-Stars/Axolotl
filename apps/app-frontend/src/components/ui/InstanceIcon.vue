<script setup lang="ts">
import { Avatar } from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed } from 'vue'

import { isBuiltInInstanceIcon } from '@/helpers/instance-icon-frame'

const props = withDefaults(
	defineProps<{
		iconPath?: string | null
		instanceId?: string | null
	}>(),
	{
		iconPath: null,
		instanceId: null,
	},
)

const iconUrl = computed(() => (props.iconPath ? convertFileSrc(props.iconPath) : null))
</script>

<template>
	<Avatar
		:src="iconUrl"
		:tint-by="instanceId"
		:class="{
			'!border-0 !rounded-none !bg-transparent !shadow-none': isBuiltInInstanceIcon(iconPath),
		}"
	/>
</template>
