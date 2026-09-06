<script setup lang="ts">
import { VisAxis, VisLine, VisXYContainer } from '@unovis/vue'
import { computed, ref } from 'vue'

import type { DailyPointDto } from '~/shared/types/telemetry'
import { formatNumber, formatUtcDay } from '~/utils/format'

const props = defineProps<{
	data: DailyPointDto[]
	series: Array<{
		key: 'activeInstallations' | 'newInstallations'
		label: string
		color: string
	}>
}>()

const hovered = ref<number | null>(null)
const available = computed(() =>
	props.data.some((point) => props.series.some((series) => Number(point[series.key]) > 0)),
)
const yAccessors = computed(() =>
	props.series.map((series) => (point: DailyPointDto) => point[series.key]),
)
const colors = computed(() => props.series.map((series) => series.color))

function move(event: PointerEvent): void {
	if (!props.data.length) return
	const box = (event.currentTarget as HTMLElement).getBoundingClientRect()
	const ratio = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width))
	hovered.value = Math.min(props.data.length - 1, Math.round(ratio * (props.data.length - 1)))
}
</script>

<template>
	<div class="relative h-64 min-w-0" @pointermove="move" @pointerleave="hovered = null">
		<div v-if="available" class="h-56 w-full">
			<VisXYContainer :data="data" :margin="{ left: 16, right: 12, top: 12, bottom: 28 }">
				<VisLine
					:x="(_point: DailyPointDto, index: number) => index"
					:y="yAccessors"
					:color="colors"
					:line-width="2"
				/>
				<VisAxis
					type="x"
					:x="(_point: DailyPointDto, index: number) => index"
					:tick-format="(value: number) => formatUtcDay(data[Math.round(value)]?.day || '')"
					:num-ticks="4"
				/>
				<VisAxis type="y" :num-ticks="4" :tick-format="(value: number) => formatNumber(value)" />
			</VisXYContainer>
		</div>
		<div
			v-else
			class="flex h-56 items-center justify-center text-sm text-muted-foreground"
			data-state="empty"
		>
			当前 UTC 时间范围内暂无数据
		</div>
		<div
			class="flex h-8 flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground"
			aria-label="图表图例"
		>
			<span v-for="item in series" :key="item.key" class="inline-flex items-center gap-1.5">
				<span class="size-2 rounded-sm" :style="{ backgroundColor: item.color }"></span>
				{{ item.label }}
			</span>
		</div>
		<div
			v-if="hovered !== null && available"
			class="pointer-events-none absolute right-2 top-2 z-10 min-w-40 rounded-md border bg-popover p-2 text-xs text-popover-foreground shadow-md"
			role="tooltip"
		>
			<p class="mb-1 font-medium">{{ data[hovered].day }}（UTC）</p>
			<p
				v-for="item in series"
				:key="item.key"
				class="flex justify-between gap-4 py-0.5 text-muted-foreground"
			>
				<span>{{ item.label }}</span>
				<span class="font-medium tabular-nums text-foreground">{{
					formatNumber(data[hovered][item.key])
				}}</span>
			</p>
		</div>
	</div>
</template>
