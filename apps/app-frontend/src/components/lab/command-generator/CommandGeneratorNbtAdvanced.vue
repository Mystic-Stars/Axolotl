<script setup lang="ts">
import { CodeIcon, PlusIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import {
    createItemNbtTree,
    createNbtTag,
    serializeNbt,
    type NbtTagNode,
} from '@/lab/command-generator/nbt-tree.ts'

import CommandGeneratorNbtNode from './CommandGeneratorNbtNode.vue'

const props = defineProps<{
    modelValue?: string
}>()

const emit = defineEmits<{
    'update:modelValue': [value: string]
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
    title: {
        id: 'app.lab.command-generator.nbt.advanced.title',
        defaultMessage: 'Advanced NBT tree',
    },
    addRoot: {
        id: 'app.lab.command-generator.nbt.advanced.add-root',
        defaultMessage: 'Add root tag',
    },
    snbtPreview: {
        id: 'app.lab.command-generator.nbt.advanced.snbt-preview',
        defaultMessage: 'SNBT preview',
    },
    presetEnchanted: {
        id: 'app.lab.command-generator.nbt.advanced.preset-enchanted',
        defaultMessage: 'Enchanted item',
    },
    presetCustomName: {
        id: 'app.lab.command-generator.nbt.advanced.preset-custom-name',
        defaultMessage: 'Custom name',
    },
})

const nodes = ref<NbtTagNode[]>([])

const presets = [
    { label: messages.presetEnchanted, factory: () => createItemNbtTree() },
    {
        label: messages.presetCustomName,
        factory: () => [
            createNbtTag('display', 'compound', '', [
                createNbtTag('Name', 'string', '{"text":"我的物品"}'),
            ]),
        ],
    },
]

const snbtPreview = computed(() => serializeNbt(nodes.value))

watch(
    snbtPreview,
    (val) => {
        emit('update:modelValue', val)
    },
    { immediate: true },
)

watch(
    () => props.modelValue,
    (val) => {
        if (val && nodes.value.length === 0) {
            nodes.value = [createNbtTag('value', 'string', val)]
        }
    },
    { immediate: true },
)

function applyPreset(preset: (typeof presets)[number]) {
    nodes.value = preset.factory()
}

function onUpdate(id: string, field: string, value: string) {
    const node = findNodeById(nodes.value, id)
    if (node) {
        if (field === 'name') node.name = value
        else if (field === 'value') node.value = value
        else if (field === 'type') node.type = value as NbtTagNode['type']
    }
}

function onRemove(id: string) {
    removeNodeById(nodes.value, id)
}

function onAddChild(parentId: string) {
    const parent = findNodeById(nodes.value, parentId)
    if (parent) {
        if (!parent.children) parent.children = []
        parent.children.push(createNbtTag('new_tag', 'string', ''))
    }
}

function addRootNode() {
    nodes.value.push(createNbtTag('new_tag', 'string', ''))
}

function findNodeById(list: NbtTagNode[], id: string): NbtTagNode | undefined {
    for (const node of list) {
        if (node.id === id) return node
        if (node.children) {
            const found = findNodeById(node.children, id)
            if (found) return found
        }
    }
    return undefined
}

function removeNodeById(list: NbtTagNode[], id: string): boolean {
    for (let i = 0; i < list.length; i++) {
        if (list[i].id === id) {
            list.splice(i, 1)
            return true
        }
        if (list[i].children) {
            if (removeNodeById(list[i].children!, id)) return true
        }
    }
    return false
}
</script>

<template>
    <div class="nbt-editor">
        <header class="nbt-head">
            <span class="nbt-title">
                <CodeIcon />
                {{ formatMessage(messages.title) }}
            </span>
            <div class="nbt-presets">
                <Button
                    v-for="preset in presets"
                    :key="preset.label.id"
                    type="outlined"
                    size="2xs"
                    @click="applyPreset(preset)"
                >
                    {{ formatMessage(preset.label) }}
                </Button>
            </div>
        </header>

        <div class="nbt-tree">
            <CommandGeneratorNbtNode
                v-for="node in nodes"
                :key="node.id"
                :node="node"
                :depth="0"
                @update="onUpdate"
                @remove="onRemove"
                @add-child="onAddChild"
            />
            <button class="nbt-add-root" type="button" @click="addRootNode">
                <PlusIcon />
                {{ formatMessage(messages.addRoot) }}
            </button>
        </div>

        <div v-if="nodes.length" class="nbt-preview">
            <span class="nbt-preview-label">{{ formatMessage(messages.snbtPreview) }}</span>
            <code class="nbt-preview-code">{{ snbtPreview }}</code>
        </div>
    </div>
</template>

<style scoped>
.nbt-editor {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-lg);
    background: var(--surface-2);
    padding: 0.85rem 0.9rem;
}

.nbt-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
}

.nbt-title {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.78rem;
    font-weight: 800;
    color: var(--color-text-primary);
}

.nbt-title :deep(svg) {
    width: 0.9rem;
    height: 0.9rem;
    color: var(--color-brand);
}

.nbt-presets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
}

.nbt-tree {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    border-radius: var(--radius-md);
    background: var(--surface-1);
    padding: 0.55rem 0.6rem;
}

.nbt-add-root {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.3rem;
    border: 1px dashed var(--color-divider);
    border-radius: var(--radius-sm);
    background: transparent;
    padding: 0.4rem 0.55rem;
    color: var(--color-text-tertiary);
    font-size: 0.68rem;
    cursor: pointer;
    transition:
        border-color 0.15s ease,
        color 0.15s ease;
}

.nbt-add-root:hover {
    border-color: var(--color-brand);
    color: var(--color-brand);
}

.nbt-add-root :deep(svg) {
    width: 0.75rem;
    height: 0.75rem;
}

.nbt-preview {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
}

.nbt-preview-label {
    font-size: 0.62rem;
    font-weight: 800;
    text-transform: uppercase;
    color: var(--color-text-tertiary);
}

.nbt-preview-code {
    overflow: auto;
    border-radius: var(--radius-sm);
    background: var(--surface-1);
    padding: 0.55rem 0.65rem;
    color: var(--color-text-secondary);
    font-family: var(--mono-font, monospace);
    font-size: 0.68rem;
    white-space: pre-wrap;
    word-break: break-all;
}
</style>
