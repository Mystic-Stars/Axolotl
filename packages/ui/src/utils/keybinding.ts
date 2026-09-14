/**
 * The device a binding was recorded from. Keyboard bindings are matched against
 * `KeyboardEvent`, mouse bindings against `MouseEvent` and `WheelEvent`.
 */
export type KeyBindingDevice = 'keyboard' | 'mouse'

/**
 * A single shortcut. `mod` is the primary modifier held down: the launcher has
 * always treated Ctrl and Cmd as one and the same, so a binding recorded on one
 * platform keeps working on the other.
 */
export interface KeyBinding {
	device: KeyBindingDevice
	/** A `KeyboardEvent.code`, or one of the mouse codes below. */
	code: string
	mod: boolean
	shift: boolean
	alt: boolean
}

/** `MouseEvent.button`, in the order browsers report it. */
export const MOUSE_BUTTON_CODES = ['Mouse0', 'Mouse1', 'Mouse2', 'Mouse3', 'Mouse4'] as const
export const WHEEL_CODES = ['WheelUp', 'WheelDown'] as const

const MODIFIER_CODES = new Set([
	'ControlLeft',
	'ControlRight',
	'ShiftLeft',
	'ShiftRight',
	'AltLeft',
	'AltRight',
	'MetaLeft',
	'MetaRight',
	'CapsLock',
])

/**
 * Codes that may be bound on their own. Anything else has to carry a modifier,
 * so a shortcut cannot swallow ordinary typing or list navigation.
 */
const MODIFIER_FREE_CODES = new Set<string>([
	'Home',
	'End',
	'PageUp',
	'PageDown',
	...Array.from({ length: 24 }, (_, index) => `F${index + 1}`),
	...MOUSE_BUTTON_CODES.filter((code) => code !== 'Mouse0'),
	...WHEEL_CODES,
])

/**
 * Codes that always need a modifier, whatever else is allowed. A bare left
 * click is how the whole interface is operated, so it can never be a shortcut.
 */
const MODIFIER_REQUIRED_CODES = new Set<string>(['Mouse0'])

/** A combination someone else already answers to. */
export interface BindingConflict {
	id: string
	label: string
	binding: KeyBinding
	/** The shell or the operating system owns it rather than another action. */
	reserved?: boolean
}

export type BindingIssue =
	{ kind: 'conflict'; label: string; reserved: boolean } | { kind: 'needs-modifier' }

export interface KeybindingLabels {
	mod: string
	shift: string
	alt: string
	/** Keyed by mouse code, e.g. `Mouse0`. */
	mouseButton: Record<string, string>
	wheelUp: string
	wheelDown: string
}

export function isModifierCode(code: string): boolean {
	return MODIFIER_CODES.has(code)
}

export function isMouseCode(code: string): boolean {
	return code.startsWith('Mouse') || (WHEEL_CODES as readonly string[]).includes(code)
}

/**
 * Canonical form used to compare bindings, so two ways of recording the same
 * combination still collide.
 */
export function bindingKey(binding: KeyBinding): string {
	const modifiers = [binding.mod && 'mod', binding.alt && 'alt', binding.shift && 'shift']
		.filter(Boolean)
		.join('+')
	return `${binding.device}|${binding.code}|${modifiers}`
}

export function bindingsEqual(a: KeyBinding, b: KeyBinding): boolean {
	return bindingKey(a) === bindingKey(b)
}

function eventModifiers(event: MouseEvent | KeyboardEvent | WheelEvent) {
	return {
		mod: event.ctrlKey || event.metaKey,
		shift: event.shiftKey,
		alt: event.altKey,
	}
}

/**
 * The combination an event represents, or null while only modifiers are held -
 * a lone Ctrl is not a shortcut.
 */
export function bindingFromKeyboardEvent(event: KeyboardEvent): KeyBinding | null {
	if (event.repeat || isModifierCode(event.code)) return null
	return { device: 'keyboard', code: event.code, ...eventModifiers(event) }
}

export function bindingFromMouseEvent(event: MouseEvent): KeyBinding | null {
	const code = `Mouse${event.button}`
	if (!(MOUSE_BUTTON_CODES as readonly string[]).includes(code)) return null
	return { device: 'mouse', code, ...eventModifiers(event) }
}

export function bindingFromWheelEvent(event: WheelEvent): KeyBinding | null {
	if (event.deltaY === 0) return null
	const code = event.deltaY < 0 ? 'WheelUp' : 'WheelDown'
	return { device: 'mouse', code, ...eventModifiers(event) }
}

function modifiersMatch(binding: KeyBinding, event: MouseEvent | KeyboardEvent | WheelEvent) {
	const { mod, shift, alt } = eventModifiers(event)
	return binding.mod === mod && binding.shift === shift && binding.alt === alt
}

export function bindingMatchesKeyboardEvent(binding: KeyBinding, event: KeyboardEvent): boolean {
	return (
		binding.device === 'keyboard' && binding.code === event.code && modifiersMatch(binding, event)
	)
}

export function bindingMatchesMouseEvent(binding: KeyBinding, event: MouseEvent): boolean {
	return (
		binding.device === 'mouse' &&
		binding.code === `Mouse${event.button}` &&
		modifiersMatch(binding, event)
	)
}

export function bindingMatchesWheelEvent(binding: KeyBinding, event: WheelEvent): boolean {
	if (binding.device !== 'mouse' || event.deltaY === 0) return false
	const code = event.deltaY < 0 ? 'WheelUp' : 'WheelDown'
	return binding.code === code && modifiersMatch(binding, event)
}

/**
 * Whether a combination may be saved, ignoring who it belongs to. Returns the
 * problem to show instead of the binding when it may not.
 */
export function validateBinding(
	binding: KeyBinding,
	conflicts: BindingConflict[],
): BindingIssue | null {
	const unmodified = !binding.mod && !binding.shift && !binding.alt
	if (unmodified && MODIFIER_REQUIRED_CODES.has(binding.code)) {
		return { kind: 'needs-modifier' }
	}
	if (unmodified && !MODIFIER_FREE_CODES.has(binding.code)) {
		return { kind: 'needs-modifier' }
	}

	const conflict = conflicts.find((entry) => bindingsEqual(entry.binding, binding))
	if (conflict) {
		return { kind: 'conflict', label: conflict.label, reserved: conflict.reserved ?? false }
	}

	return null
}

const CODE_LABELS: Record<string, string> = {
	ArrowUp: '\u2191',
	ArrowDown: '\u2193',
	ArrowLeft: '\u2190',
	ArrowRight: '\u2192',
	PageUp: 'Page Up',
	PageDown: 'Page Down',
	Escape: 'Esc',
	Space: 'Space',
	Insert: 'Insert',
	Delete: 'Delete',
	Comma: ',',
	Period: '.',
	Slash: '/',
	Semicolon: ';',
	Quote: "'",
	BracketLeft: '[',
	BracketRight: ']',
	Backslash: '\\',
	Minus: '-',
	Equal: '=',
	Backquote: '`',
}

/** How a single code reads on a key cap. */
export function keyCodeLabel(code: string): string {
	if (code.startsWith('Key')) return code.slice(3)
	if (code.startsWith('Digit')) return code.slice(5)
	if (code.startsWith('Numpad')) return `Num ${code.slice(6)}`
	return CODE_LABELS[code] ?? code
}

/** The combination split into the pieces a key cap is drawn for. */
export function bindingParts(binding: KeyBinding, labels: KeybindingLabels): string[] {
	const parts: string[] = []
	if (binding.mod) parts.push(labels.mod)
	if (binding.alt) parts.push(labels.alt)
	if (binding.shift) parts.push(labels.shift)

	if (binding.code === 'WheelUp') parts.push(labels.wheelUp)
	else if (binding.code === 'WheelDown') parts.push(labels.wheelDown)
	else if (binding.device === 'mouse') parts.push(labels.mouseButton[binding.code] ?? binding.code)
	else parts.push(keyCodeLabel(binding.code))

	return parts
}

export function bindingText(binding: KeyBinding, labels: KeybindingLabels): string {
	return bindingParts(binding, labels).join(' + ')
}
