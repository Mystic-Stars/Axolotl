<script setup lang="ts">
import { toRef } from 'vue'

import type { OnboardingMode } from './onboardingConfig'
import OnboardingDialogue from './OnboardingDialogue.vue'
import OnboardingWelcome from './OnboardingWelcome.vue'
import { useOnboardingTour } from './useOnboardingTour'

const props = defineProps<{
	visible: boolean
	mode: OnboardingMode
}>()

const emit = defineEmits<{
	complete: []
	skip: []
	requestCloseSettings: []
}>()

const {
	bubbleElement,
	bubblePlacement,
	controlSpotlightStyle,
	handleManualClick,
	isDialogueStep,
	isWelcomeStep,
	step,
	stepIndex,
	steps,
	targetRect,
	advance,
} = useOnboardingTour(toRef(props, 'visible'), toRef(props, 'mode'), {
	complete: () => emit('complete'),
	skip: () => emit('skip'),
	closeSettings: () => emit('requestCloseSettings'),
})
</script>

<template>
	<div v-if="visible" class="fixed inset-0 z-[10001] overflow-hidden pointer-events-none" aria-live="polite">
		<div
			v-if="targetRect && step.spotlight === 'control'"
			class="onboarding-spotlight"
			:style="controlSpotlightStyle"
			aria-hidden="true"
		>
			<span class="onboarding-corner onboarding-corner-top-left"></span>
			<span class="onboarding-corner onboarding-corner-top-right"></span>
			<span class="onboarding-corner onboarding-corner-bottom-right"></span>
			<span class="onboarding-corner onboarding-corner-bottom-left"></span>
		</div>

		<section
			ref="bubbleElement"
			data-onboarding-overlay-ui
			class="onboarding-surface"
			:class="[
				`onboarding-surface-${bubblePlacement.direction}`,
				{
					'onboarding-surface-centered': !targetRect,
					'onboarding-surface-dialogue': isDialogueStep,
					'onboarding-surface-welcome': isWelcomeStep,
				},
			]"
			:style="bubblePlacement.style"
			:aria-labelledby="`onboarding-title-${step.id}`"
			:aria-describedby="`onboarding-description-${step.id}`"
			@click="step.interaction === 'inspect' ? advance() : undefined"
		>
			<OnboardingWelcome
				v-if="isWelcomeStep"
				:step="step"
				@start="handleManualClick"
				@skip="emit('skip')"
			/>
			<OnboardingDialogue
				v-else
				:key="`${mode}-${step.id}`"
				:step="step"
				:current="stepIndex + 1"
				:total="steps.length"
				:docked="isDialogueStep"
				@advance="handleManualClick"
				@skip="emit('skip')"
			/>
		</section>
	</div>
</template>

<style scoped lang="scss">
.onboarding-spotlight {
	position: fixed;
	pointer-events: none;
	transform-origin: center;
	animation: onboarding-focus 1.6s ease-in-out infinite;
}

.onboarding-corner {
	position: absolute;
	width: 0.75rem;
	height: 0.75rem;
	border-color: var(--color-brand);
	border-style: solid;
	border-width: 0;
}

.onboarding-corner-top-left {
	top: 0;
	left: 0;
	border-top-width: 2px;
	border-left-width: 2px;
}

.onboarding-corner-top-right {
	top: 0;
	right: 0;
	border-top-width: 2px;
	border-right-width: 2px;
}

.onboarding-corner-bottom-right {
	right: 0;
	bottom: 0;
	border-right-width: 2px;
	border-bottom-width: 2px;
}

.onboarding-corner-bottom-left {
	bottom: 0;
	left: 0;
	border-bottom-width: 2px;
	border-left-width: 2px;
}

.onboarding-surface {
	position: fixed;
	z-index: 1;
	width: min(32rem, calc(100vw - 2rem));
	max-height: calc(100vh - 4rem);
	overflow-y: auto;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-lg);
	background: var(--color-super-raised-bg);
	padding: 1.25rem;
	box-sizing: border-box;
	pointer-events: auto;
}

.onboarding-surface-centered {
	top: 50%;
	left: 50%;
	transform: translate(-50%, -50%);
}

.onboarding-surface-welcome {
	inset: 0;
	z-index: 3;
	width: auto;
	height: 100dvh;
	min-height: 0;
	max-height: none;
	transform: none;
	overflow: hidden;
	border: 0;
	border-radius: 0;
	padding: 0;
}

.onboarding-surface-dialogue {
	top: auto !important;
	right: 0;
	bottom: 0;
	left: 0 !important;
	z-index: 2;
	width: auto;
	min-height: 0;
	max-height: min(15rem, 40vh);
	transform: none;
	overflow-y: auto;
	border: 0;
	border-top: 1px solid var(--color-divider);
	border-radius: 0;
	padding: 0.75rem 1.5rem;
}

:global(body.onboarding-reserve-dialogue-space .modal-container) {
	height: calc(100% - var(--onboarding-dialogue-reserved-space, 0px));
}

@keyframes onboarding-focus {
	0%,
	100% {
		transform: scale(1);
	}
	50% {
		transform: scale(1.015);
	}
}

@media (prefers-reduced-motion: reduce) {
	.onboarding-spotlight {
		animation: none;
	}
}

@media (max-width: 700px) {
	.onboarding-surface {
		top: auto !important;
		bottom: 1rem;
		left: 1rem !important;
		width: calc(100vw - 2rem);
		max-height: min(22rem, calc(100vh - 4rem));
		transform: none;
		padding: 1rem;
	}

	.onboarding-surface-dialogue {
		right: 0;
		bottom: 0;
		left: 0 !important;
		width: auto;
		min-height: 0;
		max-height: min(18rem, 52vh);
		padding: 0.75rem 1rem;
	}

	.onboarding-surface-welcome {
		inset: 0 !important;
		width: auto;
		height: 100dvh;
		max-height: none;
		padding: 0;
	}
}
</style>
