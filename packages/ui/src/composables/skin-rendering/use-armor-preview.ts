import type * as THREE from 'three'
import { onUnmounted, type Ref, watch } from 'vue'

import {
	applyPreparedArmorPreview,
	prepareArmorPreview,
	removeArmorPreview,
} from '../../utils/webgl/armor-rendering'
import { ARMOR_SLOTS, type ArmorPreviewConfig } from './armor-preview-types'

function cloneArmorConfig(config: ArmorPreviewConfig): ArmorPreviewConfig {
	return Object.fromEntries(
		ARMOR_SLOTS.map((slot) => [slot, { ...config[slot] }]),
	) as ArmorPreviewConfig
}

export function useArmorPreview({
	scene,
	config,
	enabled,
}: {
	scene: Ref<THREE.Object3D | null>
	config: Ref<ArmorPreviewConfig>
	enabled: Ref<boolean>
}) {
	let applyVersion = 0

	watch(
		[scene, enabled, () => JSON.stringify(config.value)],
		async ([root, isEnabled]) => {
			const version = ++applyVersion
			if (!root) return

			if (!isEnabled) {
				removeArmorPreview(root)
				return
			}

			const snapshot = cloneArmorConfig(config.value)
			try {
				const prepared = await prepareArmorPreview(snapshot)
				if (version !== applyVersion || scene.value !== root) return
				applyPreparedArmorPreview(root, snapshot, prepared)
			} catch (error) {
				console.error('Failed to update armor preview:', error)
			}
		},
		{ immediate: true },
	)

	onUnmounted(() => {
		applyVersion++
		if (scene.value) removeArmorPreview(scene.value)
	})
}
