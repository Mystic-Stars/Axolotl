import assert from 'node:assert/strict'
import test from 'node:test'

import * as THREE from 'three'

import { ARMOR_PREVIEW_MARKER } from './armor-preview-object.ts'
import { applyTexture, applyThreeDSkinLayers, configureSkinMaterial } from './skin-rendering.ts'

function withMockSkinPixels(
	run: (texture: THREE.Texture, setPixels: (pixels: Uint8ClampedArray) => void) => void,
): void {
	let pixels = new Uint8ClampedArray(64 * 64 * 4)
	const originalDocument = Object.getOwnPropertyDescriptor(globalThis, 'document')
	Object.defineProperty(globalThis, 'document', {
		configurable: true,
		value: {
			createElement: () => ({
				getContext: () => ({
					drawImage: () => undefined,
					getImageData: () => ({ data: pixels }),
				}),
			}),
		},
	})

	try {
		const texture = new THREE.Texture()
		texture.image = {} as CanvasImageSource
		run(texture, (nextPixels) => {
			pixels = nextPixels
		})
	} finally {
		if (originalDocument) {
			Object.defineProperty(globalThis, 'document', originalDocument)
		} else {
			Reflect.deleteProperty(globalThis, 'document')
		}
	}
}

test('skin material preserves every non-zero 8-bit alpha value', () => {
	const material = new THREE.MeshStandardMaterial({
		alphaTest: 0.1,
		transparent: false,
	})

	configureSkinMaterial(material, true)

	assert.equal(material.transparent, true)
	assert.equal(material.depthWrite, true)
	assert.equal(material.alphaToCoverage, false)
	assert.ok(material.alphaTest > 0)
	assert.ok(material.alphaTest < 1 / 255)
})

test('binary skin material keeps the opaque depth-writing path', () => {
	const material = new THREE.MeshStandardMaterial()

	configureSkinMaterial(material, false)

	assert.equal(material.transparent, false)
	assert.equal(material.depthWrite, true)
	assert.equal(material.alphaToCoverage, true)
	assert.equal(material.alphaTest, 0.1)
})

test('skin layers render after inner parts while retaining surface depth', () => {
	const model = new THREE.Group()
	const inner = new THREE.Mesh(new THREE.BoxGeometry(), new THREE.MeshStandardMaterial())
	inner.name = 'Body_2'
	const outer = new THREE.Mesh(new THREE.BoxGeometry(), new THREE.MeshStandardMaterial())
	outer.name = 'Body_Layer'
	model.add(inner, outer)

	applyTexture(model, new THREE.Texture())

	assert.equal(inner.renderOrder, 0)
	assert.equal(outer.renderOrder, 1)
	assert.equal((inner.material as THREE.MeshStandardMaterial).depthWrite, true)
	assert.equal((outer.material as THREE.MeshStandardMaterial).depthWrite, true)
})

test('changing a skin texture does not overwrite armor preview materials', () => {
	const armorTexture = new THREE.Texture()
	const skinTexture = new THREE.Texture()
	const armorMaterial = new THREE.MeshStandardMaterial({ map: armorTexture })
	const armor = new THREE.Mesh(new THREE.BoxGeometry(), armorMaterial)
	armor.userData[ARMOR_PREVIEW_MARKER] = true
	const model = new THREE.Group()
	model.add(armor)

	applyTexture(model, skinTexture)

	assert.equal(armorMaterial.map, armorTexture)
})

test('changing textures rebuilds voxel geometry from the original skin layer', () => {
	withMockSkinPixels((texture, setPixels) => {
		const layer = new THREE.Mesh(new THREE.BoxGeometry(), new THREE.MeshStandardMaterial())
		layer.name = 'Hat_Layer'
		const model = new THREE.Group()
		model.add(layer)

		let pixels = new Uint8ClampedArray(64 * 64 * 4)
		pixels[(8 * 64 + 40) * 4 + 3] = 255
		setPixels(pixels)
		applyThreeDSkinLayers(model, texture)
		const firstVertexCount = layer.geometry.getAttribute('position').count

		pixels = new Uint8ClampedArray(64 * 64 * 4)
		pixels[(8 * 64 + 40) * 4 + 3] = 255
		pixels[(8 * 64 + 42) * 4 + 3] = 255
		setPixels(pixels)
		applyThreeDSkinLayers(model, texture)
		const secondVertexCount = layer.geometry.getAttribute('position').count

		assert.ok(secondVertexCount > firstVertexCount)
	})
})

test('matches the mod part transforms for classic and slim outer layers', () => {
	withMockSkinPixels((texture) => {
		const model = new THREE.Group()
		const parts = [
			{
				name: 'Hat_Layer',
				sourceSize: [0.5625, 0.5625, 0.5625],
				expectedCenter: [0, 0.00295, 0],
				expectedSize: [0.59, 0.59, 0.59],
			},
			{
				name: 'Body_Layer',
				sourceSize: [0.53125, 0.78125, 0.28125],
				expectedCenter: [0, -0.0001875, 0],
				expectedSize: [0.525, 0.77625, 0.2875],
			},
			{
				name: 'Right_Arm_Layer',
				sourceSize: [0.28125, 0.78125, 0.28125],
				expectedCenter: [0.00923125, -0.00228125, 0],
				expectedSize: [0.2875, 0.77625, 0.2875],
			},
			{
				name: 'Left_Arm_Layer',
				sourceSize: [0.21875, 0.78125, 0.28125],
				expectedCenter: [-0.004615625, -0.00228125, 0],
				expectedSize: [0.215625, 0.77625, 0.2875],
			},
		] as const

		for (const part of parts) {
			const mesh = new THREE.Mesh(
				new THREE.BoxGeometry(...part.sourceSize),
				new THREE.MeshStandardMaterial(),
			)
			mesh.name = part.name
			model.add(mesh)
		}

		applyThreeDSkinLayers(model, texture)

		for (const [index, part] of parts.entries()) {
			const mesh = model.children[index] as THREE.Mesh
			const bounds = new THREE.Box3().setFromBufferAttribute(
				mesh.geometry.getAttribute('position') as THREE.BufferAttribute,
			)
			const center = bounds.getCenter(new THREE.Vector3())
			const size = bounds.getSize(new THREE.Vector3())
			for (let axis = 0; axis < 3; axis++) {
				assert.ok(Math.abs(center.getComponent(axis) - part.expectedCenter[axis]) < 1e-7)
				assert.ok(Math.abs(size.getComponent(axis) - part.expectedSize[axis]) < 1e-7)
			}
		}
	})
})
