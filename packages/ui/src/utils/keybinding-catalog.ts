import { defineMessages } from '../composables/i18n'
import type { KeyBindingDevice } from './keybinding'

/** Structurally identical to the i18n `MessageDescriptor`, without the dependency. */
export interface CatalogMessage {
    id: string
    defaultMessage: string
}

export interface KeybindingCatalogItem {
    device: KeyBindingDevice
    code: string
    /** Set for inputs whose name has to be translated; key caps read the same everywhere. */
    label?: CatalogMessage
}

export interface KeybindingCatalogGroup {
    id: string
    label: CatalogMessage
    items: KeybindingCatalogItem[]
}

export interface KeybindingCatalogDevice {
    id: KeyBindingDevice
    label: CatalogMessage
    groups: KeybindingCatalogGroup[]
}

const messages = defineMessages({
    mouseLeft: { id: 'ui.keybinding-modal.mouse-left', defaultMessage: 'Left click' },
    mouseMiddle: { id: 'ui.keybinding-modal.mouse-middle', defaultMessage: 'Middle click' },
    mouseRight: { id: 'ui.keybinding-modal.mouse-right', defaultMessage: 'Right click' },
    mouseBack: { id: 'ui.keybinding-modal.mouse-back', defaultMessage: 'Back button' },
    mouseForward: { id: 'ui.keybinding-modal.mouse-forward', defaultMessage: 'Forward button' },
    wheelUp: { id: 'ui.keybinding-modal.wheel-up', defaultMessage: 'Wheel up' },
    wheelDown: { id: 'ui.keybinding-modal.wheel-down', defaultMessage: 'Wheel down' },
    deviceKeyboard: { id: 'ui.keybinding-modal.device-keyboard', defaultMessage: 'Keyboard' },
    deviceMouse: { id: 'ui.keybinding-modal.device-mouse', defaultMessage: 'Mouse' },
    groupLetters: { id: 'ui.keybinding-modal.group-letters', defaultMessage: 'Letters' },
    groupDigits: { id: 'ui.keybinding-modal.group-digits', defaultMessage: 'Digits' },
    groupFunction: { id: 'ui.keybinding-modal.group-function', defaultMessage: 'Function keys' },
    groupNavigation: {
        id: 'ui.keybinding-modal.group-navigation',
        defaultMessage: 'Navigation keys',
    },
    groupPunctuation: {
        id: 'ui.keybinding-modal.group-punctuation',
        defaultMessage: 'Punctuation',
    },
    groupMouseButtons: {
        id: 'ui.keybinding-modal.group-mouse-buttons',
        defaultMessage: 'Buttons',
    },
    groupWheel: { id: 'ui.keybinding-modal.group-wheel', defaultMessage: 'Wheel' },
})

/**
 * Names for the pointer inputs. Shared by the picker tree and by the chips that
 * render a saved combination, so both read the same way.
 */
export const MOUSE_CODE_MESSAGES: Record<string, CatalogMessage> = {
    Mouse0: messages.mouseLeft,
    Mouse1: messages.mouseMiddle,
    Mouse2: messages.mouseRight,
    Mouse3: messages.mouseBack,
    Mouse4: messages.mouseForward,
    WheelUp: messages.wheelUp,
    WheelDown: messages.wheelDown,
}

const LETTER_CODES: KeybindingCatalogItem[] = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'
    .split('')
    .map((letter) => ({ device: 'keyboard', code: `Key${letter}` }))

const DIGIT_CODES: KeybindingCatalogItem[] = '0123456789'
    .split('')
    .map((digit) => ({ device: 'keyboard', code: `Digit${digit}` }))

const FUNCTION_CODES: KeybindingCatalogItem[] = Array.from({ length: 12 }, (_, index) => ({
    device: 'keyboard',
    code: `F${index + 1}`,
}))

const NAVIGATION_CODES: KeybindingCatalogItem[] = [
    'Home',
    'End',
    'PageUp',
    'PageDown',
    'Insert',
    'Delete',
    'ArrowUp',
    'ArrowDown',
    'ArrowLeft',
    'ArrowRight',
].map((code) => ({ device: 'keyboard', code }))

const PUNCTUATION_CODES: KeybindingCatalogItem[] = [
    'Comma',
    'Period',
    'Slash',
    'Semicolon',
    'Quote',
    'BracketLeft',
    'BracketRight',
    'Backslash',
    'Minus',
    'Equal',
    'Backquote',
    'Space',
].map((code) => ({ device: 'keyboard', code }))

const MOUSE_ITEMS: KeybindingCatalogItem[] = Object.entries(MOUSE_CODE_MESSAGES).map(
    ([code, label]) => ({ device: 'mouse', code, label }),
)

/**
 * Everything a shortcut can be attached to, grouped for the picker. Inputs that
 * cannot be reproduced by pressing them - mouse buttons, the wheel, and any
 * combination of those with the keyboard - are only reachable from here.
 */
export const KEYBINDING_CATALOG: KeybindingCatalogDevice[] = [
    {
        id: 'keyboard',
        label: messages.deviceKeyboard,
        groups: [
            { id: 'letters', label: messages.groupLetters, items: LETTER_CODES },
            { id: 'digits', label: messages.groupDigits, items: DIGIT_CODES },
            { id: 'function', label: messages.groupFunction, items: FUNCTION_CODES },
            { id: 'navigation', label: messages.groupNavigation, items: NAVIGATION_CODES },
            { id: 'punctuation', label: messages.groupPunctuation, items: PUNCTUATION_CODES },
        ],
    },
    {
        id: 'mouse',
        label: messages.deviceMouse,
        groups: [
            {
                id: 'buttons',
                label: messages.groupMouseButtons,
                items: MOUSE_ITEMS.filter((item) => item.code.startsWith('Mouse')),
            },
            {
                id: 'wheel',
                label: messages.groupWheel,
                items: MOUSE_ITEMS.filter((item) => item.code.startsWith('Wheel')),
            },
        ],
    },
]
