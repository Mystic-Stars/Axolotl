<script setup lang="ts">
import { ChevronRightIcon } from '@modrinth/assets'

import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import type { GameInstance } from '@/helpers/types'

defineProps<{
	instance: GameInstance
}>()
</script>
<template>
	<span class="flex items-center gap-2 text-lg font-semibold text-primary">
		<InstanceIcon :icon-path="instance.icon_path" :instance-id="instance.id" size="24px" />
		{{ instance.name }} <ChevronRightIcon />
	</span>
</template>
