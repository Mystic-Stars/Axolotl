<script setup lang="ts">
defineProps<{
    modelValue: string
    label: string
    items: Array<{ value: string; label: string }>
}>()

defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
    <div
        role="tablist"
        :aria-label="label"
        class="inline-flex h-9 items-center rounded-md border border-surface-4 bg-surface-3 p-1"
    >
        <button
            v-for="item in items"
            :key="item.value"
            type="button"
            role="tab"
            :aria-selected="modelValue === item.value"
            class="h-7 cursor-pointer rounded px-2.5 text-xs font-medium text-muted-foreground transition-[color,background-color,box-shadow] hover:text-foreground active:bg-surface-4"
            :class="modelValue === item.value && 'bg-surface-2 text-foreground shadow-sm'"
            @click="$emit('update:modelValue', item.value)"
        >
            {{ item.label }}
        </button>
    </div>
</template>
