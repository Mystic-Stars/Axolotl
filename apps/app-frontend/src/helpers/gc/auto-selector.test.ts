import assert from 'node:assert/strict'
import test from 'node:test'

import { resolveAutoGcStrategy } from './auto-selector.ts'
import type { GcContext } from './types.ts'

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

test('hard fallback: unknown Java version falls back to G1GC', () => {
	const context = createContext({ javaMajorVersion: null })
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(result.reasonChain.includes('Java 版本未知'))
})

test('hard fallback: Java < 15 falls back to G1GC', () => {
	const context = createContext({ javaMajorVersion: 11 })
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(result.reasonChain.includes('Java 太旧，Shenandoah/ZGC 不可靠'))
})

test('hard fallback: memory < 4GB falls back to G1GC', () => {
	const context = createContext({ allocatedMemoryMb: 2048 })
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(result.reasonChain.some((r) => r.includes('内存不足')))
})

test('hard fallback: insufficient CPU resources falls back to G1GC', () => {
	const context = createContext({
		systemCpuCores: 4,
		systemLogicalProcessors: 4,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(result.reasonChain.some((r) => r.includes('CPU 资源不足')))
})

test('hard fallback: large modpack with insufficient resources falls back to G1GC', () => {
	const context = createContext({
		modCount: 200,
		allocatedMemoryMb: 6144,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(result.reasonChain.some((r) => r.includes('大型 ModPack')))
})

test('lightweight vanilla instance selects G1GC', () => {
	const context = createContext({
		loader: 'vanilla',
		modCount: 5,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(result.reasonChain.some((r) => r.includes('轻量实例')))
})

test('lightweight fabric instance selects G1GC', () => {
	const context = createContext({
		loader: 'fabric',
		modCount: 20,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(result.reasonChain.some((r) => r.includes('轻量实例')))
})

test('low resources selects G1GC', () => {
	const context = createContext({
		allocatedMemoryMb: 6143,
		systemCpuCores: 6,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'g1gc-mojang')
	assert.ok(result.reasonChain.some((r) => r.includes('资源低')))
})

test('medium resources selects Shenandoah', () => {
	const context = createContext({
		allocatedMemoryMb: 8192,
		systemCpuCores: 8,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'shenandoah')
	assert.ok(result.reasonChain.some((r) => r.includes('资源中')))
})

test('high resources with Java < 21 selects Shenandoah', () => {
	const context = createContext({
		javaMajorVersion: 17,
		allocatedMemoryMb: 16384,
		systemCpuCores: 16,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'shenandoah')
	assert.ok(result.reasonChain.some((r) => r.includes('Java < 21')))
})

test('high resources with Java >= 21 selects ZGC', () => {
	const context = createContext({
		javaMajorVersion: 21,
		allocatedMemoryMb: 16384,
		systemCpuCores: 16,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'zgc')
	assert.ok(result.reasonChain.some((r) => r.includes('内存充足且 CPU 核心数高')))
})

test('Java 21 with insufficient resources for ZGC selects Shenandoah', () => {
	const context = createContext({
		javaMajorVersion: 21,
		allocatedMemoryMb: 10240,
		systemCpuCores: 12,
	})
	const result = resolveAutoGcStrategy(context)
	assert.equal(result.resolvedStrategy, 'shenandoah')
	assert.ok(result.reasonChain.some((r) => r.includes('资源未达到 ZGC 推荐配置')))
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
