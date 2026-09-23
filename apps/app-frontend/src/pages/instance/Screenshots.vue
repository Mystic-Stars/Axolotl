<script setup lang="ts">
import { useRoute } from 'vue-router'

import ScreenshotsPage from '@/components/ui/screenshots-page/index.vue'

const route = useRoute()
</script>

<template>
	<div class="box-border h-full p-6">
		<ScreenshotsPage :instance-id="String(route.params.id)" show-heading />
	</div>
</template>
