import assert from 'node:assert/strict'
import test from 'node:test'

import * as THREE from 'three'

import { createSolidSkinLayerGeometry, type SolidSkinLayerDefinition } from './solid-skin-layer.ts'

const HAT: SolidSkinLayerDefinition = {
	width: 8,
	height: 8,
	depth: 8,
	u: 32,
	v: 0,
	pixelScale: [1.18, 1.18, 1.18],
	centerOffset: [0, 0.00295, 0],
}

function pixels(...entries: Array<[number, number, number]>): Uint8ClampedArray {
	const result = new Uint8ClampedArray(64 * 64 * 4)
	for (const [u, v, alpha] of entries) result[(v * 64 + u) * 4 + 3] = alpha
	return result
}

function geometryFor(
	entries: Array<[number, number, number]>,
	definition: SolidSkinLayerDefinition = HAT,
) {
	const mesh = new THREE.Mesh(new THREE.BoxGeometry(0.5625, 0.5625, 0.5625))
	return createSolidSkinLayerGeometry(mesh, pixels(...entries), definition)!
}

function insetStart(value: number): number {
	return (value + 1 / 64) / 64
}

function insetEnd(value: number): number {
	return (value - 1 / 64) / 64
}

function assertUv(
	attribute: THREE.BufferAttribute,
	index: number,
	expected: readonly [number, number],
): void {
	assert.ok(Math.abs(attribute.getX(index) - expected[0]) < 1e-7)
	assert.ok(Math.abs(attribute.getY(index) - expected[1]) < 1e-7)
}

test('uses the canonical Java outer-layer UV regions on all six faces', () => {
	const representativePixels: Array<[number, number, number]> = [
		[43, 3, 255],
		[51, 3, 255],
		[43, 11, 255],
		[59, 11, 255],
		[35, 11, 255],
		[51, 11, 255],
	]

	for (const representativePixel of representativePixels) {
		const geometry = geometryFor([representativePixel])
		assert.ok(geometry.getAttribute('position').count > 36)
	}
})

test('matches the authored GLTF UV orientation and edge inset on all six surfaces', () => {
	const geometry = geometryFor([])
	const uv = geometry.getAttribute('uv') as THREE.BufferAttribute
	const faceCorners = [
		[
			[insetStart(40), insetEnd(16)],
			[insetEnd(48), insetEnd(16)],
			[insetEnd(48), insetStart(8)],
			[insetStart(40), insetStart(8)],
		],
		[
			[insetStart(56), insetEnd(16)],
			[insetEnd(64), insetEnd(16)],
			[insetEnd(64), insetStart(8)],
			[insetStart(56), insetStart(8)],
		],
		[
			[insetEnd(56), insetEnd(8)],
			[insetStart(48), insetEnd(8)],
			[insetStart(48), insetStart(0)],
			[insetEnd(56), insetStart(0)],
		],
		[
			[insetEnd(48), insetStart(0)],
			[insetStart(40), insetStart(0)],
			[insetStart(40), insetEnd(8)],
			[insetEnd(48), insetEnd(8)],
		],
		[
			[insetStart(48), insetEnd(16)],
			[insetEnd(56), insetEnd(16)],
			[insetEnd(56), insetStart(8)],
			[insetStart(48), insetStart(8)],
		],
		[
			[insetStart(32), insetEnd(16)],
			[insetEnd(40), insetEnd(16)],
			[insetEnd(40), insetStart(8)],
			[insetStart(32), insetStart(8)],
		],
	] as const
	const triangleIndices = [0, 1, 2, 0, 2, 3]

	for (let faceIndex = 0; faceIndex < faceCorners.length; faceIndex++) {
		for (let vertexIndex = 0; vertexIndex < triangleIndices.length; vertexIndex++) {
			assertUv(
				uv,
				faceIndex * triangleIndices.length + vertexIndex,
				faceCorners[faceIndex][triangleIndices[vertexIndex]],
			)
		}
	}
})

test('uses the canonical three-pixel-wide UV regions for slim arms', () => {
	const slimArm: SolidSkinLayerDefinition = {
		width: 3,
		height: 12,
		depth: 4,
		u: 40,
		v: 32,
		pixelScale: [1.15, 1.035, 1.15],
		centerOffset: [0.004615625, -0.00228125, 0],
	}
	const representativePixels: Array<[number, number, number]> = [
		[45, 33, 255],
		[48, 33, 255],
		[45, 40, 255],
		[52, 40, 255],
		[41, 40, 255],
		[48, 40, 255],
	]

	for (const representativePixel of representativePixels) {
		const geometry = geometryFor([representativePixel], slimArm)
		assert.ok(geometry.getAttribute('position').count > 36)
	}
})

test('matches the mod default head voxel scale instead of the authored shell size', () => {
	const geometry = geometryFor([])
	const size = geometry.boundingBox!.getSize(new THREE.Vector3())

	assert.ok(Math.abs(size.x - 8 * 1.18 * (1 / 16)) < 1e-6)
	assert.ok(Math.abs(size.y - 8 * 1.18 * (1 / 16)) < 1e-6)
	assert.ok(Math.abs(size.z - 8 * 1.18 * (1 / 16)) < 1e-6)
})

test('samples every extrusion face from the center of its source texel', () => {
	const geometry = geometryFor([[43, 11, 255]])
	const uv = geometry.getAttribute('uv') as THREE.BufferAttribute

	for (let index = 36; index < uv.count; index++) {
		assert.ok(Math.abs(uv.getX(index) - 43.5 / 64) < 1e-7)
		assert.ok(Math.abs(uv.getY(index) - 11.5 / 64) < 1e-7)
	}
})

test('keeps the opaque side facing a translucent neighbour', () => {
	const opaquePair = geometryFor([
		[42, 10, 255],
		[43, 10, 255],
	])
	const translucentPair = geometryFor([
		[42, 10, 255],
		[43, 10, 128],
	])

	assert.equal(
		translucentPair.getAttribute('position').count,
		opaquePair.getAttribute('position').count + 6,
	)
})
