<template>
    <transition name="fade">
        <div v-show="shown" ref="contextMenu" class="context-menu select-none" :style="menuStyle">
            <div
                v-for="(option, index) in options"
                :key="option.name ?? option.id ?? index"
                @click.stop="optionClicked(option)"
            >
                <hr v-if="option.type === 'divider'" class="divider" />
                <div
                    v-else-if="!(isInstanceLink(item) && optionName(option) === `add_content`)"
                    class="item clickable"
                    :class="[optionColor(option), { disabled: option.disabled }]"
                >
                    <component :is="option.icon" v-if="option.icon" class="size-5" />
                    <slot :name="optionName(option)">
                        {{ option.label ?? optionName(option) }}
                    </slot>
                </div>
            </div>
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

const isInstanceLink = (item) => {
    if (item?.instance != undefined && item.instance.link) {
        return true
    } else if (item?.link) {
        return true
    }
    return false
}

const hideContextMenu = () => {
    shown.value = false
    cleanupPositionListeners()
    emit('menu-closed')
}

const optionClicked = (option) => {
    if (option.disabled) return
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

const onEscKeyRelease = (event) => {
    if (event.keyCode === 27) {
        hideContextMenu()
    }
}

const handleClickOutside = (event) => {
    const elements = document.elementsFromPoint(event.clientX, event.clientY)
    if (
        contextMenu.value &&
        contextMenu.value.$el !== event.target &&
        !elements.includes(contextMenu.value.$el)
    ) {
        hideContextMenu()
    }
}

const handleCloseOthers = () => {
    if (!justOpened) {
        hideContextMenu()
    }
}

onMounted(() => {
    window.addEventListener('click', handleClickOutside)
    window.addEventListener(CLOSE_ALL_EVENT, handleCloseOthers)
    document.body.addEventListener('keyup', onEscKeyRelease)
})

onBeforeUnmount(() => {
    cleanupPositionListeners()
    window.removeEventListener('click', handleClickOutside)
    window.removeEventListener(CLOSE_ALL_EVENT, handleCloseOthers)
    document.body.removeEventListener('keyup', onEscKeyRelease)
})
</script>

<style lang="scss" scoped>
.context-menu {
    background-color: var(--surface-3);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-floating);
    border: 1px solid var(--color-divider);
    margin: 0;
    position: fixed;
    z-index: 1000000;
    overflow-x: hidden;
    overflow-y: auto;
    padding: var(--gap-sm);
    box-sizing: border-box;

    .item {
        align-items: center;
        color: var(--color-text-default);
        cursor: pointer;
        display: flex;
        gap: var(--gap-sm);
        padding: var(--gap-sm);
        border-radius: var(--radius-sm);
        white-space: normal;

        &.disabled {
            cursor: not-allowed;
            opacity: 0.5;
        }

        &:hover,
        &:active {
            &.base {
                background-color: var(--surface-4);
                color: var(--color-text-primary);
            }

            &.primary {
                background-color: var(--color-brand);
                color: var(--color-accent-contrast);
                font-weight: bold;
            }

            &.danger {
                background-color: var(--color-red);
                color: var(--color-accent-contrast);
                font-weight: bold;
            }

            &.contrast {
                background-color: var(--color-orange);
                color: var(--color-accent-contrast);
                font-weight: bold;
            }
        }
    }

    .divider {
        border: 1px solid var(--color-divider);
        margin: var(--gap-sm);
        pointer-events: none;
    }
}

.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease-in-out;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>
