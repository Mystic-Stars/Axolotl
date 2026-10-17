export type NbtTagType =
    'byte' | 'short' | 'int' | 'long' | 'float' | 'double' | 'string' | 'list' | 'compound'

export interface NbtTagNode {
    id: string
    name: string
    type: NbtTagType
    value: string | number | boolean
    children?: NbtTagNode[]
}

let counter = 0

export function createNbtTag(
    name: string,
    type: NbtTagType,
    value: string | number | boolean,
    children?: NbtTagNode[],
): NbtTagNode {
    return {
        id: `nbt-${counter++}`,
        name,
        type,
        value,
        children,
    }
}

export function serializeNbt(nodes: NbtTagNode[]): string {
    if (nodes.length === 0) return '{}'

    const pairs = nodes.map((node) => {
        const key = `"${node.name}"`
        const value = serializeNbtValue(node)
        return `${key}:${value}`
    })

    return `{${pairs.join(',')}}`
}

function serializeNbtValue(node: NbtTagNode): string {
    switch (node.type) {
        case 'byte':
            return `${node.value}b`
        case 'short':
            // 修复：short 类型需要 s 后缀，否则会被解析为 int
            return `${node.value}s`
        case 'int':
            return String(node.value)
        case 'long':
            return `${node.value}L`
        case 'float':
            return `${node.value}f`
        case 'double':
            return `${node.value}d`
        case 'string':
            // 修复：SNBT 双引号字符串只需转义 \ 和 "
            return `"${String(node.value).replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`
        case 'compound':
            return serializeNbt(node.children ?? [])
        case 'list': {
            const items = (node.children ?? []).map((c) => serializeNbtValue(c)).join(',')
            return `[${items}]`
        }
        default:
            return String(node.value)
    }
}

export function createItemNbtTree(): NbtTagNode[] {
    return [
        createNbtTag('display', 'compound', '', [
            createNbtTag('Name', 'string', '{"text":"自定义名称","color":"gold"}'),
            createNbtTag('Lore', 'list', '', [
                createNbtTag('0', 'string', '{"text":"第一行描述","color":"gray"}'),
            ]),
        ]),
        createNbtTag('Enchantments', 'list', '', [
            createNbtTag('0', 'compound', '', [
                createNbtTag('id', 'string', 'minecraft:sharpness'),
                createNbtTag('lvl', 'short', 5),
            ]),
        ]),
        createNbtTag('Unbreakable', 'byte', 1),
    ]
}
