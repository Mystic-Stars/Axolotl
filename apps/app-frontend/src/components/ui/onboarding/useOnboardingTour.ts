import { computed, nextTick, onBeforeUnmount, onMounted, type Ref, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import {
	type CreationPath,
	type OnboardingMode,
	onboardingTargetSelector,
	onboardingTours,
	type StepDestination,
} from './onboardingConfig'

type TourEvents = {
	complete: () => void
	skip: () => void
	closeSettings: () => void
}

const controlSpotlightPadding = 3
const missingTargetRetryLimit = 8
const missingTargetRetryDelay = 250

export function useOnboardingTour(
	visible: Ref<boolean>,
	mode: Ref<OnboardingMode>,
	events: TourEvents,
) {
	const route = useRoute()
	const stepIndex = ref(0)
	const targetRect = ref<DOMRect | null>(null)
	const bubbleElement = ref<HTMLElement>()
	const bubbleSize = ref({ width: 512, height: 160 })
	const waitingForRoute = ref(false)
	const creationPath = ref<CreationPath>()
	const targetElement = ref<HTMLElement>()
	let targetObserver: ResizeObserver | undefined
	let bubbleObserver: ResizeObserver | undefined
	let targetRetryTimer: ReturnType<typeof setTimeout> | undefined
	let advanceTimer: ReturnType<typeof setTimeout> | undefined
	let unlockTimer: ReturnType<typeof setTimeout> | undefined
	let targetRetryCount = 0
	let transitionLocked = false

	const steps = computed(() => onboardingTours[mode.value])
	const step = computed(() => steps.value[stepIndex.value])
	const isWelcomeStep = computed(() => step.value.id === 'welcome')
	const isDialogueStep = computed(
		() => !!step.value.targetId && (step.value.spotlight !== 'control' || !targetRect.value),
	)
	const controlSpotlightStyle = computed(() => {
		if (!targetRect.value) return {}
		const rect = targetRect.value
		return {
			left: `${Math.max(0, rect.left - controlSpotlightPadding)}px`,
			top: `${Math.max(0, rect.top - controlSpotlightPadding)}px`,
			width: `${rect.width + controlSpotlightPadding * 2}px`,
			height: `${rect.height + controlSpotlightPadding * 2}px`,
		}
	})
	const bubblePlacement = computed(() => {
		if (!targetRect.value || isDialogueStep.value) return { direction: 'center', style: {} }

		const rect = targetRect.value
		const safeInset = 16
		const safeTop = 48
		const bubbleWidth = Math.min(bubbleSize.value.width, window.innerWidth - safeInset * 2)
		const bubbleHeight = Math.min(bubbleSize.value.height, window.innerHeight - safeTop - safeInset)
		const gap = 20
		const positions = [
			{
				direction: 'right',
				left: rect.right + gap,
				top: rect.top + rect.height / 2 - bubbleHeight / 2,
			},
			{
				direction: 'bottom',
				left: rect.left + rect.width / 2 - bubbleWidth / 2,
				top: rect.bottom + gap,
			},
			{
				direction: 'left',
				left: rect.left - bubbleWidth - gap,
				top: rect.top + rect.height / 2 - bubbleHeight / 2,
			},
			{
				direction: 'top',
				left: rect.left + rect.width / 2 - bubbleWidth / 2,
				top: rect.top - bubbleHeight - gap,
			},
		]
		const position =
			positions.find(
				(candidate) =>
					candidate.left >= safeInset &&
					candidate.top >= safeTop &&
					candidate.left + bubbleWidth <= window.innerWidth - safeInset &&
					candidate.top + bubbleHeight <= window.innerHeight - safeInset,
			) ?? positions[0]

		return {
			direction: position.direction,
			style: {
				left: `${Math.min(
					Math.max(safeInset, position.left),
					window.innerWidth - bubbleWidth - safeInset,
				)}px`,
				top: `${Math.min(
					Math.max(safeTop, position.top),
					window.innerHeight - bubbleHeight - safeInset,
				)}px`,
			},
		}
	})

	function clearTargetTracking() {
		targetObserver?.disconnect()
		targetObserver = undefined
		if (targetRetryTimer) clearTimeout(targetRetryTimer)
		targetRetryTimer = undefined
	}

	function clearModalReservation() {
		document.body.classList.remove('onboarding-reserve-dialogue-space')
		document.body.style.removeProperty('--onboarding-dialogue-reserved-space')
	}

	function updateModalReservation() {
		const targetIsInModal = !!targetElement.value?.closest('[role="dialog"]')
		if (!visible.value || !isDialogueStep.value || !targetIsInModal) {
			clearModalReservation()
			return
		}

		document.body.classList.add('onboarding-reserve-dialogue-space')
		document.body.style.setProperty(
			'--onboarding-dialogue-reserved-space',
			`${Math.ceil(bubbleSize.value.height)}px`,
		)
	}

	function updateBubbleSize() {
		if (!bubbleElement.value) return
		const { width, height } = bubbleElement.value.getBoundingClientRect()
		bubbleSize.value = { width, height }
		updateModalReservation()
	}

	function scheduleMissingTargetRetry(stepId: string) {
		if (targetRetryCount >= missingTargetRetryLimit) {
			void advance()
			return
		}

		targetRetryCount++
		targetRetryTimer = setTimeout(() => {
			if (visible.value && step.value.id === stepId && !targetRect.value) updateTarget()
		}, missingTargetRetryDelay)
	}

	function updateTarget() {
		clearTargetTracking()
		if (!visible.value || !step.value.targetId) {
			targetElement.value = undefined
			targetRect.value = null
			clearModalReservation()
			return
		}

		const target = document.querySelector<HTMLElement>(
			onboardingTargetSelector(step.value.targetId),
		)
		const rect = target?.getBoundingClientRect()
		if (!target || !rect || rect.width < 1 || rect.height < 1) {
			targetElement.value = undefined
			targetRect.value = null
			clearModalReservation()
			scheduleMissingTargetRetry(step.value.id)
			return
		}

		targetRetryCount = 0
		targetElement.value = target
		const updateRect = () => {
			targetRect.value = target.getBoundingClientRect()
		}
		updateRect()
		targetObserver = new ResizeObserver(updateRect)
		targetObserver.observe(target)
		updateModalReservation()
		requestAnimationFrame(updateRect)
	}

	function goTo(destination: StepDestination) {
		if (destination === 'complete') {
			events.complete()
			return
		}

		const destinationIndex = steps.value.findIndex((candidate) => candidate.id === destination)
		if (destinationIndex === -1) {
			events.complete()
			return
		}

		stepIndex.value = destinationIndex
	}

	async function advance() {
		if (transitionLocked) return
		transitionLocked = true

		const destination = creationPath.value
			? step.value.nextByCreationPath?.[creationPath.value]
			: step.value.next
		if (destination) {
			goTo(destination)
			scheduleUnlock()
			return
		}

		if (stepIndex.value === steps.value.length - 1) {
			events.complete()
			scheduleUnlock()
			return
		}

		if (step.value.closeSettingsAfter) events.closeSettings()
		stepIndex.value++
		await nextTick()
		updateTarget()
		scheduleUnlock()
	}

	function scheduleUnlock() {
		if (unlockTimer) clearTimeout(unlockTimer)
		unlockTimer = setTimeout(() => {
			transitionLocked = false
			unlockTimer = undefined
		}, 180)
	}

	function scheduleDestination(destination?: StepDestination) {
		if (advanceTimer || transitionLocked) return
		if (destination) transitionLocked = true
		advanceTimer = setTimeout(() => {
			advanceTimer = undefined
			if (destination) {
				goTo(destination)
				scheduleUnlock()
			} else {
				void advance()
			}
		}, 150)
	}

	function handleManualClick() {
		if (step.value.interaction === 'manual') void advance()
	}

	function handleBranchClick(target: Element) {
		if (!step.value.branchByTarget) return false

		for (const [targetId, branch] of Object.entries(step.value.branchByTarget)) {
			if (!target.closest(onboardingTargetSelector(targetId))) continue
			creationPath.value = branch.creationPath
			scheduleDestination(branch.next)
			return true
		}
		return false
	}

	function handleDocumentClick(event: MouseEvent) {
		if (!visible.value) return
		const clickedElement = event.target instanceof Element ? event.target : null
		if (clickedElement?.closest('[data-onboarding-overlay-ui]')) return

		if (step.value.interaction === 'inspect') {
			event.preventDefault()
			event.stopImmediatePropagation()
			void advance()
			return
		}

		if (!step.value.targetId || !['navigate', 'activate'].includes(step.value.interaction)) return
		if (clickedElement && handleBranchClick(clickedElement)) return
		if (!targetElement.value?.contains(event.target as Node)) return

		if (step.value.expectedPath && route.path !== step.value.expectedPath) {
			waitingForRoute.value = true
			return
		}
		scheduleDestination()
	}

	function handleKeydown(event: KeyboardEvent) {
		if (!visible.value || event.key !== 'Escape') return
		event.preventDefault()
		events.skip()
	}

	watch(
		() => route.path,
		(path) => {
			if (!waitingForRoute.value || !step.value.expectedPath) return
			if (path !== step.value.expectedPath) return
			waitingForRoute.value = false
			void advance()
		},
	)

	watch(visible, async (isVisible) => {
		if (isVisible) {
			stepIndex.value = 0
			waitingForRoute.value = false
			creationPath.value = undefined
			targetRetryCount = 0
			await nextTick()
			updateBubbleSize()
			if (bubbleElement.value) bubbleObserver?.observe(bubbleElement.value)
		} else {
			bubbleObserver?.disconnect()
			clearModalReservation()
		}
		updateTarget()
	})

	watch(step, async () => {
		if (!visible.value) return
		targetRetryCount = 0
		await nextTick()
		updateBubbleSize()
		updateTarget()
	})

	watch(mode, () => {
		stepIndex.value = 0
	})

	onMounted(() => {
		document.addEventListener('click', handleDocumentClick, true)
		document.addEventListener('keydown', handleKeydown)
		window.addEventListener('resize', updateTarget)
		window.addEventListener('scroll', updateTarget, true)
		bubbleObserver = new ResizeObserver(updateBubbleSize)
		if (bubbleElement.value) bubbleObserver.observe(bubbleElement.value)
	})

	onBeforeUnmount(() => {
		clearTargetTracking()
		clearModalReservation()
		bubbleObserver?.disconnect()
		if (advanceTimer) clearTimeout(advanceTimer)
		if (unlockTimer) clearTimeout(unlockTimer)
		document.removeEventListener('click', handleDocumentClick, true)
		document.removeEventListener('keydown', handleKeydown)
		window.removeEventListener('resize', updateTarget)
		window.removeEventListener('scroll', updateTarget, true)
	})

	return {
		advance,
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
	}
}
