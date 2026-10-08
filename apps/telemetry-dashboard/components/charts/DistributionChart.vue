<script setup lang="ts">
import type { DistributionItemDto } from '~/shared/types/telemetry'
import { formatNumber } from '~/utils/format'

const props = defineProps<{ items: DistributionItemDto[]; color: string }>()
const maximum = computed(() => Math.max(1, ...props.items.map((item) => item.value)))
</script>

<template>
    <div v-if="items.length" class="h-56 space-y-3.5 overflow-y-auto pr-1">
        <div
            v-for="item in items"
            :key="item.label"
            class="group"
            :title="`${item.label}: ${formatNumber(item.value)}`"
        >
            <div class="mb-1 flex items-center justify-between gap-3 text-xs">
                <span class="truncate font-medium">{{ item.label }}</span>
                <span class="shrink-0 tabular-nums text-muted-foreground">{{
                    formatNumber(item.value)
                }}</span>
            </div>
            <div class="h-1.5 overflow-hidden rounded-sm bg-surface-3">
                <div
                    class="h-full rounded-sm transition-[width] duration-300"
                    :style="{ width: `${(item.value / maximum) * 100}%`, backgroundColor: color }"
                ></div>
            </div>
        </div>
    </div>
    <div
        v-else
        class="flex h-56 items-center justify-center text-sm text-muted-foreground"
        data-state="empty"
    >
        当前 UTC 时间范围内暂无数据
    </div>
</template>
