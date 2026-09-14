<template>
	<div ref="root" class="relative w-full font-mono text-base">
		<StyledInput
			v-if="!enhanced"
			v-model="fallbackValue"
			:icon="TerminalSquareIcon"
			:placeholder="placeholder"
			:disabled="disabled"
			wrapper-class="w-full"
			input-class="!h-9"
			autocomplete="off"
			:spellcheck="false"
			@keydown.enter="submitFallback"
		/>
		<div
			v-else
			class="relative min-h-9 overflow-hidden rounded-xl bg-surface-4 pl-10 pr-3 ring-brand-shadow focus-within:ring-4"
		>
			<TerminalSquareIcon
				class="pointer-events-none absolute left-3 top-2 h-5 w-5 text-secondary"
				aria-hidden="true"
			/>
			<div
				class="pointer-events-none min-h-9 whitespace-pre-wrap break-all py-2 font-medium leading-5 text-primary"
				aria-hidden="true"
			>
				<span v-for="(segment, index) in styledInput" :key="index" :style="segment.style">{{
					segment.text
				}}</span>
			</div>
			<textarea
				ref="input"
				:value="prompt?.value ?? ''"
				:placeholder="placeholder"
				:disabled="disabled"
				rows="1"
				class="absolute inset-0 h-full min-h-9 w-full resize-none overflow-hidden border-0 bg-transparent py-2 pl-10 pr-3 font-mono font-medium leading-5 text-transparent caret-[var(--color-text-default)] outline-none placeholder:text-secondary"
				autocomplete="off"
				autocorrect="off"
				autocapitalize="off"
				:spellcheck="false"
				@keydown="handleKeydown"
				@click="syncPointerCursor"
				@pointerdown="pointerSelecting = true"
				@beforeinput="handleBeforeInput"
				@paste="handlePaste"
				@compositionstart="handleCompositionStart"
				@compositionend="handleCompositionEnd"
			/>
		</div>
		<div
			v-if="candidates.length"
			class="absolute bottom-full left-0 z-20 mb-2 flex w-full min-w-64 max-w-lg flex-col overflow-hidden rounded-lg border border-solid border-surface-4 bg-surface-3 shadow-lg"
		>
			<div class="border-0 border-b border-solid border-surface-4 p-2">
				<StyledInput
					v-model="candidateSearchQuery"
					:icon="SearchIcon"
					:placeholder="formatMessage(messages.searchCompletions)"
					wrapper-class="w-full"
					input-class="!h-9 font-sans"
					autocomplete="off"
					:spellcheck="false"
					clearable
					@keydown.down.prevent="moveCandidateFocus(1)"
					@keydown.up.prevent="moveCandidateFocus(-1)"
					@keydown.tab.prevent="moveCandidateFocus($event.shiftKey ? -1 : 1)"
					@keydown.enter.prevent="selectFocusedCandidate"
					@keydown.escape.prevent="closeCandidateSearch"
				/>
			</div>
			<div
				v-if="filteredCandidates.length"
				role="listbox"
				class="flex max-h-56 flex-col gap-1 overflow-y-auto overscroll-contain p-2"
			>
				<button
					v-for="(candidate, index) in filteredCandidates"
					:key="`${candidate.row}:${candidate.column}:${candidate.text}`"
					type="button"
					role="option"
					:aria-selected="index === focusedCandidateIndex"
					:data-completion-focused="index === focusedCandidateIndex"
					class="min-h-9 min-w-0 shrink-0 rounded-md border-0 px-3 py-2 text-left font-mono text-sm transition-colors hover:bg-surface-4"
					:class="
						index === focusedCandidateIndex
							? 'bg-surface-4 text-contrast'
							: 'bg-transparent text-primary'
					"
					:style="candidateStyleToCss(candidate, index === focusedCandidateIndex)"
					@mouseenter="focusedCandidateIndex = index"
					@mousedown.prevent
					@click="selectCandidate(candidate)"
				>
					<span class="block truncate">{{ candidate.text }}</span>
				</button>
			</div>
			<div v-else class="px-3 py-4 text-center font-sans text-sm text-secondary">
				{{ formatMessage(messages.noCompletions) }}
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { SearchIcon, TerminalSquareIcon } from '@modrinth/assets'
import type { IBufferCell, Terminal } from '@xterm/xterm'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import StyledInput from '#ui/components/base/StyledInput.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

import {
	applyJLineCandidate,
	createJLineLineReplacementSequence,
	extractJLinePrompt,
	type JLineCandidate,
	type JLineCell,
	type JLineCellStyle,
	jlineKeySequence,
	type JLineRow,
	parseJLineCandidateConfirmation,
	parseJLineCandidates,
	replaceJLineSelection,
} from '../jline'

const props = defineProps<{
	disabled?: boolean
	placeholder?: string
	sendInput: (data: Uint8Array) => void | Promise<void>
	sendCommand: (command: string) => void | Promise<void>
	resizeConsole: (cols: number, rows: number) => void | Promise<void>
}>()

const root = ref<HTMLElement | null>(null)
const input = ref<HTMLTextAreaElement | null>(null)
const fallbackValue = ref('')
const prompt = ref<ReturnType<typeof extractJLinePrompt>>(null)
const shellPrompt = ref<ReturnType<typeof extractJLinePrompt>>(null)
const completionPrompt = ref<ReturnType<typeof extractJLinePrompt>>(null)
const candidates = ref<JLineCandidate[]>([])
const candidateSearchQuery = ref('')
const focusedCandidateIndex = ref(-1)
const enhanced = ref(false)
const composing = ref(false)
const menuRequested = ref(false)
const pointerSelecting = ref(false)
let terminal: Terminal | null = null
let resizeObserver: ResizeObserver | null = null
let pendingWrites: Uint8Array[] = []
let inputWriteQueue = Promise.resolve()
const encoder = new TextEncoder()
const TERMINAL_ROWS = 12
const INITIAL_CANDIDATE_TERMINAL_ROWS = 512
const MAX_CANDIDATE_TERMINAL_ROWS = 8192
let terminalRows = TERMINAL_ROWS
let candidateConfirmationPending = false
let suppressCompositionEnter = false
let compositionEnterTimer: ReturnType<typeof setTimeout> | null = null
let compositionSelection: { start: number; end: number } | null = null

const { formatMessage } = useVIntl()
const messages = defineMessages({
	searchCompletions: {
		id: 'console.command-completions.search',
		defaultMessage: 'Search command completions',
	},
	noCompletions: {
		id: 'console.command-completions.no-results',
		defaultMessage: 'No matching completions',
	},
})

const filteredCandidates = computed(() => {
	const query = candidateSearchQuery.value.trim().toLowerCase()
	if (!query) return candidates.value
	return candidates.value.filter((candidate) => candidate.text.toLowerCase().includes(query))
})

watch(candidates, (list) => {
	if (list.length === 0) {
		candidateSearchQuery.value = ''
		focusedCandidateIndex.value = -1
		return
	}
	focusedCandidateIndex.value = filteredCandidates.value.length ? 0 : -1
})

watch(candidateSearchQuery, () => {
	focusedCandidateIndex.value = filteredCandidates.value.length > 0 ? 0 : -1
})

const styledInput = computed(() => {
	if (!prompt.value) return []
	const cells = prompt.value.rows.flatMap((row) => row.cells).slice(2)
	const segments: Array<{ text: string; style: Record<string, string> }> = []
	let remaining = prompt.value.value.length
	for (const cell of cells) {
		if (remaining <= 0 || !cell.text) continue
		const text = cell.text.slice(0, remaining)
		remaining -= text.length
		const style = styleToCss(cell.style)
		const previous = segments.at(-1)
		if (previous && JSON.stringify(previous.style) === JSON.stringify(style)) {
			previous.text += text
		} else {
			segments.push({ text, style })
		}
	}
	return segments
})

onMounted(async () => {
	window.addEventListener('pointerup', finishPointerSelection)
	window.addEventListener('pointercancel', finishPointerSelection)
	const { Terminal } = await import('@xterm/xterm')
	terminal = new Terminal({
		cols: 80,
		rows: terminalRows,
		scrollback: 0,
		convertEol: false,
		allowProposedApi: true,
	})
	for (const bytes of pendingWrites) terminal.write(bytes, updateFromTerminal)
	pendingWrites = []
	resizeObserver = new ResizeObserver(resize)
	if (root.value) resizeObserver.observe(root.value)
	resize()
	await sendText('\x0c')
})

onBeforeUnmount(() => {
	if (compositionEnterTimer) window.clearTimeout(compositionEnterTimer)
	window.removeEventListener('pointerup', finishPointerSelection)
	window.removeEventListener('pointercancel', finishPointerSelection)
	resizeObserver?.disconnect()
	terminal?.dispose()
})

function write(data: Uint8Array) {
	if (!terminal) {
		pendingWrites.push(data)
		return
	}
	terminal.write(data, updateFromTerminal)
}

function resize() {
	if (!root.value || !terminal) return
	const fontSize = Number.parseFloat(getComputedStyle(root.value).fontSize) || 16
	const cellWidth = fontSize * 0.61
	const cols = Math.max(20, Math.floor((root.value.clientWidth - 52) / cellWidth))
	if (terminal.cols !== cols || terminal.rows !== terminalRows) {
		terminal.resize(cols, terminalRows)
	}
	void Promise.resolve(props.resizeConsole(cols, terminalRows)).catch(() => {})
}

function updateFromTerminal() {
	if (!terminal) return
	const restoreFocus = document.activeElement === input.value
	const buffer = terminal.buffer.active
	const cursorRow = buffer.baseY + buffer.cursorY
	const rows: JLineRow[] = []
	for (let index = buffer.viewportY; index < buffer.viewportY + terminal.rows; index++) {
		const line = buffer.getLine(index)
		if (!line) continue
		const cells: JLineCell[] = []
		const reusable = buffer.getNullCell()
		for (let column = 0; column < terminal.cols; column++) {
			const cell = line.getCell(column, reusable)
			if (!cell) continue
			cells.push({
				text: cell.getWidth() === 0 ? '' : cell.getChars() || ' ',
				width: cell.getWidth(),
				style: cellStyle(cell),
			})
		}
		rows.push({ index, wrapped: line.isWrapped, cells })
	}
	const nextPrompt = extractJLinePrompt(rows, cursorRow, buffer.cursorX)
	if (nextPrompt) shellPrompt.value = nextPrompt
	prompt.value = menuRequested.value && completionPrompt.value ? completionPrompt.value : nextPrompt
	if (nextPrompt) enhanced.value = true
	if (menuRequested.value) {
		const confirmation = parseJLineCandidateConfirmation(rows)
		if (confirmation && !candidateConfirmationPending) {
			candidateConfirmationPending = true
			void confirmCandidateMenu(confirmation.lineCount)
			return
		}
		const parsedCandidates = parseJLineCandidates(rows, cursorRow)
		if (parsedCandidates.length > 0) {
			candidateConfirmationPending = false
			if (!sameCandidates(candidates.value, parsedCandidates)) candidates.value = parsedCandidates
		} else if (parsedCandidates.length === 0 && candidates.value.length === 0) {
			if (nextPrompt) {
				completionPrompt.value = nextPrompt
				prompt.value = nextPrompt
			}
		}
	}
	void nextTick(() => {
		if (!input.value || !prompt.value || composing.value) return
		if (pointerSelecting.value || hasTextSelection()) return
		if (restoreFocus) input.value.focus()
		const cursor = Math.min(prompt.value.cursor, prompt.value.value.length)
		input.value.setSelectionRange(cursor, cursor)
	})
}

function cellStyle(cell: IBufferCell): JLineCellStyle {
	return {
		foreground: resolveColor(cell, 'foreground'),
		background: resolveColor(cell, 'background'),
		bold: Boolean(cell.isBold()),
		italic: Boolean(cell.isItalic()),
		underline: Boolean(cell.isUnderline()),
		dim: Boolean(cell.isDim()),
		inverse: Boolean(cell.isInverse()),
	}
}

function resolveColor(cell: IBufferCell, kind: 'foreground' | 'background') {
	const rgb = kind === 'foreground' ? cell.isFgRGB() : cell.isBgRGB()
	const palette = kind === 'foreground' ? cell.isFgPalette() : cell.isBgPalette()
	const color = kind === 'foreground' ? cell.getFgColor() : cell.getBgColor()
	if (rgb) return `#${color.toString(16).padStart(6, '0')}`
	if (!palette) return undefined
	return ansiPaletteColor(color)
}

function styleToCss(style: JLineCellStyle): Record<string, string> {
	let foreground = style.foreground ?? 'var(--color-text-default)'
	let background = style.background ?? 'var(--surface-4)'
	if (style.inverse) [foreground, background] = [background, foreground]
	return {
		...(foreground ? { color: foreground } : {}),
		...(background ? { backgroundColor: background } : {}),
		...(style.bold ? { fontWeight: '700' } : {}),
		...(style.italic ? { fontStyle: 'italic' } : {}),
		...(style.underline ? { textDecoration: 'underline' } : {}),
		...(style.dim ? { opacity: '0.65' } : {}),
	}
}

function candidateStyleToCss(candidate: JLineCandidate, focused: boolean): Record<string, string> {
	const style = candidate.style
	return {
		...(!focused && style.foreground ? { color: style.foreground } : {}),
		...(style.bold ? { fontWeight: '700' } : {}),
		...(style.italic ? { fontStyle: 'italic' } : {}),
		...(style.underline ? { textDecoration: 'underline' } : {}),
		...(style.dim ? { opacity: '0.65' } : {}),
	}
}

function sameCandidates(current: JLineCandidate[], next: JLineCandidate[]) {
	return (
		current.length === next.length &&
		current.every(
			(candidate, index) =>
				candidate.text === next[index]?.text &&
				candidate.row === next[index]?.row &&
				candidate.column === next[index]?.column,
		)
	)
}

function ansiPaletteColor(index: number): string {
	if (ANSI_COLORS[index]) return ANSI_COLORS[index]
	if (index >= 16 && index <= 231) {
		const value = index - 16
		const red = Math.floor(value / 36)
		const green = Math.floor((value % 36) / 6)
		const blue = value % 6
		const channel = (part: number) => (part === 0 ? 0 : 55 + part * 40)
		return `rgb(${channel(red)}, ${channel(green)}, ${channel(blue)})`
	}
	const gray = 8 + Math.max(0, Math.min(23, index - 232)) * 10
	return `rgb(${gray}, ${gray}, ${gray})`
}

function handleKeydown(event: KeyboardEvent) {
	if (composing.value || event.isComposing || event.keyCode === 229) return
	if (event.key === 'Enter' && suppressCompositionEnter) {
		event.preventDefault()
		suppressCompositionEnter = false
		if (compositionEnterTimer) window.clearTimeout(compositionEnterTimer)
		return
	}
	suppressCompositionEnter = false
	const selection = inputSelection()
	if (
		selection &&
		selection.start !== selection.end &&
		(event.key === 'Backspace' || event.key === 'Delete')
	) {
		event.preventDefault()
		replaceSelection('', selection.start, selection.end)
		return
	}
	if (candidates.value.length > 0) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault()
			moveCandidateFocus(event.key === 'ArrowDown' ? 1 : -1)
			return
		}
		if (event.key === 'Tab') {
			event.preventDefault()
			moveCandidateFocus(event.shiftKey ? -1 : 1)
			return
		}
		if (event.key === ' ' || event.key === 'Enter') {
			event.preventDefault()
			selectFocusedCandidate()
			return
		}
	}
	if (event.key === 'Tab' && menuRequested.value && candidates.value.length > 0) {
		event.preventDefault()
		return
	}
	const sequence = keySequence(event.key)
	if (!sequence) return
	event.preventDefault()
	if (menuRequested.value) {
		if (event.key === 'Tab') {
			void sendText(sequence)
			return
		}
		continueFromCompletion(event.key === 'Escape' ? '' : sequence)
		return
	}
	if (event.key === 'Tab' && !menuRequested.value) {
		completionPrompt.value = prompt.value
		menuRequested.value = true
		void openCandidateMenu(sequence)
		return
	}
	clearCompletion()
	void sendText(sequence)
}

function syncPointerCursor() {
	if (!input.value || !prompt.value) return
	if (hasTextSelection()) return
	const target = input.value.selectionStart ?? prompt.value.cursor
	const distance = target - prompt.value.cursor
	if (distance === 0) return
	const key = distance < 0 ? 'ArrowLeft' : 'ArrowRight'
	const sequence = keySequence(key)
	if (sequence) void sendText(sequence.repeat(Math.abs(distance)))
}

function keySequence(key: string) {
	return jlineKeySequence(key, terminal?.modes.applicationCursorKeysMode ?? false)
}

function handleBeforeInput(event: InputEvent) {
	if (composing.value || event.inputType === 'insertFromPaste') return
	const selection = inputSelection()
	if (event.inputType.startsWith('delete')) {
		if (!selection) return
		let { start, end } = selection
		if (start === end && event.inputType.includes('Backward') && start > 0) start--
		if (
			start === end &&
			event.inputType.includes('Forward') &&
			end < (prompt.value?.value.length ?? 0)
		) {
			end++
		}
		if (start === end) return
		event.preventDefault()
		replaceSelection('', start, end)
		return
	}
	if (!event.inputType.startsWith('insert') || !event.data) return
	event.preventDefault()
	if (selection && (selection.start !== selection.end || menuRequested.value)) {
		replaceSelection(event.data, selection.start, selection.end)
		return
	}
	clearCompletion()
	void sendText(event.data)
}

function handlePaste(event: ClipboardEvent) {
	const text = event.clipboardData?.getData('text')
	if (!text) return
	event.preventDefault()
	const selection = inputSelection()
	if (selection && (selection.start !== selection.end || menuRequested.value)) {
		replaceSelection(text, selection.start, selection.end)
		return
	}
	clearCompletion()
	void sendText(text)
}

function handleCompositionStart() {
	composing.value = true
	compositionSelection = inputSelection()
	suppressCompositionEnter = false
	if (compositionEnterTimer) window.clearTimeout(compositionEnterTimer)
}

function handleCompositionEnd(event: CompositionEvent) {
	composing.value = false
	suppressCompositionEnter = true
	if (compositionEnterTimer) window.clearTimeout(compositionEnterTimer)
	compositionEnterTimer = window.setTimeout(() => {
		suppressCompositionEnter = false
		compositionEnterTimer = null
	}, 100)
	const selection = compositionSelection
	compositionSelection = null
	if (event.data && selection && (selection.start !== selection.end || menuRequested.value)) {
		replaceSelection(event.data, selection.start, selection.end)
	} else {
		clearCompletion()
		if (event.data) void sendText(event.data)
	}
}

function sendText(text: string) {
	const data = encoder.encode(text)
	inputWriteQueue = inputWriteQueue
		.then(() => props.sendInput(data))
		.catch(() => {
			// A stop can race the final keystroke; keep the queue usable for the next write.
		})
	return inputWriteQueue
}

function selectCandidate(target: JLineCandidate) {
	const base = completionPrompt.value ?? prompt.value
	const current = shellPrompt.value ?? prompt.value
	if (!base || !current) return
	const next = applyJLineCandidate(base, target.text)
	clearCompletion()
	void sendText(
		createJLineLineReplacementSequence(
			current.value,
			next,
			terminal?.modes.applicationCursorKeysMode ?? false,
		),
	)
	input.value?.focus()
}

function selectFocusedCandidate() {
	const candidate = filteredCandidates.value[focusedCandidateIndex.value]
	if (candidate) void selectCandidate(candidate)
}

function moveCandidateFocus(delta: number) {
	const count = filteredCandidates.value.length
	if (count === 0) return
	focusedCandidateIndex.value = (focusedCandidateIndex.value + delta + count) % count
	void nextTick(() => {
		root.value
			?.querySelector('[data-completion-focused="true"]')
			?.scrollIntoView({ block: 'nearest' })
	})
}

function closeCandidateSearch() {
	candidateSearchQuery.value = ''
	input.value?.focus()
}

function replaceSelection(replacement: string, start: number, end: number) {
	const displayed = prompt.value
	const current = shellPrompt.value ?? displayed
	if (!displayed || !current) return
	const next = replaceJLineSelection(displayed, start, end, replacement)
	clearCompletion()
	void sendText(
		createJLineLineReplacementSequence(
			current.value,
			next,
			terminal?.modes.applicationCursorKeysMode ?? false,
		),
	)
}

function continueFromCompletion(sequence: string) {
	const base = completionPrompt.value ?? prompt.value
	const current = shellPrompt.value ?? prompt.value
	if (!base || !current) return
	const next = { value: base.value, cursor: base.cursor }
	clearCompletion()
	void sendText(
		`${createJLineLineReplacementSequence(current.value, next, terminal?.modes.applicationCursorKeysMode ?? false)}${sequence}`,
	)
}

async function openCandidateMenu(sequence: string) {
	if (terminal) {
		terminalRows = INITIAL_CANDIDATE_TERMINAL_ROWS
		if (terminal.rows !== terminalRows) terminal.resize(terminal.cols, terminalRows)
		await Promise.resolve(props.resizeConsole(terminal.cols, terminalRows)).catch(() => {})
	}
	await sendText(sequence)
}

async function confirmCandidateMenu(lineCount: number) {
	if (!terminal || lineCount <= 0 || lineCount + 4 > MAX_CANDIDATE_TERMINAL_ROWS) {
		await sendText('n')
		clearCompletion()
		return
	}
	terminalRows = Math.max(INITIAL_CANDIDATE_TERMINAL_ROWS, lineCount + 4)
	if (terminal.rows !== terminalRows) terminal.resize(terminal.cols, terminalRows)
	await Promise.resolve(props.resizeConsole(terminal.cols, terminalRows)).catch(() => {})
	await sendText('y')
}

function clearCompletion() {
	candidateConfirmationPending = false
	menuRequested.value = false
	completionPrompt.value = null
	candidates.value = []
	candidateSearchQuery.value = ''
	resetTerminalRows()
}

function resetTerminalRows() {
	if (!terminal || terminalRows === TERMINAL_ROWS) return
	terminalRows = TERMINAL_ROWS
	if (terminal.rows !== terminalRows) terminal.resize(terminal.cols, terminalRows)
	void Promise.resolve(props.resizeConsole(terminal.cols, terminalRows)).catch(() => {})
}

function inputSelection() {
	if (!input.value) return null
	return {
		start: input.value.selectionStart ?? 0,
		end: input.value.selectionEnd ?? 0,
	}
}

function hasTextSelection() {
	const selection = inputSelection()
	return Boolean(selection && selection.start !== selection.end)
}

function finishPointerSelection() {
	pointerSelecting.value = false
}

function submitFallback() {
	const command = fallbackValue.value.trim()
	if (!command || props.disabled) return
	void Promise.resolve(props.sendCommand(command)).catch(() => {})
	fallbackValue.value = ''
}

defineExpose({ write })

const ANSI_COLORS = [
	'#1d1f23',
	'#ff496e',
	'#1bd96a',
	'#ffa347',
	'#4a9eff',
	'#bc3fbc',
	'#96a2b0',
	'#b0bac5',
	'#42444a',
	'#ff496e',
	'#1bd96a',
	'#ffa347',
	'#4a9eff',
	'#bc3fbc',
	'#96a2b0',
	'#ffffff',
]
</script>
