<script setup lang="ts">
defineProps<{ title: string; description: string }>()
</script>

<template>
    <header class="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div class="min-w-0">
            <h1 class="text-[22px] font-semibold leading-8">{{ title }}</h1>
            <p class="mt-1 max-w-3xl text-sm leading-6 text-muted-foreground">{{ description }}</p>
        </div>
        <slot />
    </header>
</template>
