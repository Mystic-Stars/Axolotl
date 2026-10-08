import { mapTaskEventsToTimeline } from './timeline.ts'
import type {
    ModTranslationJob,
    ModTranslationPhase,
    ModTranslationProgress,
    ModTranslationTaskEvent,
    ModTranslationTaskSnapshot,
} from './types.ts'

export const MOD_TRANSLATION_PHASES: readonly ModTranslationPhase[] = [
    'prepare',
    'research',
    'language',
    'repair',
    'class',
    'validation',
    'packaging',
]

export function phaseIndex(phase: ModTranslationPhase): number {
    return MOD_TRANSLATION_PHASES.indexOf(phase)
}

export function countModTranslationJobs(jobs: ReadonlyArray<Pick<ModTranslationJob, 'status'>>): {
    running: number
    completed: number
    failed: number
} {
    return jobs.reduce(
        (counts, job) => {
            counts[job.status] += 1
            return counts
        },
        { running: 0, completed: 0, failed: 0 },
    )
}

export function modTranslationPercent(
    progress: Pick<ModTranslationProgress, 'weightVerified' | 'weightTotal' | 'finished' | 'ok'>,
): number {
    if (progress.finished && progress.ok) return 100
    if (progress.weightTotal <= 0) return 0
    const measured = Math.round(Math.min(1, progress.weightVerified / progress.weightTotal) * 100)
    return progress.finished ? measured : Math.min(99, measured)
}

function monotonicPhase(current: ModTranslationPhase, incoming: ModTranslationPhase) {
    return phaseIndex(incoming) >= phaseIndex(current) ? incoming : current
}

function applyProgress(
    job: ModTranslationJob,
    progress: ModTranslationProgress,
): ModTranslationJob {
    const failedPackagingSignal =
        progress.finished && !progress.ok && progress.phase === 'packaging'
    const phase = failedPackagingSignal ? job.phase : monotonicPhase(job.phase, progress.phase)
    const hasWorkWeight = progress.weightTotal > 0
    const hasItemCount = progress.total > 0
    const weightVerified = hasWorkWeight ? progress.weightVerified : job.weightVerified
    const weightTotal = hasWorkWeight ? progress.weightTotal : job.weightTotal
    return {
        ...job,
        phase,
        message: progress.message,
        percent: modTranslationPercent({
            weightVerified,
            weightTotal,
            finished: progress.finished,
            ok: progress.ok,
        }),
        completed: hasItemCount ? progress.completed : job.completed,
        total: hasItemCount ? progress.total : job.total,
        weightVerified,
        weightTotal,
        sample: progress.sample ?? job.sample,
        level: progress.level,
    }
}

export function jobFromSnapshot(snapshot: ModTranslationTaskSnapshot): ModTranslationJob {
    const base: ModTranslationJob = {
        taskId: snapshot.taskId,
        inputPath: snapshot.inputPath,
        outputPath: snapshot.outputPath,
        inputHash: snapshot.inputHash,
        startedAt: snapshot.startedAt,
        updatedAt: snapshot.updatedAt,
        status: snapshot.status,
        lastSequence: 0,
        phase: 'prepare',
        message: '',
        percent: 0,
        completed: 0,
        total: 0,
        weightVerified: 0,
        weightTotal: 0,
        level: 'info',
        events: [],
        timeline: [],
        report: snapshot.report ?? undefined,
        error: snapshot.error ?? undefined,
    }
    const events = [...snapshot.events].sort((left, right) => left.sequence - right.sequence)
    let job = base
    for (const event of events) job = reduceModTranslationJob(job, event)
    if (!events.length && snapshot.progress) job = applyProgress(job, snapshot.progress)
    job.status = snapshot.status
    job.updatedAt = snapshot.updatedAt
    job.lastSequence = Math.max(job.lastSequence, snapshot.sequence)
    job.report = snapshot.report ?? job.report
    job.error = snapshot.error ?? job.error
    if (job.status === 'completed') job.percent = 100
    return job
}

export function reduceModTranslationJob(
    job: ModTranslationJob,
    event: ModTranslationTaskEvent,
): ModTranslationJob {
    if (event.sequence <= job.lastSequence) return job
    let next: ModTranslationJob = {
        ...job,
        status: event.status,
        updatedAt: event.occurredAt,
        lastSequence: event.sequence,
        events: [...job.events, event]
            .sort((left, right) => left.sequence - right.sequence)
            .slice(-400),
        report: event.report ?? job.report,
        error: event.error ?? job.error,
    }
    if (event.progress) next = applyProgress(next, event.progress)
    if (next.status === 'completed') next.percent = 100
    next.timeline = mapTaskEventsToTimeline(next.events)
    return next
}

export function replayContiguousTaskEvents(
    job: ModTranslationJob,
    events: ModTranslationTaskEvent[],
): { job: ModTranslationJob; pending: ModTranslationTaskEvent[] } {
    let next = job
    const pending: ModTranslationTaskEvent[] = []
    for (const event of [...events].sort((left, right) => left.sequence - right.sequence)) {
        if (event.sequence <= next.lastSequence) continue
        if (event.sequence === next.lastSequence + 1) next = reduceModTranslationJob(next, event)
        else pending.push(event)
    }
    return { job: next, pending }
}

export function mergeTaskSnapshot(
    job: ModTranslationJob | undefined,
    snapshot: ModTranslationTaskSnapshot,
): ModTranslationJob {
    if (!job) return jobFromSnapshot(snapshot)
    let next = job
    for (const event of [...snapshot.events].sort(
        (left, right) => left.sequence - right.sequence,
    )) {
        next = reduceModTranslationJob(next, event)
    }
    return {
        ...next,
        status: snapshot.status,
        updatedAt: snapshot.updatedAt,
        report: snapshot.report ?? next.report,
        error: snapshot.error ?? next.error,
        percent: snapshot.status === 'completed' ? 100 : next.percent,
    }
}
