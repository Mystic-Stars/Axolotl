<template>
    <span class="inline-flex items-center gap-1 font-semibold text-[var(--color-text-tertiary)]">
        <component :is="icon" v-if="icon" :aria-hidden="true" class="shrink-0" />
        {{ label }}
    </span>
</template>

<script setup lang="ts">
import type { Component } from 'vue'

defineProps<{
    icon?: Component
    label: string
}>()
</script>
