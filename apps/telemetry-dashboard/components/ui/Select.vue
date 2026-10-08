<script setup lang="ts">
import { ChevronDown } from 'lucide-vue-next'

defineProps<{
    modelValue: string
    label: string
    options: Array<{ value: string; label: string }>
}>()

defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
    <label class="relative inline-flex min-w-0 items-center">
        <span class="sr-only">{{ label }}</span>
        <select
            :value="modelValue"
            class="h-9 max-w-full cursor-pointer appearance-none rounded-md border border-surface-4 bg-surface-2 py-0 pl-3 pr-8 text-sm text-foreground outline-none transition-colors hover:border-surface-5 hover:bg-surface-3 focus:ring-2 focus:ring-ring"
            @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
        >
            <option v-for="option in options" :key="option.value" :value="option.value">
                {{ option.label }}
            </option>
        </select>
        <ChevronDown class="pointer-events-none absolute right-2 size-4 text-muted-foreground" />
    </label>
</template>
