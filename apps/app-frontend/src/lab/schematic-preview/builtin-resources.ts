import blockModelsData from './assets/vanilla/block-model-index.json'
import blockNameData from './assets/vanilla/block-name-index.json'
import blockDefaultPropertiesData from './assets/vanilla/block-property-defaults.json'
import blockDefinitionsData from './assets/vanilla/block-state-index.json'
import blocksAtlasUrl from './assets/vanilla/texture-atlas.png?url'
import atlasUvData from './assets/vanilla/texture-layout.json'
import type { SchematicBlockState } from './backend.ts'

type JsonObject = Record<string, unknown>
type RawTextureRegion = [number, number, number, number]

export type BuiltinTextureRegion = {
    x: number
    y: number
    width: number
    height: number
}

const blockDefinitions = blockDefinitionsData as Record<string, unknown>
const blockModels = blockModelsData as Record<string, unknown>
const blockDefaultProperties = blockDefaultPropertiesData as Record<string, Record<string, string>>
const atlasUvs = atlasUvData as Record<string, RawTextureRegion>
const blockNames = blockNameData as {
    en_us: Record<string, string>
    zh_cn: Record<string, string>
}

const BLOCK_ALIASES: Record<string, string> = {
    trapdoor: 'oak_trapdoor',
    chain: 'iron_chain',
    grass_path: 'dirt_path',
    grass: 'short_grass',
    sign: 'oak_sign',
    wall_sign: 'oak_wall_sign',
    banner: 'white_banner',
    wall_banner: 'white_wall_banner',
    bed: 'red_bed',
    skull: 'skeleton_skull',
    wall_skull: 'skeleton_wall_skull',
}

function identifier(value: string) {
    const normalized = value.trim().toLowerCase()
    return normalized.includes(':') ? normalized : `minecraft:${normalized}`
}

function minecraftPath(value: string) {
    const normalized = value.trim().toLowerCase()
    const [namespace, path] = normalized.includes(':')
        ? (normalized.split(':', 2) as [string, string])
        : ['minecraft', normalized]
    if (namespace !== 'minecraft') return undefined
    return BLOCK_ALIASES[path] ?? path
}

function collectModelReferences(value: unknown, references: Set<string>) {
    if (Array.isArray(value)) {
        for (const item of value) collectModelReferences(item, references)
        return
    }
    if (!value || typeof value !== 'object') return
    for (const [key, item] of Object.entries(value as JsonObject)) {
        if (key === 'model' && typeof item === 'string') {
            references.add(identifier(item))
        } else {
            collectModelReferences(item, references)
        }
    }
}

function builtinModel(modelId: string) {
    const [namespace, path] = modelId.split(':', 2)
    const model = namespace === 'minecraft' && path ? blockModels[path] : undefined
    if (!model || typeof model !== 'object' || Array.isArray(model)) return model
    const textures = (model as JsonObject).textures
    if (!textures || typeof textures !== 'object' || Array.isArray(textures)) return model

    return {
        ...(model as JsonObject),
        textures: Object.fromEntries(
            Object.entries(textures).map(([key, value]) => [
                key,
                value && typeof value === 'object' && !Array.isArray(value)
                    ? ((value as JsonObject).sprite ?? value)
                    : value,
            ]),
        ),
    }
}

export function completeBuiltinModelDependencies(
    _version: string,
    definitions: Record<string, unknown>,
    models: Record<string, unknown>,
) {
    const pendingModels = new Set<string>()
    for (const definition of Object.values(definitions)) {
        collectModelReferences(definition, pendingModels)
    }

    const visitedModels = new Set<string>()
    while (pendingModels.size > 0) {
        const modelId = pendingModels.values().next().value as string | undefined
        if (!modelId) break
        pendingModels.delete(modelId)
        if (visitedModels.has(modelId)) continue
        visitedModels.add(modelId)

        let model = models[modelId]
        if (!model) {
            model = builtinModel(modelId)
            if (model) models[modelId] = model
        }
        if (!model || typeof model !== 'object' || Array.isArray(model)) continue
        const parent = (model as JsonObject).parent
        if (typeof parent === 'string') pendingModels.add(identifier(parent))
    }
}

export function loadBuiltinBlockResources(
    version: string,
    palette: readonly SchematicBlockState[],
) {
    const definitions: Record<string, unknown> = {}
    const models: Record<string, unknown> = {}
    const defaultProperties: Record<string, Record<string, string>> = {}
    for (const state of palette) {
        const path = minecraftPath(state.name)
        if (!path) continue
        const definition = blockDefinitions[path]
        if (definition) definitions[state.name] = definition
        const defaults = blockDefaultProperties[path]
        if (defaults) defaultProperties[state.name] = defaults
    }
    completeBuiltinModelDependencies(version, definitions, models)

    return {
        blockDefinitions: definitions,
        blockModels: models,
        defaultBlockProperties: defaultProperties,
    }
}

export function listBuiltinBlockStates(): SchematicBlockState[] {
    return Object.keys(blockDefinitions).map((path) => ({
        name: `minecraft:${path}`,
        properties: { ...(blockDefaultProperties[path] ?? {}) },
    }))
}

export function loadBuiltinBlockNames() {
    return {
        en_us: { ...blockNames.en_us },
        zh_cn: { ...blockNames.zh_cn },
    }
}

function normalizeTextureId(value: string) {
    const normalized = value.trim().toLowerCase()
    return normalized.includes(':') ? normalized : `minecraft:${normalized}`
}

export function builtinTextureRegion(textureId: string): BuiltinTextureRegion | undefined {
    const normalized = normalizeTextureId(textureId)
    const [namespace, path] = normalized.split(':', 2)
    if (namespace !== 'minecraft' || !path) return undefined
    const region = atlasUvs[path]
    if (!region) return undefined
    return { x: region[0], y: region[1], width: region[2], height: region[3] }
}

export function builtinTextureUvs(
    canvasWidth = BUILTIN_ATLAS_WIDTH,
    canvasHeight = BUILTIN_ATLAS_HEIGHT,
) {
    const textureUvs: Record<string, [number, number, number, number]> = {}
    for (const [name, region] of Object.entries(atlasUvs)) {
        const visibleHeight = Math.min(region[2], region[3])
        textureUvs[`minecraft:${name}`] = [
            region[0] / canvasWidth,
            region[1] / canvasHeight,
            (region[0] + region[2]) / canvasWidth,
            (region[1] + visibleHeight) / canvasHeight,
        ]
    }
    return textureUvs
}

export async function loadBuiltinAtlas(signal?: AbortSignal) {
    const response = await fetch(blocksAtlasUrl, { signal })
    if (!response.ok) throw new Error('Unable to load the built-in Minecraft texture atlas.')
    const bitmap = await createImageBitmap(await response.blob())
    if (signal?.aborted) {
        bitmap.close()
        signal.throwIfAborted()
    }
    return bitmap
}

export const BUILTIN_ATLAS_WIDTH = 2048
export const BUILTIN_ATLAS_IMAGE_HEIGHT = 2128
export const BUILTIN_ATLAS_HEIGHT = 4096
