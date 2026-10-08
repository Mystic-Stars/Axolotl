function nonEmptyLines(content: string): string[] {
    return content.split(/\r\n|\r|\n/).filter(Boolean)
}

function overlapLength(history: readonly string[], liveBuffer: readonly string[]): number {
    const maximum = Math.min(history.length, liveBuffer.length)

    for (let length = maximum; length > 0; length--) {
        let overlaps = true
        for (let index = 0; index < length; index++) {
            if (history[history.length - length + index] !== liveBuffer[index]) {
                overlaps = false
                break
            }
        }
        if (overlaps) return length
    }

    return 0
}

/**
 * Combines the persisted Minecraft log with output that may not have reached
 * latest.log yet. The ring buffer commonly repeats the tail of latest.log, so
 * retain only its non-overlapping suffix.
 */
export function mergeLiveLogHistory(latestLog: string, liveBuffer: string): string {
    const history = nonEmptyLines(latestLog)
    const buffer = nonEmptyLines(liveBuffer)
    return [...history, ...buffer.slice(overlapLength(history, buffer))].join('\n')
}
