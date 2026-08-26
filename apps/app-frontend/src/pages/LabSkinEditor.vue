<script setup lang="ts">
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { platform } from '@tauri-apps/plugin-os'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import { computed, onMounted, onUnmounted, ref } from 'vue'

const { locale, formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const platformName = ref<string>()
const messages = defineMessages({
	title: { id: 'app.lab.skin-editor.title', defaultMessage: 'Skin editor' },
	loading: { id: 'app.lab.skin-editor.loading', defaultMessage: 'Loading skin editor' },
	exportSkin: { id: 'app.lab.skin-editor.export-skin', defaultMessage: 'Minecraft skin PNG' },
})

const blockbenchLocale = computed(() => {
	const normalized = locale.value.toLowerCase().replace('_', '-')
	if (normalized === 'zh-tw' || normalized === 'zh-hk') return 'zh_tw'
	if (normalized.startsWith('zh')) return 'zh'
	if (normalized === 'pt-br') return 'pt_br'
	return normalized.split('-')[0]
})

const editorUrl = computed(
	() => {
		if (import.meta.env.DEV) {
			return `/__blockbench_skin__/index.html?embed=skin&lang=${encodeURIComponent(blockbenchLocale.value)}`
		}
		if (!platformName.value) return ''
		const baseUrl = platformName.value === 'windows' ? 'http://axolotl-skin.localhost' : 'axolotl-skin://localhost'
		return `${baseUrl}/index.html?embed=skin&lang=${encodeURIComponent(blockbenchLocale.value)}`
	},
)

async function saveExportedSkin(event: MessageEvent<unknown>) {
	if (event.source !== frame.value?.contentWindow) return
	if (!event.data || typeof event.data !== 'object') return
	const message = event.data as { type?: unknown; name?: unknown; dataUrl?: unknown }
	if (message.type !== 'axolotl-skin-export' || typeof message.name !== 'string' || typeof message.dataUrl !== 'string') return
	try {
		const path = await save({
			defaultPath: message.name,
			filters: [{ name: formatMessage(messages.exportSkin), extensions: ['png'] }],
		})
		if (!path) return
		const response = await fetch(message.dataUrl)
		if (!response.ok) throw new Error(`Failed to read exported skin: ${response.status}`)
		await writeFile(path, new Uint8Array(await response.arrayBuffer()))
	} catch (error) {
		handleError(error)
	}
}

const frame = ref<HTMLIFrameElement>()
onMounted(async () => {
	platformName.value = await platform()
	window.addEventListener('message', saveExportedSkin)
})
onUnmounted(() => window.removeEventListener('message', saveExportedSkin))
</script>

<template>
	<main class="skin-editor-page">
		<h1 class="sr-only">{{ formatMessage(messages.title) }}</h1>
		<iframe
			ref="frame"
			:title="formatMessage(messages.title)"
			:src="editorUrl"
			class="skin-editor-frame"
			:aria-label="formatMessage(messages.loading)"
		/>
	</main>
</template>

<style scoped>
.skin-editor-page {
	display: flex;
	height: 100%;
	min-height: 0;
	width: 100%;
	flex: 1;
}

.skin-editor-frame {
	flex: 1;
	height: 100%;
	min-height: 0;
	width: 100%;
	border: 0;
}
</style>

<style>
.app-viewport:has(.skin-editor-page),
.app-viewport:has(.skin-editor-page) .page-transition-grid,
.app-viewport:has(.skin-editor-page) .page-transition-layer {
	height: 100%;
	min-height: 0;
	overflow: hidden;
}
</style>
