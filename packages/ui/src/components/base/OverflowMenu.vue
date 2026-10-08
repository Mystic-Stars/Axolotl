<template>
    <PopoutMenu
        ref="dropdown"
        data-button-group-item
        v-bind="$attrs"
        :disabled="disabled"
        :dropdown-id="dropdownId"
        :tooltip="tooltip"
        :placement="placement"
    >
        <slot></slot>
        <template #menu>
            <slot name="menu-header" />
            <template
                v-for="(option, index) in options.filter((x) => x.shown === undefined || x.shown)"
                :key="isDivider(option) ? `divider-${index}` : `option-${option.id}`"
            >
                <div v-if="isDivider(option)" class="h-px mx-[0.625rem] my-2 bg-surface-5"></div>
                <ButtonLink
                    v-else-if="option.link"
                    v-tooltip="option.tooltip"
                    :as="isInternalLink(option.link) ? RouterLink : 'a'"
                    :to="isInternalLink(option.link) ? option.link : undefined"
                    :href="isInternalLink(option.link) ? undefined : option.link"
                    type="quiet"
                    :color="normalizedColor(option.color)"
                    :interaction="normalizedInteraction(option)"
                    :download="option.download || undefined"
                    :target="option.external ? '_blank' : '_self'"
                    :disabled="option.disabled"
                    :class="optionClasses(option)"
                    @click="handleLinkClick(option, $event)"
                >
                    <template v-if="!$slots[option.id]">
                        <component :is="option.icon" v-if="option.icon" class="size-5" />
                        {{ option.id }}
                    </template>
                    <slot :name="option.id"></slot>
                    <ExternalIcon v-if="option.external" class="!size-3" />
                </ButtonLink>
                <Button
                    v-else
                    v-tooltip="option.tooltip"
                    type="quiet"
                    :color="normalizedColor(option.color)"
                    :interaction="normalizedInteraction(option)"
                    :disabled="option.disabled"
                    :class="optionClasses(option)"
                    @click="option.action ? handleActionClick(option, $event) : undefined"
                >
                    <template v-if="!$slots[option.id]">
                        <component :is="option.icon" v-if="option.icon" class="size-5" />
                        {{ option.id }}
                    </template>
                    <slot :name="option.id"></slot>
                </Button>
            </template>
        </template>
    </PopoutMenu>
</template>

<script setup lang="ts">
import ExternalIcon from '@modrinth/assets/icons/external.svg?component'
import { type Component, type Ref, ref } from 'vue'
import { RouterLink } from 'vue-router'

import Button from './buttons/Button.vue'
import ButtonLink from './buttons/ButtonLink.vue'
import type { ButtonColor, ButtonInteraction } from './buttons/types'
import PopoutMenu from './PopoutMenu.vue'

interface BaseOption {
    shown?: boolean
}

interface Divider extends BaseOption {
    divider?: boolean
}

type LegacyColor =
    | 'default'
    | 'primary'
    | 'danger'
    | 'secondary'
    | 'highlight'
    | 'red'
    | 'orange'
    | 'green'
    | 'blue'
    | 'purple'

interface Item extends BaseOption {
    id: string
    icon?: Component
    action?: (event?: MouseEvent) => void
    link?: string
    download?: string
    external?: boolean
    color?: LegacyColor
    hoverFilled?: boolean
    hoverFilledOnly?: boolean
    remainOnClick?: boolean
    disabled?: boolean
    tooltip?: string
}

export type Option = Divider | Item

withDefaults(
    defineProps<{
        options: Option[]
        disabled?: boolean
        dropdownId?: string
        tooltip?: string
        placement?: string
    }>(),
    {
        options: () => [],
        disabled: false,
        dropdownId: undefined,
        tooltip: undefined,
        placement: 'bottom-end',
    },
)

defineOptions({
    inheritAttrs: false,
})

const dropdown: Ref<InstanceType<typeof PopoutMenu> | null> = ref(null)

const close = () => {
    dropdown.value?.hide()
}

const open = () => {
    dropdown.value?.show()
}

function isDivider(option: BaseOption): option is Divider {
    return 'divider' in option
}

function isInternalLink(link: string): boolean {
    return link.startsWith('/')
}

function normalizedColor(color: LegacyColor | undefined): ButtonColor | undefined {
    switch (color) {
        case 'primary':
        case 'secondary':
            return 'brand'
        case 'danger':
            return 'red'
        case 'highlight':
            return 'orange'
        case 'red':
        case 'orange':
        case 'green':
        case 'blue':
        case 'purple':
            return color
        default:
            return undefined
    }
}

function normalizedInteraction(option: Item): ButtonInteraction {
    return (option.hoverFilled || option.hoverFilledOnly) && normalizedColor(option.color)
        ? 'filled'
        : 'surface'
}

function optionClasses(option: Item): string[] {
    return [
        'w-full !justify-start',
        option.hoverFilledOnly
            ? '[&:not(:hover):not(:focus-visible)]:!text-[var(--color-text-default)]'
            : '',
    ]
}

function handleActionClick(option: Item, event: MouseEvent) {
    option.action?.(event)
    if (!option.remainOnClick) close()
}

function handleLinkClick(option: Item, event: MouseEvent) {
    option.action?.(event)
    if (!option.remainOnClick) close()
}

defineExpose({ open, close })
</script>
