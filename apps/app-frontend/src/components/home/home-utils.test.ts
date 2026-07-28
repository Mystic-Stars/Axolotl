import assert from 'node:assert/strict'
import test from 'node:test'

import {
	buildHeatmapDays,
	getActivePlayerName,
	getPlaytimeLevel,
	getTimeBucket,
	stableGreetingIndex,
	toDateKey,
} from './home-utils.ts'

test('uses the six local greeting time buckets', () => {
	const hour = (value: number) => new Date(2026, 6, 25, value, 0)
	assert.equal(getTimeBucket(hour(0)), 'late-night')
	assert.equal(getTimeBucket(hour(5)), 'dawn')
	assert.equal(getTimeBucket(hour(8)), 'morning')
	assert.equal(getTimeBucket(hour(12)), 'afternoon')
	assert.equal(getTimeBucket(hour(17)), 'evening')
	assert.equal(getTimeBucket(hour(21)), 'night')
	assert.equal(
		stableGreetingIndex('2026-07-25:morning', 16),
		stableGreetingIndex('2026-07-25:morning', 16),
	)
	assert.equal(stableGreetingIndex('any', 0), 0)
})

test('builds Monday-first month and year heatmap grids', () => {
	const month = buildHeatmapDays(new Date(2026, 1, 17, 12), 'month')
	assert.equal(month[0]?.date.getDay(), 1)
	assert.equal(month.at(-1)?.date.getDay(), 0)
	assert.equal(month.filter((day) => day.inPeriod).length, 28)
	assert.equal(month.find((day) => day.inPeriod)?.dateKey, '2026-02-01')

	const year = buildHeatmapDays(new Date(2024, 6, 1, 12), 'year')
	assert.equal(year[0]?.date.getDay(), 1)
	assert.equal(year.at(-1)?.date.getDay(), 0)
	assert.equal(year.filter((day) => day.inPeriod).length, 366)
})

test('maps playtime thresholds and missing days deterministically', () => {
	assert.deepEqual(
		[0, 1, 30 * 60, 30 * 60 + 1, 90 * 60, 90 * 60 + 1, 180 * 60, 180 * 60 + 1].map(
			getPlaytimeLevel,
		),
		[0, 1, 1, 2, 2, 3, 3, 4],
	)
	assert.equal(toDateKey(new Date(2026, 6, 25, 12)), '2026-07-25')
})

test('uses only active online accounts for player greetings', () => {
	const accounts = [
		{ account_type: 'offline', profile: { id: 'offline', name: 'Local player' } },
		{ account_type: 'microsoft', profile: { id: 'microsoft', name: 'Alex' } },
		{ account_type: 'yggdrasil', profile: { id: 'yggdrasil', name: 'Steve' } },
	]
	assert.equal(getActivePlayerName('microsoft', accounts), 'Alex')
	assert.equal(getActivePlayerName('yggdrasil', accounts), 'Steve')
	assert.equal(getActivePlayerName('offline', accounts), null)
	assert.equal(getActivePlayerName(undefined, accounts), null)
	assert.equal(getActivePlayerName('missing', accounts), null)
})
