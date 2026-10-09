<template>
    <transition name="fade">
        <div
            v-show="shown"
            ref="contextMenu"
            role="menu"
            class="select-none bg-surface-3 rounded-[var(--radius-md)] shadow-[var(--shadow-floating)] border border-solid border-divider m-0 fixed z-[1000000] overflow-x-hidden overflow-y-auto p-2 box-border"
            :style="menuStyle"
        >
            <template v-for="(option, index) in options" :key="option.name ?? option.id ?? index">
                <hr
                    v-if="option.type === 'divider'"
                    role="separator"
                    class="border border-solid border-divider m-2"
                />
                <button
                    v-else-if="!(isInstanceLink(item) && optionName(option) === `add_content`)"
                    type="button"
                    role="menuitem"
                    :disabled="option.disabled"
                    class="w-full border-0 bg-transparent text-left items-center text-[var(--color-text-default)] cursor-pointer flex gap-2 p-2 rounded-[var(--radius-sm)] whitespace-normal disabled:cursor-not-allowed disabled:opacity-50"
                    :class="optionClasses(option)"
                    @click.stop="optionClicked(option)"
                >
                    <component :is="option.icon" v-if="option.icon" class="size-5" />
                    <slot :name="optionName(option)">
                        {{ option.label ?? optionName(option) }}
                    </slot>
                </button>
            </template>
        </div>
    </transition>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

import { placeContextMenu } from './context-menu-placement'

const emit = defineEmits(['menu-closed', 'option-clicked'])

const item = ref(null)
const contextMenu = ref(null)
const options = ref([])
const left = ref('0px')
const top = ref('0px')
const shown = ref(false)
let justOpened = false
let anchorPoint = { clientX: 0, clientY: 0 }
let activeViewport = null
let resizeObserver = null
let disposed = false

const SAFE_GAP = 10
const viewportBoundary = () => {
    const viewport = activeViewport
    if (viewport) {
        const rect = viewport.getBoundingClientRect()
        if (rect.width > 0 && rect.height > 0) return rect
    }
    return { left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight }
}

const updatePosition = () => {
    if (!shown.value || !contextMenu.value) return
    const rect = viewportBoundary()
    const menuWidth = contextMenu.value.clientWidth || 200
    const menuHeight = contextMenu.value.clientHeight || 100
    const placement = placeContextMenu({
        ...anchorPoint,
        menuWidth,
        menuHeight,
        boundary: rect,
        safeGap: SAFE_GAP,
    })
    left.value = `${placement.left}px`
    top.value = `${placement.top}px`
}

const menuStyle = computed(() => ({
    left: left.value,
    top: top.value,
    maxWidth: `${Math.max(0, viewportBoundary().right - viewportBoundary().left - SAFE_GAP * 2)}px`,
    maxHeight: `${Math.max(0, viewportBoundary().bottom - viewportBoundary().top - SAFE_GAP * 2)}px`,
}))

const cleanupPositionListeners = () => {
    window.removeEventListener('resize', updatePosition)
    window.removeEventListener('scroll', updatePosition, true)
    activeViewport?.removeEventListener('scroll', updatePosition)
    resizeObserver?.disconnect()
    activeViewport = null
    resizeObserver = null
}

const setupPositionListeners = (event) => {
    cleanupPositionListeners()
    const target = event.target instanceof Element ? event.target : null
    activeViewport = target?.closest('.app-viewport') ?? null
    window.addEventListener('resize', updatePosition)
    window.addEventListener('scroll', updatePosition, true)
    activeViewport?.addEventListener('scroll', updatePosition)
    if (activeViewport && typeof ResizeObserver !== 'undefined') {
        resizeObserver = new ResizeObserver(updatePosition)
        resizeObserver.observe(activeViewport)
    }
}

const CLOSE_ALL_EVENT = 'close-all-context-menus'

const showMenu = (event, passedItem, passedOptions) => {
    if (disposed) return
    window.dispatchEvent(new CustomEvent(CLOSE_ALL_EVENT))

    item.value = passedItem
    options.value = passedOptions
    anchorPoint = { clientX: event.clientX, clientY: event.clientY }
    setupPositionListeners(event)

    justOpened = true
    nextTick(() => {
        justOpened = false
    })

    // show to get dimensions
    shown.value = true

    // Render first so dimensions are available, then place against the content pane.
    nextTick(updatePosition)
}

const optionName = (option) => option.name ?? option.id

const optionColor = (option) => option.color ?? (option.tone === 'red' ? 'danger' : 'base')

const optionClasses = (option) => ({
    'enabled:hover:bg-surface-4 enabled:hover:text-[var(--color-text-primary)] enabled:active:bg-surface-4':
        optionColor(option) === 'base',
    'enabled:hover:bg-brand enabled:hover:text-[var(--color-accent-contrast)] enabled:hover:font-bold':
        optionColor(option) === 'primary',
    'enabled:hover:bg-red enabled:hover:text-[var(--color-accent-contrast)]':
        optionColor(option) === 'danger',
    'enabled:hover:bg-orange enabled:hover:text-[var(--color-accent-contrast)]':
        optionColor(option) === 'contrast',
})

const isInstanceLink = (item) => {
    if (item?.instance != undefined && item.instance.link) {
        return true
    } else if (item?.link) {
        return true
    }
    return false
}

const hideContextMenu = () => {
    if (!shown.value) return
    shown.value = false
    cleanupPositionListeners()
    emit('menu-closed')
}

const optionClicked = (option) => {
    if (!shown.value || option.disabled || option.type === 'divider') return
    if (option.action) {
        option.action()
    } else {
        emit('option-clicked', {
            item: item.value,
            option: optionName(option),
        })
    }
    hideContextMenu()
}

defineExpose({
    showMenu,
    open: (event, passedOptions) => showMenu(event, null, passedOptions),
    close: hideContextMenu,
})

const onEscape = (event) => {
    if (shown.value && event.key === 'Escape' && !event.defaultPrevented) {
        event.preventDefault()
        hideContextMenu()
    }
}

const handleClickOutside = (event) => {
    if (!shown.value || justOpened) return
    const menu = contextMenu.value
    if (
        menu &&
        !event.composedPath().includes(menu) &&
        !(event.target instanceof Node && menu.contains(event.target))
    ) {
        hideContextMenu()
    }
}

const handleCloseOthers = () => {
    hideContextMenu()
}

onMounted(() => {
    window.addEventListener('click', handleClickOutside)
    window.addEventListener(CLOSE_ALL_EVENT, handleCloseOthers)
    window.addEventListener('keydown', onEscape)
})

onBeforeUnmount(() => {
    disposed = true
    cleanupPositionListeners()
    window.removeEventListener('click', handleClickOutside)
    window.removeEventListener(CLOSE_ALL_EVENT, handleCloseOthers)
    window.removeEventListener('keydown', onEscape)
})
</script>

<style lang="scss" scoped>
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease-in-out;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>
