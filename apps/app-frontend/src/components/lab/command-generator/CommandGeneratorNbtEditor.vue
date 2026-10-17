<script setup lang="ts">
import { CodeIcon, SparklesIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import type { NbtKind } from '@/lab/command-generator/commands/types.ts'

import CommandGeneratorItemNbt from './CommandGeneratorItemNbt.vue'
import CommandGeneratorNbtAdvanced from './CommandGeneratorNbtAdvanced.vue'

const props = defineProps<{
    modelValue?: string
    kind?: NbtKind
    version?: string
    forceDataComponents?: boolean
}>()

const emit = defineEmits<{
    'update:modelValue': [value: string]
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
    simple: { id: 'app.lab.command-generator.nbt.simple', defaultMessage: 'Simple' },
    advanced: { id: 'app.lab.command-generator.nbt.advanced', defaultMessage: 'Advanced' },
})

const mode = ref<'simple' | 'advanced'>(props.kind === 'item' ? 'simple' : 'advanced')

watch(
    () => props.kind,
    (kind) => {
        mode.value = kind === 'item' ? 'simple' : 'advanced'
    },
)
</script>

<template>
    <div class="nbt-editor-wrap">
        <div v-if="kind === 'item'" class="nbt-mode-tabs">
            <button
                type="button"
                class="nbt-mode-tab"
                :class="{ 'nbt-mode-tab--active': mode === 'simple' }"
                @click="mode = 'simple'"
            >
                <SparklesIcon />
                {{ formatMessage(messages.simple) }}
            </button>
            <button
                type="button"
                class="nbt-mode-tab"
                :class="{ 'nbt-mode-tab--active': mode === 'advanced' }"
                @click="mode = 'advanced'"
            >
                <CodeIcon />
                {{ formatMessage(messages.advanced) }}
            </button>
        </div>

        <CommandGeneratorItemNbt
            v-if="kind === 'item' && mode === 'simple'"
            :model-value="modelValue"
            :version="version"
            :force-data-components="forceDataComponents"
            @update:model-value="emit('update:modelValue', $event)"
        />
        <CommandGeneratorNbtAdvanced
            v-else
            :model-value="modelValue"
            @update:model-value="emit('update:modelValue', $event)"
        />
    </div>
</template>

<style scoped>
.nbt-editor-wrap {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
}

.nbt-mode-tabs {
    display: inline-flex;
    gap: 0.25rem;
    border-radius: var(--radius-md);
    background: var(--surface-1);
    padding: 0.2rem;
    align-self: flex-start;
}

.nbt-mode-tab {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    padding: 0.35rem 0.7rem;
    color: var(--color-text-tertiary);
    font-size: 0.7rem;
    font-weight: 700;
    cursor: pointer;
    transition:
        background 0.15s ease,
        color 0.15s ease;
}

.nbt-mode-tab:hover {
    color: var(--color-text-primary);
}

.nbt-mode-tab--active {
    background: color-mix(in srgb, var(--color-brand) 14%, var(--surface-4));
    color: var(--color-brand);
}

.nbt-mode-tab :deep(svg) {
    width: 0.78rem;
    height: 0.78rem;
}
</style>
