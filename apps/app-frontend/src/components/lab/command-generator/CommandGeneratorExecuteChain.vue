<script setup lang="ts">
import {
    ChevronDownIcon,
    ChevronUpIcon,
    PlusIcon,
    SparklesIcon,
    XIcon,
} from '@modrinth/assets'
import { useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import type {
    ExecuteChainCategory,
    ExecuteChainSubcommand,
} from '@/lab/command-generator/commands/types.ts'
import { commandGeneratorMessages as messages } from '@/lab/command-generator/i18n/messages.ts'

import CommandGeneratorArgument from './CommandGeneratorArgument.vue'

interface ChainStep {
    id: number
    subcommandId: string
    values: Record<string, string | number | boolean>
    expanded: boolean
}

export interface ExecuteChainState {
    chain: Array<{
        subcommandId: string
        values: Record<string, string | number | boolean>
    }>
    finalCommand: string
}

const props = defineProps<{
    categories: ExecuteChainCategory[]
    modelValue: string
}>()

const emit = defineEmits<{
    'update:modelValue': [value: string]
}>()

const { formatMessage } = useVIntl()

/** 动态 key 查表；找不到就用 key 本身作为 fallback */
function t(key: string, fallback?: string): string {
    return formatMessage({ id: key, defaultMessage: fallback ?? key })
}

// 修复：nextId 放在 setup 作用域内，每个组件实例独立，避免 HMR 后 id 冲突
let nextId = 1

const chain = ref<ChainStep[]>([])
const finalCommand = ref('')
const finalInputRef = ref<HTMLInputElement | null>(null)

/** 快速预设（整条链） */
const QUICK_PRESETS = [
    {
        labelKey: messages.quickPresetAllSayHello,
        build: () => {
            chain.value = [
                { id: nextId++, subcommandId: 'as', values: { target: '@a' }, expanded: false },
                { id: nextId++, subcommandId: 'at', values: { target: '@s' }, expanded: false },
            ]
            finalCommand.value = 'say hello'
        },
    },
    {
        labelKey: messages.quickPresetCheckZombies,
        build: () => {
            chain.value = [
                {
                    id: nextId++,
                    subcommandId: 'if-entity',
                    values: { target: '@e[type=zombie]' },
                    expanded: false,
                },
            ]
            finalCommand.value = 'say zombies'
        },
    },
    {
        labelKey: messages.quickPresetKillNearbyZombies,
        build: () => {
            chain.value = [
                {
                    id: nextId++,
                    subcommandId: 'store-result-score',
                    values: { target: '@s', objective: 'kills' },
                    expanded: false,
                },
            ]
            finalCommand.value = 'kill @e[type=zombie,distance=..10]'
        },
    },
    {
        labelKey: messages.quickPresetGrassBelow,
        build: () => {
            chain.value = [
                { id: nextId++, subcommandId: 'as', values: { target: '@s' }, expanded: false },
                { id: nextId++, subcommandId: 'at', values: { target: '@s' }, expanded: false },
                {
                    id: nextId++,
                    subcommandId: 'if-block',
                    values: { pos: '~ ~-1 ~', block: 'minecraft:grass_block' },
                    expanded: false,
                },
            ]
            finalCommand.value = 'say grass below'
        },
    },
    {
        labelKey: messages.quickPresetMoveForward,
        build: () => {
            chain.value = [
                { id: nextId++, subcommandId: 'as', values: { target: '@e' }, expanded: false },
                { id: nextId++, subcommandId: 'at', values: { target: '@s' }, expanded: false },
            ]
            finalCommand.value = 'tp ^ ^ ^1'
        },
    },
]

/** 最终命令的快捷前缀（label 本身就是命令名，不需要 i18n） */
const QUICK_FINAL_COMMANDS = [
    { label: 'say' },
    { label: 'tellraw' },
    { label: 'title' },
    { label: 'tp' },
    { label: 'kill' },
    { label: 'give' },
    { label: 'summon' },
    { label: 'setblock' },
    { label: 'fill' },
    { label: 'effect' },
    { label: 'playsound' },
    { label: 'particle' },
    { label: 'scoreboard' },
    { label: 'tag' },
]

/** 所有子命令的 id → 定义 */
const subcommandMap = computed(() => {
    const map = new Map<string, ExecuteChainSubcommand>()
    for (const cat of props.categories) {
        for (const sub of cat.subcommands) {
            map.set(sub.id, sub)
        }
    }
    return map
})

/** 渲染单个 step */
function renderStep(step: ChainStep): string {
    const sub = subcommandMap.value.get(step.subcommandId)
    if (!sub) return ''
    let text = sub.template
    for (const arg of sub.arguments) {
        const value = step.values[arg.name]
        const replacement =
            value !== undefined && value !== null && value !== '' ? String(value) : ''
        // 修复：使用 split/join 替换所有同名占位符，避免 String.replace 只替换首个的问题
        text = text.split(`{${arg.name}}`).join(replacement)
    }
    return text.trim()
}

/** 组装完整输出 */
const output = computed(() => {
    const parts = chain.value.map(renderStep).filter(Boolean)
    const final = finalCommand.value.trim()
    if (final) parts.push('run ' + final)
    return parts.join(' ')
})

watch(
    output,
    (v) => {
        emit('update:modelValue', v)
    },
    { immediate: true },
)

function addSubcommand(sub: ExecuteChainSubcommand) {
    const values: Record<string, string | number | boolean> = {}
    for (const arg of sub.arguments) {
        if (arg.defaultValue !== undefined) {
            values[arg.name] = arg.defaultValue
        }
    }
    chain.value.push({
        id: nextId++,
        subcommandId: sub.id,
        values,
        expanded: true,
    })
}

function removeStep(index: number) {
    chain.value.splice(index, 1)
}

function moveStep(index: number, delta: number) {
    const target = index + delta
    if (target < 0 || target >= chain.value.length) return
    const [item] = chain.value.splice(index, 1)
    chain.value.splice(target, 0, item)
}

function getSubcommand(id: string): ExecuteChainSubcommand | undefined {
    return subcommandMap.value.get(id)
}

function applyQuickPreset(preset: (typeof QUICK_PRESETS)[number]) {
    preset.build()
}

function clearChain() {
    chain.value = []
    finalCommand.value = ''
}

/** 点击常用命令芯片 → 填入前缀，聚焦输入框 */
function applyQuickFinalCommand(prefix: string) {
    finalCommand.value = prefix + ' '
    queueMicrotask(() => {
        finalInputRef.value?.focus()
    })
}

// ============ 对外暴露状态接口（供 preset 使用）============

function getState(): ExecuteChainState {
    return {
        chain: chain.value.map((s) => ({
            subcommandId: s.subcommandId,
            values: { ...s.values },
        })),
        finalCommand: finalCommand.value,
    }
}

function setState(state: ExecuteChainState) {
    chain.value = state.chain.map((s) => ({
        id: nextId++,
        subcommandId: s.subcommandId,
        values: { ...s.values },
        expanded: false,
    }))
    finalCommand.value = state.finalCommand
}

defineExpose({ getState, setState })
</script>

<template>
    <div class="execute-chain">
        <!-- 快速预设 -->
        <div class="chain-presets">
            <span class="chain-presets-label">
                <SparklesIcon />
                {{ formatMessage(messages.executeQuickExamples) }}
            </span>
            <div class="chain-preset-list">
                <button
                    v-for="preset in QUICK_PRESETS"
                    :key="preset.labelKey.id"
                    type="button"
                    class="preset-btn"
                    @click="applyQuickPreset(preset)"
                >
                    {{ formatMessage(preset.labelKey) }}
                </button>
                <button
                    v-if="chain.length || finalCommand"
                    type="button"
                    class="preset-btn preset-btn--clear"
                    @click="clearChain"
                >
                    {{ formatMessage(messages.executeClear) }}
                </button>
            </div>
        </div>

        <!-- 已添加的链 -->
        <div class="chain-section">
            <div class="chain-section-head">
                <span class="chain-section-title">
                    {{ formatMessage(messages.executeChainTitle) }}
                </span>
                <span class="chain-count">{{ chain.length }}</span>
            </div>

            <p v-if="!chain.length" class="chain-empty">
                {{ formatMessage(messages.executeChainEmpty) }}
            </p>

            <div v-else class="chain-steps">
                <div
                    v-for="(step, index) in chain"
                    :key="step.id"
                    class="chain-step"
                    :class="{ 'chain-step--expanded': step.expanded }"
                >
                    <div class="chain-step-head">
                        <span class="chain-step-index">{{ index + 1 }}</span>
                        <span class="chain-step-label">
                            {{
                                getSubcommand(step.subcommandId)
                                    ? t(getSubcommand(step.subcommandId)!.labelKey)
                                    : step.subcommandId
                            }}
                        </span>
                        <div class="chain-step-actions">
                            <button
                                type="button"
                                class="icon-btn"
                                :disabled="index === 0"
                                @click="moveStep(index, -1)"
                            >
                                <ChevronUpIcon />
                            </button>
                            <button
                                type="button"
                                class="icon-btn"
                                :disabled="index === chain.length - 1"
                                @click="moveStep(index, 1)"
                            >
                                <ChevronDownIcon />
                            </button>
                            <button
                                type="button"
                                class="icon-btn"
                                @click="step.expanded = !step.expanded"
                            >
                                <ChevronDownIcon
                                    :style="{
                                        transform: step.expanded
                                            ? 'rotate(180deg)'
                                            : 'none',
                                        transition: 'transform 0.15s ease',
                                    }"
                                />
                            </button>
                            <button
                                type="button"
                                class="icon-btn icon-btn--danger"
                                @click="removeStep(index)"
                            >
                                <XIcon />
                            </button>
                        </div>
                    </div>
                    <div class="chain-step-preview">
                        <code>{{ renderStep(step) }}</code>
                    </div>
                    <div v-if="step.expanded" class="chain-step-body">
                        <CommandGeneratorArgument
                            v-for="arg in getSubcommand(step.subcommandId)?.arguments ?? []"
                            :key="arg.name"
                            v-model="step.values[arg.name]"
                            :arg="arg"
                        />
                    </div>
                </div>
            </div>
        </div>

        <!-- 子命令选择器 -->
        <div class="chain-section">
            <div class="chain-section-head">
                <span class="chain-section-title">
                    {{ formatMessage(messages.executePickerTitle) }}
                </span>
            </div>

            <div class="picker-categories">
                <div
                    v-for="cat in categories"
                    :key="cat.id"
                    class="picker-category"
                >
                    <span class="picker-category-label">{{ t(cat.labelKey) }}</span>
                    <div class="picker-category-grid">
                        <button
                            v-for="sub in cat.subcommands"
                            :key="sub.id"
                            type="button"
                            class="picker-btn"
                            :title="t(sub.descriptionKey)"
                            @click="addSubcommand(sub)"
                        >
                            <PlusIcon />
                            <span>{{ t(sub.labelKey) }}</span>
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- 最终执行的命令 -->
        <div class="chain-section">
            <div class="chain-section-head">
                <span class="chain-section-title">
                    {{ formatMessage(messages.executeFinalTitle) }}
                </span>
            </div>

            <div class="quick-commands">
                <span class="quick-commands-label">
                    {{ formatMessage(messages.executeCommonCommands) }}
                </span>
                <div class="quick-commands-list">
                    <button
                        v-for="qc in QUICK_FINAL_COMMANDS"
                        :key="qc.label"
                        type="button"
                        class="quick-command-chip"
                        @click="applyQuickFinalCommand(qc.label)"
                    >
                        /{{ qc.label }}
                    </button>
                </div>
            </div>

            <input
                ref="finalInputRef"
                v-model="finalCommand"
                class="final-input"
                :placeholder="formatMessage(messages.executeFinalPlaceholder)"
                spellcheck="false"
            />
            <p class="final-hint">{{ formatMessage(messages.executeFinalHint) }}</p>
        </div>
    </div>
</template>

<style scoped>
.execute-chain {
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.chain-presets {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.chain-presets-label {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.72rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-brand);
}

.chain-presets-label :deep(svg) {
    width: 0.9rem;
    height: 0.9rem;
}

.chain-preset-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
}

.preset-btn {
    border: 1px solid var(--color-divider);
    border-radius: 999px;
    background: var(--surface-1);
    padding: 0.35rem 0.8rem;
    color: var(--color-text-secondary);
    font-size: 0.72rem;
    cursor: pointer;
    transition:
        border-color 0.15s ease,
        background 0.15s ease,
        color 0.15s ease;
}

.preset-btn:hover {
    border-color: var(--color-brand);
    background: var(--surface-4);
    color: var(--color-brand);
}

.preset-btn--clear {
    border-color: var(--color-red);
    color: var(--color-red);
}

.preset-btn--clear:hover {
    border-color: var(--color-red);
    color: var(--color-red);
}

.chain-section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-lg);
    background: var(--surface-3);
    padding: 1rem 1.1rem;
}

.chain-section-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.chain-section-title {
    font-size: 0.72rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-tertiary);
}

.chain-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.25rem;
    height: 1.25rem;
    padding: 0 0.4rem;
    border-radius: 999px;
    background: var(--surface-4);
    color: var(--color-text-tertiary);
    font-size: 0.65rem;
    font-weight: 700;
}

.chain-empty {
    margin: 0;
    border: 1px dashed var(--color-divider);
    border-radius: var(--radius-md);
    padding: 0.7rem 0.85rem;
    font-size: 0.72rem;
    color: var(--color-text-tertiary);
}

.chain-steps {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.chain-step {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-2);
    padding: 0.65rem 0.75rem;
}

.chain-step--expanded {
    border-color: var(--color-brand);
}

.chain-step-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.chain-step-index {
    display: inline-flex;
    width: 1.35rem;
    height: 1.35rem;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: var(--surface-4);
    font-size: 0.65rem;
    font-weight: 800;
    color: var(--color-text-tertiary);
}

.chain-step-label {
    flex: 1;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--color-text-primary);
}

.chain-step-actions {
    display: flex;
    gap: 0.15rem;
}

.chain-step-preview {
    border-radius: var(--radius-sm);
    background: var(--surface-1);
    padding: 0.35rem 0.6rem;
}

.chain-step-preview code {
    font-family: var(--mono-font, monospace);
    font-size: 0.72rem;
    color: var(--color-brand);
}

.chain-step-body {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
    gap: 0.5rem;
    padding-top: 0.35rem;
    border-top: 1px solid var(--color-divider);
}

.icon-btn {
    display: inline-flex;
    width: 1.5rem;
    height: 1.5rem;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    transition:
        background 0.15s ease,
        color 0.15s ease;
}

.icon-btn:hover:not(:disabled) {
    background: var(--surface-4);
    color: var(--color-text-primary);
}

.icon-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
}

.icon-btn--danger:hover:not(:disabled) {
    color: var(--color-red);
}

.icon-btn :deep(svg) {
    width: 0.75rem;
    height: 0.75rem;
}

.picker-categories {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
}

.picker-category {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
}

.picker-category-label {
    font-size: 0.68rem;
    font-weight: 700;
    color: var(--color-text-tertiary);
}

.picker-category-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
    gap: 0.4rem;
}

.picker-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-1);
    padding: 0.45rem 0.7rem;
    color: var(--color-text-primary);
    font-size: 0.72rem;
    text-align: left;
    cursor: pointer;
    transition:
        border-color 0.15s ease,
        background 0.15s ease;
}

.picker-btn:hover {
    border-color: var(--color-brand);
    background: var(--surface-4);
    color: var(--color-brand);
}

.picker-btn :deep(svg) {
    width: 0.7rem;
    height: 0.7rem;
    flex: 0 0 auto;
    color: var(--color-brand);
}

.picker-btn span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.quick-commands {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
}

.quick-commands-label {
    font-size: 0.65rem;
    font-weight: 700;
    color: var(--color-text-tertiary);
}

.quick-commands-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
}

.quick-command-chip {
    border: 1px solid var(--color-divider);
    border-radius: 999px;
    background: var(--surface-1);
    padding: 0.25rem 0.6rem;
    color: var(--color-text-secondary);
    font-family: var(--mono-font, monospace);
    font-size: 0.68rem;
    cursor: pointer;
    transition:
        border-color 0.15s ease,
        background 0.15s ease,
        color 0.15s ease;
}

.quick-command-chip:hover {
    border-color: var(--color-brand);
    background: var(--surface-4);
    color: var(--color-brand);
}

.final-input {
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-1);
    padding: 0.6rem 0.8rem;
    color: var(--color-text-primary);
    font-family: var(--mono-font, monospace);
    font-size: 0.82rem;
    outline: none;
    transition: border-color 0.15s ease;
}

.final-input:focus {
    border-color: var(--color-brand);
}

.final-hint {
    margin: 0;
    font-size: 0.66rem;
    color: var(--color-text-tertiary);
}
</style>
