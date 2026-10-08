import { computed } from 'vue'

import {
    bindingParts,
    bindingText,
    type KeyBinding,
    type KeybindingLabels,
} from '../utils/keybinding'
import { MOUSE_CODE_MESSAGES } from '../utils/keybinding-catalog'
import { defineMessages, useVIntl } from './i18n'

const messages = defineMessages({
    mod: { id: 'ui.keybinding-modal.mod', defaultMessage: 'Ctrl/Cmd' },
    shift: { id: 'ui.keybinding-modal.shift', defaultMessage: 'Shift' },
    alt: { id: 'ui.keybinding-modal.alt', defaultMessage: 'Alt' },
})

/**
 * The names a combination is built from, in the reader's language. Shared so a
 * shortcut reads the same wherever it is shown.
 */
export function useKeybindingLabels() {
    const { formatMessage } = useVIntl()

    return computed<KeybindingLabels>(() => ({
        mod: formatMessage(messages.mod),
        shift: formatMessage(messages.shift),
        alt: formatMessage(messages.alt),
        mouseButton: Object.fromEntries(
            Object.entries(MOUSE_CODE_MESSAGES).map(([code, message]) => [
                code,
                formatMessage(message),
            ]),
        ),
        wheelUp: formatMessage(MOUSE_CODE_MESSAGES.WheelUp),
        wheelDown: formatMessage(MOUSE_CODE_MESSAGES.WheelDown),
    }))
}

/** Splits a combination into the pieces a key cap is drawn for. */
export function useBindingParts(binding: () => KeyBinding | null) {
    const labels = useKeybindingLabels()

    return computed(() => {
        const value = binding()
        return value ? bindingParts(value, labels.value) : []
    })
}

export function useBindingText(binding: () => KeyBinding | null) {
    const labels = useKeybindingLabels()

    return computed(() => {
        const value = binding()
        return value ? bindingText(value, labels.value) : ''
    })
}
