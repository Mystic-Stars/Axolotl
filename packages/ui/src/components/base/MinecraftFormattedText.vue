<script setup lang="ts">
import { autoToHTML } from '@sfirew/minecraft-motd-parser'
import { computed } from 'vue'

const props = defineProps<{
	text: string
}>()

const renderedText = computed(() => autoToHTML(props.text))
</script>

<template>
	<!-- eslint-disable-next-line vue/no-v-html -->
	<span v-html="renderedText" />
</template>
