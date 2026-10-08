import assert from 'node:assert/strict'
import test from 'node:test'

import {
    type BindingConflict,
    bindingFromKeyboardEvent,
    bindingFromMouseEvent,
    bindingFromWheelEvent,
    bindingKey,
    bindingMatchesKeyboardEvent,
    bindingMatchesMouseEvent,
    bindingMatchesWheelEvent,
    bindingParts,
    bindingsEqual,
    type KeyBinding,
    type KeybindingLabels,
    keyCodeLabel,
    validateBinding,
} from './keybinding.ts'

const labels: KeybindingLabels = {
    mod: 'Ctrl/Cmd',
    shift: 'Shift',
    alt: 'Alt',
    mouseButton: { Mouse0: 'Left click', Mouse2: 'Right click' },
    wheelUp: 'Wheel up',
    wheelDown: 'Wheel down',
}

function keyboardEvent(init: Partial<KeyboardEvent> = {}): KeyboardEvent {
    return {
        code: 'KeyK',
        repeat: false,
        ctrlKey: false,
        shiftKey: false,
        altKey: false,
        metaKey: false,
        ...init,
    } as KeyboardEvent
}

function mouseEvent(init: Partial<MouseEvent> = {}): MouseEvent {
    return {
        button: 0,
        ctrlKey: false,
        shiftKey: false,
        altKey: false,
        metaKey: false,
        ...init,
    } as MouseEvent
}

function wheelEvent(init: Partial<WheelEvent> = {}): WheelEvent {
    return {
        deltaY: 0,
        ctrlKey: false,
        shiftKey: false,
        altKey: false,
        metaKey: false,
        ...init,
    } as WheelEvent
}

function key(code: string, modifiers: Partial<KeyBinding> = {}): KeyBinding {
    return { device: 'keyboard', code, mod: false, shift: false, alt: false, ...modifiers }
}

test('the same combination recorded with Ctrl or with Cmd compares equal', () => {
    const recordedOnWindows = bindingFromKeyboardEvent(
        keyboardEvent({ code: 'KeyK', ctrlKey: true }),
    )
    const recordedOnMac = bindingFromKeyboardEvent(keyboardEvent({ code: 'KeyK', metaKey: true }))

    assert.ok(recordedOnWindows && recordedOnMac)
    assert.equal(bindingKey(recordedOnWindows), bindingKey(recordedOnMac))
    assert.equal(bindingsEqual(recordedOnWindows, recordedOnMac), true)
})

test('modifier order does not change how a combination compares', () => {
    const first = key('KeyK', { mod: true, alt: true, shift: true })
    const second = { ...first }
    assert.equal(bindingsEqual(first, second), true)
    assert.equal(bindingKey(first), bindingKey(second))
})

test('holding only modifiers is not a combination yet', () => {
    assert.equal(
        bindingFromKeyboardEvent(keyboardEvent({ code: 'ControlLeft', ctrlKey: true })),
        null,
    )
    assert.equal(
        bindingFromKeyboardEvent(keyboardEvent({ code: 'ShiftRight', shiftKey: true })),
        null,
    )
})

test('a key repeat does not start a recording', () => {
    assert.equal(bindingFromKeyboardEvent(keyboardEvent({ repeat: true })), null)
    assert.ok(bindingFromKeyboardEvent(keyboardEvent()))
})

test('a combination only fires when its modifiers match exactly', () => {
    const binding = key('KeyK', { mod: true })

    assert.equal(bindingMatchesKeyboardEvent(binding, keyboardEvent({ ctrlKey: true })), true)
    assert.equal(
        bindingMatchesKeyboardEvent(binding, keyboardEvent({ ctrlKey: true, shiftKey: true })),
        false,
    )
    assert.equal(bindingMatchesKeyboardEvent(binding, keyboardEvent()), false)
    assert.equal(
        bindingMatchesKeyboardEvent(binding, keyboardEvent({ ctrlKey: true, code: 'KeyJ' })),
        false,
    )
})

test('a bare key is refused unless it can be pressed without disturbing typing', () => {
    const conflicts: BindingConflict[] = []

    assert.deepEqual(validateBinding(key('KeyA'), conflicts), { kind: 'needs-modifier' })
    assert.deepEqual(validateBinding(key('Digit1'), conflicts), { kind: 'needs-modifier' })
    assert.deepEqual(validateBinding(key('ArrowDown'), conflicts), { kind: 'needs-modifier' })
    assert.deepEqual(validateBinding(key('Escape'), conflicts), { kind: 'needs-modifier' })

    assert.equal(validateBinding(key('Home'), conflicts), null)
    assert.equal(validateBinding(key('F8'), conflicts), null)
    assert.equal(validateBinding(key('KeyA', { mod: true }), conflicts), null)
})

test('an unmodified pointer input is allowed, except a bare left click', () => {
    const conflicts: BindingConflict[] = []
    const click: KeyBinding = {
        device: 'mouse',
        code: 'Mouse0',
        mod: false,
        shift: false,
        alt: false,
    }
    const wheel: KeyBinding = {
        device: 'mouse',
        code: 'WheelUp',
        mod: false,
        shift: false,
        alt: false,
    }

    assert.deepEqual(validateBinding(click, conflicts), { kind: 'needs-modifier' })
    assert.equal(validateBinding({ ...click, mod: true }, conflicts), null)
    assert.equal(validateBinding(wheel, conflicts), null)
})

test('a combination another action already answers to is reported by name', () => {
    const conflicts: BindingConflict[] = [
        { id: 'shortcutNavHome', label: 'Home', binding: key('Digit1', { mod: true }) },
    ]

    assert.deepEqual(validateBinding(key('Digit1', { mod: true }), conflicts), {
        kind: 'conflict',
        label: 'Home',
        reserved: false,
    })
    assert.equal(validateBinding(key('KeyK', { mod: true }), conflicts), null)
})

test('a reserved combination is reported as reserved', () => {
    const conflicts: BindingConflict[] = [
        { id: 'reserved-find', label: 'Find', binding: key('KeyF', { mod: true }), reserved: true },
    ]

    assert.deepEqual(validateBinding(key('KeyF', { mod: true }), conflicts), {
        kind: 'conflict',
        label: 'Find',
        reserved: true,
    })
})

test('pointer inputs match by button and by wheel direction', () => {
    const middle: KeyBinding = {
        device: 'mouse',
        code: 'Mouse1',
        mod: false,
        shift: false,
        alt: false,
    }
    const wheelUp: KeyBinding = {
        device: 'mouse',
        code: 'WheelUp',
        mod: false,
        shift: false,
        alt: false,
    }

    assert.equal(bindingMatchesMouseEvent(middle, mouseEvent({ button: 1 })), true)
    assert.equal(bindingMatchesMouseEvent(middle, mouseEvent({ button: 0 })), false)
    assert.equal(bindingMatchesWheelEvent(wheelUp, wheelEvent({ deltaY: -120 })), true)
    assert.equal(bindingMatchesWheelEvent(wheelUp, wheelEvent({ deltaY: 120 })), false)
    assert.equal(bindingMatchesMouseEvent(wheelUp, mouseEvent({ button: 0 })), false)
})

test('a pointer binding can require a keyboard modifier', () => {
    const ctrlClick = {
        device: 'mouse',
        code: 'Mouse0',
        mod: true,
        shift: false,
        alt: false,
    } as KeyBinding

    assert.equal(
        bindingMatchesMouseEvent(ctrlClick, mouseEvent({ button: 0, ctrlKey: true })),
        true,
    )
    assert.equal(
        bindingMatchesMouseEvent(ctrlClick, mouseEvent({ button: 0, metaKey: true })),
        true,
    )
    assert.equal(bindingMatchesMouseEvent(ctrlClick, mouseEvent({ button: 0 })), false)
})

test('recording a pointer input reports what was pressed', () => {
    assert.equal(bindingFromMouseEvent(mouseEvent({ button: 2 }))?.code, 'Mouse2')
    assert.equal(bindingFromMouseEvent(mouseEvent({ button: 9 })), null)
    assert.equal(bindingFromWheelEvent(wheelEvent({ deltaY: -120 }))?.code, 'WheelUp')
    assert.equal(bindingFromWheelEvent(wheelEvent({ deltaY: 120 }))?.code, 'WheelDown')
    assert.equal(bindingFromWheelEvent(wheelEvent({ deltaY: 0 })), null)
})

test('a key cap reads like the key it stands for', () => {
    assert.equal(keyCodeLabel('KeyK'), 'K')
    assert.equal(keyCodeLabel('Digit1'), '1')
    assert.equal(keyCodeLabel('Comma'), ',')
    assert.equal(keyCodeLabel('PageUp'), 'Page Up')
    assert.equal(keyCodeLabel('F8'), 'F8')
})

test('a combination reads as modifiers first and the key last', () => {
    assert.deepEqual(bindingParts(key('KeyK', { mod: true, shift: true, alt: true }), labels), [
        'Ctrl/Cmd',
        'Alt',
        'Shift',
        'K',
    ])
    assert.deepEqual(bindingParts(key('Home'), labels), ['Home'])
    assert.deepEqual(
        bindingParts(
            { device: 'mouse', code: 'Mouse0', mod: true, shift: false, alt: false },
            labels,
        ),
        ['Ctrl/Cmd', 'Left click'],
    )
    assert.deepEqual(
        bindingParts(
            { device: 'mouse', code: 'WheelDown', mod: false, shift: false, alt: false },
            labels,
        ),
        ['Wheel down'],
    )
})
