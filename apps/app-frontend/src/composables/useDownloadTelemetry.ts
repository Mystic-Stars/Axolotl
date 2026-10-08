import { onScopeDispose, type Ref, shallowRef } from 'vue'

import type { InstallJobSnapshot } from '@/helpers/install'

type DownloadTelemetry = Pick<
    InstallJobSnapshot['summary'],
    'speed_bytes_per_second' | 'eta_seconds'
>

type TelemetryJob = Pick<InstallJobSnapshot, 'job_id' | 'status'> & {
    summary: DownloadTelemetry
}

/** Samples display metrics without throttling download progress or scheduling. */
export function useDownloadTelemetry(jobs: Readonly<Ref<TelemetryJob[]>>) {
    function sample() {
        return new Map<string, DownloadTelemetry>(
            jobs.value
                .filter((job) => job.status === 'running')
                .map((job) => [
                    job.job_id,
                    {
                        speed_bytes_per_second: job.summary.speed_bytes_per_second,
                        eta_seconds: job.summary.eta_seconds,
                    },
                ]),
        )
    }

    const telemetry = shallowRef(sample())
    const timer = setInterval(() => {
        telemetry.value = sample()
    }, 1000)
    onScopeDispose(() => clearInterval(timer))

    return (job?: TelemetryJob): DownloadTelemetry | undefined => {
        if (!job || job.status !== 'running') return undefined
        const sampled = telemetry.value.get(job.job_id)
        if (!sampled) return undefined
        return {
            speed_bytes_per_second:
                job.summary.speed_bytes_per_second == null ? null : sampled.speed_bytes_per_second,
            eta_seconds: job.summary.eta_seconds == null ? null : sampled.eta_seconds,
        }
    }
}
