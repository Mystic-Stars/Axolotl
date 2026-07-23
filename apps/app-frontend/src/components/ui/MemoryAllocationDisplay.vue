<script setup lang="ts">
import { SparklesIcon, SpinnerIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { get_memory_status, optimize_memory } from '@/helpers/jre.js'

const props = withDefaults(
	defineProps<{
		instanceId?: string
		memory: { maximum: number; automatic: boolean }
		showOptimizeButton?: boolean
	}>(),
	{ instanceId: undefined, showOptimizeButton: false },
)

const { addNotification, handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	used: { id: 'app.memory-display.used', defaultMessage: 'Memory in use' },
	game: { id: 'app.memory-display.game', defaultMessage: 'Game allocation' },
	remaining: { id: 'app.memory-display.remaining', defaultMessage: 'Remaining after allocation' },
	available: { id: 'app.memory-display.available', defaultMessage: '{memory} available' },
	optimize: { id: 'app.memory-display.optimize', defaultMessage: 'Optimize memory' },
	optimizing: { id: 'app.memory-display.optimizing', defaultMessage: 'Optimizing...' },
	optimized: { id: 'app.memory-display.optimized', defaultMessage: 'Memory optimization complete' },
	optimizationDescription: {
		id: 'app.memory-display.optimization-description',
		defaultMessage: 'Free unused Windows working sets and standby memory.',
	},
	unsupported: {
		id: 'app.memory-display.unsupported',
		defaultMessage: 'Memory optimization is only available on Windows.',
	},
	reclaimed: { id: 'app.memory-display.reclaimed', defaultMessage: 'Freed {memory} of memory.' },
})

type MemoryStatus = {
	total_bytes: number
	available_bytes: number
	allocated_mb: number
	optimization_supported: boolean
}

const status = ref<MemoryStatus | null>(null)
const optimizing = ref(false)
let timer: ReturnType<typeof setInterval> | undefined

const totalGiB = computed(() => (status.value?.total_bytes ?? 0) / 1024 ** 3)
const availableGiB = computed(() => (status.value?.available_bytes ?? 0) / 1024 ** 3)
const usedGiB = computed(() => Math.max(totalGiB.value - availableGiB.value, 0))
const allocatedGiB = computed(() => (status.value?.allocated_mb ?? props.memory.maximum) / 1024)
const allocatedAvailableGiB = computed(() => Math.min(allocatedGiB.value, availableGiB.value))
const remainingGiB = computed(() => Math.max(availableGiB.value - allocatedGiB.value, 0))
const allocationLimited = computed(() => allocatedGiB.value > availableGiB.value)
const optimizationSupported = computed(() => status.value?.optimization_supported ?? false)

function percentage(value: number) {
	return totalGiB.value > 0
		? `${Math.max(0, Math.min(100, (value / totalGiB.value) * 100))}%`
		: '0%'
}

function formatGiB(value: number) {
	return `${value.toFixed(1)} GB`
}

async function refresh() {
	try {
		status.value = await get_memory_status(
			props.instanceId ?? null,
			props.memory.maximum,
			props.memory.automatic,
		)
	} catch {
		// The display retries on the next interval while the backend is starting.
	}
}

async function handleOptimize() {
	if (optimizing.value || !optimizationSupported.value) return
	optimizing.value = true
	try {
		const result = await optimize_memory()
		if (result?.supported) {
			await refresh()
			addNotification({
				type: 'success',
				title: formatMessage(messages.optimized),
				text: formatMessage(messages.reclaimed, {
					memory: formatGiB(result.reclaimed_bytes / 1024 ** 3),
				}),
			})
		}
	} catch (error) {
		handleError(error)
	} finally {
		optimizing.value = false
	}
}

watch(
	() => [props.instanceId, props.memory.maximum, props.memory.automatic],
	() => void refresh(),
)

onMounted(() => {
	void refresh()
	timer = setInterval(() => void refresh(), 1000)
})

onBeforeUnmount(() => {
	if (timer) clearInterval(timer)
})
</script>

<template>
	<div class="mt-2 min-w-0">
		<div v-if="!status" class="h-10 animate-pulse rounded-lg bg-button-bg" />
		<template v-else>
			<div
				class="flex h-2 w-full overflow-hidden rounded-full bg-bg-gray"
				role="meter"
				:aria-label="formatMessage(messages.used)"
				:aria-valuenow="usedGiB + allocatedAvailableGiB"
				aria-valuemin="0"
				:aria-valuemax="totalGiB"
			>
				<span
					class="bg-gray transition-[width] duration-500"
					:style="{ width: percentage(usedGiB) }"
				/>
				<span
					class="bg-brand transition-[width] duration-500"
					:style="{ width: percentage(allocatedAvailableGiB) }"
				/>
			</div>
			<div class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs">
				<span class="flex min-w-0 items-center gap-1.5">
					<span class="size-2 shrink-0 rounded-full bg-gray" />
					<span class="text-secondary">{{ formatMessage(messages.used) }}</span>
					<span class="whitespace-nowrap font-semibold tabular-nums text-contrast">
						{{ formatGiB(usedGiB) }} / {{ formatGiB(totalGiB) }}
					</span>
				</span>
				<span class="flex min-w-0 items-center gap-1.5">
					<span class="size-2 shrink-0 rounded-full bg-brand" />
					<span class="text-secondary">{{ formatMessage(messages.game) }}</span>
					<span class="whitespace-nowrap font-semibold tabular-nums text-contrast">
						{{ formatGiB(allocatedGiB) }}
						<span v-if="allocationLimited"
							>({{ formatMessage(messages.available, { memory: formatGiB(availableGiB) }) }})</span
						>
					</span>
				</span>
				<span class="flex min-w-0 items-center gap-1.5">
					<span class="size-2 shrink-0 rounded-full bg-bg-gray ring-1 ring-inset ring-divider" />
					<span class="text-secondary">{{ formatMessage(messages.remaining) }}</span>
					<span class="whitespace-nowrap font-semibold tabular-nums text-contrast">
						{{ formatGiB(remainingGiB) }}
					</span>
				</span>
			</div>
			<div
				v-if="showOptimizeButton"
				class="mt-3 flex flex-col items-start justify-between gap-3 border-t border-divider pt-3 sm:flex-row sm:items-center"
			>
				<p class="m-0 min-w-0 flex-1 text-xs leading-tight text-secondary">
					{{
						formatMessage(
							optimizationSupported ? messages.optimizationDescription : messages.unsupported,
						)
					}}
				</p>
				<ButtonStyled>
					<button
						type="button"
						:disabled="optimizing || !optimizationSupported"
						@click="handleOptimize"
					>
						<SpinnerIcon v-if="optimizing" class="animate-spin" />
						<SparklesIcon v-else />
						{{ formatMessage(optimizing ? messages.optimizing : messages.optimize) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</div>
</template>
