<script setup lang="ts">
import type { Component } from 'vue'

import Card from '~/components/ui/Card.vue'
import { formatNumber } from '~/utils/format'

const props = withDefaults(
    defineProps<{
        title: string
        value: number
        detail: string
        icon: Component
        tone?: 'green' | 'cyan' | 'gold' | 'coral' | 'neutral'
    }>(),
    { tone: 'neutral' },
)

const tones = {
    green: 'bg-emerald-500/12 text-emerald-700 dark:text-emerald-300',
    cyan: 'bg-cyan-500/12 text-cyan-700 dark:text-cyan-300',
    gold: 'bg-amber-500/12 text-amber-700 dark:text-amber-300',
    coral: 'bg-rose-500/12 text-rose-700 dark:text-rose-300',
    neutral: 'bg-surface-3 text-muted-foreground',
}
</script>

<template>
    <Card class="min-w-0 overflow-hidden">
        <div class="flex items-start justify-between gap-3 p-4 pb-3">
            <div class="min-w-0">
                <p class="truncate text-xs font-medium text-muted-foreground">{{ title }}</p>
                <p class="mt-2 text-[28px] font-semibold tabular-nums leading-8">
                    {{ formatNumber(value) }}
                </p>
            </div>
            <div
                class="flex size-9 shrink-0 items-center justify-center rounded-md"
                :class="tones[props.tone]"
            >
                <component :is="icon" class="size-[18px]" />
            </div>
        </div>
        <p
            class="truncate border-t border-surface-4 bg-surface-3/45 px-4 py-2 text-[11px] text-muted-foreground"
            :title="detail"
        >
            {{ detail }}
        </p>
    </Card>
</template>
