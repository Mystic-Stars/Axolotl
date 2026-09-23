import { onUnmounted } from 'vue'

import { instance_listener } from '@/helpers/events.js'

export interface InstanceAppEvent {
	event: string
	instance_id?: string
}

export function useAppEvent(type: 'instance', callback: (event: InstanceAppEvent) => void) {
	if (type !== 'instance') return
	let unlisten: (() => void) | undefined
	void instance_listener(callback).then((dispose) => {
		unlisten = dispose
	})
	onUnmounted(() => unlisten?.())
}
