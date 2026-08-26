<script setup lang="ts">
import { RightArrowIcon, XIcon } from '@modrinth/assets'
import { ButtonStyled, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import { onboardingMessages, type OnboardingStep } from './onboardingConfig'
import OnboardingMascotStage from './OnboardingMascotStage.vue'

const props = defineProps<{
	step: OnboardingStep
	current: number
	total: number
	docked: boolean
}>()

defineEmits<{
	advance: []
	skip: []
}>()

const { formatMessage } = useVIntl()
const progressWidth = computed(() => `${Math.min(100, (props.current / props.total) * 100)}%`)
</script>

<template>
	<div class="onboarding-dialogue-layout" :class="{ 'onboarding-dialogue-layout-docked': docked }">
		<div v-if="docked" class="onboarding-dialogue-progress" aria-hidden="true">
			<span :style="{ width: progressWidth }"></span>
		</div>
		<OnboardingMascotStage :alt="formatMessage(onboardingMessages.mascotAlt)" />
		<div class="onboarding-dialogue-copy">
			<div class="onboarding-dialogue-header">
				<div class="onboarding-dialogue-heading">
					<p class="m-0 mb-1.5 text-[0.8125rem] font-bold text-brand">{{ current }} / {{ total }}</p>
					<h2 :id="`onboarding-title-${step.id}`">{{ formatMessage(step.title) }}</h2>
				</div>
				<ButtonStyled circular type="transparent">
					<button :aria-label="formatMessage(onboardingMessages.skip)" @click.stop="$emit('skip')">
						<XIcon />
					</button>
				</ButtonStyled>
			</div>
			<div class="onboarding-dialogue-body">
				<p :id="`onboarding-description-${step.id}`">
					{{ formatMessage(step.description) }}
				</p>
				<ButtonStyled v-if="step.interaction === 'manual'" color="brand">
					<button @click="$emit('advance')">
						{{ formatMessage(step.action) }}
						<RightArrowIcon />
					</button>
				</ButtonStyled>
				<p v-else class="onboarding-action-hint">
					<RightArrowIcon aria-hidden="true" />
					{{ formatMessage(step.action) }}
				</p>
			</div>
		</div>
	</div>
</template>

<style scoped lang="scss">
.onboarding-dialogue-layout {
	display: grid;
	grid-template-columns: auto minmax(0, 1fr);
	align-items: start;
	gap: 0.875rem;
}

.onboarding-dialogue-layout > :deep(.onboarding-mascot) {
	width: 6rem;
	align-self: center;
	transform: scale(1.35);
	transform-origin: center;
}

.onboarding-dialogue-layout-docked {
	width: min(72rem, 100%);
	margin-inline: auto;
	gap: 1.25rem;
}

.onboarding-dialogue-progress {
	position: absolute;
	top: -1px;
	left: 0;
	width: 100%;
	height: 2px;
	overflow: hidden;
	pointer-events: none;
}

.onboarding-dialogue-progress span {
	display: block;
	height: 100%;
	background: var(--color-brand);
	transition: width 280ms cubic-bezier(0.22, 1, 0.36, 1);
}

.onboarding-dialogue-layout-docked > :deep(.onboarding-mascot) {
	width: 8rem;
	align-self: end;
	transform: none;
}

.onboarding-dialogue-copy {
	min-width: 0;
	color: var(--color-contrast);
}

.onboarding-dialogue-header {
	display: grid;
	grid-template-columns: minmax(0, 1fr) auto;
	align-items: flex-start;
	gap: 1rem;
	padding-bottom: 0.375rem;
}

.onboarding-dialogue-heading h2,
.onboarding-dialogue-body p {
	margin: 0;
}

.onboarding-dialogue-heading h2 {
	color: var(--color-contrast);
	font-size: 1.25rem;
	font-weight: 700;
	line-height: 1.25;
	letter-spacing: 0;
}

.onboarding-dialogue-body > p:not(.onboarding-action-hint) {
	max-width: 46ch;
	color: var(--color-contrast);
	font-size: 1rem;
	line-height: 1.55;
}

.onboarding-dialogue-layout-docked .onboarding-dialogue-body > p:not(.onboarding-action-hint) {
	max-width: 68ch;
}

.onboarding-progress,
.onboarding-action-hint {
	font-size: 0.8125rem;
	font-weight: 700;
}

.onboarding-progress {
	margin-bottom: 0.375rem;
	color: var(--color-brand);
}

.onboarding-action-hint {
	display: flex;
	align-items: center;
	gap: 0.375rem;
	margin-top: 0.625rem !important;
	color: var(--color-brand);
	text-wrap: pretty;
}

.onboarding-action-hint :deep(svg) {
	flex: none;
}

.onboarding-dialogue-body :deep(.button-outer) {
	margin-top: 1rem;
}

.onboarding-dialogue-header :deep(.button-outer) {
	flex: none;
	margin-top: 0;
}

.onboarding-dialogue-copy :deep(svg) {
	width: 1rem;
	height: 1rem;
}

@media (max-width: 700px) {
	.onboarding-dialogue-layout > :deep(.onboarding-mascot) {
		width: 4.5rem;
		transform: scale(1.3);
	}

	.onboarding-dialogue-layout-docked {
		width: 100%;
		gap: 0.75rem;
	}

	.onboarding-dialogue-layout-docked > :deep(.onboarding-mascot) {
		width: 5rem;
		transform: none;
	}
}

@media (prefers-reduced-motion: reduce) {
	.onboarding-dialogue-progress span {
		transition: none;
	}
}
</style>
