<template>
	<div v-if="count > 1" class="flex items-center gap-1" :class="{ 'opacity-60': loading }">
		<ButtonStyled v-if="page > 1" circular type="transparent">
			<a
				v-if="linkFunction"
				aria-label="Previous Page"
				:href="linkFunction(page - 1)"
				:aria-disabled="loading"
				@click.prevent="!loading && switchPage(page - 1)"
			>
				<ChevronLeftIcon />
			</a>
			<button v-else aria-label="Previous Page" :disabled="loading" @click="switchPage(page - 1)">
				<ChevronLeftIcon />
			</button>
		</ButtonStyled>
		<div
			v-for="(item, index) in pages"
			:key="'page-' + item + '-' + index"
			:class="{
				'page-number': page !== item,
				shrink: item !== '-' && item > 99,
			}"
			class="page-number-container"
		>
			<template v-if="item === '-'">
				<input
					v-if="activeGapIndex === index"
					ref="gapInput"
					v-model="gapInputValue"
					class="h-8 w-12 rounded-full border border-solid border-brand bg-surface-1 px-2 text-center text-sm text-contrast outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow"
					type="number"
					inputmode="numeric"
					min="1"
					:max="count"
					:placeholder="formatMessage(messages.goToPagePlaceholder)"
					:aria-label="formatMessage(messages.goToPage)"
					@keydown.enter.prevent="commitGapInput"
					@keydown.esc.prevent="closeGapInput"
					@blur="commitGapInput"
				/>
				<button
					v-else
					type="button"
					class="grid h-8 w-8 place-content-center rounded-full text-secondary transition-colors hover:bg-surface-3 hover:text-contrast focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow"
					:aria-label="formatMessage(messages.goToPage)"
					aria-expanded="false"
					:disabled="loading"
					@click="openGapInput(index)"
				>
					<span class="rotate-90 grid place-content-center">
						<EllipsisVerticalIcon />
					</span>
				</button>
			</template>
			<template v-else-if="loading && page === item">
				<span
					class="grid size-8 place-content-center rounded-full bg-button-bg text-brand"
					:aria-label="formatMessage(messages.loadingPage)"
					role="status"
				>
					<SpinnerIcon class="size-4 animate-spin" />
				</span>
			</template>
			<ButtonStyled
				v-else
				circular
				:color="page === item ? 'brand' : 'standard'"
				:type="page === item ? 'highlight' : 'transparent'"
			>
				<a
					v-if="linkFunction"
					:href="linkFunction(item)"
					:class="page === item ? '!text-brand' : ''"
					:aria-disabled="loading"
					@click.prevent="!loading && page !== item ? switchPage(item) : null"
				>
					{{ item }}
				</a>
				<button
					v-else
					:class="page === item ? '!text-brand' : ''"
					:disabled="loading"
					@click="page !== item ? switchPage(item) : null"
				>
					{{ item }}
				</button>
			</ButtonStyled>
		</div>

		<ButtonStyled v-if="page !== pages[pages.length - 1]" circular type="transparent">
			<a
				v-if="linkFunction"
				aria-label="Next Page"
				:href="linkFunction(page + 1)"
				:aria-disabled="loading"
				@click.prevent="!loading && switchPage(page + 1)"
			>
				<ChevronRightIcon />
			</a>
			<button v-else aria-label="Next Page" :disabled="loading" @click="switchPage(page + 1)">
				<ChevronRightIcon />
			</button>
		</ButtonStyled>
	</div>
</template>
<script setup lang="ts">
import {
	ChevronLeftIcon,
	ChevronRightIcon,
	EllipsisVerticalIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import { computed, nextTick, ref, watch } from 'vue'

import { defineMessages, useVIntl } from '../../composables/i18n'
import ButtonStyled from './ButtonStyled.vue'

const emit = defineEmits<{
	'switch-page': [page: number]
}>()

const props = withDefaults(
	defineProps<{
		page: number
		count: number
		loading?: boolean
		linkFunction?: (page: number) => string | undefined
	}>(),
	{
		page: 1,
		count: 1,
		loading: false,
	},
)

const { formatMessage } = useVIntl()

const messages = defineMessages({
	goToPage: {
		id: 'pagination.go-to-page',
		defaultMessage: 'Go to page',
	},
	goToPagePlaceholder: {
		id: 'pagination.go-to-page.placeholder',
		defaultMessage: 'Page',
	},
	loadingPage: {
		id: 'pagination.loading-page',
		defaultMessage: 'Loading page…',
	},
})

const pages = computed(() => {
	const pages: ('-' | number)[] = []

	const first = 1
	const last = props.count
	const current = props.page
	const prev = current - 1
	const next = current + 1
	const gap = '-'

	if (prev > first) {
		pages.push(first)
	}
	if (prev > first + 1) {
		pages.push(gap)
	}
	if (prev >= first) {
		pages.push(prev)
	}
	pages.push(current)
	if (next <= last) {
		pages.push(next)
	}
	if (next < last - 1) {
		pages.push(gap)
	}
	if (next < last) {
		pages.push(last)
	}

	return pages
})

const activeGapIndex = ref<number | null>(null)
const gapInputValue = ref('')
const gapInput = ref<HTMLInputElement[] | HTMLInputElement>()

async function openGapInput(index: number) {
	activeGapIndex.value = index
	gapInputValue.value = ''
	await nextTick()
	const el = Array.isArray(gapInput.value) ? gapInput.value[0] : gapInput.value
	el?.focus()
	el?.select()
}

function closeGapInput() {
	activeGapIndex.value = null
	gapInputValue.value = ''
}

function commitGapInput() {
	if (activeGapIndex.value === null) return

	const parsed = Number.parseInt(gapInputValue.value, 10)
	if (Number.isFinite(parsed) && parsed >= 1 && parsed <= props.count && parsed !== props.page) {
		switchPage(parsed)
	}
	closeGapInput()
}

watch(
	() => props.page,
	() => {
		closeGapInput()
	},
)

function switchPage(newPage: number) {
	emit('switch-page', Math.min(Math.max(newPage, 1), props.count))
}
</script>
