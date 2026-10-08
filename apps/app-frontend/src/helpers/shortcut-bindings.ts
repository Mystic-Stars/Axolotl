import type { BindingConflict, KeyBinding } from '@modrinth/ui'

import { SHORTCUT_ACTIONS, shortcutAction } from '@/helpers/shortcut-actions'

const BINDING_STORAGE_PREFIX = 'axolotl-shortcut-binding-'

/**
 * Combinations the shell or the operating system answers to before the launcher
 * sees them. Saved shortcuts may not take these over.
 */
export const RESERVED_BINDINGS: BindingConflict[] = [
    {
        id: 'system-find',
        label: 'Find in page (Ctrl/Cmd + F)',
        binding: keyboard('KeyF', true),
        reserved: true,
    },
    {
        id: 'system-devtools',
        label: 'Developer tools (F12)',
        binding: keyboard('F12'),
        reserved: true,
    },
    {
        id: 'system-close',
        label: 'Close window (Ctrl/Cmd + W)',
        binding: keyboard('KeyW', true),
        reserved: true,
    },
    {
        id: 'system-quit',
        label: 'Quit (Ctrl/Cmd + Q)',
        binding: keyboard('KeyQ', true),
        reserved: true,
    },
    {
        id: 'system-reload',
        label: 'Reload (Ctrl/Cmd + R)',
        binding: keyboard('KeyR', true),
        reserved: true,
    },
    { id: 'system-refresh', label: 'Refresh (F5)', binding: keyboard('F5'), reserved: true },
    {
        id: 'system-exit',
        label: 'Close window (Alt + F4)',
        binding: keyboard('F4', false, true),
        reserved: true,
    },
]

function keyboard(code: string, mod = false, alt = false, shift = false): KeyBinding {
    return { device: 'keyboard', code, mod, alt, shift }
}

function isKeyBinding(value: unknown): value is KeyBinding {
    if (!value || typeof value !== 'object') return false
    const candidate = value as Partial<KeyBinding>
    return (
        (candidate.device === 'keyboard' || candidate.device === 'mouse') &&
        typeof candidate.code === 'string' &&
        candidate.code.length > 0 &&
        typeof candidate.mod === 'boolean' &&
        typeof candidate.shift === 'boolean' &&
        typeof candidate.alt === 'boolean'
    )
}

/** The combination recorded for an action, if it has one. */
export function readStoredBinding(id: string): KeyBinding | null {
    const raw = localStorage.getItem(BINDING_STORAGE_PREFIX + id)
    if (!raw) return null

    try {
        const parsed = JSON.parse(raw)
        return isKeyBinding(parsed) ? parsed : null
    } catch {
        // A value that cannot be read is treated as unset, so the action keeps
        // working with its default rather than becoming unreachable.
        return null
    }
}

export function storeBinding(id: string, binding: KeyBinding) {
    localStorage.setItem(BINDING_STORAGE_PREFIX + id, JSON.stringify(binding))
}

/** Drops the recorded combination, putting the action back on its default. */
export function clearStoredBinding(id: string) {
    if (readStoredBinding(id)) localStorage.removeItem(BINDING_STORAGE_PREFIX + id)
}

/** The combination an action answers to now: the recorded one, or its default. */
export function resolveBinding(id: string): KeyBinding | null {
    const action = shortcutAction(id)
    if (!action) return null
    return readStoredBinding(id) ?? action.defaultBinding
}

export function resolveAllBindings(): Record<string, KeyBinding> {
    const bindings: Record<string, KeyBinding> = {}
    for (const action of SHORTCUT_ACTIONS) {
        bindings[action.id] = resolveBinding(action.id) ?? action.defaultBinding
    }
    return bindings
}

/**
 * Everything a new combination would collide with: the other actions, plus the
 * combinations the shell owns. `label` is what the dialog names in the error.
 */
export function buildConflicts(exceptId: string, label: (id: string) => string): BindingConflict[] {
    const conflicts = SHORTCUT_ACTIONS.filter((action) => action.id !== exceptId).flatMap(
        (action) => {
            const binding = resolveBinding(action.id)
            return binding ? [{ id: action.id, label: label(action.id), binding }] : []
        },
    )

    return [...conflicts, ...RESERVED_BINDINGS]
}
