import * as THREE from 'three'

import {
	type ArmorTextureLayer,
	getArmorTextureUrl,
	getLeatherOverlayUrl,
	getTrimPaletteUrl,
	getTrimPatternTextureUrl,
	getTrimSourcePaletteUrl,
} from '../../composables/skin-rendering/armor-preview-assets'
import type {
	ArmorMaterial,
	ArmorPreviewConfig,
	ArmorSlot,
	ArmorTrimMaterial,
} from '../../composables/skin-rendering/armor-preview-types'
import { type ArmorBodyPart, type ArmorGeometryLayer, createArmorGeometry } from './armor-geometry'
import { ARMOR_PREVIEW_MARKER } from './armor-preview-object'

const DEFAULT_LEATHER_COLOR = 0xa06540

interface ArmorPartDefinition {
	sourceName: string
	bodyPart: ArmorBodyPart
}

interface PreparedArmorPiece {
	baseTexture: THREE.Texture
	overlayTexture?: THREE.Texture
	trimTexture?: THREE.Texture
}

export type PreparedArmorPreview = Partial<Record<ArmorSlot, PreparedArmorPiece>>

const ARMOR_PARTS: Record<ArmorSlot, readonly ArmorPartDefinition[]> = {
	helmet: [{ sourceName: 'Head_2', bodyPart: 'head' }],
	chestplate: [
		{ sourceName: 'Body_2', bodyPart: 'body' },
		{ sourceName: 'Right_Arm_2', bodyPart: 'rightArm' },
		{ sourceName: 'Left_Arm_2', bodyPart: 'leftArm' },
	],
	leggings: [
		{ sourceName: 'Body_2', bodyPart: 'body' },
		{ sourceName: 'Right_Leg_2', bodyPart: 'rightLeg' },
		{ sourceName: 'Left_Leg_2', bodyPart: 'leftLeg' },
	],
	boots: [
		{ sourceName: 'Right_Leg_2', bodyPart: 'rightLeg' },
		{ sourceName: 'Left_Leg_2', bodyPart: 'leftLeg' },
	],
}

const textureCache = new Map<string, Promise<THREE.Texture>>()
const imageCache = new Map<string, Promise<HTMLImageElement>>()
const trimTextureCache = new Map<string, Promise<THREE.Texture>>()

function textureLayerForSlot(slot: ArmorSlot): ArmorTextureLayer {
	return slot === 'leggings' ? 'leggings' : 'humanoid'
}

function geometryLayerForSlot(slot: ArmorSlot): ArmorGeometryLayer {
	return slot === 'leggings' ? 'leggings' : 'outer'
}

function loadTexture(url: string): Promise<THREE.Texture> {
	const cached = textureCache.get(url)
	if (cached) return cached

	const promise = new THREE.TextureLoader().loadAsync(url).then((texture) => {
		texture.colorSpace = THREE.SRGBColorSpace
		texture.flipY = false
		texture.magFilter = THREE.NearestFilter
		texture.minFilter = THREE.NearestFilter
		texture.generateMipmaps = false
		return texture
	})
	textureCache.set(url, promise)
	return promise
}

function loadImage(url: string): Promise<HTMLImageElement> {
	const cached = imageCache.get(url)
	if (cached) return cached

	const promise = new Promise<HTMLImageElement>((resolve, reject) => {
		const image = new Image()
		image.crossOrigin = 'anonymous'
		image.onload = () => resolve(image)
		image.onerror = () => reject(new Error(`Failed to load armor preview image: ${url}`))
		image.src = url
	})
	imageCache.set(url, promise)
	return promise
}

function imagePixels(image: CanvasImageSource, width: number, height: number): ImageData {
	const canvas = document.createElement('canvas')
	canvas.width = width
	canvas.height = height
	const context = canvas.getContext('2d', { willReadFrequently: true })
	if (!context) throw new Error('Unable to read armor preview texture pixels')
	context.drawImage(image, 0, 0, width, height)
	return context.getImageData(0, 0, width, height)
}

function closestPaletteIndex(
	red: number,
	green: number,
	blue: number,
	palette: Uint8ClampedArray,
): number {
	let closest = 0
	let closestDistance = Number.POSITIVE_INFINITY
	for (let index = 0; index < palette.length; index += 4) {
		const redDistance = red - palette[index]
		const greenDistance = green - palette[index + 1]
		const blueDistance = blue - palette[index + 2]
		const distance =
			redDistance * redDistance + greenDistance * greenDistance + blueDistance * blueDistance
		if (distance < closestDistance) {
			closest = index
			closestDistance = distance
		}
	}
	return closest
}

async function createTrimTexture(patternUrl: string, paletteUrl: string): Promise<THREE.Texture> {
	const cacheKey = `${patternUrl}|${paletteUrl}`
	const cached = trimTextureCache.get(cacheKey)
	if (cached) return cached

	const promise = Promise.all([
		loadImage(patternUrl),
		loadImage(getTrimSourcePaletteUrl()),
		loadImage(paletteUrl),
	]).then(([patternImage, sourcePaletteImage, targetPaletteImage]) => {
		const width = patternImage.naturalWidth
		const height = patternImage.naturalHeight
		const pattern = imagePixels(patternImage, width, height)
		const sourcePalette = imagePixels(
			sourcePaletteImage,
			sourcePaletteImage.naturalWidth,
			sourcePaletteImage.naturalHeight,
		).data
		const targetPalette = imagePixels(
			targetPaletteImage,
			targetPaletteImage.naturalWidth,
			targetPaletteImage.naturalHeight,
		).data

		for (let index = 0; index < pattern.data.length; index += 4) {
			if (pattern.data[index + 3] === 0) continue
			const paletteIndex = closestPaletteIndex(
				pattern.data[index],
				pattern.data[index + 1],
				pattern.data[index + 2],
				sourcePalette,
			)
			pattern.data[index] = targetPalette[paletteIndex]
			pattern.data[index + 1] = targetPalette[paletteIndex + 1]
			pattern.data[index + 2] = targetPalette[paletteIndex + 2]
		}

		const canvas = document.createElement('canvas')
		canvas.width = width
		canvas.height = height
		const context = canvas.getContext('2d')
		if (!context) throw new Error('Unable to create armor trim texture')
		context.putImageData(pattern, 0, 0)

		const texture = new THREE.CanvasTexture(canvas)
		texture.colorSpace = THREE.SRGBColorSpace
		texture.flipY = false
		texture.magFilter = THREE.NearestFilter
		texture.minFilter = THREE.NearestFilter
		texture.generateMipmaps = false
		texture.needsUpdate = true
		return texture
	})
	trimTextureCache.set(cacheKey, promise)
	return promise
}

function usesDarkerTrim(material: ArmorMaterial, trimMaterial: ArmorTrimMaterial): boolean {
	return (
		material !== 'leather' &&
		material !== 'chainmail' &&
		material !== 'turtle' &&
		material === trimMaterial
	)
}

async function prepareArmorPiece(
	slot: ArmorSlot,
	selection: ArmorPreviewConfig[ArmorSlot],
): Promise<PreparedArmorPiece | undefined> {
	if (!selection.material) return undefined
	const layer = textureLayerForSlot(slot)
	const tasks: Array<Promise<THREE.Texture | undefined>> = [
		loadTexture(getArmorTextureUrl(selection.material, layer)),
		selection.material === 'leather'
			? loadTexture(getLeatherOverlayUrl(layer))
			: Promise.resolve(undefined),
	]

	if (selection.trimPattern) {
		tasks.push(
			createTrimTexture(
				getTrimPatternTextureUrl(selection.trimPattern, layer),
				getTrimPaletteUrl(
					selection.trimMaterial,
					usesDarkerTrim(selection.material, selection.trimMaterial),
				),
			),
		)
	} else {
		tasks.push(Promise.resolve(undefined))
	}

	const [baseTexture, overlayTexture, trimTexture] = await Promise.all(tasks)
	return { baseTexture: baseTexture!, overlayTexture, trimTexture }
}

export async function prepareArmorPreview(
	config: ArmorPreviewConfig,
): Promise<PreparedArmorPreview> {
	const entries = await Promise.all(
		(Object.entries(config) as Array<[ArmorSlot, ArmorPreviewConfig[ArmorSlot]]>).map(
			async ([slot, selection]) => [slot, await prepareArmorPiece(slot, selection)] as const,
		),
	)
	return Object.fromEntries(entries.filter((entry) => entry[1])) as PreparedArmorPreview
}

function createArmorMaterial(
	texture: THREE.Texture,
	options: { color?: number; polygonOffset?: number; renderOrder: number },
): THREE.MeshStandardMaterial {
	const material = new THREE.MeshStandardMaterial({
		map: texture,
		alphaTest: 0.1,
		color: options.color ?? 0xffffff,
		depthTest: true,
		depthWrite: true,
		flatShading: true,
		metalness: 0,
		roughness: 1,
		side: THREE.FrontSide,
		toneMapped: false,
		transparent: false,
	})
	if (options.polygonOffset !== undefined) {
		material.polygonOffset = true
		material.polygonOffsetFactor = options.polygonOffset
		material.polygonOffsetUnits = options.polygonOffset
	}
	material.userData.armorPreviewRenderOrder = options.renderOrder
	return material
}

function attachArmorMesh(
	source: THREE.Mesh,
	geometry: THREE.BufferGeometry,
	material: THREE.MeshStandardMaterial,
	name: string,
): void {
	const mesh = new THREE.Mesh(geometry, material)
	mesh.name = name
	mesh.position.copy(source.position)
	mesh.quaternion.copy(source.quaternion)
	mesh.scale.copy(source.scale)
	mesh.renderOrder = material.userData.armorPreviewRenderOrder as number
	mesh.frustumCulled = source.frustumCulled
	mesh.userData[ARMOR_PREVIEW_MARKER] = true
	source.parent?.add(mesh)
}

function attachArmorPart(
	source: THREE.Mesh,
	slot: ArmorSlot,
	bodyPart: ArmorBodyPart,
	selection: ArmorPreviewConfig[ArmorSlot],
	prepared: PreparedArmorPiece,
): void {
	const geometry = createArmorGeometry(source.geometry, geometryLayerForSlot(slot), bodyPart)
	attachArmorMesh(
		source,
		geometry,
		createArmorMaterial(prepared.baseTexture, {
			color: selection.material === 'leather' ? DEFAULT_LEATHER_COLOR : undefined,
			renderOrder: 2,
		}),
		`Armor_${slot}_${source.name}`,
	)

	if (prepared.overlayTexture) {
		attachArmorMesh(
			source,
			geometry.clone(),
			createArmorMaterial(prepared.overlayTexture, { polygonOffset: -1, renderOrder: 3 }),
			`Armor_${slot}_${source.name}_Overlay`,
		)
	}
	if (prepared.trimTexture) {
		attachArmorMesh(
			source,
			geometry.clone(),
			createArmorMaterial(prepared.trimTexture, { polygonOffset: -2, renderOrder: 4 }),
			`Armor_${slot}_${source.name}_Trim`,
		)
	}
}

export function removeArmorPreview(root: THREE.Object3D): void {
	const owned: THREE.Mesh[] = []
	root.traverse((object) => {
		const mesh = object as THREE.Mesh
		if (mesh.isMesh && mesh.userData[ARMOR_PREVIEW_MARKER]) owned.push(mesh)
	})
	for (const mesh of owned) {
		mesh.parent?.remove(mesh)
		mesh.geometry.dispose()
		const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material]
		materials.forEach((material) => material.dispose())
	}
}

export function applyPreparedArmorPreview(
	root: THREE.Object3D,
	config: ArmorPreviewConfig,
	prepared: PreparedArmorPreview,
): void {
	removeArmorPreview(root)
	for (const [slot, piece] of Object.entries(prepared) as Array<[ArmorSlot, PreparedArmorPiece]>) {
		const selection = config[slot]
		for (const part of ARMOR_PARTS[slot]) {
			const source = root.getObjectByName(part.sourceName) as THREE.Mesh | undefined
			if (!source?.isMesh || !source.parent) continue
			attachArmorPart(source, slot, part.bodyPart, selection, piece)
		}
	}
}
