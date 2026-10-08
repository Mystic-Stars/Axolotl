import assert from 'node:assert/strict'
import test from 'node:test'

import { effectScope, ref } from 'vue'

import type { InstallJobStatus } from '../helpers/install.ts'
import { useDownloadTelemetry } from './useDownloadTelemetry.ts'

function job(id = 'job') {
    return {
        job_id: id,
        status: 'running' as InstallJobStatus,
        summary: {
            bytes_downloaded: 0,
            speed_bytes_per_second: 10 as number | null,
            eta_seconds: 100 as number | null,
        },
    }
}

test('samples the latest speed and ETA once per second while progress remains live', (t) => {
    t.mock.timers.enable({ apis: ['setInterval'] })
    const scope = effectScope()
    t.after(() => scope.stop())
    const jobs = ref([job(), job('second')])
    const display = scope.run(() => useDownloadTelemetry(jobs))!

    for (let tick = 1; tick <= 9; tick++) {
        t.mock.timers.tick(100)
        jobs.value[0].summary.bytes_downloaded = tick * 100
        jobs.value[0].summary.speed_bytes_per_second = tick * 5
        jobs.value[0].summary.eta_seconds = 100 - tick
        assert.equal(display(jobs.value[0])?.speed_bytes_per_second, 10)
        assert.equal(display(jobs.value[0])?.eta_seconds, 100)
        assert.equal(jobs.value[0].summary.bytes_downloaded, tick * 100)
    }
    t.mock.timers.tick(100)
    assert.equal(display(jobs.value[0])?.speed_bytes_per_second, 45)
    assert.equal(display(jobs.value[0])?.eta_seconds, 91)
    assert.equal(display(jobs.value[1])?.speed_bytes_per_second, 10)
    jobs.value[0].summary.speed_bytes_per_second = 50
    t.mock.timers.tick(999)
    assert.equal(display(jobs.value[0])?.speed_bytes_per_second, 45)
    t.mock.timers.tick(1)
    assert.equal(display(jobs.value[0])?.speed_bytes_per_second, 50)

    scope.stop()
    jobs.value[0].summary.speed_bytes_per_second = 100
    t.mock.timers.tick(1000)
    assert.equal(display(jobs.value[0])?.speed_bytes_per_second, 50)
})

test('clears unavailable metrics and completed jobs without waiting for the next sample', (t) => {
    t.mock.timers.enable({ apis: ['setInterval'] })
    const scope = effectScope()
    t.after(() => scope.stop())
    const jobs = ref([job()])
    const display = scope.run(() => useDownloadTelemetry(jobs))!

    jobs.value[0].summary.speed_bytes_per_second = null
    jobs.value[0].summary.eta_seconds = null
    assert.deepEqual(display(jobs.value[0]), {
        speed_bytes_per_second: null,
        eta_seconds: null,
    })
    jobs.value[0].status = 'succeeded'
    assert.equal(display(jobs.value[0]), undefined)
    jobs.value = [job('new')]
    assert.equal(display(jobs.value[0]), undefined)
    t.mock.timers.tick(1000)
    assert.equal(display(jobs.value[0])?.speed_bytes_per_second, 10)
    assert.equal(display(), undefined)
})
