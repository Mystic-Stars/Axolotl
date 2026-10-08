import { defineStore } from 'pinia'

import {
    cancelModTranslation,
    dismissModTranslationTask,
    listenToModTranslationTasks,
    listModTranslationTasks,
    translateMod,
} from '@/lab/mod-translation/backend'
import {
    mergeTaskSnapshot,
    reduceModTranslationJob,
    replayContiguousTaskEvents,
} from '@/lab/mod-translation/job-state'
import type {
    ModTranslationAnalysis,
    ModTranslationJob,
    ModTranslationOptions,
    ModTranslationTaskEvent,
    ModTranslationTaskSnapshot,
} from '@/lab/mod-translation/types.ts'

let unlistenTasks: (() => void) | undefined
let initPromise: Promise<void> | undefined
const earlyEvents = new Map<string, ModTranslationTaskEvent[]>()

async function ensureModTranslationRuntime(): Promise<void> {
    if (initPromise) return await initPromise
    initPromise = (async () => {
        unlistenTasks = await listenToModTranslationTasks((event) => {
            useModTranslationStore().applyTaskEvent(event)
        })
        const snapshots = await listModTranslationTasks()
        useModTranslationStore().restoreSnapshots(snapshots)
    })().catch((error) => {
        unlistenTasks?.()
        unlistenTasks = undefined
        initPromise = undefined
        throw error
    })
    return await initPromise
}

export const useModTranslationStore = defineStore('modTranslation', {
    state: () => ({
        inputPath: '',
        analysis: null as ModTranslationAnalysis | null,
        analysisInputPath: '',
        providerId: '',
        modelId: '',
        options: {
            batchSize: 40,
            deepBatchSize: 24,
            generateModName: false,
            repairEnabled: true,
            classTextEnabled: false,
            maxClassBatch: 16,
        } as ModTranslationOptions,
        jobs: [] as ModTranslationJob[],
    }),
    getters: {
        activeJobs(state): ModTranslationJob[] {
            return state.jobs.filter((job) => job.status === 'running')
        },
    },
    actions: {
        async init() {
            await ensureModTranslationRuntime()
        },
        setAnalysis(analysis: ModTranslationAnalysis | null, inputPath = '') {
            this.analysis = analysis
            this.analysisInputPath = analysis ? inputPath : ''
        },
        async startTranslation() {
            if (!this.inputPath || !this.providerId || !this.modelId) return
            await ensureModTranslationRuntime()
            const outputPath = this.inputPath.replace(/\.jar$/i, '-zh_cn.jar')
            const reusableAnalysis =
                this.analysis && this.analysisInputPath === this.inputPath ? this.analysis : null
            const snapshot = await translateMod({
                inputPath: this.inputPath,
                outputPath,
                providerId: this.providerId,
                modelId: this.modelId,
                analysisId: reusableAnalysis?.analysisId,
                inputHash: reusableAnalysis?.inputHash,
                options: this.options,
            })
            this.upsertSnapshot(snapshot)
            this.setAnalysis(null)
        },
        async cancelJob(taskId: string) {
            await cancelModTranslation(taskId)
        },
        async removeJob(taskId: string) {
            await dismissModTranslationTask(taskId)
            this.jobs = this.jobs.filter((job) => job.taskId !== taskId)
        },
        restoreSnapshots(snapshots: ModTranslationTaskSnapshot[]) {
            for (const snapshot of snapshots) this.upsertSnapshot(snapshot)
        },
        upsertSnapshot(snapshot: ModTranslationTaskSnapshot) {
            const index = this.jobs.findIndex((job) => job.taskId === snapshot.taskId)
            const current = index >= 0 ? this.jobs[index] : undefined
            let job = mergeTaskSnapshot(current, snapshot)
            const replayed = replayContiguousTaskEvents(job, earlyEvents.get(snapshot.taskId) ?? [])
            job = replayed.job
            if (replayed.pending.length) earlyEvents.set(snapshot.taskId, replayed.pending)
            else earlyEvents.delete(snapshot.taskId)
            if (index >= 0) this.jobs.splice(index, 1, job)
            else this.jobs.unshift(job)
            this.jobs.sort((left, right) => right.startedAt.localeCompare(left.startedAt))
        },
        applyTaskEvent(event: ModTranslationTaskEvent) {
            const index = this.jobs.findIndex((job) => job.taskId === event.taskId)
            if (index < 0) {
                const buffered = earlyEvents.get(event.taskId) ?? []
                if (!buffered.some((candidate) => candidate.sequence === event.sequence)) {
                    buffered.push(event)
                    earlyEvents.set(event.taskId, buffered.slice(-400))
                }
                return
            }
            let job = this.jobs[index]
            if (event.sequence <= job.lastSequence) return
            if (event.sequence > job.lastSequence + 1) {
                const buffered = earlyEvents.get(event.taskId) ?? []
                if (!buffered.some((candidate) => candidate.sequence === event.sequence)) {
                    buffered.push(event)
                    earlyEvents.set(event.taskId, buffered.slice(-400))
                }
                return
            }
            job = reduceModTranslationJob(job, event)
            const replayed = replayContiguousTaskEvents(job, earlyEvents.get(event.taskId) ?? [])
            job = replayed.job
            if (replayed.pending.length) earlyEvents.set(event.taskId, replayed.pending)
            else earlyEvents.delete(event.taskId)
            this.jobs.splice(index, 1, job)
        },
    },
})
