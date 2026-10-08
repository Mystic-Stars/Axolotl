export interface JLineCellStyle {
    foreground?: string
    background?: string
    bold?: boolean
    italic?: boolean
    underline?: boolean
    dim?: boolean
    inverse?: boolean
}

export interface JLineCell {
    text: string
    width: number
    style: JLineCellStyle
}

export interface JLineRow {
    index: number
    wrapped: boolean
    cells: JLineCell[]
}

export interface JLinePrompt {
    value: string
    cursor: number
    rows: JLineRow[]
}

export interface JLineCandidate {
    text: string
    row: number
    column: number
    selected: boolean
    style: JLineCellStyle
}

export interface JLineCandidateConfirmation {
    lineCount: number
}

export interface JLineCandidateInsertion {
    deleteBefore: number
    deleteAfter: number
    text: string
}

export interface JLineInputEdit {
    value: string
    cursor: number
}

export const JLINE_KEY_SEQUENCES: Record<string, string> = {
    Tab: '\t',
    ArrowUp: '\x1b[A',
    ArrowDown: '\x1b[B',
    ArrowRight: '\x1b[C',
    ArrowLeft: '\x1b[D',
    Home: '\x1b[H',
    End: '\x1b[F',
    Delete: '\x1b[3~',
    Backspace: '\x7f',
    Enter: '\r',
    Escape: '\x1b',
}

export function jlineKeySequence(key: string, applicationCursorKeys = false): string | undefined {
    if (applicationCursorKeys) {
        const applicationSequences: Record<string, string> = {
            ArrowUp: '\x1bOA',
            ArrowDown: '\x1bOB',
            ArrowRight: '\x1bOC',
            ArrowLeft: '\x1bOD',
            Home: '\x1bOH',
            End: '\x1bOF',
        }
        if (applicationSequences[key]) return applicationSequences[key]
    }
    return JLINE_KEY_SEQUENCES[key]
}

export function extractJLinePrompt(
    rows: JLineRow[],
    cursorRow: number,
    cursorColumn: number,
): JLinePrompt | null {
    const cursorIndex = rows.findIndex((row) => row.index === cursorRow)
    if (cursorIndex < 0) return null
    let start = cursorIndex
    while (start > 0 && rows[start]?.wrapped) start--
    const logicalRows = rows.slice(start, cursorIndex + 1)
    const logicalText = logicalRows.map(rowText).join('')
    if (!logicalText.startsWith('> ')) return null
    const charactersBeforeCursor =
        logicalRows
            .slice(0, -1)
            .reduce(
                (total, row) => total + row.cells.reduce((sum, cell) => sum + cell.text.length, 0),
                0,
            ) +
        (logicalRows.at(-1)?.cells ?? [])
            .slice(0, cursorColumn)
            .reduce((sum, cell) => sum + cell.text.length, 0)
    const cursor = Math.max(0, charactersBeforeCursor - 2)
    const commandText = logicalText.slice(2)
    const valueEnd = Math.max(cursor, commandText.trimEnd().length)
    return {
        value: commandText.slice(0, valueEnd),
        cursor,
        rows: logicalRows,
    }
}

export function createJLineCandidateInsertion(
    prompt: Pick<JLinePrompt, 'value' | 'cursor'>,
    candidateText: string,
): JLineCandidateInsertion {
    const cursor = Math.max(0, Math.min(prompt.cursor, prompt.value.length))
    let start = cursor
    while (start > 0 && !/\s/.test(prompt.value[start - 1]!)) start--
    let end = cursor
    while (end < prompt.value.length && !/\s/.test(prompt.value[end]!)) end++
    const needsTrailingSpace = end === prompt.value.length && !candidateText.endsWith(' ')
    return {
        deleteBefore: cursor - start,
        deleteAfter: end - cursor,
        text: `${candidateText}${needsTrailingSpace ? ' ' : ''}`,
    }
}

export function applyJLineCandidate(
    prompt: Pick<JLinePrompt, 'value' | 'cursor'>,
    candidateText: string,
): JLineInputEdit {
    const insertion = createJLineCandidateInsertion(prompt, candidateText)
    const start = prompt.cursor - insertion.deleteBefore
    const end = prompt.cursor + insertion.deleteAfter
    return {
        value: `${prompt.value.slice(0, start)}${insertion.text}${prompt.value.slice(end)}`,
        cursor: start + insertion.text.length,
    }
}

export function replaceJLineSelection(
    prompt: Pick<JLinePrompt, 'value'>,
    selectionStart: number,
    selectionEnd: number,
    replacement: string,
): JLineInputEdit {
    const start = Math.max(0, Math.min(selectionStart, selectionEnd, prompt.value.length))
    const end = Math.max(
        start,
        Math.min(Math.max(selectionStart, selectionEnd), prompt.value.length),
    )
    return {
        value: `${prompt.value.slice(0, start)}${replacement}${prompt.value.slice(end)}`,
        cursor: start + replacement.length,
    }
}

export function createJLineLineReplacementSequence(
    currentValue: string,
    next: JLineInputEdit,
    applicationCursorKeys = false,
): string {
    const cursor = Math.max(0, Math.min(next.cursor, next.value.length))
    return (
        (currentValue ? '\x01\x0b' : '') +
        next.value +
        jlineKeySequence('ArrowLeft', applicationCursorKeys)!.repeat(next.value.length - cursor)
    )
}

export function parseJLineCandidates(rows: JLineRow[], cursorRow: number): JLineCandidate[] {
    const candidates: JLineCandidate[] = []
    for (const row of rows) {
        if (row.index <= cursorRow) continue
        const text = rowText(row)
        if (!text.trim()) continue
        if (/--more--|\.\.\.$/i.test(text.trim())) return []

        let start = 0
        while (start < row.cells.length) {
            while (start < row.cells.length && !row.cells[start]?.text.trim()) start++
            if (start >= row.cells.length) break
            let end = start
            let blankRun = 0
            while (end < row.cells.length) {
                if (row.cells[end]?.text.trim()) {
                    blankRun = 0
                } else {
                    blankRun++
                    if (blankRun >= 2) break
                }
                end++
            }
            const contentEnd = Math.max(start, end - blankRun + 1)
            const candidateText = row.cells
                .slice(start, contentEnd)
                .map((cell) => cell.text)
                .join('')
                .trim()
            if (candidateText) {
                const first = row.cells[start]!
                candidates.push({
                    text: candidateText,
                    row: row.index,
                    column: start,
                    selected: row.cells.slice(start, contentEnd).some((cell) => cell.style.inverse),
                    style: first.style,
                })
            }
            start = end + 1
        }
    }
    return candidates
}

export function parseJLineCandidateConfirmation(
    rows: JLineRow[],
): JLineCandidateConfirmation | null {
    const text = rows.map(rowText).join(' ').replace(/\s+/g, ' ').trim()
    const match = text.match(
        /(?:do you wish to see all|display all)\s+\d+\s+possibilit(?:y|ies)\b.*?\((\d+)\s+lines?\)\?\s*$/i,
    )
    if (!match) return null
    const lineCount = Number.parseInt(match[1] ?? '', 10)
    return { lineCount: Number.isFinite(lineCount) ? lineCount : 0 }
}

function rowText(row: JLineRow): string {
    return row.cells.map((cell) => cell.text).join('')
}
