/** Matches `.page-slide-enter-active` opacity timing in global.scss, plus a frame. */
export const PAGE_TRANSITION_SETTLE_MS = 200

/** Runs after first paint when the browser is idle (fallback: short timeout). */
export function runWhenIdle(task: () => void, timeout = 2000): void {
	if (typeof requestIdleCallback === 'function') {
		requestIdleCallback(() => task(), { timeout })
		return
	}
	setTimeout(task, Math.min(timeout, 200))
}

/**
 * Runs a task after the route enter animation has had a chance to paint.
 * Heavy first-page fetches (browse search, CurseForge categories) otherwise
 * compete with the transition and make nav switches janky.
 */
export function runAfterPageTransitionSettle(task: () => void): void {
	const start = () => {
		if (typeof requestIdleCallback === 'function') {
			requestIdleCallback(
				() => {
					task()
				},
				{ timeout: 500 },
			)
			return
		}
		setTimeout(task, 32)
	}

	setTimeout(start, PAGE_TRANSITION_SETTLE_MS)
}
