import assert from 'node:assert/strict'
import test from 'node:test'

import { resolveAutoGcStrategy } from './auto-selector.ts'
import type { GcContext, GcReason } from './types.ts'

function createContext(overrides: Partial<GcContext> = {}): GcContext {
	return {
		javaMajorVersion: 21,
		allocatedMemoryMb: 8192,
		systemCpuCores: 8,
		systemLogicalProcessors: 8,
		modCount: 50,
		loader: 'forge',
		...overrides,
	}
}

function reasonIds(result: { reasonChain: (string | GcReason)[] }): string[] {
	return result.reasonChain.flatMap((entry) => (typeof entry === 'string' ? [] : [entry.id]))
}

test('hard fallback: unknown Java version falls back to G1GC', () => {
	const context = createContext({ javaMajorVersion: null })
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.java-version-unknown'))
})

test('hard fallback: Java < 15 falls back to G1GC', () => {
	const context = createContext({ javaMajorVersion: 11 })
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.java-too-old'))
})

test('hard fallback: memory < 4GB falls back to G1GC', () => {
	const context = createContext({ allocatedMemoryMb: 2048 })
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.insufficient-memory'))
})

test('hard fallback: insufficient CPU resources falls back to G1GC', () => {
	const context = createContext({
		systemCpuCores: 4,
		systemLogicalProcessors: 4,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.insufficient-cpu'))
})

test('hard fallback: large modpack with insufficient resources falls back to G1GC', () => {
	const context = createContext({
		modCount: 200,
		allocatedMemoryMb: 6144,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.large-modpack-low-memory'))
})

test('lightweight vanilla instance selects G1GC', () => {
	const context = createContext({
		loader: 'vanilla',
		modCount: 5,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.lightweight-instance'))
})

test('lightweight fabric instance selects G1GC', () => {
	const context = createContext({
		loader: 'fabric',
		modCount: 20,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.lightweight-instance'))
})

test('low resources selects G1GC', () => {
	const context = createContext({
		allocatedMemoryMb: 6143,
		systemCpuCores: 6,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.low-resources'))
})

test('medium resources selects Shenandoah', () => {
	const context = createContext({
		allocatedMemoryMb: 8192,
		systemCpuCores: 8,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'shenandoah')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.medium-resources'))
})

test('high resources with Java < 21 selects Shenandoah', () => {
	const context = createContext({
		javaMajorVersion: 17,
		allocatedMemoryMb: 16384,
		systemCpuCores: 16,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'shenandoah')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.zgc-non-generational'))
})

test('high resources with Java >= 21 selects ZGC', () => {
	const context = createContext({
		javaMajorVersion: 21,
		allocatedMemoryMb: 16384,
		systemCpuCores: 16,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'zgc')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.ample-memory-high-cores'))
})

test('Java 21 with insufficient resources for ZGC selects Shenandoah', () => {
	const context = createContext({
		javaMajorVersion: 21,
		allocatedMemoryMb: 10240,
		systemCpuCores: 12,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'shenandoah')
	assert.ok(reasonIds(result).includes('app.java-arguments.gc.reason.below-zgc-recommendation'))
})

test('reason chain contains all decision nodes', () => {
	const context = createContext({
		javaMajorVersion: 21,
		allocatedMemoryMb: 16384,
		systemCpuCores: 16,
		modCount: 100,
		loader: 'forge',
	})
	const result = resolveAutoGcStrategy(context)
	assert.ok(result.reasonChain.length > 0)
	assert.equal(result.reasonChain[0], 'Java 21')
})
