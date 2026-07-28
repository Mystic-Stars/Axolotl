<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onUnmounted, ref } from 'vue'

import { getTimeBucket, stableGreetingIndex } from './home-utils'

const props = withDefaults(
	defineProps<{
		playerName?: string | null
		variant?: 'standard' | 'minimal'
	}>(),
	{
		playerName: null,
		variant: 'standard',
	},
)

const { formatMessage, locale } = useVIntl()
const now = ref(new Date())
const messages = defineMessages({
	withPlayer: {
		id: 'app.home.greeting.with-player',
		defaultMessage: 'Welcome back, {name}. {greeting}',
	},
	minimalWithPlayer: {
		id: 'app.home.greeting.minimal.with-player',
		defaultMessage: '{greeting}, {name}',
	},
	minimalLateNight: {
		id: 'app.home.greeting.minimal.late-night',
		defaultMessage: 'Good evening',
	},
	minimalDawn: {
		id: 'app.home.greeting.minimal.dawn',
		defaultMessage: 'Good morning',
	},
	minimalMorning: {
		id: 'app.home.greeting.minimal.morning',
		defaultMessage: 'Good morning',
	},
	minimalAfternoon: {
		id: 'app.home.greeting.minimal.afternoon',
		defaultMessage: 'Good afternoon',
	},
	minimalEvening: {
		id: 'app.home.greeting.minimal.evening',
		defaultMessage: 'Good evening',
	},
	minimalNight: {
		id: 'app.home.greeting.minimal.night',
		defaultMessage: 'Good evening',
	},
	lateNight: {
		id: 'app.home.greeting.late-night',
		defaultMessage:
			'The moon is still working overtime.\nA quiet world is waiting.\nNight shifts build great stories.\nOne more block before dawn?\nThe stars have your server covered.\nLate hours, legendary saves.\nThe torchlight looks especially good now.\nYour next adventure is still awake.\nThe caves are calmer after midnight.\nA peaceful spawn point awaits.\nThe night belongs to patient builders.\nKeep the soundtrack low and the ideas loud.\nEvery great base starts with one block.\nThe End can wait, unless it cannot.\nA small session still counts.\nThe world has been saved for you.',
	},
	dawn: {
		id: 'app.home.greeting.dawn',
		defaultMessage:
			'First light, fresh chunks.\nA new day is loading in.\nThe sunrise buff is active.\nMorning worlds feel brand new.\nCoffee first, diamonds second.\nYour base missed you overnight.\nA calm start makes a fine adventure.\nThe overworld is waking up.\nFresh air, fresh resource packs.\nToday is a good day to explore.\nThe village is already open for trade.\nA quiet morning suits a big build.\nNew day, new coordinates.\nThe creepers are not morning people either.\nStart small and see where it goes.\nYour next session is ready when you are.',
	},
	morning: {
		id: 'app.home.greeting.morning',
		defaultMessage:
			'Good morning, adventurer.\nThe day is full of unexplored chunks.\nA fine time for a fresh start.\nYour tools are ready for the day.\nThe overworld has excellent plans.\nBuild something your future self will love.\nA new session is a clean canvas.\nThe sun is up and so are the villagers.\nLet today be a little more blocky.\nThe mines have been suspiciously quiet.\nYour next project is only one launch away.\nA good morning for a good world.\nThere is always room for one more idea.\nThe crafting table is on standby.\nThe map is waiting for new markers.\nSettle in and make some progress.',
	},
	afternoon: {
		id: 'app.home.greeting.afternoon',
		defaultMessage:
			'Afternoon break, excellent timing.\nA short session can become a great one.\nThe world is ready for your next move.\nTime to check on that half-finished build.\nA little exploration goes a long way.\nThe next biome is calling.\nYour inventory has been waiting patiently.\nThe village market is still open.\nA good hour for a focused project.\nThe redstone probably behaves today.\nYour pickaxe is ready to work.\nA new route is waiting beyond spawn.\nThe afternoon is made for side quests.\nOne quick visit to your world?\nThe next chapter starts here.\nTake a moment and make something.',
	},
	evening: {
		id: 'app.home.greeting.evening',
		defaultMessage:
			"Evening is prime building time.\nThe day is winding down; the world is opening up.\nA familiar world makes a good landing spot.\nTime to return to your favorite project.\nThe sunset looks better from a new tower.\nYour base lights are waiting.\nA relaxed session sounds about right.\nThe villagers are closing shop soon.\nYour next build deserves an evening glow.\nA good time to wander without a plan.\nThe campfire is already lit.\nOne more room for the base?\nThe horizon is looking especially inviting.\nA quiet night starts with a good world.\nThe next block is yours to place.\nMake tonight's progress count.",
	},
	night: {
		id: 'app.home.greeting.night',
		defaultMessage:
			'The night shift is ready.\nA good evening for familiar worlds.\nYour favorite instance is waiting nearby.\nThe stars are out; the plans are in.\nA calm session can end the day well.\nThe world is quieter after dark.\nTime to put a few more blocks in place.\nYour base is glowing in the distance.\nThe next adventure starts at sunset.\nA night well spent has a good save file.\nThe campfire crackles, somewhere.\nThe moon makes every build look dramatic.\nA little Minecraft before tomorrow.\nYour worlds are ready for a visit.\nThe night is still young, in chunks.\nSettle in for a well-earned session.',
	},
})

const minimalGreetingMessages = {
	'late-night': messages.minimalLateNight,
	dawn: messages.minimalDawn,
	morning: messages.minimalMorning,
	afternoon: messages.minimalAfternoon,
	evening: messages.minimalEvening,
	night: messages.minimalNight,
}

const greeting = computed(() => {
	const bucket = getTimeBucket(now.value)
	const variants = formatMessage(messages[bucket]).split('\n').filter(Boolean)
	const seed = `${locale.value}:${now.value.toDateString()}:${bucket}:${props.playerName ?? ''}`
	return variants[stableGreetingIndex(seed, variants.length)] ?? ''
})

const minimalGreeting = computed(() =>
	formatMessage(minimalGreetingMessages[getTimeBucket(now.value)]),
)

const heading = computed(() => {
	if (props.variant === 'minimal') {
		return props.playerName
			? formatMessage(messages.minimalWithPlayer, {
					name: props.playerName,
					greeting: minimalGreeting.value,
				})
			: minimalGreeting.value
	}

	return props.playerName
		? formatMessage(messages.withPlayer, { name: props.playerName, greeting: greeting.value })
		: greeting.value
})

const updateClock = () => {
	now.value = new Date()
}
const timer = window.setInterval(updateClock, 60_000)

onUnmounted(() => window.clearInterval(timer))
</script>

<template>
	<header
		class="flex min-w-0 flex-col"
		:class="variant === 'minimal' ? 'items-center gap-3 text-center' : 'gap-1 py-2'"
	>
		<h1 class="m-0 max-w-full break-words text-2xl font-extrabold text-contrast">{{ heading }}</h1>
		<div v-if="variant === 'minimal'" class="h-0.5 w-8 rounded-full bg-brand" aria-hidden="true" />
	</header>
</template>
