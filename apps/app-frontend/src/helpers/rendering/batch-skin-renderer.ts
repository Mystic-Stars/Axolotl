import { reactive } from 'vue'

import type { Skin } from '../skins'
import { get_normalized_skin_texture } from '../skins'
import { headStorage } from '../storage/head-storage'
import { skinPreviewStorage } from '../storage/skin-preview-storage'

export interface RenderResult {
    forwards: string
}

export interface RawRenderResult {
    forwards: Blob
}

export const skinBlobUrlMap = reactive(new Map<string, RenderResult>())
export const headBlobUrlMap = reactive(new Map<string, string>())
const headRenderPromises = new Map<string, Promise<string>>()

const DEBUG_MODE = false
const HEAD_RENDER_VERSION = 8

export function getHeadRenderKey(textureKey: string): string {
    return `${textureKey}-head-v${HEAD_RENDER_VERSION}`
}

export async function cleanupUnusedPreviews(skins: Skin[]): Promise<void> {
    const validKeys = new Set<string>()
    const validHeadKeys = new Set<string>()

    for (const skin of skins) {
        const key = `${skin.texture_key}+${skin.variant}+${skin.cape_id ?? 'no-cape'}`
        const headKey = getHeadRenderKey(skin.texture_key)
        validKeys.add(key)
        validHeadKeys.add(headKey)
    }

    try {
        await skinPreviewStorage.cleanupInvalidKeys(validKeys)
        await headStorage.cleanupInvalidKeys(validHeadKeys)
    } catch (error) {
        console.warn('Failed to cleanup unused skin previews:', error)
    }
}

export async function generatePlayerHeadBlob(skinUrl: string): Promise<Blob> {
    return new Promise((resolve, reject) => {
        const img = new Image()
        img.crossOrigin = 'anonymous'

        img.onload = () => {
            try {
                if (img.width !== 64 || img.height !== 64) {
                    throw new Error(
                        `Expected normalized 64x64 skin texture, got ${img.width}x${img.height}`,
                    )
                }

                const outerLayerCanvas = document.createElement('canvas')
                outerLayerCanvas.width = 8
                outerLayerCanvas.height = 8
                const outerLayerCtx = outerLayerCanvas.getContext('2d')

                if (!outerLayerCtx) {
                    throw new Error('Could not get 2D context for outer skin layer')
                }

                outerLayerCtx.drawImage(img, 40, 8, 8, 8, 0, 0, 8, 8)
                const hasOuterLayer = outerLayerCtx
                    .getImageData(0, 0, 8, 8)
                    .data.some((channel, index) => index % 4 === 3 && channel > 0)
                const outputSize = hasOuterLayer ? 72 : 64
                const baseOffset = hasOuterLayer ? 4 : 0
                const outputCanvas = document.createElement('canvas')
                outputCanvas.width = outputSize
                outputCanvas.height = outputSize
                const outputCtx = outputCanvas.getContext('2d')

                if (!outputCtx) {
                    throw new Error('Could not get 2D context from output canvas')
                }

                outputCtx.imageSmoothingEnabled = false

                outputCtx.drawImage(img, 8, 8, 8, 8, baseOffset, baseOffset, 64, 64)

                if (hasOuterLayer) {
                    outputCtx.drawImage(outerLayerCanvas, 0, 0, 8, 8, 0, 0, outputSize, outputSize)
                }

                outputCanvas.toBlob((blob) => {
                    if (blob) {
                        resolve(blob)
                    } else {
                        reject(new Error('Failed to create blob from canvas'))
                    }
                }, 'image/png')
            } catch (error) {
                reject(error)
            }
        }

        img.onerror = () => {
            reject(new Error('Failed to load skin texture image'))
        }

        img.src = skinUrl
    })
}

export async function generateHeadRender(skin: Skin): Promise<string> {
    const headKey = getHeadRenderKey(skin.texture_key)

    if (headBlobUrlMap.has(headKey)) {
        if (DEBUG_MODE) {
            const url = headBlobUrlMap.get(headKey)!
            URL.revokeObjectURL(url)
            headBlobUrlMap.delete(headKey)
        } else {
            return headBlobUrlMap.get(headKey)!
        }
    }

    const pendingRender = headRenderPromises.get(headKey)
    if (pendingRender) return await pendingRender

    const renderPromise = loadHeadRender(skin, headKey)
    headRenderPromises.set(headKey, renderPromise)

    try {
        return await renderPromise
    } finally {
        if (headRenderPromises.get(headKey) === renderPromise) {
            headRenderPromises.delete(headKey)
        }
    }
}

async function loadHeadRender(skin: Skin, headKey: string): Promise<string> {
    if (!DEBUG_MODE) {
        try {
            const cachedHeadUrl = await headStorage.retrieve(headKey)
            if (cachedHeadUrl) {
                headBlobUrlMap.set(headKey, cachedHeadUrl)
                return cachedHeadUrl
            }
        } catch (error) {
            console.warn('Failed to retrieve cached head render:', error)
        }
    }

    const skinUrl = await get_normalized_skin_texture(skin)
    const headBlob = await generatePlayerHeadBlob(skinUrl)
    const headUrl = URL.createObjectURL(headBlob)

    headBlobUrlMap.set(headKey, headUrl)

    try {
        await headStorage.store(headKey, headBlob)
    } catch (error) {
        console.warn('Failed to store head render in persistent storage:', error)
    }

    return headUrl
}

export async function getPlayerHeadUrl(skin: Skin): Promise<string> {
    return await generateHeadRender(skin)
}
