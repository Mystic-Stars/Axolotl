<script setup lang="ts">
import { PlusIcon, SparklesIcon, XIcon } from '@modrinth/assets'
import { Combobox, type ComboboxOption, defineMessages, Toggle, useVIntl } from '@modrinth/ui'
import { computed, nextTick, ref, watch } from 'vue'

interface EnchantEntry {
    key: number
    id: string
    level: number
}

interface EnchantDefinition {
    id: string
    labelKey: string
    maxLevel: number
}

const props = defineProps<{
    modelValue?: string
    /** 目标游戏版本，如 '1.21'、'1.16'、'1.20' */
    version?: string
    /** 1.20.x 下由用户切换：是否使用 1.20.5+ 数据组件 */
    forceDataComponents?: boolean
}>()

const emit = defineEmits<{
    'update:modelValue': [value: string]
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
    title: {
        id: 'app.lab.command-generator.item-nbt.title',
        defaultMessage: 'Item customisation',
    },
    nameLabel: {
        id: 'app.lab.command-generator.item-nbt.name-label',
        defaultMessage: 'Custom name',
    },
    nameHint: {
        id: 'app.lab.command-generator.item-nbt.name-hint',
        defaultMessage: 'Shown as the item title.',
    },
    namePlaceholder: {
        id: 'app.lab.command-generator.item-nbt.name-placeholder',
        defaultMessage: 'Legendary sword',
    },
    loreLabel: {
        id: 'app.lab.command-generator.item-nbt.lore-label',
        defaultMessage: 'Lore lines',
    },
    loreHint: {
        id: 'app.lab.command-generator.item-nbt.lore-hint',
        defaultMessage: 'Each line appears under the item name.',
    },
    lorePlaceholder: {
        id: 'app.lab.command-generator.item-nbt.lore-placeholder',
        defaultMessage: 'A line of lore…',
    },
    addLore: {
        id: 'app.lab.command-generator.item-nbt.add-lore',
        defaultMessage: 'Add line',
    },
    enchLabel: {
        id: 'app.lab.command-generator.item-nbt.ench-label',
        defaultMessage: 'Enchantments',
    },
    enchHint: {
        id: 'app.lab.command-generator.item-nbt.ench-hint',
        defaultMessage: 'Pick an enchantment and set its level.',
    },
    enchAdd: {
        id: 'app.lab.command-generator.item-nbt.ench-add',
        defaultMessage: 'Add enchantment',
    },
    enchLevel: {
        id: 'app.lab.command-generator.item-nbt.ench-level',
        defaultMessage: 'Level',
    },
    enchEmpty: {
        id: 'app.lab.command-generator.item-nbt.ench-empty',
        defaultMessage: 'No enchantments yet.',
    },
    unbreakableLabel: {
        id: 'app.lab.command-generator.item-nbt.unbreakable-label',
        defaultMessage: 'Unbreakable',
    },
    unbreakableHint: {
        id: 'app.lab.command-generator.item-nbt.unbreakable-hint',
        defaultMessage: 'Prevents the item from taking durability damage.',
    },
    previewLabel: {
        id: 'app.lab.command-generator.item-nbt.preview-label',
        defaultMessage: 'NBT preview',
    },
    previewEmpty: {
        id: 'app.lab.command-generator.item-nbt.preview-empty',
        defaultMessage: 'Empty — no NBT will be applied.',
    },
})

const ENCHANTMENTS: EnchantDefinition[] = [
    { id: 'minecraft:sharpness', labelKey: 'app.lab.command-generator.enchantment.sharpness', maxLevel: 5 },
    { id: 'minecraft:smite', labelKey: 'app.lab.command-generator.enchantment.smite', maxLevel: 5 },
    { id: 'minecraft:bane_of_arthropods', labelKey: 'app.lab.command-generator.enchantment.bane-of-arthropods', maxLevel: 5 },
    { id: 'minecraft:knockback', labelKey: 'app.lab.command-generator.enchantment.knockback', maxLevel: 2 },
    { id: 'minecraft:fire_aspect', labelKey: 'app.lab.command-generator.enchantment.fire-aspect', maxLevel: 2 },
    { id: 'minecraft:looting', labelKey: 'app.lab.command-generator.enchantment.looting', maxLevel: 3 },
    { id: 'minecraft:sweeping_edge', labelKey: 'app.lab.command-generator.enchantment.sweeping-edge', maxLevel: 3 },
    { id: 'minecraft:efficiency', labelKey: 'app.lab.command-generator.enchantment.efficiency', maxLevel: 5 },
    { id: 'minecraft:silk_touch', labelKey: 'app.lab.command-generator.enchantment.silk-touch', maxLevel: 1 },
    { id: 'minecraft:unbreaking', labelKey: 'app.lab.command-generator.enchantment.unbreaking', maxLevel: 3 },
    { id: 'minecraft:fortune', labelKey: 'app.lab.command-generator.enchantment.fortune', maxLevel: 3 },
    { id: 'minecraft:mending', labelKey: 'app.lab.command-generator.enchantment.mending', maxLevel: 1 },
    { id: 'minecraft:power', labelKey: 'app.lab.command-generator.enchantment.power', maxLevel: 5 },
    { id: 'minecraft:punch', labelKey: 'app.lab.command-generator.enchantment.punch', maxLevel: 2 },
    { id: 'minecraft:flame', labelKey: 'app.lab.command-generator.enchantment.flame', maxLevel: 1 },
    { id: 'minecraft:infinity', labelKey: 'app.lab.command-generator.enchantment.infinity', maxLevel: 1 },
    { id: 'minecraft:multishot', labelKey: 'app.lab.command-generator.enchantment.multishot', maxLevel: 1 },
    { id: 'minecraft:quick_charge', labelKey: 'app.lab.command-generator.enchantment.quick-charge', maxLevel: 3 },
    { id: 'minecraft:piercing', labelKey: 'app.lab.command-generator.enchantment.piercing', maxLevel: 4 },
    { id: 'minecraft:protection', labelKey: 'app.lab.command-generator.enchantment.protection', maxLevel: 4 },
    { id: 'minecraft:fire_protection', labelKey: 'app.lab.command-generator.enchantment.fire-protection', maxLevel: 4 },
    { id: 'minecraft:feather_falling', labelKey: 'app.lab.command-generator.enchantment.feather-falling', maxLevel: 4 },
    { id: 'minecraft:blast_protection', labelKey: 'app.lab.command-generator.enchantment.blast-protection', maxLevel: 4 },
    { id: 'minecraft:projectile_protection', labelKey: 'app.lab.command-generator.enchantment.projectile-protection', maxLevel: 4 },
    { id: 'minecraft:respiration', labelKey: 'app.lab.command-generator.enchantment.respiration', maxLevel: 3 },
    { id: 'minecraft:aqua_affinity', labelKey: 'app.lab.command-generator.enchantment.aqua-affinity', maxLevel: 1 },
    { id: 'minecraft:thorns', labelKey: 'app.lab.command-generator.enchantment.thorns', maxLevel: 3 },
    { id: 'minecraft:depth_strider', labelKey: 'app.lab.command-generator.enchantment.depth-strider', maxLevel: 3 },
    { id: 'minecraft:frost_walker', labelKey: 'app.lab.command-generator.enchantment.frost-walker', maxLevel: 2 },
    { id: 'minecraft:binding_curse', labelKey: 'app.lab.command-generator.enchantment.binding-curse', maxLevel: 1 },
    { id: 'minecraft:vanishing_curse', labelKey: 'app.lab.command-generator.enchantment.vanishing-curse', maxLevel: 1 },
    { id: 'minecraft:soul_speed', labelKey: 'app.lab.command-generator.enchantment.soul-speed', maxLevel: 3 },
    { id: 'minecraft:swift_sneak', labelKey: 'app.lab.command-generator.enchantment.swift-sneak', maxLevel: 3 },
    { id: 'minecraft:loyalty', labelKey: 'app.lab.command-generator.enchantment.loyalty', maxLevel: 3 },
    { id: 'minecraft:impaling', labelKey: 'app.lab.command-generator.enchantment.impaling', maxLevel: 5 },
    { id: 'minecraft:riptide', labelKey: 'app.lab.command-generator.enchantment.riptide', maxLevel: 3 },
    { id: 'minecraft:channeling', labelKey: 'app.lab.command-generator.enchantment.channeling', maxLevel: 1 },
    { id: 'minecraft:luck_of_the_sea', labelKey: 'app.lab.command-generator.enchantment.luck-of-the-sea', maxLevel: 3 },
    { id: 'minecraft:lure', labelKey: 'app.lab.command-generator.enchantment.lure', maxLevel: 3 },
]

const enchantById = computed(() => new Map(ENCHANTMENTS.map((e) => [e.id, e])))

function enchantLabel(id: string): string {
    const def = enchantById.value.get(id)
    if (!def) return id
    return formatMessage({ id: def.labelKey, defaultMessage: def.labelKey })
}

const enchantOptions = computed<ComboboxOption<string>[]>(() =>
    ENCHANTMENTS.map((e) => ({
        value: e.id,
        label: enchantLabel(e.id),
    })),
)

const name = ref('')
const lore = ref<string[]>([])
const enchantments = ref<EnchantEntry[]>([])
const unbreakable = ref(false)
let nextKey = 1

// ============ 版本判断 ============

function parseVersion(v: string): number[] {
    return v.split('.').map((n) => {
        const num = Number.parseInt(n, 10)
        return Number.isFinite(num) ? num : 0
    })
}

/**
 * 决定最终使用数据组件（1.20.5+）还是旧版 NBT。
 *
 * 三档：
 * - 1.20.5 及以上（含 1.21+）：始终数据组件
 * - 1.20.x 但低于 1.20.5：
 *     - forceDataComponents 显式传了 → 跟随用户切换
 *     - 否则默认数据组件（1.20.x 用户大概率在用新语法）
 * - 1.16 ~ 1.19：始终旧版 NBT
 *
 * 注意 "1.20" 这种无小版本的字符串会走第二档，
 * 因为 manifest 里 1.20 这一项就是 "1.20"。
 */
const useDataComponents = computed(() => {
    const v = parseVersion(props.version ?? '1.21')

    const isAtLeast1205 =
        v[0] > 1 ||
        (v[0] === 1 && v[1] > 20) ||
        (v[0] === 1 && v[1] === 20 && (v[2] ?? 0) >= 5)
    if (isAtLeast1205) return true

    const is120Family = v[0] === 1 && v[1] === 20
    if (is120Family) {
        // 用户切换优先
        if (props.forceDataComponents !== undefined) {
            return props.forceDataComponents
        }
        // 默认走数据组件
        return true
    }

    return false
})

// ============ 序列化 ============

/**
 * SNBT 字符串内容转义。
 * - 单引号包裹：转义 \ 和 '
 * - 双引号包裹：转义 \ 和 "
 */
function escapeSnbt(value: string, quote: "'" | '"' = "'"): string {
    let result = value.replace(/\\/g, '\\\\')
    if (quote === "'") {
        result = result.replace(/'/g, "\\'")
    } else {
        result = result.replace(/"/g, '\\"')
    }
    return result
}

/**
 * 旧版 NBT 格式（1.20.4 及之前）
 * 使用 {display:{Name:...}} + Enchantments + Unbreakable
 */
function serializeLegacy(): string {
    const parts: string[] = []

    const displayParts: string[] = []
    if (name.value.trim()) {
        displayParts.push(`Name:'{"text":"${escapeSnbt(name.value, "'")}"}'`)
    }
    const cleanLore = lore.value.map((l) => l.trim()).filter(Boolean)
    if (cleanLore.length) {
        const loreItems = cleanLore
            .map((l) => `'{"text":"${escapeSnbt(l, "'")}"}'`)
            .join(',')
        displayParts.push(`Lore:[${loreItems}]`)
    }
    if (displayParts.length) {
        parts.push(`display:{${displayParts.join(',')}}`)
    }

    if (enchantments.value.length) {
        // 旧版附魔 lvl 是 short 类型，必须加 s 后缀
        const items = enchantments.value
            .map((e) => `{id:"${e.id}",lvl:${e.level}s}`)
            .join(',')
        parts.push(`Enchantments:[${items}]`)
    }

    if (unbreakable.value) {
        parts.push('Unbreakable:1b')
    }

    return parts.length ? `{${parts.join(',')}}` : ''
}

/**
 * 新版数据组件格式（1.20.5+）
 * 使用 [custom_name=..., lore=..., enchantments={...}, unbreakable={}]
 */
function serializeDataComponents(): string {
    const parts: string[] = []

    if (name.value.trim()) {
        parts.push(`custom_name='{"text":"${escapeSnbt(name.value, "'")}"}'`)
    }
    const cleanLore = lore.value.map((l) => l.trim()).filter(Boolean)
    if (cleanLore.length) {
        const loreItems = cleanLore
            .map((l) => `'{"text":"${escapeSnbt(l, "'")}"}'`)
            .join(',')
        parts.push(`lore=[${loreItems}]`)
    }

    if (enchantments.value.length) {
        // 新版附魔是键值对映射
        const enchMap = enchantments.value
            .map((e) => `"${e.id}":${e.level}`)
            .join(',')
        parts.push(`enchantments={${enchMap}}`)
    }

    if (unbreakable.value) {
        parts.push('unbreakable={}')
    }

    return parts.length ? `[${parts.join(',')}]` : ''
}

const snbt = computed(() =>
    useDataComponents.value ? serializeDataComponents() : serializeLegacy(),
)

// ============ 双向同步 ============

let syncing = false

// 初始：从 prop 恢复内部状态
if (props.modelValue) {
    parseSnbt(props.modelValue)
}

watch(
    snbt,
    (val) => {
        syncing = true
        emit('update:modelValue', val)
        nextTick(() => {
            syncing = false
        })
    },
    { immediate: true },
)

watch(
    () => props.modelValue,
    (val, oldVal) => {
        if (val === oldVal) return
        if (syncing) return
        parseSnbt(val ?? '')
    },
)

// 版本或用户切换变化时，用新格式重新序列化并 emit
watch(
    () => [props.version, props.forceDataComponents],
    () => {
        emit('update:modelValue', snbt.value)
    },
)

// ============ 反序列化 ============

function parseSnbt(snbtStr: string): void {
    name.value = ''
    lore.value = []
    enchantments.value = []
    unbreakable.value = false

    if (!snbtStr) return

    // 优先尝试解析新版数据组件格式（方括号包裹）
    if (snbtStr.startsWith('[')) {
        parseDataComponents(snbtStr)
        return
    }

    // 旧版 NBT 格式（花括号包裹）
    parseLegacy(snbtStr)
}

function parseLegacy(snbtStr: string): void {
    const nameMatch = snbtStr.match(/Name:'((?:\\.|[^'\\])*)'/)
    if (nameMatch) {
        const content = unescapeSnbt(nameMatch[1])
        try {
            const parsed = JSON.parse(content)
            if (parsed && typeof parsed.text === 'string') {
                name.value = parsed.text
            }
        } catch { /* ignore */ }
    }

    const loreMatch = snbtStr.match(/Lore:\[((?:'(?:\\.|[^'\\])*',?)+)\]/)
    if (loreMatch) {
        const items = loreMatch[1].match(/'(?:\\.|[^'\\])*'/g) ?? []
        lore.value = items
            .map((raw) => {
                const content = unescapeSnbt(raw.slice(1, -1))
                try {
                    const parsed = JSON.parse(content)
                    return typeof parsed?.text === 'string' ? parsed.text : ''
                } catch { return '' }
            })
            .filter((s) => s.length > 0)
    }

    const enchMatch = snbtStr.match(/Enchantments:\[((?:\{[^{}]*\},?)+)\]/)
    if (enchMatch) {
        const items = enchMatch[1].match(/\{[^{}]*\}/g) ?? []
        enchantments.value = items
            .map((raw) => {
                const idMatch = raw.match(/id:"([^"]*)"/)
                const lvlMatch = raw.match(/lvl:(\d+)/)
                return {
                    key: nextKey++,
                    id: idMatch ? idMatch[1] : '',
                    level: lvlMatch ? Number(lvlMatch[1]) : 1,
                }
            })
            .filter((e) => e.id)
    }

    if (/Unbreakable:1b/.test(snbtStr)) {
        unbreakable.value = true
    }
}

function parseDataComponents(snbtStr: string): void {
    const nameMatch = snbtStr.match(/custom_name='((?:\\.|[^'\\])*)'/)
    if (nameMatch) {
        const content = unescapeSnbt(nameMatch[1])
        try {
            const parsed = JSON.parse(content)
            if (parsed && typeof parsed.text === 'string') {
                name.value = parsed.text
            }
        } catch { /* ignore */ }
    }

    const loreMatch = snbtStr.match(/lore=\[((?:'(?:\\.|[^'\\])*',?)+)\]/)
    if (loreMatch) {
        const items = loreMatch[1].match(/'(?:\\.|[^'\\])*'/g) ?? []
        lore.value = items
            .map((raw) => {
                const content = unescapeSnbt(raw.slice(1, -1))
                try {
                    const parsed = JSON.parse(content)
                    return typeof parsed?.text === 'string' ? parsed.text : ''
                } catch { return '' }
            })
            .filter((s) => s.length > 0)
    }

    const enchMatch = snbtStr.match(/enchantments=\{([^}]*)\}/)
    if (enchMatch) {
        const pairs = enchMatch[1].match(/"([^"]+)":(\d+)/g) ?? []
        enchantments.value = pairs
            .map((pair) => {
                const m = pair.match(/"([^"]+)":(\d+)/)
                if (!m) return null
                return {
                    key: nextKey++,
                    id: m[1],
                    level: Number(m[2]),
                }
            })
            .filter((e): e is EnchantEntry => e !== null)
    }

    if (/unbreakable=\{\}/.test(snbtStr)) {
        unbreakable.value = true
    }
}

function unescapeSnbt(value: string): string {
    return value.replace(/\\(.)/g, '$1')
}

// ============ 编辑操作 ============

function addLore() {
    lore.value.push('')
}

function removeLore(index: number) {
    lore.value.splice(index, 1)
}

function addEnchant() {
    const used = new Set(enchantments.value.map((e) => e.id))
    const next = ENCHANTMENTS.find((e) => !used.has(e.id))
    if (!next) return
    enchantments.value.push({ key: nextKey++, id: next.id, level: 1 })
}

function removeEnchant(index: number) {
    enchantments.value.splice(index, 1)
}

function setEnchantId(index: number, id: unknown) {
    const entry = enchantments.value[index]
    if (!entry) return
    entry.id = String(id)
    const def = enchantById.value.get(entry.id)
    if (def && entry.level > def.maxLevel) entry.level = def.maxLevel
}

function setEnchantLevel(index: number, level: number) {
    const entry = enchantments.value[index]
    if (!entry) return
    const def = enchantById.value.get(entry.id)
    const max = def?.maxLevel ?? 10
    const min = 1
    entry.level = Math.min(max, Math.max(min, Math.floor(level || min)))
}
</script>

<template>
    <div class="item-nbt">
        <header class="item-nbt-head">
            <span class="item-nbt-title">
                <SparklesIcon />
                {{ formatMessage(messages.title) }}
            </span>
            <span v-if="version" class="item-nbt-version">
                {{ useDataComponents ? '1.20.5+' : version }}
            </span>
        </header>

        <div class="field">
            <label class="field-label">
                <span>{{ formatMessage(messages.nameLabel) }}</span>
                <span class="field-hint">{{ formatMessage(messages.nameHint) }}</span>
            </label>
            <input
                v-model="name"
                class="text-input"
                :placeholder="formatMessage(messages.namePlaceholder)"
            />
        </div>

        <div class="field">
            <label class="field-label">
                <span>{{ formatMessage(messages.loreLabel) }}</span>
                <span class="field-hint">{{ formatMessage(messages.loreHint) }}</span>
            </label>
            <div class="lore-list">
                <div v-for="(_, i) in lore" :key="i" class="lore-row">
                    <input
                        v-model="lore[i]"
                        class="text-input"
                        :placeholder="formatMessage(messages.lorePlaceholder)"
                    />
                    <button type="button" class="icon-btn icon-btn--danger" @click="removeLore(i)">
                        <XIcon />
                    </button>
                </div>
                <button type="button" class="ghost-add" @click="addLore">
                    <PlusIcon />
                    {{ formatMessage(messages.addLore) }}
                </button>
            </div>
        </div>

        <div class="field">
            <label class="field-label">
                <span>{{ formatMessage(messages.enchLabel) }}</span>
                <span class="field-hint">{{ formatMessage(messages.enchHint) }}</span>
            </label>
            <div class="enchant-list">
                <div v-if="!enchantments.length" class="enchant-empty">
                    {{ formatMessage(messages.enchEmpty) }}
                </div>
                <div v-for="(ench, i) in enchantments" :key="ench.key" class="enchant-row">
                    <div class="enchant-select">
                        <Combobox
                            :model-value="ench.id"
                            :options="enchantOptions"
                            searchable
                            @update:model-value="setEnchantId(i, $event)"
                        >
                            <template #selected="{ label }">
                                <span class="truncate">
                                    {{ label ?? enchantLabel(ench.id) }}
                                </span>
                            </template>
                        </Combobox>
                    </div>
                    <div class="enchant-level">
                        <span class="enchant-level-label">
                            {{ formatMessage(messages.enchLevel) }}
                        </span>
                        <input
                            class="text-input text-input--num"
                            type="number"
                            min="1"
                            :max="enchantById.get(ench.id)?.maxLevel ?? 10"
                            :value="ench.level"
                            @input="
                                setEnchantLevel(
                                    i,
                                    Number(($event.target as HTMLInputElement).value),
                                )
                            "
                        />
                    </div>
                    <button
                        type="button"
                        class="icon-btn icon-btn--danger"
                        @click="removeEnchant(i)"
                    >
                        <XIcon />
                    </button>
                </div>
                <button type="button" class="ghost-add" @click="addEnchant">
                    <PlusIcon />
                    {{ formatMessage(messages.enchAdd) }}
                </button>
            </div>
        </div>

        <div class="field field--row">
            <div class="toggle-text">
                <label class="field-label">
                    <span>{{ formatMessage(messages.unbreakableLabel) }}</span>
                    <span class="field-hint">
                        {{ formatMessage(messages.unbreakableHint) }}
                    </span>
                </label>
            </div>
            <Toggle v-model="unbreakable" />
        </div>

        <div class="preview">
            <span class="preview-label">{{ formatMessage(messages.previewLabel) }}</span>
            <code v-if="snbt" class="preview-code">{{ snbt }}</code>
            <span v-else class="preview-empty">{{ formatMessage(messages.previewEmpty) }}</span>
        </div>
    </div>
</template>

<style scoped>
.item-nbt {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-lg);
    background: var(--surface-2);
    padding: 0.9rem 0.95rem;
}

.item-nbt-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.item-nbt-title {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.78rem;
    font-weight: 800;
    color: var(--color-text-primary);
}

.item-nbt-title :deep(svg) {
    width: 0.9rem;
    height: 0.9rem;
    color: var(--color-brand);
}

.item-nbt-version {
    border-radius: 999px;
    background: var(--surface-4);
    padding: 0.15rem 0.5rem;
    font-size: 0.6rem;
    font-weight: 700;
    color: var(--color-text-tertiary);
}

.field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
}

.field--row {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
}

.field-label {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    font-size: 0.74rem;
    font-weight: 700;
    color: var(--color-text-primary);
}

.field-hint {
    font-size: 0.63rem;
    font-weight: 400;
    color: var(--color-text-tertiary);
}

.text-input {
    min-width: 0;
    flex: 1;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--surface-1);
    padding: 0.45rem 0.65rem;
    color: var(--color-text-primary);
    font-size: 0.76rem;
    outline: none;
    transition: border-color 0.15s ease;
}

.text-input:focus {
    border-color: var(--color-brand);
}

.text-input--num {
    flex: 0 0 auto;
    width: 4.5rem;
    text-align: center;
}

.lore-list,
.enchant-list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
}

.lore-row,
.enchant-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
}

.enchant-select {
    min-width: 0;
    flex: 1;
}

.enchant-level {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex: 0 0 auto;
}

.enchant-level-label {
    font-size: 0.65rem;
    color: var(--color-text-tertiary);
}

.enchant-empty {
    border: 1px dashed var(--color-divider);
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.65rem;
    font-size: 0.7rem;
    color: var(--color-text-tertiary);
}

.icon-btn {
    display: inline-flex;
    width: 1.6rem;
    height: 1.6rem;
    flex: 0 0 auto;
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

.icon-btn:hover {
    background: var(--surface-4);
}

.icon-btn--danger:hover {
    color: var(--color-red);
}

.icon-btn :deep(svg) {
    width: 0.78rem;
    height: 0.78rem;
}

.ghost-add {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.3rem;
    border: 1px dashed var(--color-divider);
    border-radius: var(--radius-sm);
    background: transparent;
    padding: 0.4rem 0.6rem;
    color: var(--color-text-tertiary);
    font-size: 0.7rem;
    font-weight: 700;
    cursor: pointer;
    transition:
        border-color 0.15s ease,
        color 0.15s ease;
}

.ghost-add:hover {
    border-color: var(--color-brand);
    color: var(--color-brand);
}

.ghost-add :deep(svg) {
    width: 0.75rem;
    height: 0.75rem;
}

.toggle-text {
    min-width: 0;
    flex: 1;
}

.preview {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    border-top: 1px solid var(--color-divider);
    padding-top: 0.7rem;
}

.preview-label {
    font-size: 0.62rem;
    font-weight: 800;
    text-transform: uppercase;
    color: var(--color-text-tertiary);
}

.preview-code {
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

.preview-empty {
    font-size: 0.68rem;
    color: var(--color-text-tertiary);
}
</style>
