<script setup lang="ts">
import MultiplayerSettings from './MultiplayerSettings.vue'
import ResourceManagementSettings from './ResourceManagementSettings.vue'
</script>

<template>
	<div class="flex flex-col gap-6">
		<ResourceManagementSettings scope="network-multiplayer" />
		<MultiplayerSettings />
	</div>
</template>
