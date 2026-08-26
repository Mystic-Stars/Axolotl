<template>
	<section :data-instance-id="flow.instanceId.value" class="flex flex-col gap-2 py-2">
		<h2 class="m-0 text-xl font-semibold text-contrast">{{ title }}</h2>
		<p class="m-0 max-w-2xl text-secondary">{{ description }}</p>
	</section>
</template>

<script setup lang="ts">
import { useInstanceUpgradeFlow } from './flow'

defineProps<{ title: string; description: string }>()
const flow = useInstanceUpgradeFlow()
</script>
