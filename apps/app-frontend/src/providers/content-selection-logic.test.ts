import assert from 'node:assert/strict'
import test from 'node:test'

import {
	aggregateContentSelectionDependencies,
	getActiveDependencyConflictIdentities,
} from './content-selection-logic.ts'

function dependency(id: string, requiredBy: string, ownerKey: string) {
	return {
		id,
		title: `Dependency ${id}`,
		requiredBy: [requiredBy],
		requiredByKeys: [ownerKey],
		alreadyInstalled: false,
	}
}

test('deduplicates a shared dependency and merges its owners', () => {
	const result = aggregateContentSelectionDependencies(
		[
			{
				ownerKey: 'modrinth:first',
				dependencies: [dependency('modrinth:shared:v1', 'First', 'modrinth:first')],
			},
			{
				ownerKey: 'curseforge:second',
				dependencies: [dependency('modrinth:shared:v1', 'Second', 'curseforge:second')],
			},
		],
		(item) => `Conflict: ${item.title}`,
	)

	assert.equal(result.dependencies.length, 1)
	assert.deepEqual(result.dependencies[0].requiredBy, ['First', 'Second'])
	assert.deepEqual(result.dependencies[0].requiredByKeys, [
		'modrinth:first',
		'curseforge:second',
	])
	assert.equal(result.conflicts.size, 0)
})

test('marks every affected primary when one dependency resolves to different versions', () => {
	const result = aggregateContentSelectionDependencies(
		[
			{
				ownerKey: 'modrinth:first',
				dependencies: [dependency('modrinth:shared:v1', 'First', 'modrinth:first')],
			},
			{
				ownerKey: 'modrinth:second',
				dependencies: [dependency('modrinth:shared:v2', 'Second', 'modrinth:second')],
			},
		],
		(item) => `Conflict: ${item.title}`,
	)

	assert.equal(result.dependencies.length, 2)
	assert.match(result.conflicts.get('modrinth:first') ?? '', /^Conflict:/)
	assert.match(result.conflicts.get('modrinth:second') ?? '', /^Conflict:/)
	assert.deepEqual(result.conflictIdentities.get('modrinth:first'), ['modrinth:shared'])
	assert.deepEqual(result.conflictIdentities.get('modrinth:second'), ['modrinth:shared'])
})

test('keeps provider-qualified dependencies separate', () => {
	const result = aggregateContentSelectionDependencies(
		[
			{
				ownerKey: 'primary',
				dependencies: [
					dependency('modrinth:42:v1', 'Primary', 'primary'),
					dependency('curseforge:42:v1', 'Primary', 'primary'),
				],
			},
		],
		(item) => `Conflict: ${item.title}`,
	)

	assert.equal(result.dependencies.length, 2)
	assert.equal(result.conflicts.size, 0)
})

test('merges required owners for a shared dependency', () => {
	const optional = dependency('modrinth:shared:v1', 'First', 'modrinth:first')
	const required = {
		...dependency('modrinth:shared:v1', 'Second', 'curseforge:second'),
		required: true,
	}
	const result = aggregateContentSelectionDependencies(
		[
			{ ownerKey: 'modrinth:first', dependencies: [optional] },
			{ ownerKey: 'curseforge:second', dependencies: [required] },
		],
		(item) => `Conflict: ${item.title}`,
	)

	assert.equal(result.dependencies[0].required, true)
	assert.deepEqual(result.dependencies[0].requiredForKeys, ['curseforge:second'])
})

test('removing one conflicting owner clears the active conflict', () => {
	const dependencies = [
		dependency('modrinth:shared:v1', 'First', 'modrinth:first'),
		dependency('modrinth:shared:v2', 'Second', 'modrinth:second'),
	]

	assert.deepEqual(
		[...getActiveDependencyConflictIdentities(dependencies, new Set(['modrinth:first', 'modrinth:second']))],
		['modrinth:shared'],
	)
	assert.equal(
		getActiveDependencyConflictIdentities(dependencies, new Set(['modrinth:second'])).size,
		0,
	)
})
