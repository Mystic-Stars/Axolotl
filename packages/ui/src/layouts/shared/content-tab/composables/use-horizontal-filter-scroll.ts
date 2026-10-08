import { onBeforeUnmount, type Ref, ref, watch } from 'vue'

export interface HorizontalFilterScroll {
    suppressHoverOpen: Ref<boolean>
    handleScroll: () => void
}

/**
 * Drives a horizontally scrollable filter strip:
 * - converts vertical wheel input into smooth horizontal scrolling,
 * - briefly suppresses hover-open dropdowns while the strip is scrolling,
 * - renders a small custom scrollbar thumb that only appears on hover.
 */
export function useHorizontalFilterScroll(
    containerRef: Ref<HTMLElement | null>,
    thumbRef: Ref<HTMLElement | null>,
): HorizontalFilterScroll {
    const suppressHoverOpen = ref(false)
    let filterScrollWheelHandler: ((event: WheelEvent) => void) | null = null
    let filterHoverSuppressTimer: ReturnType<typeof setTimeout> | null = null
    let scrollTarget: number | null = null
    let scrollAnimationFrame: number | null = null
    let scrollbarObserver: ResizeObserver | null = null

    function animateFilterScroll() {
        const container = containerRef.value
        if (!container || scrollTarget === null) return
        const maxScroll = Math.max(container.scrollWidth - container.clientWidth, 0)
        scrollTarget = Math.min(Math.max(scrollTarget, 0), maxScroll)
        const current = container.scrollLeft
        const diff = scrollTarget - current
        if (Math.abs(diff) < 0.5) {
            container.scrollLeft = scrollTarget
            scrollTarget = null
            scrollAnimationFrame = null
            return
        }
        container.scrollLeft = current + diff * 0.25
        scrollAnimationFrame = requestAnimationFrame(animateFilterScroll)
    }

    function cancelFilterScrollAnimation() {
        if (scrollAnimationFrame !== null) {
            cancelAnimationFrame(scrollAnimationFrame)
            scrollAnimationFrame = null
        }
        scrollTarget = null
    }

    function updateFilterScrollbar() {
        const container = containerRef.value
        const thumb = thumbRef.value
        if (!container || !thumb) return
        const maxScroll = container.scrollWidth - container.clientWidth
        if (maxScroll <= 0) {
            thumb.style.opacity = '0'
            return
        }
        const track = container.clientWidth
        const thumbWidth = Math.max(24, (track / container.scrollWidth) * track)
        thumb.style.width = `${thumbWidth}px`
        thumb.style.transform = `translateX(${(container.scrollLeft / maxScroll) * (track - thumbWidth)}px)`
        thumb.style.opacity = ''
    }

    function handleScroll() {
        suppressHoverOpen.value = true
        if (filterHoverSuppressTimer) clearTimeout(filterHoverSuppressTimer)
        filterHoverSuppressTimer = setTimeout(() => {
            suppressHoverOpen.value = false
            filterHoverSuppressTimer = null
        }, 500)
        updateFilterScrollbar()
    }

    watch(
        containerRef,
        (container, previous) => {
            if (previous && filterScrollWheelHandler) {
                previous.removeEventListener('wheel', filterScrollWheelHandler)
                filterScrollWheelHandler = null
            }
            if (scrollbarObserver) {
                scrollbarObserver.disconnect()
                scrollbarObserver = null
            }
            if (!container) return

            filterScrollWheelHandler = (event) => {
                if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return
                if (container.scrollWidth <= container.clientWidth + 1) return
                event.preventDefault()
                const delta = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY
                const maxScroll = Math.max(container.scrollWidth - container.clientWidth, 0)
                scrollTarget = Math.min(
                    Math.max((scrollTarget ?? container.scrollLeft) + delta * 0.5, 0),
                    maxScroll,
                )
                if (scrollAnimationFrame === null) {
                    scrollAnimationFrame = requestAnimationFrame(animateFilterScroll)
                }
            }
            container.addEventListener('wheel', filterScrollWheelHandler, {
                passive: false,
            })

            scrollbarObserver = new ResizeObserver(updateFilterScrollbar)
            scrollbarObserver.observe(container)
            updateFilterScrollbar()
        },
        { immediate: true },
    )

    onBeforeUnmount(() => {
        if (containerRef.value && filterScrollWheelHandler) {
            containerRef.value.removeEventListener('wheel', filterScrollWheelHandler)
            filterScrollWheelHandler = null
        }
        cancelFilterScrollAnimation()
        if (scrollbarObserver) {
            scrollbarObserver.disconnect()
            scrollbarObserver = null
        }
        if (filterHoverSuppressTimer) {
            clearTimeout(filterHoverSuppressTimer)
            filterHoverSuppressTimer = null
        }
    })

    return {
        suppressHoverOpen,
        handleScroll,
    }
}
