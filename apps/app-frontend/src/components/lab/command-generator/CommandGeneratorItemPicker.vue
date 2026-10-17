<script setup lang="ts">
import { defineMessages, StyledInput, useVIntl } from '@modrinth/ui'
import Fuse from 'fuse.js'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import RecipeItemIcon from '@/components/lab/recipe-generator/RecipeItemIcon.vue'
import type { SlotDisplay } from '@/lab/recipe-generator/display'
import type { TextureAtlas } from '@/lab/recipe-generator/resources'
import type { SlotValue } from '@/lab/recipe-generator/types'

export type CommandItemEntry = {
    id: string
    name: string
    display: SlotDisplay
    value: SlotValue
}

const props = defineProps<{
    entries: CommandItemEntry[]
    atlas: TextureAtlas
    loading: boolean
}>()

const emit = defineEmits<{
    pick: [value: SlotValue]
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
    searchPlaceholder: {
        id: 'app.lab.command-generator.item-picker.search-placeholder',
        defaultMessage: 'Search items by name or ID',
    },
    loading: {
        id: 'app.lab.command-generator.item-picker.loading',
        defaultMessage: 'Loading items',
    },
    empty: {
        id: 'app.lab.command-generator.item-picker.empty',
        defaultMessage: 'No items match your search.',
    },
    pickItem: {
        id: 'app.lab.command-generator.item-picker.pick',
        defaultMessage: 'Pick item',
    },
})

const search = ref('')
const debouncedSearch = ref('')

// 修复：使用 ReturnType<typeof setTimeout> 而非 window.setTimeout，避免 SSR/类型环境差异
let searchTimer: ReturnType<typeof setTimeout> | undefined

watch(search, (value) => {
    if (searchTimer) clearTimeout(searchTimer)
    searchTimer = setTimeout(() => {
        debouncedSearch.value = value
    }, 120)
})

// 修复：组件卸载时清理挂起的 timer
onBeforeUnmount(() => {
    if (searchTimer) {
        clearTimeout(searchTimer)
        searchTimer = undefined
    }
})

const fuse = computed(
    () =>
        new Fuse(props.entries, {
            keys: ['name', 'id'],
            threshold: 0.35,
            ignoreLocation: true,
        }),
)

const visibleEntries = computed(() => {
    const query = debouncedSearch.value.trim()
    if (!query) return props.entries
    return fuse.value.search(query).map((result) => result.item)
})

function pick(value: SlotValue) {
    emit('pick', value)
}
</script>

<template>
    <div class="item-picker flex min-h-0 min-w-0 flex-1 flex-col gap-2 p-3">
        <StyledInput
            v-model="search"
            :placeholder="formatMessage(messages.searchPlaceholder)"
            clearable
            class="w-full shrink-0"
        />
        <div
            v-if="loading"
            class="flex min-h-24 items-center justify-center text-sm text-[var(--color-text-tertiary)]"
        >
            {{ formatMessage(messages.loading) }}
        </div>
        <div
            v-else-if="!visibleEntries.length"
            class="flex min-h-24 items-center justify-center px-4 text-sm text-[var(--color-text-tertiary)]"
        >
            {{ formatMessage(messages.empty) }}
        </div>
        <div v-else class="item-grid">
            <button
                v-for="entry in visibleEntries"
                :key="entry.id"
                type="button"
                class="item-cell"
                :title="`${formatMessage(messages.pickItem)}: ${entry.name} (${entry.id})`"
                :aria-label="`${formatMessage(messages.pickItem)}: ${entry.name}`"
                @click="pick(entry.value)"
            >
                <RecipeItemIcon
                    :display="entry.display"
                    :atlas="atlas"
                    :size="34"
                    :show-count="false"
                />
                <span class="item-name">{{ entry.name }}</span>
                <span class="item-id">{{ entry.id }}</span>
            </button>
        </div>
    </div>
</template>

<style scoped>
.item-picker {
    min-height: 0;
}

.item-grid {
    display: grid;
    min-height: 0;
    flex: 1;
    grid-template-columns: repeat(auto-fill, minmax(5.5rem, 1fr));
    grid-auto-rows: 5.75rem;
    gap: 0.4rem;
    overflow-y: auto;
    overscroll-behavior: contain;
    /* 给 hover 上浮位移和阴影留出空间，避免被 overflow 裁切 */
    padding: 0.45rem 0.55rem 0.6rem 0.45rem;
}

.item-cell {
    display: flex;
    min-width: 0;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.15rem;
    overflow: hidden;
    border: 1px solid var(--color-surface-5);
    border-radius: var(--radius-sm);
    background: var(--color-surface-4);
    padding: 0.2rem 0.1rem;
    color: var(--color-text-primary);
    cursor: pointer;
    transition:
        background-color 0.15s ease,
        border-color 0.15s ease,
        box-shadow 0.15s ease,
        transform 0.15s ease;
}

.item-cell:hover,
.item-cell:focus-visible {
    border-color: var(--color-brand);
    background: var(--color-surface-3);
    /* 品牌色描边环 + 投影，做出"卡片浮起"的观感 */
    box-shadow:
        0 0 0 1px var(--color-brand),
        0 6px 16px rgba(0, 0, 0, 0.28);
    transform: translateY(-2px);
    outline: none;
}

.item-cell:active {
    transform: translateY(0) scale(0.98);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.24);
}

.item-name {
    width: 100%;
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 0.6rem;
    line-height: 1.15;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.item-id {
    width: 100%;
    overflow: hidden;
    color: var(--color-text-tertiary);
    font-family: var(--mono-font, monospace);
    font-size: 0.5rem;
    line-height: 1.1;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.8;
}
</style>
