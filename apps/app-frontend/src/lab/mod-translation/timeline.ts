import type { ModTranslationTaskEvent, ModTranslationTimelineEntry } from './types.ts'

export function mapTaskEventsToTimeline(
    events: ModTranslationTaskEvent[],
): ModTranslationTimelineEntry[] {
    const timeline: ModTranslationTimelineEntry[] = []
    for (const event of [...events].sort((left, right) => left.sequence - right.sequence)) {
        if (event.activity) {
            timeline.push({
                id: event.eventId,
                sequence: event.sequence,
                time: event.occurredAt,
                phase: 'repair',
                pass: event.activity.pass,
                kind: event.activity.kind,
                status: event.activity.status,
                title: event.activity.title,
                summary: event.activity.summary,
                count: event.activity.count,
                issueIds: event.activity.issueIds,
                debug: event.activity.debug,
            })
            continue
        }
        const progress = event.progress
        if (!progress?.message) continue
        const previous = timeline[timeline.length - 1]
        if (
            previous?.kind === 'progress' &&
            previous.phase === progress.phase &&
            previous.title === progress.message &&
            progress.level === 'info'
        ) {
            continue
        }
        timeline.push({
            id: event.eventId,
            sequence: event.sequence,
            time: event.occurredAt,
            phase: progress.phase,
            kind: 'progress',
            status: progress.finished ? (progress.ok ? 'success' : 'error') : progress.level,
            title: progress.message,
            issueIds: [],
        })
    }
    return timeline.slice(-200)
}

export function groupTimelineByRepairPass(entries: ModTranslationTimelineEntry[]) {
    const groups: Array<{
        id: string
        pass?: number
        entries: ModTranslationTimelineEntry[]
    }> = []
    for (const entry of entries) {
        const id = entry.pass ? `pass-${entry.pass}` : `phase-${entry.phase}`
        const last = groups[groups.length - 1]
        if (last?.id === id) last.entries.push(entry)
        else groups.push({ id, pass: entry.pass, entries: [entry] })
    }
    return groups
}
