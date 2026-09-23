import type * as THREE from 'three'

export const ARMOR_PREVIEW_MARKER = 'armorPreviewOwned'

export function isArmorPreviewMesh(mesh: THREE.Mesh): boolean {
	return Boolean(mesh.userData[ARMOR_PREVIEW_MARKER])
}
