<script setup lang="ts">
import { CircleCheck, CircleMinus, TriangleAlert } from 'lucide-vue-next'

import Badge from '~/components/ui/Badge.vue'
import type { ServiceCheckDto } from '~/shared/types/telemetry'

defineProps<{ name: string; check: ServiceCheckDto }>()

const icons = { available: CircleCheck, degraded: TriangleAlert, unavailable: CircleMinus }
const variants = { available: 'success', degraded: 'warning', unavailable: 'secondary' } as const
</script>

<template>
    <div
        class="flex flex-wrap items-center gap-3 border-b border-surface-4 px-4 py-3.5 transition-colors last:border-0 hover:bg-surface-3/45"
    >
        <component
            :is="icons[check.status]"
            class="size-4 shrink-0"
            :class="
                check.status === 'available'
                    ? 'text-emerald-600'
                    : check.status === 'degraded'
                      ? 'text-amber-600'
                      : 'text-muted-foreground'
            "
        />
        <div class="min-w-48 flex-1">
            <p class="text-sm font-medium">{{ name }}</p>
            <p class="mt-0.5 text-xs text-muted-foreground">{{ check.detail }}</p>
        </div>
        <Badge :variant="variants[check.status]">{{ check.label }}</Badge>
    </div>
</template>
