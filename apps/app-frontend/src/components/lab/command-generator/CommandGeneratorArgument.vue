<script setup lang="ts">
import { Combobox, type ComboboxOption, useVIntl } from '@modrinth/ui'
import { computed, ref, useTemplateRef } from 'vue'

import type { ArgumentPreset, CommandArgument } from '@/lab/command-generator/commands/types.ts'
import { getCurrentVersion } from '@/lab/command-generator/commands/registry.ts'
import { commandGeneratorMessages as messages } from '@/lab/command-generator/i18n/messages.ts'

import CommandGeneratorItemPickerModal from './CommandGeneratorItemPickerModal.vue'
import CommandGeneratorNbtEditor from './CommandGeneratorNbtEditor.vue'

const props = defineProps<{
    arg: CommandArgument
    modelValue: string | number | boolean | undefined
    /** 当前选中的游戏版本，用于 NBT 序列化分支 */
    version?: string
    /**
     * 是否强制使用 1.20.5+ 数据组件格式。
     * - undefined：由 NBT 编辑器内部根据 version 自动判定
     * - true / false：仅在 1.20.x 下由用户显式切换时传入
     */
    forceDataComponents?: boolean
}>()

const emit = defineEmits<{
    'update:modelValue': [value: string | number | boolean]
}>()

const { formatMessage } = useVIntl()

function t(key: string, fallback?: string): string {
    return formatMessage({ id: key, defaultMessage: fallback ?? key })
}

// ============ 内置预设 ============

const ENTITY_SELECTOR_DEFAULT_PRESETS: ArgumentPreset[] = [
    { labelKey: 'app.lab.command-generator.preset.entity.self', value: '@s' },
    { labelKey: 'app.lab.command-generator.preset.entity.nearest-player', value: '@p' },
    { labelKey: 'app.lab.command-generator.preset.entity.all-players', value: '@a' },
    { labelKey: 'app.lab.command-generator.preset.entity.random-player', value: '@r' },
    { labelKey: 'app.lab.command-generator.preset.entity.all-entities', value: '@e' },
    { labelKey: 'app.lab.command-generator.preset.entity.nearest-entity', value: '@n' },
    { labelKey: 'app.lab.command-generator.preset.entity.zombies', value: '@e[type=zombie]' },
    { labelKey: 'app.lab.command-generator.preset.entity.skeletons', value: '@e[type=skeleton]' },
    { labelKey: 'app.lab.command-generator.preset.entity.creepers', value: '@e[type=creeper]' },
    { labelKey: 'app.lab.command-generator.preset.entity.endermen', value: '@e[type=enderman]' },
    { labelKey: 'app.lab.command-generator.preset.entity.villagers', value: '@e[type=villager]' },
    { labelKey: 'app.lab.command-generator.preset.entity.items', value: '@e[type=item]' },
    { labelKey: 'app.lab.command-generator.preset.entity.creative-players', value: '@a[gamemode=creative]' },
    { labelKey: 'app.lab.command-generator.preset.entity.survival-players', value: '@a[gamemode=survival]' },
    { labelKey: 'app.lab.command-generator.preset.entity.nearby-10', value: '@p[distance=..10]' },
    { labelKey: 'app.lab.command-generator.preset.entity.nearby-entities-5', value: '@e[distance=..5]' },
]

const ADVANCEMENT_DEFAULT_PRESETS: ArgumentPreset[] = [
    {
        labelKey: 'app.lab.command-generator.preset.advancement.everything',
        value: 'everything',
        descriptionKey: 'app.lab.command-generator.preset.advancement.everything.desc',
    },
    { labelKey: 'app.lab.command-generator.preset.advancement.root', value: 'minecraft:story/root' },
    { labelKey: 'app.lab.command-generator.preset.advancement.stone-age', value: 'minecraft:story/mine_stone' },
    { labelKey: 'app.lab.command-generator.preset.advancement.diamonds', value: 'minecraft:story/mine_diamond' },
    { labelKey: 'app.lab.command-generator.preset.advancement.enter-nether', value: 'minecraft:story/enter_the_nether' },
    { labelKey: 'app.lab.command-generator.preset.advancement.kill-dragon', value: 'minecraft:end/kill_dragon' },
    { labelKey: 'app.lab.command-generator.preset.advancement.elytra', value: 'minecraft:end/elytra' },
    { labelKey: 'app.lab.command-generator.preset.advancement.hero-village', value: 'minecraft:adventure/hero_of_the_village' },
    { labelKey: 'app.lab.command-generator.preset.advancement.breed-animals', value: 'minecraft:husbandry/breed_an_animal' },
]

const GAMERULE_DEFAULT_PRESETS: ArgumentPreset[] = [
    { labelKey: 'app.lab.command-generator.preset.gamerule.keep-inventory', value: 'keepInventory' },
    { labelKey: 'app.lab.command-generator.preset.gamerule.no-mob-griefing', value: 'mobGriefing' },
    { labelKey: 'app.lab.command-generator.preset.gamerule.no-daylight-cycle', value: 'doDaylightCycle' },
    { labelKey: 'app.lab.command-generator.preset.gamerule.no-mob-spawning', value: 'doMobSpawning' },
    { labelKey: 'app.lab.command-generator.preset.gamerule.no-fire-tick', value: 'doFireTick' },
    { labelKey: 'app.lab.command-generator.preset.gamerule.no-weather-cycle', value: 'doWeatherCycle' },
    { labelKey: 'app.lab.command-generator.preset.gamerule.no-natural-regen', value: 'naturalRegeneration' },
    { labelKey: 'app.lab.command-generator.preset.gamerule.no-insomnia', value: 'doInsomnia' },
    { labelKey: 'app.lab.command-generator.preset.gamerule.no-command-feedback', value: 'sendCommandFeedback' },
]

// ============ 类型判断 ============

const isNumber = computed(() => props.arg.type === 'number' || props.arg.type === 'integer')
const isNbt = computed(() => props.arg.supportsNbtEditor === true)
const isItem = computed(() => props.arg.type === 'item')

/**
 * 这三种类型统一走「普通 input + 下方预设 chip」的路径，
 * 显式排除出 Combobox 分支，避免被 enumValues 抢占。
 */
const hasInlinePresets = computed(
    () =>
        props.arg.type === 'entity-selector' ||
        props.arg.type === 'advancement' ||
        props.arg.type === 'gamerule',
)

const isEnumDropdown = computed(
    () =>
        !hasInlinePresets.value &&
        !!props.arg.enumValues &&
        props.arg.enumValues.length > 0,
)

const enumOptions = computed<ComboboxOption<string>[]>(() =>
    (props.arg.enumValues ?? []).map((value) => ({ value, label: value })),
)

const itemPickerRef = useTemplateRef<InstanceType<typeof CommandGeneratorItemPickerModal>>(
    'itemPickerRef',
)

// ============ 预设合并 ============

const mergedPresets = computed<ArgumentPreset[]>(() => {
    const custom = props.arg.presets ?? []

    const builtin =
        props.arg.type === 'entity-selector'
            ? ENTITY_SELECTOR_DEFAULT_PRESETS
            : props.arg.type === 'advancement'
              ? ADVANCEMENT_DEFAULT_PRESETS
              : props.arg.type === 'gamerule'
                ? GAMERULE_DEFAULT_PRESETS
                : null

    if (!builtin) return custom

    const seen = new Set<string>()
    const result: ArgumentPreset[] = []

    for (const p of custom) {
        const key = String(p.value)
        if (!seen.has(key)) {
            seen.add(key)
            result.push(p)
        }
    }

    for (const p of builtin) {
        const key = String(p.value)
        if (!seen.has(key)) {
            seen.add(key)
            result.push(p)
        }
    }

    return result
})

const showPresets = computed(
    () =>
        !isEnumDropdown.value &&
        !isNbt.value &&
        !isItem.value &&
        mergedPresets.value.length > 0,
)

const VISIBLE_PRESET_COUNT = 4
const showAllPresets = ref(false)
const visiblePresets = computed(() => {
    const list = mergedPresets.value
    if (showAllPresets.value || list.length <= VISIBLE_PRESET_COUNT) return list
    return list.slice(0, VISIBLE_PRESET_COUNT)
})
const hiddenPresetCount = computed(() => {
    const list = mergedPresets.value
    return list.length > VISIBLE_PRESET_COUNT ? list.length - VISIBLE_PRESET_COUNT : 0
})

// ============ 事件处理 ============

function onEnumChange(value: unknown) {
    emit('update:modelValue', String(value))
}

function onTextInput(value: string) {
    if (isNumber.value) {
        emit('update:modelValue', value === '' ? '' : Number(value))
    } else {
        emit('update:modelValue', value)
    }
}

function onCheckboxChange(checked: boolean) {
    emit('update:modelValue', checked)
}

function applyPreset(value: string | number | boolean) {
    emit('update:modelValue', value)
}

function openItemPicker() {
    itemPickerRef.value?.show()
}

function onItemPicked(id: string) {
    emit('update:modelValue', id)
}
</script>

<template>
    <div class="argument-card" :class="{ 'argument-card--nbt': isNbt }">
        <header class="argument-head">
            <span class="argument-name">{{ t(arg.displayNameKey) }}</span>
            <span v-if="arg.required" class="argument-required">
                {{ formatMessage(messages.argRequired) }}
            </span>
            <span class="argument-type">{{ arg.type }}</span>
        </header>

        <CommandGeneratorNbtEditor
            v-if="isNbt"
            :model-value="String(modelValue ?? '')"
            :kind="arg.nbtKind"
            :version="version"
            :force-data-components="forceDataComponents"
            @update:model-value="emit('update:modelValue', $event)"
        />

        <Combobox
            v-else-if="isEnumDropdown"
            :model-value="String(modelValue ?? '')"
            :options="enumOptions"
            @update:model-value="onEnumChange"
        >
            <template #selected="{ label }">
                <span v-if="label" class="truncate">{{ label }}</span>
                <span v-else class="argument-placeholder">
                    {{ formatMessage(messages.argSelectPlaceholder) }}
                </span>
            </template>
        </Combobox>

        <label v-else-if="arg.type === 'boolean'" class="argument-toggle">
            <input
                type="checkbox"
                :checked="modelValue === true || modelValue === 'true'"
                @change="onCheckboxChange(($event.target as HTMLInputElement).checked)"
            />
            <span>{{ t(arg.descriptionKey) }}</span>
        </label>

        <div v-else-if="isItem" class="item-input-column">
            <input
                :value="modelValue"
                type="text"
                :placeholder="arg.placeholder ?? t(arg.descriptionKey)"
                class="argument-input"
                @input="onTextInput(($event.target as HTMLInputElement).value)"
            />
            <button type="button" class="item-pick-button" @click="openItemPicker">
                {{ t('app.lab.command-generator.item-picker.button') }}
            </button>
        </div>

        <input
            v-else
            :value="modelValue"
            :type="isNumber ? 'number' : 'text'"
            :placeholder="arg.placeholder ?? t(arg.descriptionKey)"
            class="argument-input"
            @input="onTextInput(($event.target as HTMLInputElement).value)"
        />

        <div v-if="showPresets" class="argument-presets">
            <button
                v-for="preset in visiblePresets"
                :key="preset.labelKey"
                type="button"
                class="preset-chip"
                :title="preset.descriptionKey ? t(preset.descriptionKey) : undefined"
                @click="applyPreset(preset.value)"
            >
                {{ t(preset.labelKey) }}
            </button>
            <button
                v-if="hiddenPresetCount > 0 && !showAllPresets"
                type="button"
                class="preset-chip preset-chip--more"
                @click="showAllPresets = true"
            >
                +{{ hiddenPresetCount }}
            </button>
        </div>

        <p v-if="!isNbt && arg.type !== 'boolean'" class="argument-hint">
            {{ t(arg.descriptionKey) }}
        </p>

        <CommandGeneratorItemPickerModal
            v-if="isItem"
            ref="itemPickerRef"
            :version="getCurrentVersion()"
            @select="onItemPicked"
        />
    </div>
</template>

<style scoped>
.argument-card {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.55rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-lg);
    background: var(--surface-3);
    padding: 0.9rem 1rem;
}

.argument-card--nbt {
    grid-column: 1 / -1;
}

.argument-head {
    display: flex;
    align-items: center;
    gap: 0.45rem;
}

.argument-name {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--color-text-primary);
}

.argument-required {
    border-radius: 999px;
    background: var(--surface-4);
    border: 1px solid var(--color-red);
    padding: 0.08rem 0.45rem;
    font-size: 0.6rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--color-red);
}

.argument-type {
    margin-left: auto;
    font-family: var(--mono-font, monospace);
    font-size: 0.65rem;
    color: var(--color-text-tertiary);
}

.argument-input {
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-1);
    padding: 0.5rem 0.7rem;
    color: var(--color-text-primary);
    font-size: 0.8rem;
    outline: none;
    transition: border-color 0.15s ease;
}

.argument-input:focus {
    border-color: var(--color-brand);
}

.item-input-column {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.5rem;
}

.item-input-column .argument-input {
    width: 100%;
}

.item-pick-button {
    display: inline-flex;
    align-self: center;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-brand);
    border-radius: var(--radius-md);
    background: var(--surface-1);
    padding: 0.5rem 1.2rem;
    color: var(--color-brand);
    font-size: 0.75rem;
    font-weight: 700;
    white-space: nowrap;
    cursor: pointer;
    transition: background 0.15s ease;
}

.item-pick-button:hover {
    background: var(--surface-4);
}

.argument-toggle {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    font-size: 0.75rem;
    color: var(--color-text-secondary);
}

.argument-toggle input {
    accent-color: var(--color-brand);
}

.argument-placeholder {
    color: var(--color-text-tertiary);
}

.argument-hint {
    margin: 0;
    font-size: 0.68rem;
    line-height: 1.5;
    color: var(--color-text-tertiary);
}

.argument-presets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-top: 0.1rem;
}

.preset-chip {
    border: 1px solid var(--color-divider);
    border-radius: 999px;
    background: var(--surface-1);
    padding: 0.28rem 0.65rem;
    color: var(--color-text-secondary);
    font-size: 0.7rem;
    cursor: pointer;
    transition:
        border-color 0.15s ease,
        background 0.15s ease,
        color 0.15s ease;
}

.preset-chip:hover {
    border-color: var(--color-brand);
    background: var(--surface-4);
    color: var(--color-brand);
}

.preset-chip--more {
    font-weight: 700;
    color: var(--color-text-tertiary);
}
</style>
