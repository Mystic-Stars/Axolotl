import itemNameIndexData from './assets/vanilla/item-name-index.json'

type RecipeItemNameIndex = {
    en_us: Record<string, string>
    zh_cn: Record<string, string>
}

const itemNameIndex = itemNameIndexData as RecipeItemNameIndex

function translationKey(id: string): string | null {
    const normalized = id.trim().toLowerCase()
    const namespaced = normalized.includes(':') ? normalized : `minecraft:${normalized}`
    if (!namespaced.startsWith('minecraft:') || /:\d+$/.test(namespaced)) return null
    const path = namespaced.slice('minecraft:'.length)
    if (!path) return null
    const blockKey = `block.minecraft.${path}`
    if (blockKey in itemNameIndex.en_us || blockKey in itemNameIndex.zh_cn) return blockKey
    const itemKey = `item.minecraft.${path}`
    if (itemKey in itemNameIndex.en_us || itemKey in itemNameIndex.zh_cn) return itemKey
    return null
}

export function resolveRecipeItemName(id: string, locale: string, readable: string): string {
    const preferred = locale.toLowerCase().startsWith('zh') ? 'zh_cn' : 'en_us'
    const key = translationKey(id)
    if (!key) return readable
    return itemNameIndex[preferred][key] ?? itemNameIndex.en_us[key] ?? readable
}
