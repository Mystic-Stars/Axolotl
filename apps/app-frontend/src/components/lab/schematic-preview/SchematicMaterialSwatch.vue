<script setup lang="ts">
import { onBeforeUnmount, onMounted, useTemplateRef, watch } from 'vue'

import type { SchematicBlockState } from '@/lab/schematic-preview/backend'
import { renderSchematicBlockPreview } from '@/lab/schematic-preview/block-preview'
import type { LoadedSchematicResources } from '@/lab/schematic-preview/resources'

const props = defineProps<{
    atlas: HTMLCanvasElement
    uv?: [number, number, number, number]
    fallbackColor: string
    state?: SchematicBlockState
    resources?: LoadedSchematicResources
}>()

const canvas = useTemplateRef<HTMLCanvasElement>('canvas')
let visible = false
let observer: IntersectionObserver | undefined

function render() {
    const target = canvas.value
    if (!target || !visible) return
    if (
        props.state &&
        props.resources &&
        renderSchematicBlockPreview(target, props.state, props.resources)
    ) {
        return
    }
    const context = target.getContext('2d')
    if (!context) return
    context.clearRect(0, 0, target.width, target.height)
    context.imageSmoothingEnabled = false
    if (!props.uv) {
        context.fillStyle = props.fallbackColor
        context.fillRect(0, 0, target.width, target.height)
        return
    }
    const [u0, v0, u1, v1] = props.uv
    const sourceX = Math.round(u0 * props.atlas.width)
    const sourceY = Math.round(v0 * props.atlas.height)
    const sourceWidth = Math.max(1, Math.round((u1 - u0) * props.atlas.width))
    const sourceHeight = Math.max(1, Math.round((v1 - v0) * props.atlas.height))
    context.drawImage(
        props.atlas,
        sourceX,
        sourceY,
        sourceWidth,
        sourceHeight,
        0,
        0,
        target.width,
        target.height,
    )
}

watch(() => [props.atlas, props.uv, props.fallbackColor, props.state, props.resources], render)
onMounted(() => {
    const target = canvas.value
    if (!target || typeof IntersectionObserver === 'undefined') {
        visible = true
        render()
        return
    }
    observer = new IntersectionObserver((entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return
        visible = true
        observer?.disconnect()
        observer = undefined
        render()
    })
    observer.observe(target)
})
onBeforeUnmount(() => observer?.disconnect())
</script>

<template>
    <canvas ref="canvas" :width="64" :height="64" aria-hidden="true"></canvas>
</template>
