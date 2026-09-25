<script setup lang="ts">
import { TagItem, TagTagItem } from '../base'
import TagOverflowPopover from './TagOverflowPopover.vue'

defineProps<{
	tags: string[]
}>()

defineOptions({
	inheritAttrs: false,
})
</script>

<template>
	<TagOverflowPopover v-if="tags.length > 0" :count="tags.length" v-bind="$attrs">
		<template #trigger>
			<TagItem as="span">+{{ tags.length }}</TagItem>
		</template>
		<TagTagItem v-for="tag in tags" :key="'overflow-tag-' + tag" hide-non-loader-icon :tag="tag" />
	</TagOverflowPopover>
</template>
