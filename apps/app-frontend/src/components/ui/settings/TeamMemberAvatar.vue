<script setup lang="ts">
import { Avatar } from '@modrinth/ui'
import { onUnmounted, ref, watch } from 'vue'

const props = withDefaults(
	defineProps<{
		name: string
		src: string
		remote?: string
		size?: string
		timeout?: number
	}>(),
	{
		remote: undefined,
		size: '4rem',
		timeout: 4000,
	},
)

// The bundled avatar renders first so a member never appears without an image.
// When a remote avatar is configured it replaces the bundled one as soon as the
// browser confirms it can be decoded, and is dropped if that takes too long.
const resolved = ref(props.src)

let timer: ReturnType<typeof setTimeout> | undefined
let probe: HTMLImageElement | undefined

function disposeProbe() {
	if (timer !== undefined) {
		clearTimeout(timer)
		timer = undefined
	}
	if (probe) {
		probe.onload = null
		probe.onerror = null
		probe.src = ''
		probe = undefined
	}
}

function resolveAvatar() {
	disposeProbe()
	resolved.value = props.src

	const remote = props.remote
	if (!remote) return

	const image = new Image()
	probe = image
	let settled = false

	const settle = (useRemote: boolean) => {
		if (settled) return
		settled = true
		if (timer !== undefined) {
			clearTimeout(timer)
			timer = undefined
		}
		image.onload = null
		image.onerror = null
		if (probe === image) probe = undefined
		if (useRemote) {
			resolved.value = remote
		} else {
			image.src = ''
		}
	}

	timer = setTimeout(() => settle(false), props.timeout)
	image.onload = () => settle(true)
	image.onerror = () => settle(false)
	image.src = remote
}

watch(() => [props.src, props.remote, props.timeout], resolveAvatar, { immediate: true })
onUnmounted(disposeProbe)
</script>

<template>
	<Avatar :src="resolved" :alt="name" :size="size" circle no-shadow />
</template>
