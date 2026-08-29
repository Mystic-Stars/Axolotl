import assert from 'node:assert/strict'
import test from 'node:test'

import {
	pruneContentFilterSelections,
	pruneMetadataFilterSelections,
} from './content-filter-state.ts'

test('keeps content filters while options are still loading', () => {
	assert.deepEqual(
		pruneContentFilterSelections(
			{ typeFilters: ['mod'], statusFilters: ['disabled'] },
			{ type: [], status: [] },
			false,
		),
		{ typeFilters: ['mod'], statusFilters: ['disabled'] },
	)
})

test('prunes content filters only after the option set is ready', () => {
	assert.deepEqual(
		pruneContentFilterSelections(
			{ typeFilters: ['mod', 'shader'], statusFilters: ['disabled', 'updates'] },
			{ type: ['mod'], status: ['disabled'] },
			true,
		),
		{ typeFilters: ['mod'], statusFilters: ['disabled'] },
	)
})

test('keeps valid filters when the current search has no matches', () => {
	assert.deepEqual(
		pruneContentFilterSelections(
			{ typeFilters: ['mod'], statusFilters: ['disabled'] },
			{ type: ['mod', 'shader'], status: ['enabled', 'disabled'] },
			true,
		),
		{ typeFilters: ['mod'], statusFilters: ['disabled'] },
	)
})

test('keeps the duplicate filter when sorting changes the available type options', () => {
	assert.deepEqual(
		pruneContentFilterSelections(
			{ typeFilters: ['duplicates'], statusFilters: [] },
			{ type: ['mod', 'duplicates'], status: [] },
			true,
		),
		{ typeFilters: ['duplicates'], statusFilters: [] },
	)
})

test('keeps metadata exclusions through an empty loading state', () => {
	const selections = { state: ['enabled'], loader: ['forge'] }
	assert.deepEqual(pruneMetadataFilterSelections(selections, [], false), selections)
	assert.deepEqual(
		pruneMetadataFilterSelections(
			selections,
			[
				{ key: 'state', options: [{ value: 'enabled' }, { value: 'disabled' }] },
				{ key: 'loader', options: [{ value: 'fabric' }] },
			],
			true,
		),
		{ state: ['enabled'] },
	)
})

test('keeps a metadata exclusion that remains valid but is not displayed as a filter option', () => {
	assert.deepEqual(
		pruneMetadataFilterSelections(
			{ state: ['disabled'] },
			[{ key: 'state', options: [{ value: 'disabled' }] }],
			true,
		),
		{ state: ['disabled'] },
	)
})
