<script setup lang="ts">
import { SpinnerIcon } from '@modrinth/assets'
import { computed } from 'vue'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import ProgressBar from '#ui/components/base/ProgressBar.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import type { BatchDropItem, BatchDropScanState } from '#ui/composables/use-batch-drop'

const { formatMessage } = useVIntl()

const props = defineProps<{
	items: BatchDropItem[]
	done: number
	total: number
}>()

const emit = defineEmits<{
	(e: 'cancel'): void
}>()

const messages = defineMessages({
	title: {
		id: 'app.drop.batch.scan.title',
		defaultMessage: 'Scanning {count, plural, one {# file} other {# files}}…',
	},
	subtitle: {
		id: 'app.drop.batch.scan.subtitle',
		defaultMessage: 'Recognized {done} of {total}',
	},
	cancel: {
		id: 'app.drop.batch.scan.cancel',
		defaultMessage: 'Cancel',
	},
	pending: {
		id: 'app.drop.batch.scan.pending',
		defaultMessage: 'Waiting',
	},
	scanning: {
		id: 'app.drop.batch.scan.scanning',
		defaultMessage: 'Scanning',
	},
	done: {
		id: 'app.drop.batch.scan.done',
		defaultMessage: 'Recognized',
	},
	skipped: {
		id: 'app.drop.batch.scan.skipped',
		defaultMessage: 'Skipped',
	},
	error: {
		id: 'app.drop.batch.scan.error',
		defaultMessage: 'Failed',
	},
})

const statusMessages: Record<BatchDropScanState, { id: string; defaultMessage: string }> = {
	pending: messages.pending,
	scanning: messages.scanning,
	done: messages.done,
	skipped: messages.skipped,
	error: messages.error,
}

const doneCount = computed(() => props.done)

function statusLabel(state: BatchDropScanState): string {
	return formatMessage(statusMessages[state])
}

function statusClass(state: BatchDropScanState): string {
	switch (state) {
		case 'scanning':
			return 'text-brand'
		case 'done':
			return 'text-green'
		case 'skipped':
			return 'text-orange'
		case 'error':
			return 'text-danger'
		default:
			return 'text-secondary'
	}
}
</script>

<template>
	<div class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/40 p-6">
		<div class="w-full max-w-lg rounded-2xl bg-surface-2 p-6 shadow-xl">
			<div class="flex items-center gap-3">
				<SpinnerIcon class="h-6 w-6 shrink-0 animate-spin text-contrast" />
				<div class="min-w-0 flex-1">
					<p class="truncate text-sm font-semibold text-contrast">
						{{ formatMessage(messages.title, { count: total }) }}
					</p>
					<p class="text-xs text-secondary">
						{{ formatMessage(messages.subtitle, { done: doneCount, total }) }}
					</p>
				</div>
				<ButtonStyled type="transparent" size="small">
					<button type="button" @click="emit('cancel')">
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
			</div>

			<ProgressBar class="mt-3" :progress="doneCount" :max="total" full-width show-progress />

			<div class="mt-4 flex max-h-64 flex-col gap-1 overflow-y-auto">
				<div
					v-for="item in items"
					:key="item.id"
					class="flex items-center justify-between gap-2 rounded-lg bg-surface-1 px-3 py-2"
				>
					<span class="min-w-0 truncate text-sm text-contrast">{{ item.name }}</span>
					<span class="shrink-0 text-xs" :class="statusClass(item.scanState)">
						{{ statusLabel(item.scanState) }}
					</span>
				</div>
			</div>
		</div>
	</div>
</template>
