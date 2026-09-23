<script setup lang="ts">
import BackupRepositorySettings from './BackupRepositorySettings.vue'
import ResourceManagementSettings from './ResourceManagementSettings.vue'
import StorageSettings from './StorageSettings.vue'
</script>

<template>
	<div class="flex flex-col gap-6">
		<BackupRepositorySettings />
		<ResourceManagementSettings scope="storage-backups" />
		<StorageSettings />
	</div>
</template>
