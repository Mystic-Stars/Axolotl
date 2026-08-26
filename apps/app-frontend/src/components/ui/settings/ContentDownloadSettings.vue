<script setup lang="ts">
import AppearanceSettings from './AppearanceSettings.vue'
import ResourceManagementSettings from './ResourceManagementSettings.vue'
</script>

<template>
	<div class="flex flex-col gap-6">
		<AppearanceSettings scope="content-downloads" />
		<ResourceManagementSettings scope="content-downloads" />
	</div>
</template>
