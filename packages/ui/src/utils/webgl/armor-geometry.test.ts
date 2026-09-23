import assert from 'node:assert/strict'
import test from 'node:test'

import * as THREE from 'three'

import { createArmorGeometry } from './armor-geometry.ts'

function geometrySize(geometry: THREE.BufferGeometry): THREE.Vector3 {
	geometry.computeBoundingBox()
	return geometry.boundingBox!.getSize(new THREE.Vector3())
}

function geometryBounds(geometry: THREE.BufferGeometry): THREE.Box3 {
	geometry.computeBoundingBox()
	return geometry.boundingBox!
}

function uvBoundsForNormal(
	geometry: THREE.BufferGeometry,
	normalKey: string,
): { minU: number; maxU: number; minV: number; maxV: number } {
	const normal = geometry.getAttribute('normal') as THREE.BufferAttribute
	const uv = geometry.getAttribute('uv') as THREE.BufferAttribute
	const values: Array<[number, number]> = []
	for (let index = 0; index < normal.count; index++) {
		const key = `${Math.round(normal.getX(index))},${Math.round(normal.getY(index))},${Math.round(normal.getZ(index))}`
		if (key === normalKey) values.push([uv.getX(index) * 64, uv.getY(index) * 32])
	}
	assert.ok(values.length > 0, `missing face with normal ${normalKey}`)
	return {
		minU: Math.min(...values.map(([u]) => u)),
		maxU: Math.max(...values.map(([u]) => u)),
		minV: Math.min(...values.map(([, v]) => v)),
		maxV: Math.max(...values.map(([, v]) => v)),
	}
}

function assertUvBounds(
	actual: ReturnType<typeof uvBoundsForNormal>,
	expected: ReturnType<typeof uvBoundsForNormal>,
): void {
	for (const key of ['minU', 'maxU', 'minV', 'maxV'] as const) {
		assert.ok(Math.abs(actual[key] - expected[key]) < 1e-5, `${key}: ${actual[key]}`)
	}
}

function uAtFaceEdge(
	geometry: THREE.BufferGeometry,
	normalKey: string,
	axis: 'x' | 'z',
	edge: 'min' | 'max',
): number {
	const position = geometry.getAttribute('position') as THREE.BufferAttribute
	const normal = geometry.getAttribute('normal') as THREE.BufferAttribute
	const uv = geometry.getAttribute('uv') as THREE.BufferAttribute
	const candidates: Array<{ coordinate: number; u: number }> = []
	for (let index = 0; index < normal.count; index++) {
		const key = `${Math.round(normal.getX(index))},${Math.round(normal.getY(index))},${Math.round(normal.getZ(index))}`
		if (key !== normalKey) continue
		candidates.push({
			coordinate: axis === 'x' ? position.getX(index) : position.getZ(index),
			u: uv.getX(index) * 64,
		})
	}
	assert.ok(candidates.length > 0, `missing face with normal ${normalKey}`)
	const target = Math[edge](...candidates.map(({ coordinate }) => coordinate))
	const values = candidates
		.filter(({ coordinate }) => Math.abs(coordinate - target) < 1e-7)
		.map(({ u }) => u)
	assert.ok(values.length > 0)
	assert.ok(values.every((u) => Math.abs(u - values[0]) < 1e-5))
	return values[0]
}

test('outer armor expands every side by one model pixel', () => {
	const source = new THREE.BoxGeometry(8 / 16, 12 / 16, 4 / 16)
	const size = geometrySize(createArmorGeometry(source, 'outer', 'body'))

	assert.ok(Math.abs(size.x - 10 / 16) < 1e-7)
	assert.ok(Math.abs(size.y - 14 / 16) < 1e-7)
	assert.ok(Math.abs(size.z - 6 / 16) < 1e-7)
})

test('leggings legs use the inner-model dilation with the vanilla leg reduction', () => {
	const source = new THREE.BoxGeometry(4 / 16, 12 / 16, 4 / 16)
	const size = geometrySize(createArmorGeometry(source, 'leggings', 'rightLeg'))

	assert.ok(Math.abs(size.x - 4.8 / 16) < 1e-7)
	assert.ok(Math.abs(size.y - 12.8 / 16) < 1e-7)
	assert.ok(Math.abs(size.z - 4.8 / 16) < 1e-7)
})

test('slim player arms expand outwards to the classic armor bounds', () => {
	const rightSource = new THREE.BoxGeometry(3 / 16, 12 / 16, 4 / 16).translate(11 / 32, 0, 0)
	const leftSource = new THREE.BoxGeometry(3 / 16, 12 / 16, 4 / 16).translate(-11 / 32, 0, 0)
	const right = geometryBounds(createArmorGeometry(rightSource, 'outer', 'rightArm'))
	const left = geometryBounds(createArmorGeometry(leftSource, 'outer', 'leftArm'))

	assert.ok(Math.abs(right.min.x - 3 / 16) < 1e-7)
	assert.ok(Math.abs(right.max.x - 9 / 16) < 1e-7)
	assert.ok(Math.abs(left.min.x + 9 / 16) < 1e-7)
	assert.ok(Math.abs(left.max.x + 3 / 16) < 1e-7)
	assert.ok(Math.abs(right.max.x - right.min.x - 6 / 16) < 1e-7)
	assert.ok(Math.abs(left.max.x - left.min.x - 6 / 16) < 1e-7)
	assert.ok(Math.abs(right.max.y - right.min.y - 14 / 16) < 1e-7)
	assert.ok(Math.abs(right.max.z - right.min.z - 6 / 16) < 1e-7)
})

test('remaps player UV height from 64x64 skins to 64x32 armor textures', () => {
	const source = new THREE.BoxGeometry(8 / 16, 8 / 16, 8 / 16)
	const sourceUv = source.getAttribute('uv') as THREE.BufferAttribute
	const expected = Array.from({ length: sourceUv.count }, (_, index) => sourceUv.getY(index) * 2)
	const armorUv = createArmorGeometry(source, 'outer', 'head').getAttribute(
		'uv',
	) as THREE.BufferAttribute

	for (let index = 0; index < armorUv.count; index++) {
		assert.ok(Math.abs(armorUv.getY(index) - expected[index]) < 1e-7)
	}
})

test('armor arms use vanilla mirrored UV regions on their physical outer and inner faces', () => {
	const rightSource = new THREE.BoxGeometry(4 / 16, 12 / 16, 4 / 16)
	const leftSource = rightSource.clone()
	const leftPosition = leftSource.getAttribute('position') as THREE.BufferAttribute
	const leftNormal = leftSource.getAttribute('normal') as THREE.BufferAttribute
	const leftUv = leftSource.getAttribute('uv') as THREE.BufferAttribute
	for (let index = 0; index < leftPosition.count; index++) {
		leftPosition.setX(index, -leftPosition.getX(index))
		leftNormal.setX(index, -leftNormal.getX(index))
		// The authored GLTF left arm mirrors both its physical X coordinate and
		// each face's source U direction relative to the right arm.
		leftUv.setX(index, 1 - leftUv.getX(index))
	}

	const right = createArmorGeometry(rightSource, 'outer', 'rightArm')
	const left = createArmorGeometry(leftSource, 'outer', 'leftArm')

	// In the player GLTF, +X is the right arm's outside and -X is the left
	// arm's outside. Vanilla mirror() maps both outside faces to U 40..44.
	assertUvBounds(uvBoundsForNormal(right, '1,0,0'), {
		minU: 40,
		maxU: 44,
		minV: 20,
		maxV: 32,
	})
	assertUvBounds(uvBoundsForNormal(left, '-1,0,0'), {
		minU: 40,
		maxU: 44,
		minV: 20,
		maxV: 32,
	})
	assertUvBounds(uvBoundsForNormal(right, '-1,0,0'), {
		minU: 48,
		maxU: 52,
		minV: 20,
		maxV: 32,
	})
	assertUvBounds(uvBoundsForNormal(left, '1,0,0'), {
		minU: 48,
		maxU: 52,
		minV: 20,
		maxV: 32,
	})

	for (const normal of ['0,0,-1', '0,0,1', '0,-1,0', '0,1,0']) {
		assertUvBounds(uvBoundsForNormal(left, normal), uvBoundsForNormal(right, normal))
	}

	// Both physical outer edges point to the start of the shared front region.
	assert.ok(Math.abs(uAtFaceEdge(right, '0,0,-1', 'x', 'max') - 44) < 1e-5)
	assert.ok(Math.abs(uAtFaceEdge(left, '0,0,-1', 'x', 'min') - 44) < 1e-5)
	assert.ok(Math.abs(uAtFaceEdge(right, '0,0,-1', 'x', 'min') - 48) < 1e-5)
	assert.ok(Math.abs(uAtFaceEdge(left, '0,0,-1', 'x', 'max') - 48) < 1e-5)

	// The mirrored left outside face has the same front-to-back orientation as
	// the right outside face after ModelPart.Polygon reverses its vertices.
	assert.ok(Math.abs(uAtFaceEdge(right, '1,0,0', 'z', 'min') - 44) < 1e-5)
	assert.ok(Math.abs(uAtFaceEdge(left, '-1,0,0', 'z', 'min') - 44) < 1e-5)
	assert.ok(Math.abs(uAtFaceEdge(right, '1,0,0', 'z', 'max') - 40) < 1e-5)
	assert.ok(Math.abs(uAtFaceEdge(left, '-1,0,0', 'z', 'max') - 40) < 1e-5)
})

test('left leg reuses and mirrors the right leg region of the 64x32 armor texture', () => {
	const source = new THREE.BoxGeometry(4 / 16, 12 / 16, 4 / 16)
	const sourceUv = source.getAttribute('uv') as THREE.BufferAttribute
	const expected = Array.from({ length: sourceUv.count }, (_, index) => [
		1 - sourceUv.getX(index),
		sourceUv.getY(index) * 2,
	])
	for (let index = 0; index < sourceUv.count; index++) {
		sourceUv.setXY(index, sourceUv.getX(index) + 16 / 64, sourceUv.getY(index) + 32 / 64)
	}

	const armorUv = createArmorGeometry(source, 'outer', 'leftLeg').getAttribute(
		'uv',
	) as THREE.BufferAttribute

	for (let index = 0; index < armorUv.count; index++) {
		assert.ok(Math.abs(armorUv.getX(index) - expected[index][0]) < 1e-7)
		assert.ok(Math.abs(armorUv.getY(index) - expected[index][1]) < 1e-7)
	}
})
