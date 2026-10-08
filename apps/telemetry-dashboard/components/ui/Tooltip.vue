<script setup lang="ts">
defineProps<{ text: string }>()
</script>

<template>
    <span class="group relative inline-flex">
        <slot />
        <span
            role="tooltip"
            class="pointer-events-none absolute left-1/2 top-full z-50 mt-2 hidden w-max max-w-56 -translate-x-1/2 rounded-md bg-foreground px-2 py-1 text-xs text-background shadow-md group-focus-within:block group-hover:block"
        >
            {{ text }}
        </span>
    </span>
</template>
