<script setup lang="ts">
import {
	CalendarIcon,
	ChevronLeftIcon,
	ChevronRightIcon,
	PlayIcon,
	StopCircleIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	useFormatDateTime,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { instance_listener, process_listener } from '@/helpers/events'
import {
	type DailyPlaytime,
	type DailyPlaytimeEntry,
	get_daily_playtime,
	get_daily_playtime_details,
	kill,
	run,
} from '@/helpers/instance'
import { get_all } from '@/helpers/process'
import type { GameInstance } from '@/helpers/types'
import { handleSevereError } from '@/store/error'

import {
	buildHeatmapDays,
	dateFromKey,
	endOfPeriod,
	getPlaytimeLevel,
	shiftPeriod,
	startOfPeriod,
	toDateKey,
} from './home-utils'

const props = defineProps<{
	instances: GameInstance[]
}>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const handleMinecraftLaunchError = useMinecraftLaunchError()
const formatPeriod = useFormatDateTime({ month: 'long', year: 'numeric' })
const formatWeekday = useFormatDateTime({ weekday: 'narrow' })
const formatDetailDate = useFormatDateTime({ month: 'long', day: 'numeric' })
const formatFullDate = useFormatDateTime({ dateStyle: 'full' })

const messages = defineMessages({
	calendar: { id: 'app.home.calendar.title', defaultMessage: 'Calendar' },
	thisMonth: { id: 'app.home.calendar.this-month', defaultMessage: 'This month' },
	previousMonth: { id: 'app.home.calendar.previous', defaultMessage: 'Previous month' },
	nextMonth: { id: 'app.home.calendar.next', defaultMessage: 'Next month' },
	playedOn: { id: 'app.home.calendar.played-on', defaultMessage: 'On {date} you played:' },
	noActivity: {
		id: 'app.home.calendar.no-activity',
		defaultMessage: 'No playtime recorded on this day.',
	},
	playInstance: { id: 'app.home.calendar.play', defaultMessage: 'Play' },
	stopInstance: { id: 'app.home.calendar.stop', defaultMessage: 'Stop' },
	minutes: { id: 'app.home.playtime.minutes', defaultMessage: '{minutes}m' },
	hoursMinutes: { id: 'app.home.playtime.hours-minutes', defaultMessage: '{hours}h {minutes}m' },
	seconds: { id: 'app.home.playtime.seconds', defaultMessage: '{seconds}s' },
	sessions: {
		id: 'app.home.playtime.sessions',
		defaultMessage: '{count, plural, one {# successful launch} other {# successful launches}}',
	},
	mostPlayed: { id: 'app.home.playtime.most-played', defaultMessage: 'Most played: {name}' },
})

const todayKey = toDateKey(new Date())
const anchor = ref(new Date())
const selectedKey = ref(todayKey)
const dailyPlaytime = ref<DailyPlaytime[]>([])
const dayDetails = ref<DailyPlaytimeEntry[]>([])
const runningInstanceIds = ref<string[]>([])
const activeTooltip = ref<{
	dateKey: string
	lines: string[]
	left: number
	top: number
} | null>(null)

const periodStart = computed(() => startOfPeriod(anchor.value, 'month'))
const periodEnd = computed(() => endOfPeriod(anchor.value, 'month'))
const periodLabel = computed(() => formatPeriod(periodStart.value))
const days = computed(() => buildHeatmapDays(anchor.value, 'month'))
const weekdayLabels = computed(() =>
	Array.from({ length: 7 }, (_, index) => formatWeekday(new Date(2024, 0, index + 1, 12))),
)
const dailyByDate = computed(() => new Map(dailyPlaytime.value.map((entry) => [entry.date, entry])))
const canGoForward = computed(() => toDateKey(periodEnd.value) < todayKey)
const instanceById = computed(
	() => new Map(props.instances.map((instance) => [instance.id, instance])),
)
const selectedDateLabel = computed(() => formatDetailDate(dateFromKey(selectedKey.value)))
const detailRows = computed(() =>
	dayDetails.value.map((entry) => ({
		entry,
		instance: instanceById.value.get(entry.instance_id),
	})),
)

function formatDuration(seconds: number): string {
	const roundedSeconds = Math.max(0, Math.round(seconds))
	const hours = Math.floor(roundedSeconds / 3600)
	const minutes = Math.floor((roundedSeconds % 3600) / 60)
	if (hours > 0) return formatMessage(messages.hoursMinutes, { hours, minutes })
	if (minutes > 0) return formatMessage(messages.minutes, { minutes })
	return formatMessage(messages.seconds, { seconds: roundedSeconds })
}

function tooltipLinesFor(dateKey: string): string[] {
	const entry = dailyByDate.value.get(dateKey)
	const lines = [formatFullDate(dateFromKey(dateKey))]
	if (entry && entry.played_seconds > 0) {
		lines.push(
			formatDuration(entry.played_seconds),
			formatMessage(messages.sessions, { count: entry.session_count }),
		)
		if (entry.top_instance_name) {
			lines.push(formatMessage(messages.mostPlayed, { name: entry.top_instance_name }))
		}
	} else {
		lines.push(formatMessage(messages.noActivity))
	}
	return lines
}

function showTooltip(event: PointerEvent | FocusEvent) {
	const target =
		event.target instanceof Element ? event.target.closest<HTMLElement>('[data-date-key]') : null
	const dateKey = target?.dataset.dateKey
	if (!target || !dateKey) {
		activeTooltip.value = null
		return
	}

	const rect = target.getBoundingClientRect()
	const halfWidth = Math.min(144, Math.max(0, (window.innerWidth - 24) / 2))
	activeTooltip.value = {
		dateKey,
		lines: tooltipLinesFor(dateKey),
		left: Math.min(
			Math.max(rect.left + rect.width / 2, 12 + halfWidth),
			window.innerWidth - 12 - halfWidth,
		),
		top: rect.top - 8,
	}
}

async function refreshPlaytime() {
	dailyPlaytime.value = await get_daily_playtime(
		toDateKey(periodStart.value),
		toDateKey(periodEnd.value),
	).catch((error): DailyPlaytime[] => {
		handleError(error)
		return []
	})
}

async function refreshDayDetails() {
	dayDetails.value = await get_daily_playtime_details(selectedKey.value).catch(
		(error): DailyPlaytimeEntry[] => {
			handleError(error)
			return []
		},
	)
}

function movePeriod(amount: number) {
	activeTooltip.value = null
	anchor.value = shiftPeriod(anchor.value, 'month', amount)
}

function goToThisMonth() {
	activeTooltip.value = null
	anchor.value = new Date()
	selectedKey.value = todayKey
}

function selectDay(dateKey: string) {
	if (dateKey > todayKey) return
	selectedKey.value = dateKey
}

async function checkProcesses() {
	const processes = await get_all().catch(() => [])
	runningInstanceIds.value = processes.map((process) => process.instance_id)
}

async function playInstance(instance: GameInstance) {
	try {
		await run(instance.id)
		trackEvent('InstanceStart', {
			loader: instance.loader,
			game_version: instance.game_version,
			source: 'HomeCalendar',
		})
	} catch (error) {
		const handled = await handleMinecraftLaunchError(error, {
			instance_id: instance.id,
			instance_name: instance.name,
		})
		if (!handled) handleSevereError(error, { instanceId: instance.id })
	}
}

async function stopInstance(instance: GameInstance) {
	await kill(instance.id).catch(handleError)
	trackEvent('InstanceStop', {
		loader: instance.loader,
		game_version: instance.game_version,
		source: 'HomeCalendar',
	})
}

watch(() => anchor.value.getTime(), refreshPlaytime, { immediate: true })
watch(selectedKey, refreshDayDetails, { immediate: true })

const unlistenInstances = await instance_listener(async () => {
	await refreshPlaytime()
	await refreshDayDetails()
})
const unlistenProcesses = await process_listener(checkProcesses)

onMounted(() => {
	void checkProcesses()
})

onUnmounted(() => {
	unlistenInstances()
	unlistenProcesses()
})
</script>

<template>
	<section
		data-onboarding-id="home-playtime"
		class="card-shadow flex min-w-0 flex-col gap-4 rounded-2xl bg-bg-raised p-4"
	>
		<div class="flex min-w-0 items-center gap-2">
			<CalendarIcon class="size-5 shrink-0 text-brand" aria-hidden="true" />
			<h2 class="m-0 truncate text-lg font-bold text-contrast">
				{{ formatMessage(messages.calendar) }}
			</h2>
			<div class="ml-auto flex shrink-0 items-center gap-1">
				<ButtonStyled circular size="small" type="transparent">
					<button v-tooltip="formatMessage(messages.previousMonth)" @click="movePeriod(-1)">
						<ChevronLeftIcon />
					</button>
				</ButtonStyled>
				<ButtonStyled type="chip" size="small">
					<button v-tooltip="formatMessage(messages.thisMonth)" @click="goToThisMonth">
						{{ periodLabel }}
					</button>
				</ButtonStyled>
				<ButtonStyled circular size="small" type="transparent">
					<button
						v-tooltip="formatMessage(messages.nextMonth)"
						:disabled="!canGoForward"
						@click="movePeriod(1)"
					>
						<ChevronRightIcon />
					</button>
				</ButtonStyled>
			</div>
		</div>
		<div
			@pointerover="showTooltip"
			@pointerleave="activeTooltip = null"
			@focusin="showTooltip"
			@focusout="activeTooltip = null"
		>
			<div class="home-calendar-weekdays" aria-hidden="true">
				<span v-for="(weekday, index) in weekdayLabels" :key="index">{{ weekday }}</span>
			</div>
			<div class="home-calendar-grid" role="grid" :aria-label="formatMessage(messages.calendar)">
				<button
					v-for="day in days"
					:key="day.dateKey"
					type="button"
					class="home-calendar-cell"
					:class="{
						'home-calendar-cell-outside': !day.inPeriod,
						'home-calendar-cell-future': day.inPeriod && day.dateKey > todayKey,
						'home-calendar-cell-selected': day.inPeriod && day.dateKey === selectedKey,
						'home-calendar-cell-today': day.inPeriod && day.dateKey === todayKey,
						[`home-calendar-level-${getPlaytimeLevel(dailyByDate.get(day.dateKey)?.played_seconds ?? 0)}`]:
							day.inPeriod && day.dateKey <= todayKey,
					}"
					:tabindex="day.inPeriod && day.dateKey <= todayKey ? 0 : -1"
					:disabled="!day.inPeriod || day.dateKey > todayKey"
					:data-date-key="day.inPeriod && day.dateKey <= todayKey ? day.dateKey : undefined"
					:aria-label="day.inPeriod ? day.dateKey : undefined"
					:aria-pressed="day.inPeriod ? day.dateKey === selectedKey : undefined"
					:aria-describedby="
						activeTooltip?.dateKey === day.dateKey ? 'home-calendar-tooltip' : undefined
					"
					role="gridcell"
					@click="selectDay(day.dateKey)"
				>
					<span v-if="day.inPeriod" aria-hidden="true">{{ day.date.getDate() }}</span>
				</button>
			</div>
		</div>
		<div class="flex min-w-0 flex-col gap-2 border-0 border-t border-solid border-divider pt-3">
			<h3 class="m-0 text-sm font-bold text-contrast">
				{{ formatMessage(messages.playedOn, { date: selectedDateLabel }) }}
			</h3>
			<p v-if="dayDetails.length === 0" class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.noActivity) }}
			</p>
			<ul v-else class="m-0 flex list-none flex-col p-0">
				<li
					v-for="row in detailRows"
					:key="row.entry.instance_id"
					class="group flex min-w-0 items-center gap-2.5 rounded-lg px-1.5 py-1.5 transition-colors hover:bg-button-bg"
				>
					<InstanceIcon
						:icon-path="row.instance?.icon_path"
						:instance-id="row.entry.instance_id"
						size="36px"
						class="shrink-0"
					/>
					<div class="flex min-w-0 flex-1 flex-col gap-0.5">
						<span class="truncate text-sm font-semibold text-contrast">
							{{ row.instance?.name ?? row.entry.instance_name }}
						</span>
						<span class="truncate text-xs text-secondary">
							{{ formatDuration(row.entry.played_seconds) }}
						</span>
					</div>
					<div v-if="row.instance" class="ml-auto shrink-0">
						<ButtonStyled
							v-if="runningInstanceIds.includes(row.instance.id)"
							circular
							size="small"
							type="transparent"
						>
							<button
								v-tooltip="formatMessage(messages.stopInstance)"
								class="!text-red"
								@click="stopInstance(row.instance)"
							>
								<StopCircleIcon />
							</button>
						</ButtonStyled>
						<ButtonStyled v-else circular size="small" type="transparent">
							<button
								v-tooltip="formatMessage(messages.playInstance)"
								class="!text-brand opacity-60 transition-opacity group-hover:opacity-100"
								@click="playInstance(row.instance)"
							>
								<PlayIcon />
							</button>
						</ButtonStyled>
					</div>
				</li>
			</ul>
		</div>
	</section>
	<Teleport to="body">
		<Transition name="home-calendar-tooltip">
			<div
				v-if="activeTooltip"
				id="home-calendar-tooltip"
				class="home-calendar-tooltip"
				role="tooltip"
				:style="{ left: `${activeTooltip.left}px`, top: `${activeTooltip.top}px` }"
			>
				<strong>{{ activeTooltip.lines[0] }}</strong>
				<span v-for="line in activeTooltip.lines.slice(1)" :key="line">{{ line }}</span>
			</div>
		</Transition>
	</Teleport>
</template>

<style scoped>
.home-calendar-weekdays {
	display: grid;
	grid-template-columns: repeat(7, minmax(0, 1fr));
	gap: 0.25rem;
	margin-bottom: 0.375rem;
	color: var(--color-secondary);
	font-size: 0.75rem;
	font-weight: 600;
	text-align: center;
}

.home-calendar-grid {
	display: grid;
	grid-template-columns: repeat(7, minmax(0, 1fr));
	gap: 0.25rem;
}

.home-calendar-cell {
	aspect-ratio: 1 / 1;
	border: 1px solid transparent;
	border-radius: var(--radius-sm);
	background: transparent;
	color: var(--color-contrast);
	font-size: 0.75rem;
	font-weight: 600;
	outline: none;
	padding: 0;
	cursor: pointer;
	transition:
		box-shadow 100ms ease,
		background-color 100ms ease;
}

.home-calendar-cell:focus-visible {
	box-shadow: 0 0 0 2px var(--color-brand);
}

.home-calendar-cell-outside,
.home-calendar-cell-future {
	cursor: default;
}

.home-calendar-cell-future {
	color: var(--color-secondary);
	opacity: 0.5;
}

.home-calendar-cell-today {
	border-color: var(--color-brand);
}

.home-calendar-cell-selected {
	box-shadow: 0 0 0 2px var(--color-brand);
}

.home-calendar-level-0 {
	background: var(--surface-4);
}
.home-calendar-level-1 {
	background: color-mix(in oklab, var(--color-brand) 28%, var(--surface-4));
}
.home-calendar-level-2 {
	background: color-mix(in oklab, var(--color-brand) 48%, var(--surface-4));
}
.home-calendar-level-3 {
	background: color-mix(in oklab, var(--color-brand) 70%, var(--surface-4));
}
.home-calendar-level-4 {
	background: var(--color-brand);
	color: var(--color-accent-contrast);
}

.home-calendar-tooltip {
	position: fixed;
	z-index: 1000;
	display: flex;
	max-width: 18rem;
	transform: translate(-50%, -100%);
	flex-direction: column;
	gap: 0.125rem;
	pointer-events: none;
	padding: 0.5rem 0.625rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--color-tooltip-bg);
	box-shadow: 0 2px 8px rgba(0, 0, 0, 0.18);
	color: var(--color-tooltip-text);
	font-size: 0.8125rem;
	font-weight: 500;
	line-height: 1.4;
}

.home-calendar-tooltip::after {
	position: absolute;
	top: 100%;
	left: 50%;
	width: 0.5rem;
	height: 0.5rem;
	transform: translate(-50%, -50%) rotate(45deg);
	border-right: 1px solid var(--surface-5);
	border-bottom: 1px solid var(--surface-5);
	background: var(--color-tooltip-bg);
	content: '';
}

.home-calendar-tooltip strong {
	font-weight: 700;
}

.home-calendar-tooltip-enter-active,
.home-calendar-tooltip-leave-active {
	transition:
		opacity 100ms ease,
		transform 100ms ease;
}

.home-calendar-tooltip-enter-from,
.home-calendar-tooltip-leave-to {
	transform: translate(-50%, calc(-100% + 0.25rem));
	opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
	.home-calendar-tooltip-enter-active,
	.home-calendar-tooltip-leave-active {
		transition: none;
	}
}
</style>
