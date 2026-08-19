import assert from 'node:assert/strict'
import test from 'node:test'

import {
	buildGcCandidateChain,
	detectGcStrategy,
	GC_STRATEGY_DEFINITIONS,
} from './strategies.ts'

function createContext(overrides: Partial<Parameters<typeof buildGcCandidateChain>[0]> = {}) {
	return {
		javaMajorVersion: 21,
		allocatedMemoryMb: 16384,
		systemCpuCores: 16,
		systemLogicalProcessors: 16,
		modCount: 100,
		loader: 'forge',
		...overrides,
	}
}

test('Mojang G1GC args include official launcher parameters', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-mojang'].buildArgs()
	assert.ok(args.includes('-XX:+UseG1GC'))
	assert.ok(args.includes('-XX:MaxGCPauseMillis=200'))
	assert.ok(args.includes('-XX:G1MixedGCCountTarget=4'))
	assert.ok(args.includes('-XX:SurvivorRatio=8'))
	assert.ok(!args.includes('-XX:G1UncommitBias=1'))
})

test('PCL args are Shenandoah-adaptive without large pages', () => {
	const args = GC_STRATEGY_DEFINITIONS.pcl.buildArgs()
	assert.ok(args.includes('-XX:+UseShenandoahGC'))
	assert.ok(args.includes('-XX:ShenandoahGCHeuristics=adaptive'))
	assert.ok(!args.includes('-XX:+UseLargePages'))
})

test('Shenandoah args include large pages', () => {
	const args = GC_STRATEGY_DEFINITIONS.shenandoah.buildArgs()
	assert.ok(args.includes('-XX:+UseShenandoahGC'))
	assert.ok(args.includes('-XX:ShenandoahGCHeuristics=adaptive'))
	assert.ok(args.includes('-XX:+UseLargePages'))
})

test('ZGC args include -XX:+ZGenerational for Java 21+', () => {
	const args = GC_STRATEGY_DEFINITIONS.zgc.buildArgs(createContext({ javaMajorVersion: 21 }))
	assert.ok(args.includes('-XX:+UseZGC'))
	assert.ok(args.includes('-XX:+ZGenerational'))
	assert.ok(args.includes('-XX:+AlwaysPreTouch'))
	assert.ok(args.includes('-XX:-ZUncommit'))
})

test('ZGC args do not include -XX:+ZGenerational for Java < 21', () => {
	const args = GC_STRATEGY_DEFINITIONS.zgc.buildArgs(createContext({ javaMajorVersion: 17 }))
	assert.ok(args.includes('-XX:+UseZGC'))
	assert.ok(!args.includes('-XX:+ZGenerational'))
})

test('detectGcStrategy correctly identifies Mojang G1GC', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-mojang'].buildArgs()
	assert.equal(detectGcStrategy(args), 'g1gc-mojang')
})

test('detectGcStrategy correctly identifies PCL', () => {
	const args = GC_STRATEGY_DEFINITIONS.pcl.buildArgs()
	assert.equal(detectGcStrategy(args), 'pcl')
})

test('detectGcStrategy correctly identifies Shenandoah', () => {
	const args = GC_STRATEGY_DEFINITIONS.shenandoah.buildArgs()
	assert.equal(detectGcStrategy(args), 'shenandoah')
})

test('detectGcStrategy correctly identifies ZGC', () => {
	const args = GC_STRATEGY_DEFINITIONS.zgc.buildArgs()
	assert.equal(detectGcStrategy(args), 'zgc')
})

test('detectGcStrategy returns null for unknown strategy', () => {
	assert.equal(detectGcStrategy('-Xmx4G'), null)
})

test('Mojang G1GC is not misidentified as PCL or Shenandoah', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-mojang'].buildArgs()
	const detected = detectGcStrategy(args)
	assert.equal(detected, 'g1gc-mojang')
	assert.notEqual(detected, 'pcl')
})

test('PCL is not misidentified as Mojang or Shenandoah', () => {
	const args = GC_STRATEGY_DEFINITIONS.pcl.buildArgs()
	const detected = detectGcStrategy(args)
	assert.equal(detected, 'pcl')
	assert.notEqual(detected, 'shenandoah')
	assert.notEqual(detected, 'g1gc-mojang')
})

test('Shenandoah is not misidentified as PCL', () => {
	const args = GC_STRATEGY_DEFINITIONS.shenandoah.buildArgs()
	assert.equal(detectGcStrategy(args), 'shenandoah')
})

test('a bare -XX:+UseG1GC is not treated as the full official preset', () => {
	assert.equal(detectGcStrategy('-XX:+UseG1GC'), null)
})

test('a partial ZGC arg list is not auto-tagged', () => {
	assert.equal(detectGcStrategy('-XX:+UseZGC'), null)
})

test('a partial Shenandoah arg list is not auto-tagged', () => {
	assert.equal(detectGcStrategy('-XX:+UseShenandoahGC -XX:ShenandoahGCHeuristics=adaptive'), null)
})

test('ZGC is recognized if the complete base set is present regardless of order', () => {
	const args = '-XX:-ZUncommit -XX:+AlwaysPreTouch -XX:+UseZGC'
	assert.equal(detectGcStrategy(args), 'zgc')
})

test('ZGC with -XX:+ZGenerational on top of the base set is still ZGC', () => {
	const args = GC_STRATEGY_DEFINITIONS.zgc.buildArgs(createContext({ javaMajorVersion: 21 }))
	assert.equal(detectGcStrategy(args), 'zgc')
})

test('PCL complete set with large pages added is Shenandoah, not PCL', () => {
	const args = GC_STRATEGY_DEFINITIONS.pcl.buildArgs() + ' -XX:+UseLargePages'
	assert.equal(detectGcStrategy(args), 'shenandoah')
})

test('buildGcCandidateChain puts preferred first, dedupes, ends at minimal G1', () => {
	const { ids, args } = buildGcCandidateChain(createContext(), 'zgc')
	assert.deepEqual(ids, ['zgc', 'shenandoah', 'pcl', 'g1gc-mojang', 'minimal-g1'])
	assert.equal(args.length, ids.length)
	assert.deepEqual(args[args.length - 1], ['-XX:+UseG1GC'])
})

test('buildGcCandidateChain starts at a non-ZGC preferred strategy', () => {
	const { ids } = buildGcCandidateChain(createContext(), 'shenandoah')
	assert.deepEqual(ids, ['shenandoah', 'pcl', 'g1gc-mojang', 'minimal-g1'])
})

test('buildGcCandidateChain for PCL only falls back to G1', () => {
	const { ids } = buildGcCandidateChain(createContext(), 'pcl')
	assert.deepEqual(ids, ['pcl', 'g1gc-mojang', 'minimal-g1'])
})

test('buildGcCandidateChain never repeats the preferred strategy', () => {
	for (const preferred of ['zgc', 'shenandoah', 'pcl', 'g1gc-mojang']) {
		const { ids } = buildGcCandidateChain(createContext(), preferred)
		assert.equal(ids[0], preferred)
		assert.equal(new Set(ids).size, ids.length)
	}
})
