<script setup lang="ts">
import searchingVisual from '@/assets/axo-searching.svg?url'
import teachingVisual from '@/assets/axo-teaching.svg?url'

defineProps<{
    alt: string
}>()

const mascotVisuals = [teachingVisual, searchingVisual]
const mascotVisual = mascotVisuals[Math.floor(Math.random() * mascotVisuals.length)]
</script>

<template>
    <img class="onboarding-mascot" :src="mascotVisual" :alt="alt" />
</template>

<style scoped lang="scss">
.onboarding-mascot {
    width: clamp(6rem, 11vw, 9rem);
    height: auto;
    object-fit: contain;
}
</style>
