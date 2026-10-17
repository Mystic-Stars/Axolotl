<script setup lang="ts">
import { BookmarkIcon, CheckCircleIcon, SparklesIcon, XIcon } from '@modrinth/assets'
import { Button, Combobox, type ComboboxOption, Toggle, useVIntl } from '@modrinth/ui'
import { computed, nextTick, onMounted, ref, watch } from 'vue'

import CommandGeneratorArgument from '@/components/lab/command-generator/CommandGeneratorArgument.vue'
import CommandGeneratorExecuteChain, {
    type ExecuteChainState,
} from '@/components/lab/command-generator/CommandGeneratorExecuteChain.vue'
import { findGuide } from '@/lab/command-generator/command-guides.ts'
import {
    CATEGORY_KEYS,
    PERMISSION_KEYS,
    getAllCommands,
    findCommand,
    getCurrentVersion,
    initializeRegistry,
    switchVersion,
} from '@/lab/command-generator/commands/registry.ts'
import type { CommandArgument, CommandCategory } from '@/lab/command-generator/commands/types.ts'
import {
    buildCommand,
    getDefaultValues,
    validateCommand,
} from '@/lab/command-generator/generator.ts'
import { commandGeneratorMessages as messages } from '@/lab/command-generator/i18n/messages.ts'
import { loadManifest } from '@/lab/command-generator/loader.ts'
import {
    applyPresetToValues,
    deleteUserPreset,
    getPresetsForCommand,
    getUserPresets,
    saveUserPreset,
    type UserPreset,
} from '@/lab/command-generator/presets.ts'

const { formatMessage } = useVIntl()

function t(key: string, fallback?: string): string {
    return formatMessage({ id: key, defaultMessage: fallback ?? key })
}

type CommandComboboxOption = ComboboxOption<string> & {
    description: string
    category: string
}

function asCommandOption(item: ComboboxOption<string>): CommandComboboxOption {
    return item as CommandComboboxOption
}

// ============= 状态 =============
const versionOptions = ref<ComboboxOption<string>[]>([])
const selectedVersion = ref('')
const selectedCommandId = ref('')
const filterCategory = ref<string>('')
const filterPermission = ref<string>('')
const values = ref<Record<string, string | number | boolean>>({})
const executeChainOutput = ref('')
const executeChainRef = ref<InstanceType<typeof CommandGeneratorExecuteChain> | null>(null)
const result = ref('')
const errors = ref<string[]>([])
const copied = ref(false)
const presetNameInput = ref('')
const showPresetInput = ref(false)
const toast = ref('')
const showGlobalPresets = ref(false)

const userPresets = ref<UserPreset[]>([])

/**
 * 1.20.x 下用户可选择是否使用 1.20.5+ 数据组件语法。
 * 其它版本不显示该开关。
 */
const useDataComponents = ref(true)

// ============= 版本比较 =============
function isAtLeast(a: string, b: string): boolean {
    const av = a.split('.').map((n) => Number.parseInt(n, 10) || 0)
    const bv = b.split('.').map((n) => Number.parseInt(n, 10) || 0)
    const len = Math.max(av.length, bv.length)
    for (let i = 0; i < len; i++) {
        const x = av[i] ?? 0
        const y = bv[i] ?? 0
        if (x !== y) return x > y
    }
    return true
}

// 仅 1.20.x（不含 1.20.5+）显示切换开关
const showDataComponentsToggle = computed(() => {
    const v = selectedVersion.value
    if (!v) return false
    if (!(v === '1.20' || v.startsWith('1.20.'))) return false
    return !isAtLeast(v, '1.20.5')
})

// 传递给 NBT 编辑器的 forceDataComponents：
// - 未显示开关时传 undefined，让组件根据 version 自动判定
// - 显示开关时传用户选择
const forceDataComponents = computed<boolean | undefined>(() => {
    if (!showDataComponentsToggle.value) return undefined
    return useDataComponents.value
})

// ============= 筛选器选项 =============
const categoryOptions = computed<ComboboxOption<string>[]>(() => [
    { value: '', label: formatMessage(messages.filterAll) },
    ...(Object.keys(CATEGORY_KEYS) as CommandCategory[]).map((key) => ({
        value: key,
        label: t(CATEGORY_KEYS[key]),
    })),
])

const permissionOptions = computed<ComboboxOption<string>[]>(() => [
    { value: '', label: formatMessage(messages.filterAll) },
    { value: '0', label: `0 · ${t(PERMISSION_KEYS[0])}` },
    { value: '1', label: `1 · ${t(PERMISSION_KEYS[1])}` },
    { value: '2', label: `2 · ${t(PERMISSION_KEYS[2])}` },
    { value: '3', label: `3 · ${t(PERMISSION_KEYS[3])}` },
    { value: '4', label: `4 · ${t(PERMISSION_KEYS[4])}` },
])

const versionLabelMap = computed<Record<string, string>>(() => {
    const map: Record<string, string> = {}
    for (const opt of versionOptions.value) {
        map[String(opt.value)] = String(opt.label)
    }
    return map
})

// ============= 命令列表（带筛选） =============
const filteredCommands = computed(() => {
    let list = getAllCommands()
    if (filterCategory.value) {
        list = list.filter((c) => c.category === filterCategory.value)
    }
    if (filterPermission.value !== '') {
        list = list.filter((c) => String(c.permissionLevel) === filterPermission.value)
    }
    return list
})

const commandOptions = computed<CommandComboboxOption[]>(() =>
    filteredCommands.value.map((cmd) => ({
        value: cmd.id,
        label: cmd.name,
        description: t(cmd.descriptionKey),
        category: t(CATEGORY_KEYS[cmd.category]),
    })),
)

// ============= 当前选中 =============
const currentCommand = computed(() =>
    selectedCommandId.value ? (findCommand(selectedCommandId.value) ?? null) : null,
)

const currentGuide = computed(() =>
    selectedCommandId.value ? (findGuide(selectedCommandId.value) ?? null) : null,
)

/**
 * UI 上用于渲染的参数卡片顺序。
 * 把 nbt 类型的参数统一挪到末尾，与 JSON 中参数的位置无关。
 * 生成命令时的顺序由 generator.ts 的 orderArguments 另行处理。
 */
const uiArguments = computed<CommandArgument[]>(() => {
    const args = currentCommand.value?.arguments ?? []
    const nonNbt: CommandArgument[] = []
    const nbt: CommandArgument[] = []
    for (const arg of args) {
        if (arg.type === 'nbt') nbt.push(arg)
        else nonNbt.push(arg)
    }
    return [...nonNbt, ...nbt]
})

const permissionName = computed(() =>
    currentCommand.value
        ? t(PERMISSION_KEYS[currentCommand.value.permissionLevel])
        : '',
)

const currentPresets = computed<UserPreset[]>(() => {
    if (!selectedCommandId.value) return []
    return userPresets.value.filter((p) => p.commandId === selectedCommandId.value)
})

const isChainCommand = computed(() => currentCommand.value?.isChainCommand === true)

// ============= 全局预设列表 =============
const allPresets = computed<UserPreset[]>(() =>
    [...userPresets.value].sort((a, b) => b.createdAt - a.createdAt),
)

// ============= 生命周期 =============
onMounted(() => {
    const manifest = loadManifest()
    versionOptions.value = manifest.versions.map((v) => ({
        value: v.id,
        label: v.label,
    }))
    selectedVersion.value = manifest.defaultVersion
    initializeRegistry(manifest.defaultVersion)
    userPresets.value = getUserPresets()
})

watch(selectedVersion, (versionId) => {
    if (!versionId) return
    switchVersion(versionId)
    selectedCommandId.value = ''
    values.value = {}
    executeChainOutput.value = ''
    result.value = ''
    // 切换版本时重置数据组件开关
    useDataComponents.value = true
})

watch(selectedCommandId, () => {
    userPresets.value = getUserPresets()
})

// ============= 工具函数 =============
function showToast(message: string) {
    toast.value = message
    setTimeout(() => {
        toast.value = ''
    }, 1800)
}

function onCommandChange(id: unknown) {
    selectedCommandId.value = String(id)
    copied.value = false
    executeChainOutput.value = ''
    if (!selectedCommandId.value) {
        values.value = {}
        result.value = ''
        errors.value = []
        return
    }
    values.value = getDefaultValues(selectedCommandId.value)
    result.value = ''
    errors.value = []
}

function generate() {
    if (!selectedCommandId.value || !currentCommand.value) return
    copied.value = false

    if (isChainCommand.value) {
        const output = executeChainOutput.value.trim()
        if (!output) {
            errors.value = [formatMessage(messages.executeBuildFirst)]
            result.value = ''
            return
        }
        errors.value = []
        result.value = `/${selectedCommandId.value} ${output}`
        return
    }

    const validationErrors = validateCommand(selectedCommandId.value, values.value)
    if (validationErrors.length) {
        errors.value = validationErrors
        result.value = ''
        return
    }
    errors.value = []
    try {
        result.value = buildCommand(selectedCommandId.value, values.value)
    } catch (e) {
        errors.value = [e instanceof Error ? e.message : 'Unknown error']
        result.value = ''
    }
}

async function copy() {
    if (!result.value) return
    try {
        await navigator.clipboard.writeText(result.value)
        copied.value = true
        setTimeout(() => {
            copied.value = false
        }, 1600)
    } catch {
        // 剪贴板不可用时静默失败
    }
}

function saveCurrentAsPreset() {
    if (!selectedCommandId.value || !currentCommand.value) return
    const name = presetNameInput.value.trim()
    if (!name) {
        showToast(formatMessage(messages.toastPresetNameRequired))
        return
    }

    if (isChainCommand.value) {
        const state = executeChainRef.value?.getState()
        if (!state) {
            showToast('Chain builder not ready')
            return
        }
        saveUserPreset(
            name,
            selectedCommandId.value,
            getCurrentVersion(),
            {},
            {
                executeState: state,
                chainString: executeChainOutput.value,
            },
        )
    } else {
        const snapshot: Record<string, string | number | boolean> = {}
        for (const arg of currentCommand.value.arguments) {
            snapshot[arg.name] = values.value[arg.name] ?? ''
        }
        saveUserPreset(name, selectedCommandId.value, getCurrentVersion(), snapshot)
    }

    userPresets.value = getUserPresets()

    presetNameInput.value = ''
    showPresetInput.value = false
    showToast(formatMessage(messages.toastPresetSaved, { name }))
}

function loadPreset(preset: UserPreset) {
    if (!currentCommand.value) return

    if (isChainCommand.value) {
        if (preset.executeState) {
            executeChainRef.value?.setState(preset.executeState)
        } else if (preset.chainString) {
            executeChainOutput.value = preset.chainString
        }
    } else {
        values.value = applyPresetToValues(preset, currentCommand.value)
    }
    showToast(formatMessage(messages.toastPresetLoaded, { name: preset.name }))
}

function removePreset(id: string) {
    const preset = userPresets.value.find((p) => p.id === id)
    if (!preset) return

    deleteUserPreset(id)
    userPresets.value = getUserPresets()

    showToast(formatMessage(messages.toastPresetDeleted, { name: preset.name }))
}

async function jumpToPreset(preset: UserPreset) {
    if (selectedVersion.value !== preset.versionId) {
        selectedVersion.value = preset.versionId
        switchVersion(preset.versionId)
    }

    filterCategory.value = ''
    filterPermission.value = ''

    selectedCommandId.value = preset.commandId
    const cmd = findCommand(preset.commandId)
    if (!cmd) {
        showToast(
            formatMessage(messages.toastPresetCommandMissing, {
                name: preset.name,
                command: preset.commandId,
            }),
        )
        showGlobalPresets.value = false
        return
    }

    await nextTick()
    if (cmd.isChainCommand) {
        if (preset.executeState) {
            executeChainRef.value?.setState(preset.executeState)
        } else if (preset.chainString) {
            executeChainOutput.value = preset.chainString
        }
    } else {
        values.value = applyPresetToValues(preset, cmd)
    }
    result.value = ''
    errors.value = []

    showGlobalPresets.value = false
    showToast(formatMessage(messages.toastPresetLoaded, { name: preset.name }))
}

function presetCategoryLabel(commandId: string): string {
    const cmd = findCommand(commandId)
    if (!cmd) return commandId
    return `${t(CATEGORY_KEYS[cmd.category])} · /${cmd.name}`
}
</script>

<template>
    <div class="command-generator">
        <!-- ========== 标题栏 ========== -->
        <header class="page-header">
            <div class="header-titles">
                <h1>{{ formatMessage(messages.title) }}</h1>
                <p>{{ formatMessage(messages.subtitle) }}</p>
            </div>
            <button
                type="button"
                class="global-presets-button"
                @click="showGlobalPresets = !showGlobalPresets"
            >
                <BookmarkIcon />
                {{ formatMessage(messages.globalPresets) }}
                <span class="badge">{{ allPresets.length }}</span>
            </button>
            <span v-if="currentCommand" class="permission-chip">{{ permissionName }}</span>
        </header>

        <!-- ========== 全局预设面板 ========== -->
        <Transition name="slide-fade">
            <section v-if="showGlobalPresets" class="global-presets-panel">
                <header class="global-presets-head">
                    <span class="global-presets-title">
                        {{ formatMessage(messages.globalPresets) }}
                    </span>
                    <span class="global-presets-hint">
                        {{ formatMessage(messages.globalPresetsHint) }}
                    </span>
                    <button
                        type="button"
                        class="close-btn"
                        @click="showGlobalPresets = false"
                    >
                        <XIcon />
                    </button>
                </header>

                <p v-if="!allPresets.length" class="global-presets-empty">
                    {{ formatMessage(messages.globalPresetsEmpty) }}
                </p>

                <div v-else class="global-presets-list">
                    <button
                        v-for="preset in allPresets"
                        :key="preset.id"
                        type="button"
                        class="global-preset-item"
                        @click="jumpToPreset(preset)"
                    >
                        <div class="global-preset-main">
                            <span class="global-preset-name">{{ preset.name }}</span>
                            <span class="global-preset-meta">
                                {{ presetCategoryLabel(preset.commandId) }}
                            </span>
                        </div>
                        <div class="global-preset-side">
                            <span class="global-preset-version">
                                {{ versionLabelMap[preset.versionId] ?? preset.versionId }}
                            </span>
                            <span
                                class="global-preset-delete"
                                @click.stop="removePreset(preset.id)"
                            >
                                <XIcon />
                            </span>
                        </div>
                    </button>
                </div>
            </section>
        </Transition>

        <!-- ========== 版本选择 ========== -->
        <section class="panel">
            <label class="field-label">{{ formatMessage(messages.selectVersion) }}</label>
            <Combobox
                :model-value="selectedVersion"
                :options="versionOptions"
                @update:model-value="selectedVersion = String($event)"
            />
            <!-- 1.20.x 专属：数据组件切换 -->
            <div v-if="showDataComponentsToggle" class="version-toggle-row">
                <div class="version-toggle-text">
                    <span class="version-toggle-label">
                        {{ t('app.lab.command-generator.version.use-data-components') }}
                    </span>
                    <span class="version-toggle-hint">
                        {{ t('app.lab.command-generator.version.use-data-components-hint') }}
                    </span>
                </div>
                <Toggle v-model="useDataComponents" />
            </div>
        </section>

        <!-- ========== 筛选器 ========== -->
        <section class="panel">
            <div class="filter-row">
                <div class="filter-item">
                    <label class="field-label">
                        {{ formatMessage(messages.filterCategory) }}
                    </label>
                    <Combobox
                        :model-value="filterCategory"
                        :options="categoryOptions"
                        @update:model-value="filterCategory = String($event)"
                    />
                </div>
                <div class="filter-item">
                    <label class="field-label">
                        {{ formatMessage(messages.filterPermission) }}
                    </label>
                    <Combobox
                        :model-value="filterPermission"
                        :options="permissionOptions"
                        @update:model-value="filterPermission = String($event)"
                    />
                </div>
            </div>
        </section>

        <!-- ========== 命令选择 ========== -->
        <section class="panel">
            <label class="field-label">{{ formatMessage(messages.selectCommand) }}</label>
            <Combobox
                :model-value="selectedCommandId"
                :options="commandOptions"
                searchable
                @update:model-value="onCommandChange"
            >
                <template #selected="{ label }">
                    <span v-if="label" class="truncate">{{ label }}</span>
                    <span v-else class="placeholder">
                        {{ formatMessage(messages.selectPlaceholder) }}
                    </span>
                </template>
                <template #option="{ item, isSelected }">
                    <div class="option-row">
                        <span class="option-category">{{ asCommandOption(item).category }}</span>
                        <div class="option-text">
                            <strong :class="isSelected ? 'text-brand' : ''">
                                /{{ item.label }}
                            </strong>
                            <span>{{ asCommandOption(item).description }}</span>
                        </div>
                    </div>
                </template>
            </Combobox>
        </section>

        <!-- ========== 汉化指导 ========== -->
        <section v-if="currentGuide" class="guide-card">
            <div class="guide-head">
                <SparklesIcon />
                <span>{{ t('app.lab.command-generator.guide-title') }}</span>
            </div>
            <p class="guide-summary">{{ t(currentGuide.summaryKey) }}</p>
            <div class="guide-block">
                <span class="guide-label">
                    {{ t('app.lab.command-generator.guide-examples') }}
                </span>
                <div class="example-list">
                    <div
                        v-for="ex in currentGuide.examples"
                        :key="ex.command"
                        class="example-row"
                    >
                        <code class="example-code">{{ ex.command }}</code>
                        <span class="example-desc">{{ t(ex.explanationKey) }}</span>
                    </div>
                </div>
            </div>
            <div v-if="currentGuide.tipKeys?.length" class="guide-block">
                <span class="guide-label">
                    {{ t('app.lab.command-generator.guide-tips') }}
                </span>
                <ul class="tip-list">
                    <li v-for="tipKey in currentGuide.tipKeys" :key="tipKey">
                        {{ t(tipKey) }}
                    </li>
                </ul>
            </div>
        </section>

        <!-- ========== 参数区 ========== -->
        <section v-if="currentCommand" class="parameters-section">
            <h2 class="section-title">{{ formatMessage(messages.parametersTitle) }}</h2>

            <CommandGeneratorExecuteChain
                v-if="isChainCommand && currentCommand.chainCategories"
                ref="executeChainRef"
                :categories="currentCommand.chainCategories"
                v-model="executeChainOutput"
            />

            <template v-else>
                <div class="preset-toolbar">
                    <Button
                        type="outlined"
                        size="2xs"
                        @click="showPresetInput = !showPresetInput"
                    >
                        <BookmarkIcon />
                        {{ formatMessage(messages.savePreset) }}
                    </Button>
                    <div v-if="showPresetInput" class="preset-input-row">
                        <input
                            v-model="presetNameInput"
                            class="preset-name-input"
                            :placeholder="formatMessage(messages.presetName)"
                            @keydown.enter="saveCurrentAsPreset"
                        />
                        <Button
                            type="colored"
                            color="brand"
                            size="2xs"
                            @click="saveCurrentAsPreset"
                        >
                            OK
                        </Button>
                    </div>
                </div>

                <div v-if="currentPresets.length" class="user-presets">
                    <span class="user-presets-label">
                        {{ formatMessage(messages.myPresets) }}
                    </span>
                    <div class="user-preset-list">
                        <button
                            v-for="preset in currentPresets"
                            :key="preset.id"
                            type="button"
                            class="user-preset-chip"
                            @click="loadPreset(preset)"
                        >
                            {{ preset.name }}
                            <span
                                class="preset-delete"
                                @click.stop="removePreset(preset.id)"
                            >
                                <XIcon />
                            </span>
                        </button>
                    </div>
                </div>

                <div v-if="uiArguments.length" class="arguments-grid">
                    <CommandGeneratorArgument
                        v-for="arg in uiArguments"
                        :key="arg.name"
                        v-model="values[arg.name]"
                        :arg="arg"
                        :version="selectedVersion"
                        :force-data-components="forceDataComponents"
                    />
                </div>
                <p v-else class="empty-hint">
                    {{ formatMessage(messages.parametersEmpty) }}
                </p>
            </template>

            <template v-if="isChainCommand">
                <div class="preset-toolbar">
                    <Button
                        type="outlined"
                        size="2xs"
                        @click="showPresetInput = !showPresetInput"
                    >
                        <BookmarkIcon />
                        {{ formatMessage(messages.savePreset) }}
                    </Button>
                    <div v-if="showPresetInput" class="preset-input-row">
                        <input
                            v-model="presetNameInput"
                            class="preset-name-input"
                            :placeholder="formatMessage(messages.presetName)"
                            @keydown.enter="saveCurrentAsPreset"
                        />
                        <Button
                            type="colored"
                            color="brand"
                            size="2xs"
                            @click="saveCurrentAsPreset"
                        >
                            OK
                        </Button>
                    </div>
                </div>

                <div v-if="currentPresets.length" class="user-presets">
                    <span class="user-presets-label">
                        {{ formatMessage(messages.myPresets) }}
                    </span>
                    <div class="user-preset-list">
                        <button
                            v-for="preset in currentPresets"
                            :key="preset.id"
                            type="button"
                            class="user-preset-chip"
                            @click="loadPreset(preset)"
                        >
                            {{ preset.name }}
                            <span
                                class="preset-delete"
                                @click.stop="removePreset(preset.id)"
                            >
                                <XIcon />
                            </span>
                        </button>
                    </div>
                </div>
            </template>
        </section>

        <div v-if="errors.length" class="error-panel">
            <XIcon />
            <ul>
                <li v-for="(err, i) in errors" :key="i">{{ err }}</li>
            </ul>
        </div>

        <div v-if="currentCommand" class="action-bar">
            <Button type="colored" color="brand" @click="generate">
                <SparklesIcon />
                {{ formatMessage(messages.generate) }}
            </Button>
        </div>

        <section v-if="result" class="result-card">
            <div class="result-head">
                <span class="result-badge"><CheckCircleIcon /></span>
                <span class="result-title">{{ formatMessage(messages.result) }}</span>
                <Button type="outlined" size="2xs" @click="copy">
                    {{ copied ? formatMessage(messages.copied) : formatMessage(messages.copy) }}
                </Button>
            </div>
            <pre class="result-code">{{ result }}</pre>
        </section>

        <Transition name="toast-fade">
            <div v-if="toast" class="toast">{{ toast }}</div>
        </Transition>
    </div>
</template>

<style scoped>
.command-generator {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    min-width: 0;
    width: 100%;
    padding: 1rem 1.25rem 3rem;
}

.page-header {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    padding: 0.25rem 0;
}

.header-titles {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: 0.2rem;
}

.header-titles h1 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 800;
    color: var(--color-text-primary);
}

.header-titles p {
    margin: 0;
    font-size: 0.75rem;
    color: var(--color-text-tertiary);
}

.permission-chip {
    flex: 0 0 auto;
    border-radius: 999px;
    background: var(--surface-4);
    border: 1px solid var(--color-divider);
    padding: 0.25rem 0.75rem;
    font-size: 0.7rem;
    font-weight: 700;
    color: var(--color-brand);
}

.global-presets-button {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-3);
    padding: 0.45rem 0.85rem;
    color: var(--color-text-primary);
    font-size: 0.75rem;
    font-weight: 700;
    cursor: pointer;
    transition: background 0.15s ease;
}

.global-presets-button:hover {
    background: var(--surface-4);
}

.global-presets-button :deep(svg) {
    width: 0.9rem;
    height: 0.9rem;
}

.global-presets-button .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.25rem;
    height: 1.25rem;
    padding: 0 0.35rem;
    border-radius: 999px;
    background: var(--color-brand);
    color: #fff;
    font-size: 0.65rem;
    font-weight: 800;
}

.global-presets-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    border: 1px solid var(--color-brand);
    border-radius: var(--radius-lg);
    background: var(--surface-3);
    padding: 1rem 1.1rem;
}

.global-presets-head {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    flex-wrap: wrap;
}

.global-presets-title {
    font-size: 0.85rem;
    font-weight: 800;
    color: var(--color-text-primary);
}

.global-presets-hint {
    flex: 1;
    font-size: 0.7rem;
    color: var(--color-text-tertiary);
}

.close-btn {
    display: inline-flex;
    width: 1.75rem;
    height: 1.75rem;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
}

.close-btn:hover {
    background: var(--surface-4);
    color: var(--color-text-primary);
}

.close-btn :deep(svg) {
    width: 0.85rem;
    height: 0.85rem;
}

.global-presets-empty {
    margin: 0;
    padding: 0.85rem 0;
    font-size: 0.74rem;
    color: var(--color-text-tertiary);
    text-align: center;
}

.global-presets-list {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    max-height: 22rem;
    overflow-y: auto;
}

.global-preset-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-2);
    padding: 0.65rem 0.85rem;
    text-align: left;
    cursor: pointer;
    transition:
        border-color 0.15s ease,
        background 0.15s ease;
}

.global-preset-item:hover {
    border-color: var(--color-brand);
    background: var(--surface-4);
}

.global-preset-main {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: 0.15rem;
}

.global-preset-name {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--color-text-primary);
}

.global-preset-meta {
    font-size: 0.68rem;
    color: var(--color-text-tertiary);
}

.global-preset-side {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 0 0 auto;
}

.global-preset-version {
    border-radius: 999px;
    background: var(--surface-4);
    padding: 0.2rem 0.55rem;
    font-size: 0.65rem;
    font-weight: 700;
    color: var(--color-text-tertiary);
}

.global-preset-delete {
    display: inline-flex;
    width: 1.5rem;
    height: 1.5rem;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: transparent;
    color: var(--color-text-tertiary);
    transition: color 0.15s ease;
}

.global-preset-delete:hover {
    color: var(--color-red);
}

.global-preset-delete :deep(svg) {
    width: 0.7rem;
    height: 0.7rem;
}

.panel {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-lg);
    background: var(--surface-3);
    padding: 1rem 1.1rem;
}

.field-label {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--color-text-primary);
}

.placeholder {
    color: var(--color-text-tertiary);
}

/* ===== 版本切换行（仅 1.20.x） ===== */
.version-toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-top: 0.4rem;
    border-top: 1px solid var(--color-divider);
    padding-top: 0.7rem;
}

.version-toggle-text {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.15rem;
}

.version-toggle-label {
    font-size: 0.76rem;
    font-weight: 700;
    color: var(--color-text-primary);
}

.version-toggle-hint {
    font-size: 0.66rem;
    line-height: 1.5;
    color: var(--color-text-tertiary);
}

.filter-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.85rem;
}

.filter-item {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
}

.option-row {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.65rem;
}

.option-category {
    flex: 0 0 auto;
    border-radius: 999px;
    background: var(--surface-4);
    padding: 0.18rem 0.55rem;
    font-size: 0.63rem;
    font-weight: 700;
    color: var(--color-text-tertiary);
}

.option-text {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.12rem;
}

.option-text strong {
    font-size: 0.84rem;
    font-weight: 700;
    color: var(--color-text-primary);
}

.option-text span {
    overflow: hidden;
    font-size: 0.7rem;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-tertiary);
}

.guide-card {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    border: 1px solid var(--color-divider);
    border-left: 3px solid var(--color-brand);
    border-radius: var(--radius-lg);
    background: var(--surface-3);
    padding: 1rem 1.1rem;
}

.guide-head {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.72rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-brand);
}

.guide-head :deep(svg) {
    width: 0.9rem;
    height: 0.9rem;
}

.guide-summary {
    margin: 0;
    font-size: 0.8rem;
    line-height: 1.6;
    color: var(--color-text-secondary);
}

.guide-block {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
}

.guide-label {
    font-size: 0.66rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-tertiary);
}

.example-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
}

.example-row {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    border-radius: var(--radius-md);
    background: var(--surface-2);
    padding: 0.55rem 0.7rem;
}

.example-code {
    font-family: var(--mono-font, monospace);
    font-size: 0.74rem;
    color: var(--color-brand);
    word-break: break-all;
}

.example-desc {
    font-size: 0.7rem;
    color: var(--color-text-tertiary);
}

.tip-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin: 0;
    padding-left: 1.1rem;
    font-size: 0.72rem;
    line-height: 1.55;
    color: var(--color-text-tertiary);
}

.parameters-section {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
}

.section-title {
    margin: 0;
    font-size: 0.72rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-tertiary);
}

.arguments-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
    gap: 0.8rem;
}

.empty-hint {
    margin: 0;
    border: 1px dashed var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-3);
    padding: 0.85rem 1rem;
    font-size: 0.74rem;
    color: var(--color-text-tertiary);
}

.preset-toolbar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
}

.preset-input-row {
    display: flex;
    gap: 0.4rem;
    flex: 1;
    min-width: 14rem;
}

.preset-name-input {
    flex: 1;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-sm);
    background: var(--surface-1);
    padding: 0.35rem 0.6rem;
    color: var(--color-text-primary);
    font-size: 0.74rem;
    outline: none;
}

.preset-name-input:focus {
    border-color: var(--color-brand);
}

.user-presets {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-3);
    padding: 0.7rem 0.85rem;
}

.user-presets-label {
    font-size: 0.66rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-tertiary);
}

.user-preset-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
}

.user-preset-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border: 1px solid var(--color-brand);
    border-radius: 999px;
    background: var(--surface-1);
    padding: 0.28rem 0.7rem;
    color: var(--color-brand);
    font-size: 0.7rem;
    cursor: pointer;
    transition: background 0.15s ease;
}

.user-preset-chip:hover {
    background: var(--surface-4);
}

.preset-delete {
    display: inline-flex;
    width: 1rem;
    height: 1rem;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: var(--surface-4);
    color: var(--color-text-tertiary);
    transition: color 0.15s ease;
}

.preset-delete:hover {
    color: var(--color-red);
}

.preset-delete :deep(svg) {
    width: 0.6rem;
    height: 0.6rem;
}

.error-panel {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    border: 1px solid var(--color-red);
    border-radius: var(--radius-lg);
    background: var(--surface-3);
    padding: 0.8rem 0.95rem;
    color: var(--color-red);
    font-size: 0.76rem;
}

.error-panel :deep(svg) {
    width: 0.95rem;
    height: 0.95rem;
    flex: 0 0 auto;
    margin-top: 0.15rem;
}

.error-panel ul {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    margin: 0;
    padding: 0;
    list-style: none;
}

.action-bar {
    display: flex;
    justify-content: center;
    gap: 0.6rem;
    padding: 0.5rem 0;
}

.result-card {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    border: 1px solid var(--color-green);
    border-radius: var(--radius-lg);
    background: var(--surface-3);
    padding: 1rem 1.1rem;
}

.result-head {
    display: flex;
    align-items: center;
    gap: 0.55rem;
}

.result-badge {
    display: grid;
    width: 1.5rem;
    height: 1.5rem;
    place-items: center;
    border-radius: 999px;
    background: var(--surface-4);
    color: var(--color-green);
}

.result-badge :deep(svg) {
    width: 0.85rem;
    height: 0.85rem;
}

.result-title {
    flex: 1;
    font-size: 0.8rem;
    font-weight: 800;
    color: var(--color-text-primary);
}

.result-code {
    margin: 0;
    max-height: 18rem;
    overflow: auto;
    border-radius: var(--radius-md);
    background: var(--surface-1);
    border: 1px solid var(--color-divider);
    padding: 0.8rem 0.9rem;
    font-family: var(--mono-font, monospace);
    font-size: 0.8rem;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--color-text-primary);
}

.toast {
    position: fixed;
    bottom: 2rem;
    left: 50%;
    transform: translateX(-50%);
    border: 1px solid var(--color-brand);
    border-radius: 999px;
    background: var(--surface-3);
    padding: 0.55rem 1.1rem;
    color: var(--color-brand);
    font-size: 0.75rem;
    font-weight: 700;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    z-index: 1000;
    pointer-events: none;
}

.toast-fade-enter-active,
.toast-fade-leave-active {
    transition:
        opacity 0.2s ease,
        transform 0.2s ease;
}

.toast-fade-enter-from,
.toast-fade-leave-to {
    opacity: 0;
    transform: translateX(-50%) translateY(0.5rem);
}

.slide-fade-enter-active,
.slide-fade-leave-active {
    transition:
        opacity 0.2s ease,
        transform 0.2s ease;
}

.slide-fade-enter-from,
.slide-fade-leave-to {
    opacity: 0;
    transform: translateY(-0.5rem);
}
</style>
