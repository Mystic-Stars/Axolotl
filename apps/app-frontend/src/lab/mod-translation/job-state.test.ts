import assert from 'node:assert/strict'
import test from 'node:test'

import {
    countModTranslationJobs,
    jobFromSnapshot,
    modTranslationPercent,
    reduceModTranslationJob,
    replayContiguousTaskEvents,
} from './job-state.ts'
import { mapTaskEventsToTimeline } from './timeline.ts'
import type {
    ModTranslationProgress,
    ModTranslationTaskEvent,
    ModTranslationTaskSnapshot,
} from './types.ts'

function progress(overrides: Partial<ModTranslationProgress> = {}): ModTranslationProgress {
    return {
        taskId: 'TASK-1',
        phase: 'language',
        message: 'translated',
        completed: 10,
        total: 100,
        weightVerified: 10,
        weightTotal: 100,
        level: 'info',
        finished: false,
        ok: false,
        ...overrides,
    }
}

function event(
    sequence: number,
    overrides: Partial<ModTranslationTaskEvent> = {},
): ModTranslationTaskEvent {
    return {
        eventId: `EVENT-${sequence}`,
        taskId: 'TASK-1',
        sequence,
        occurredAt: `2026-08-07T00:00:0${sequence}Z`,
        eventType: 'progress',
        status: 'running',
        progress: progress(),
        ...overrides,
    }
}

function snapshot(events: ModTranslationTaskEvent[] = []): ModTranslationTaskSnapshot {
    return {
        taskId: 'TASK-1',
        inputPath: 'C:/mods/demo.jar',
        outputPath: 'C:/mods/demo-zh_cn.jar',
        inputHash: 'abc',
        startedAt: '2026-08-07T00:00:00Z',
        updatedAt: '2026-08-07T00:00:00Z',
        status: 'running',
        sequence: events.at(-1)?.sequence ?? 0,
        activities: [],
        events,
    }
}

test('percent uses verified weight and forces successful completion to 100', () => {
    assert.equal(modTranslationPercent(progress({ weightVerified: 50, weightTotal: 100 })), 50)
    assert.equal(modTranslationPercent(progress({ weightVerified: 100, weightTotal: 100 })), 99)
    assert.equal(
        modTranslationPercent(
            progress({ weightVerified: 20, weightTotal: 100, finished: true, ok: true }),
        ),
        100,
    )
})

test('snapshot replays ordered events', () => {
    const job = jobFromSnapshot(
        snapshot([
            event(1, { progress: progress({ phase: 'prepare', message: 'prepare' }) }),
            event(2, { progress: progress({ phase: 'language', message: 'language' }) }),
        ]),
    )
    assert.equal(job.lastSequence, 2)
    assert.equal(job.phase, 'language')
    assert.equal(job.timeline.length, 2)
})

test('duplicate and stale events are ignored', () => {
    const initial = jobFromSnapshot(snapshot([event(1)]))
    assert.strictEqual(reduceModTranslationJob(initial, event(1)), initial)
    assert.strictEqual(reduceModTranslationJob(initial, event(0)), initial)
})

test('out-of-order events wait for the missing sequence and then replay in order', () => {
    const initial = jobFromSnapshot(snapshot())
    const first = replayContiguousTaskEvents(initial, [
        event(2, { progress: progress({ message: 'second' }) }),
    ])
    assert.equal(first.job.lastSequence, 0)
    assert.equal(first.pending.length, 1)
    const second = replayContiguousTaskEvents(first.job, [
        ...first.pending,
        event(1, { progress: progress({ message: 'first' }) }),
    ])
    assert.equal(second.job.lastSequence, 2)
    assert.equal(second.job.message, 'second')
    assert.deepEqual(second.pending, [])
})

test('phase never regresses', () => {
    const initial = jobFromSnapshot(
        snapshot([event(1, { progress: progress({ phase: 'validation' }) })]),
    )
    const next = reduceModTranslationJob(
        initial,
        event(2, { progress: progress({ phase: 'language' }) }),
    )
    assert.equal(next.phase, 'validation')
})

test('zero-weight phase events preserve the last measurable progress', () => {
    const initial = jobFromSnapshot(
        snapshot([
            event(1, {
                progress: progress({ completed: 6, total: 21, weightVerified: 6, weightTotal: 25 }),
            }),
        ]),
    )
    const next = reduceModTranslationJob(
        initial,
        event(2, {
            progress: progress({
                phase: 'repair',
                message: 'starting validation',
                completed: 0,
                total: 0,
                weightVerified: 0,
                weightTotal: 0,
            }),
        }),
    )
    assert.equal(next.percent, 24)
    assert.equal(next.completed, 6)
    assert.equal(next.total, 21)
    assert.equal(next.weightVerified, 6)
    assert.equal(next.weightTotal, 25)
})

test('job counts distinguish failed tasks from successful completion', () => {
    assert.deepEqual(
        countModTranslationJobs([
            { status: 'completed' },
            { status: 'failed' },
            { status: 'running' },
        ]),
        { running: 1, completed: 1, failed: 1 },
    )
})

test('failed packaging signal preserves the actual working phase and error', () => {
    const initial = jobFromSnapshot(
        snapshot([event(1, { progress: progress({ phase: 'repair' }) })]),
    )
    const next = reduceModTranslationJob(
        initial,
        event(2, {
            status: 'failed',
            progress: progress({ phase: 'packaging', finished: true, level: 'error' }),
            error: { code: 'UNSUPPORTED_RESOURCE', message: 'unsupported path' },
        }),
    )
    assert.equal(next.phase, 'repair')
    assert.equal(next.error?.code, 'UNSUPPORTED_RESOURCE')
})

test('successful task is always shown as 100 percent', () => {
    const next = reduceModTranslationJob(
        jobFromSnapshot(snapshot()),
        event(1, {
            status: 'completed',
            progress: progress({ finished: true, ok: true, weightVerified: 1, weightTotal: 10 }),
        }),
    )
    assert.equal(next.percent, 100)
})

test('timeline groups typed repair activity without exposing raw JSON as the title', () => {
    const entries = mapTaskEventsToTimeline([
        event(1, {
            eventType: 'activity',
            progress: null,
            activity: {
                taskId: 'TASK-1',
                pass: 1,
                kind: 'diagnosis',
                status: 'running',
                title: '发现 16 个疑难项',
                summary: '正在批量处理',
                count: 16,
                issueIds: ['a', 'b'],
                debug: { request: { entries: [] } },
            },
        }),
    ])
    assert.equal(entries[0].title, '发现 16 个疑难项')
    assert.equal(entries[0].pass, 1)
    assert.deepEqual(entries[0].debug, { request: { entries: [] } })
})
