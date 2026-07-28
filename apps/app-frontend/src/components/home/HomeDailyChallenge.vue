<script setup lang="ts">
import { SparklesIcon, UpdatedIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import { type ChallengeDifficulty,dailyChallenges } from '@/data/daily-challenges'

import { stableGreetingIndex, toDateKey } from './home-utils'

const { formatMessage, locale } = useVIntl()

const messages = defineMessages({
	dailyChallenge: { id: 'app.home.challenge.title', defaultMessage: 'Daily challenge' },
	shuffle: { id: 'app.home.challenge.shuffle', defaultMessage: 'Try another' },
	easy: { id: 'app.home.challenge.easy', defaultMessage: 'Easy' },
	medium: { id: 'app.home.challenge.medium', defaultMessage: 'Medium' },
	hard: { id: 'app.home.challenge.hard', defaultMessage: 'Hard' },
})

const difficultyMessages = {
	easy: messages.easy,
	medium: messages.medium,
	hard: messages.hard,
} as const

const dailyIndex = stableGreetingIndex(
	`daily-challenge:${toDateKey(new Date())}`,
	dailyChallenges.length,
)
const challengeIndex = ref(dailyIndex)

const challenge = computed(() => dailyChallenges[challengeIndex.value])
const challengeText = computed(() =>
	locale.value.toLowerCase().startsWith('zh')
		? challenge.value.text['zh-CN']
		: challenge.value.text['en-US'],
)

const difficultyDotClass: Record<ChallengeDifficulty, string> = {
	easy: 'bg-brand-green',
	medium: 'bg-orange',
	hard: 'bg-red',
}

function shuffleChallenge() {
	if (dailyChallenges.length < 2) return
	let next = challengeIndex.value
	while (next === challengeIndex.value) {
		next = Math.floor(Math.random() * dailyChallenges.length)
	}
	challengeIndex.value = next
}
</script>

<template>
	<section
		class="flex min-w-0 flex-col gap-3 border-0 border-b-[1px] border-solid border-[--brand-gradient-border] p-4"
	>
		<div class="flex items-center gap-2">
			<SparklesIcon class="size-4 shrink-0 text-secondary" aria-hidden="true" />
			<h2 class="m-0 truncate text-lg">
				{{ formatMessage(messages.dailyChallenge) }}
			</h2>
			<ButtonStyled circular size="small" type="transparent" class="ml-auto">
				<button v-tooltip="formatMessage(messages.shuffle)" @click="shuffleChallenge">
					<UpdatedIcon />
				</button>
			</ButtonStyled>
		</div>
		<p class="m-0 text-sm leading-relaxed text-primary">{{ challengeText }}</p>
		<div class="flex items-center gap-1.5 text-xs text-secondary">
			<span
				class="size-2 rounded-full"
				:class="difficultyDotClass[challenge.difficulty]"
				aria-hidden="true"
			/>
			{{ formatMessage(difficultyMessages[challenge.difficulty]) }}
		</div>
	</section>
</template>
