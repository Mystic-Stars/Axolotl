<script setup lang="ts">
import { computed } from 'vue'

import { getTagMessage, sortTagsForDisplay } from '../../../utils'
import { TagTagItem } from '../../base'
import TagsOverflow from '../TagsOverflow.vue'

function isLoader(tag: string) {
    return getTagMessage(tag, 'loader') !== undefined
}

function uniqueSorted(tags?: string[]) {
    return tags ? sortTagsForDisplay([...new Set(tags)]) : undefined
}

const props = withDefaults(
    defineProps<{
        tags: string[]
        extraTags?: string[]
        deprioritizedTags?: string[]
        excludeLoaders?: boolean
        maxTags?: number
    }>(),
    {
        maxTags: 5,
        excludeLoaders: false,
        extraTags: () => [],
        deprioritizedTags: () => [],
    },
)

const sortedTags = computed(() => uniqueSorted(props.tags))
const sortedExtraTags = computed(() => uniqueSorted(props.extraTags))

const filterTag = (tag: string) =>
    !props.deprioritizedTags.includes(tag) && (!props.excludeLoaders || !isLoader(tag))

const filteredTags = computed(() => {
    if (!sortedTags.value) {
        return undefined
    }
    return sortedTags.value.filter(filterTag)
})

const filteredExtraTags = computed(() => {
    if (!sortedExtraTags.value) {
        return undefined
    }
    return sortedExtraTags.value.filter(filterTag)
})

const visibleTags = computed(() => {
    const mainTags = filteredTags.value ?? []
    const extraTags = filteredExtraTags.value ?? []
    const combined = [...mainTags, ...extraTags]

    // 将加载器标签排在最前面
    const loaders = combined.filter(isLoader)
    const nonLoaders = combined.filter((tag) => !isLoader(tag))
    const sorted = [...loaders, ...nonLoaders]

    return sorted.slice(0, props.maxTags)
})

const overflowTags = computed(() => {
    const mainTags = filteredTags.value ?? []
    const extraTags = filteredExtraTags.value ?? []
    const combined = [...mainTags, ...extraTags]
    const overflow = combined.filter((x) => !visibleTags.value?.includes(x))
    return [...new Set(overflow)]
})
</script>

<template>
    <TagTagItem
        v-for="tag in visibleTags"
        :key="'visible-tag-' + tag"
        hide-non-loader-icon
        :tag="tag"
    />
    <TagsOverflow
        v-if="overflowTags"
        :tags="overflowTags"
        class="smart-clickable:allow-pointer-events"
    />
</template>
