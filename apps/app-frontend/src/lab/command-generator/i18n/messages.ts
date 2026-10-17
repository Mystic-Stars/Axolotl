import { defineMessages } from '@modrinth/ui'

export const commandGeneratorMessages = defineMessages({
    // ===== 页面标题 =====
    title: { id: 'app.lab.command-generator.title', defaultMessage: 'Command generator' },
    subtitle: {
        id: 'app.lab.command-generator.subtitle',
        defaultMessage: 'Build Minecraft Java Edition commands visually.',
    },
    selectVersion: { id: 'app.lab.command-generator.select-version', defaultMessage: 'Game version' },
    selectCommand: { id: 'app.lab.command-generator.select-command', defaultMessage: 'Choose a command' },
    selectPlaceholder: {
        id: 'app.lab.command-generator.select-placeholder',
        defaultMessage: 'Search a command…',
    },
    parametersTitle: { id: 'app.lab.command-generator.parameters-title', defaultMessage: 'Parameters' },
    parametersEmpty: {
        id: 'app.lab.command-generator.parameters-empty',
        defaultMessage: 'This command takes no arguments.',
    },
    generate: { id: 'app.lab.command-generator.generate', defaultMessage: 'Generate' },
    copy: { id: 'app.lab.command-generator.copy', defaultMessage: 'Copy' },
    copied: { id: 'app.lab.command-generator.copied', defaultMessage: 'Copied' },
    result: { id: 'app.lab.command-generator.result', defaultMessage: 'Generated command' },

    // ===== 筛选器 =====
    filterCategory: { id: 'app.lab.command-generator.filter.category', defaultMessage: 'Category' },
    filterPermission: {
        id: 'app.lab.command-generator.filter.permission',
        defaultMessage: 'Permission level',
    },
    filterAll: { id: 'app.lab.command-generator.filter.all', defaultMessage: 'All' },

    // ===== 预设 =====
    savePreset: { id: 'app.lab.command-generator.preset.save', defaultMessage: 'Save as preset' },
    presetName: { id: 'app.lab.command-generator.preset.name', defaultMessage: 'Preset name' },
    myPresets: { id: 'app.lab.command-generator.preset.my', defaultMessage: 'My presets' },
    globalPresets: { id: 'app.lab.command-generator.preset.global', defaultMessage: 'All presets' },
    globalPresetsEmpty: {
        id: 'app.lab.command-generator.preset.global-empty',
        defaultMessage: 'No saved presets yet.',
    },
    globalPresetsHint: {
        id: 'app.lab.command-generator.preset.global-hint',
        defaultMessage: 'Click a preset to jump to its version and command.',
    },
    toastPresetSaved: {
        id: 'app.lab.command-generator.preset.toast.saved',
        defaultMessage: 'Preset "{name}" saved',
    },
    toastPresetLoaded: {
        id: 'app.lab.command-generator.preset.toast.loaded',
        defaultMessage: 'Preset "{name}" loaded',
    },
    toastPresetDeleted: {
        id: 'app.lab.command-generator.preset.toast.deleted',
        defaultMessage: 'Preset "{name}" deleted',
    },
    toastPresetNameRequired: {
        id: 'app.lab.command-generator.preset.toast.name-required',
        defaultMessage: 'Please enter a preset name',
    },
    toastPresetCommandMissing: {
        id: 'app.lab.command-generator.preset.toast.command-missing',
        defaultMessage: 'Preset "{name}" uses command "{command}" which does not exist in this version',
    },

    // ===== 参数卡片 =====
    argSelectPlaceholder: {
        id: 'app.lab.command-generator.arg.select-placeholder',
        defaultMessage: 'Select…',
    },
    argRequired: { id: 'app.lab.command-generator.arg.required', defaultMessage: 'Required' },

    // ===== Execute 链式构建器 =====
    executeChainTitle: {
        id: 'app.lab.command-generator.execute.chain-title',
        defaultMessage: 'Subcommand chain',
    },
    executeChainEmpty: {
        id: 'app.lab.command-generator.execute.chain-empty',
        defaultMessage: 'Click a subcommand below to add it to the chain.',
    },
    executePickerTitle: {
        id: 'app.lab.command-generator.execute.picker-title',
        defaultMessage: 'Available subcommands',
    },
    executeFinalTitle: {
        id: 'app.lab.command-generator.execute.final-title',
        defaultMessage: 'Command to run',
    },
    executeFinalPlaceholder: {
        id: 'app.lab.command-generator.execute.final-placeholder',
        defaultMessage: 'say hello',
    },
    executeFinalHint: {
        id: 'app.lab.command-generator.execute.final-hint',
        defaultMessage: 'Type the actual command without the leading slash.',
    },
    executeQuickExamples: {
        id: 'app.lab.command-generator.execute.quick-examples',
        defaultMessage: 'Quick examples',
    },
    executeCommonCommands: {
        id: 'app.lab.command-generator.execute.common-commands',
        defaultMessage: 'Common commands',
    },
    executeClear: { id: 'app.lab.command-generator.execute.clear', defaultMessage: 'Clear' },
    executeBuildFirst: {
        id: 'app.lab.command-generator.execute.build-first',
        defaultMessage: 'Please build the subcommand chain first',
    },

    // ===== 快速预设（execute 内置示例） =====
    quickPresetAllSayHello: {
        id: 'app.lab.command-generator.execute.quick.all-say-hello',
        defaultMessage: 'All players say hello',
    },
    quickPresetCheckZombies: {
        id: 'app.lab.command-generator.execute.quick.check-zombies',
        defaultMessage: 'Check for zombies',
    },
    quickPresetKillNearbyZombies: {
        id: 'app.lab.command-generator.execute.quick.kill-nearby-zombies',
        defaultMessage: 'Kill nearby zombies and count',
    },
    quickPresetGrassBelow: {
        id: 'app.lab.command-generator.execute.quick.grass-below',
        defaultMessage: 'Say when grass is below',
    },
    quickPresetMoveForward: {
        id: 'app.lab.command-generator.execute.quick.move-forward',
        defaultMessage: 'All entities move forward',
    },

    // ===== NBT 编辑器 =====
    nbtTitle: { id: 'app.lab.command-generator.nbt.title', defaultMessage: 'NBT editor' },
    nbtSimple: { id: 'app.lab.command-generator.nbt.simple', defaultMessage: 'Simple' },
    nbtAdvanced: { id: 'app.lab.command-generator.nbt.advanced', defaultMessage: 'Advanced' },
    nbtAdvancedTitle: {
        id: 'app.lab.command-generator.nbt.advanced-title',
        defaultMessage: 'Advanced NBT tree',
    },
    nbtAddRoot: {
        id: 'app.lab.command-generator.nbt.add-root',
        defaultMessage: 'Add root tag',
    },
    nbtSnbtPreview: {
        id: 'app.lab.command-generator.nbt.snbt-preview',
        defaultMessage: 'SNBT preview',
    },
    nbtPresetEnchanted: {
        id: 'app.lab.command-generator.nbt.preset-enchanted',
        defaultMessage: 'Enchanted item',
    },
    nbtPresetCustomName: {
        id: 'app.lab.command-generator.nbt.preset-custom-name',
        defaultMessage: 'Custom name',
    },

    // ===== 物品 NBT 傻瓜编辑器 =====
    itemNbtTitle: {
        id: 'app.lab.command-generator.item-nbt.title',
        defaultMessage: 'Item customisation',
    },
    itemNbtName: {
        id: 'app.lab.command-generator.item-nbt.name',
        defaultMessage: 'Custom name',
    },
    itemNbtNameHint: {
        id: 'app.lab.command-generator.item-nbt.name-hint',
        defaultMessage: 'Shown as the item title.',
    },
    itemNbtNamePlaceholder: {
        id: 'app.lab.command-generator.item-nbt.name-placeholder',
        defaultMessage: 'Legendary sword',
    },
    itemNbtLore: {
        id: 'app.lab.command-generator.item-nbt.lore',
        defaultMessage: 'Lore lines',
    },
    itemNbtLoreHint: {
        id: 'app.lab.command-generator.item-nbt.lore-hint',
        defaultMessage: 'Each line appears under the item name.',
    },
    itemNbtLorePlaceholder: {
        id: 'app.lab.command-generator.item-nbt.lore-placeholder',
        defaultMessage: 'A line of lore…',
    },
    itemNbtAddLore: {
        id: 'app.lab.command-generator.item-nbt.add-lore',
        defaultMessage: 'Add line',
    },
    itemNbtEnchantments: {
        id: 'app.lab.command-generator.item-nbt.enchantments',
        defaultMessage: 'Enchantments',
    },
    itemNbtEnchantmentsHint: {
        id: 'app.lab.command-generator.item-nbt.enchantments-hint',
        defaultMessage: 'Pick an enchantment and set its level.',
    },
    itemNbtAddEnchantment: {
        id: 'app.lab.command-generator.item-nbt.add-enchantment',
        defaultMessage: 'Add enchantment',
    },
    itemNbtLevel: { id: 'app.lab.command-generator.item-nbt.level', defaultMessage: 'Level' },
    itemNbtNoEnchantments: {
        id: 'app.lab.command-generator.item-nbt.no-enchantments',
        defaultMessage: 'No enchantments yet.',
    },
    itemNbtUnbreakable: {
        id: 'app.lab.command-generator.item-nbt.unbreakable',
        defaultMessage: 'Unbreakable',
    },
    itemNbtUnbreakableHint: {
        id: 'app.lab.command-generator.item-nbt.unbreakable-hint',
        defaultMessage: 'Prevents the item from taking durability damage.',
    },
    itemNbtPreview: {
        id: 'app.lab.command-generator.item-nbt.preview',
        defaultMessage: 'NBT preview',
    },
    itemNbtPreviewEmpty: {
        id: 'app.lab.command-generator.item-nbt.preview-empty',
        defaultMessage: 'Empty — no NBT will be applied.',
    },
})
