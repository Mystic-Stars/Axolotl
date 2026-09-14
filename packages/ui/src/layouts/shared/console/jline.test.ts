import assert from 'node:assert/strict'
import test from 'node:test'

import {
	applyJLineCandidate,
	createJLineCandidateInsertion,
	createJLineLineReplacementSequence,
	extractJLinePrompt,
	JLINE_KEY_SEQUENCES,
	jlineKeySequence,
	type JLineRow,
	parseJLineCandidateConfirmation,
	parseJLineCandidates,
	replaceJLineSelection,
} from './jline.ts'

const row = (index: number, text: string, wrapped = false, inverse = ''): JLineRow => ({
	index,
	wrapped,
	cells: [...text].map((character) => ({
		text: character,
		width: 1,
		style: { inverse: inverse.includes(character) },
	})),
})

test('extracts wrapped prompt input and cursor position', () => {
	const rows = [row(2, '> say a very '), row(3, 'long command', true)]
	assert.deepEqual(extractJLinePrompt(rows, 3, 4), {
		value: 'say a very long command',
		cursor: 15,
		rows,
	})
})

test('rejects a line that is not a JLine prompt', () => {
	assert.equal(extractJLinePrompt([row(1, 'server output')], 1, 3), null)
})

test('preserves spaces entered at the end of the command', () => {
	const rows = [row(1, '> say     ')]
	assert.equal(extractJLinePrompt(rows, 1, 6)?.value, 'say ')
})

test('builds a direct edit that replaces the candidate token without executing it', () => {
	assert.deepEqual(
		createJLineCandidateInsertion({ value: 'gamerule advance_t', cursor: 18 }, 'advance_weather'),
		{ deleteBefore: 9, deleteAfter: 0, text: 'advance_weather ' },
	)
	assert.deepEqual(createJLineCandidateInsertion({ value: 'give stne 1', cursor: 7 }, 'stone'), {
		deleteBefore: 2,
		deleteAfter: 2,
		text: 'stone',
	})
	assert.deepEqual(applyJLineCandidate({ value: 'gamerule keep_inventory ', cursor: 24 }, 'true'), {
		value: 'gamerule keep_inventory true ',
		cursor: 29,
	})
})

test('replaces selections and creates a deterministic JLine redraw sequence', () => {
	const edit = replaceJLineSelection({ value: 'gamerule keep_inventory true' }, 0, 28, '')
	assert.deepEqual(edit, { value: '', cursor: 0 })
	assert.equal(
		createJLineLineReplacementSequence('give stne 1', { value: 'give stone 1', cursor: 10 }),
		`\x01\x0bgive stone 1${JLINE_KEY_SEQUENCES.ArrowLeft.repeat(2)}`,
	)
	assert.ok(
		createJLineLineReplacementSequence('g', { value: 'gamemode ', cursor: 8 }, true).endsWith(
			'\x1bOD',
		),
	)
})

test('splits candidate columns and preserves terminal coordinates', () => {
	const candidates = parseJLineCandidates(
		[row(4, '> give @p '), row(5, 'stone  stick', false, 't')],
		4,
	)
	assert.deepEqual(
		candidates.map(({ text, row, column, selected }) => ({ text, row, column, selected })),
		[
			{ text: 'stone', row: 5, column: 0, selected: true },
			{ text: 'stick', row: 5, column: 7, selected: true },
		],
	)
})

test('rejects truncated candidate menus and defines terminal key bytes', () => {
	assert.deepEqual(parseJLineCandidates([row(1, '> g'), row(2, 'one  --More--')], 1), [])
	assert.equal(JLINE_KEY_SEQUENCES.ArrowLeft, '\x1b[D')
	assert.equal(jlineKeySequence('ArrowLeft', true), '\x1bOD')
	assert.equal(JLINE_KEY_SEQUENCES.Backspace, '\x7f')
})

test('detects Forge candidate confirmation prompts and their expanded row count', () => {
	assert.deepEqual(
		parseJLineCandidateConfirmation([
			row(1, 'Forge: do you wish to see all 51 '),
			row(2, 'possibilities (13 lines)?', true),
		]),
		{ lineCount: 13 },
	)
	assert.deepEqual(
		parseJLineCandidateConfirmation([
			row(1, '[Server thread/INFO] Forge: display all 2048 possibilities (512 lines)?'),
		]),
		{ lineCount: 512 },
	)
	assert.equal(
		parseJLineCandidateConfirmation([
			row(1, 'Forge: display all 10 possibilities (2 lines)?'),
			row(2, '> gamerule'),
		]),
		null,
	)
	assert.equal(parseJLineCandidateConfirmation([row(1, '> gamerule')]), null)
})
