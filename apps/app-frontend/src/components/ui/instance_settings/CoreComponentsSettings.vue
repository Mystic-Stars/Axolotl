<script setup lang="ts">
import {
	ArrowDownIcon,
	ArrowUpIcon,
	DownloadIcon,
	ExternalIcon,
	EyeIcon,
	FileArchiveIcon,
	PlusIcon,
	RestoreIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	Checkbox,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'

import {
	add_core_jar_mod,
	import_mcarchive_modloader,
	install_mcarchive_modloader,
	list_core_components,
	move_core_component,
	preview_core_jar,
	remove_core_component,
	replace_core_jar,
	restore_core_component,
	set_core_component_enabled,
} from '@/helpers/instance'
import { injectInstanceSettings } from '@/providers/instance-settings'

const { instance } = injectInstanceSettings()
const { addNotification, handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const queryClient = useQueryClient()
const busy = ref(false)

const query = useQuery({
	queryKey: computed(() => ['core-components', instance.value.id]),
	queryFn: () => list_core_components(instance.value.id),
})

const activeComponents = computed(() =>
	(query.data.value ?? []).filter((component) => !component.removed),
)
const deletedComponents = computed(() =>
	(query.data.value ?? []).filter((component) => component.removed),
)
const canInstallModLoader = computed(() => {
	const version = instance.value.game_version.trim().replace(/^v/, '')
	if (/^(?:a|b|inf-)/.test(version)) return true
	const match = /^1\.(\d+)(?:\.(\d+))?$/.exec(version)
	if (!match) return false
	const minor = Number(match[1])
	const patch = Number(match[2] ?? 0)
	return minor <= 6 && (minor < 6 || patch <= 2)
})
const manualModLoader = ref<{
	fileName: string
	pageUrl: string | null
	expectedSha256: string | null
} | null>(null)

const messages = defineMessages({
	title: {
		id: 'instance.settings.tabs.core-components.title',
		defaultMessage: 'Core components',
	},
	add: {
		id: 'instance.settings.tabs.core-components.add',
		defaultMessage: 'Add to Minecraft.jar',
	},
	replace: {
		id: 'instance.settings.tabs.core-components.replace',
		defaultMessage: 'Replace Minecraft.jar',
	},
	pickJar: {
		id: 'instance.settings.tabs.core-components.pick-jar',
		defaultMessage: 'Choose core archive',
	},
	jarFilter: {
		id: 'instance.settings.tabs.core-components.jar-filter',
		defaultMessage: 'Minecraft archives',
	},
	moveUp: {
		id: 'instance.settings.tabs.core-components.move-up',
		defaultMessage: 'Move up',
	},
	moveDown: {
		id: 'instance.settings.tabs.core-components.move-down',
		defaultMessage: 'Move down',
	},
	remove: {
		id: 'instance.settings.tabs.core-components.remove',
		defaultMessage: 'Remove',
	},
	restore: {
		id: 'instance.settings.tabs.core-components.restore',
		defaultMessage: 'Restore',
	},
	jarMod: {
		id: 'instance.settings.tabs.core-components.jar-mod',
		defaultMessage: 'JAR mod',
	},
	replacement: {
		id: 'instance.settings.tabs.core-components.replacement',
		defaultMessage: 'Replacement JAR',
	},
	sha256: {
		id: 'instance.settings.tabs.core-components.sha256',
		defaultMessage: 'SHA-256',
	},
	failure: {
		id: 'instance.settings.tabs.core-components.failure',
		defaultMessage: 'Failure',
	},
	sha1: {
		id: 'instance.settings.tabs.core-components.sha1',
		defaultMessage: 'SHA-1',
	},
	source: {
		id: 'instance.settings.tabs.core-components.source',
		defaultMessage: 'Source',
	},
	targetVersion: {
		id: 'instance.settings.tabs.core-components.target-version',
		defaultMessage: 'Target Minecraft version',
	},
	preview: {
		id: 'instance.settings.tabs.core-components.preview',
		defaultMessage: 'Preview assembled JAR',
	},
	previewed: {
		id: 'instance.settings.tabs.core-components.previewed',
		defaultMessage: 'Assembled {components} components into {entries} entries',
	},
	modLoader: {
		id: 'instance.settings.tabs.core-components.modloader',
		defaultMessage: 'Install ModLoader',
	},
	modLoaderInstalled: {
		id: 'instance.settings.tabs.core-components.modloader-installed',
		defaultMessage: 'Installed ModLoader {fileName}',
	},
	modLoaderManual: {
		id: 'instance.settings.tabs.core-components.modloader-manual',
		defaultMessage:
			'{fileName} needs a manual download. Download it from the source page, then import the verified archive here.',
	},
	openSource: {
		id: 'instance.settings.tabs.core-components.open-source',
		defaultMessage: 'Open source page',
	},
	importModLoader: {
		id: 'instance.settings.tabs.core-components.import-modloader',
		defaultMessage: 'Import verified ModLoader archive',
	},
	pickModLoader: {
		id: 'instance.settings.tabs.core-components.pick-modloader',
		defaultMessage: 'Choose downloaded ModLoader archive',
	},
})

async function refresh() {
	await queryClient.invalidateQueries({ queryKey: ['core-components', instance.value.id] })
}

async function pick(kind: 'jar_mod' | 'replacement_jar') {
	const path = await open({
		multiple: false,
		title: formatMessage(messages.pickJar),
		filters: [{ name: formatMessage(messages.jarFilter), extensions: ['jar', 'zip'] }],
	})
	if (!path || Array.isArray(path)) return
	busy.value = true
	try {
		if (kind === 'jar_mod') {
			await add_core_jar_mod(instance.value.id, path, instance.value.game_version)
		} else {
			await replace_core_jar(instance.value.id, path, instance.value.game_version)
		}
		await refresh()
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}

async function run(action: () => Promise<unknown>) {
	busy.value = true
	try {
		await action()
		await refresh()
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}

async function preview() {
	busy.value = true
	try {
		const result = await preview_core_jar(instance.value.id)
		if (result) {
			addNotification({
				type: 'success',
				title: formatMessage(messages.previewed, {
					components: result.componentCount,
					entries: result.entries,
				}),
			})
		}
		await refresh()
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}

async function installModLoader() {
	busy.value = true
	manualModLoader.value = null
	try {
		const result = await install_mcarchive_modloader(instance.value.id, instance.value.game_version)
		if (result.state === 'manual_download') {
			manualModLoader.value = result
			return
		}
		addNotification({
			type: 'success',
			title: formatMessage(messages.modLoaderInstalled, {
				fileName: result.component.fileName,
			}),
		})
		await refresh()
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}

async function importModLoader() {
	if (!manualModLoader.value) return
	const path = await open({
		multiple: false,
		title: formatMessage(messages.pickModLoader),
		filters: [{ name: formatMessage(messages.jarFilter), extensions: ['jar', 'zip'] }],
	})
	if (!path || Array.isArray(path)) return
	busy.value = true
	try {
		const result = await import_mcarchive_modloader(
			instance.value.id,
			instance.value.game_version,
			path,
		)
		if (result.state === 'manual_download') {
			manualModLoader.value = result
			return
		}
		addNotification({
			type: 'success',
			title: formatMessage(messages.modLoaderInstalled, {
				fileName: result.component.fileName,
			}),
		})
		manualModLoader.value = null
		await refresh()
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}
</script>

<template>
	<div class="flex flex-col gap-4">
		<div class="flex flex-wrap gap-2">
			<ButtonStyled>
				<button :disabled="busy" @click="pick('jar_mod')">
					<PlusIcon />
					{{ formatMessage(messages.add) }}
				</button>
			</ButtonStyled>
			<ButtonStyled type="outlined">
				<button :disabled="busy" @click="pick('replacement_jar')">
					<FileArchiveIcon />
					{{ formatMessage(messages.replace) }}
				</button>
			</ButtonStyled>
			<ButtonStyled v-if="canInstallModLoader" type="outlined">
				<button :disabled="busy" @click="installModLoader">
					<DownloadIcon />
					{{ formatMessage(messages.modLoader) }}
				</button>
			</ButtonStyled>
			<ButtonStyled type="transparent">
				<button :disabled="busy" @click="preview">
					<EyeIcon />
					{{ formatMessage(messages.preview) }}
				</button>
			</ButtonStyled>
		</div>

		<div
			v-if="manualModLoader"
			class="flex flex-wrap items-center justify-between gap-3 border-y border-surface-4 py-3"
		>
			<p class="m-0 min-w-0 flex-1 text-sm text-secondary">
				{{ formatMessage(messages.modLoaderManual, { fileName: manualModLoader.fileName }) }}
			</p>
			<ButtonStyled v-if="manualModLoader.pageUrl" type="outlined">
				<button @click="openUrl(manualModLoader!.pageUrl!)">
					<ExternalIcon />
					{{ formatMessage(messages.openSource) }}
				</button>
			</ButtonStyled>
			<ButtonStyled>
				<button :disabled="busy" @click="importModLoader">
					<FileArchiveIcon />
					{{ formatMessage(messages.importModLoader) }}
				</button>
			</ButtonStyled>
		</div>

		<div class="overflow-hidden rounded-lg border border-surface-4">
			<div
				v-for="(component, index) in activeComponents"
				:key="component.id"
				class="flex items-center gap-3 border-b border-surface-4 px-3 py-3 last:border-b-0"
			>
				<Checkbox
					:model-value="component.enabled"
					:disabled="busy"
					@update:model-value="
						(enabled) => run(() => set_core_component_enabled(instance.id, component.id, enabled))
					"
				/>
				<div class="min-w-0 flex-1">
					<div class="truncate font-medium text-contrast">{{ component.fileName }}</div>
					<div class="text-xs text-secondary">
						{{
							formatMessage(
								component.kind === 'replacement_jar' ? messages.replacement : messages.jarMod,
							)
						}}
						<span v-if="component.targetGameVersion">
							· {{ formatMessage(messages.targetVersion) }} {{ component.targetGameVersion }}
						</span>
						<span v-if="component.source">
							· {{ formatMessage(messages.source) }} {{ component.source.provider }}</span
						>
					</div>
					<div v-if="component.sha256 || component.sha1" class="truncate text-xs text-secondary">
						<span v-if="component.sha256"
							>{{ formatMessage(messages.sha256) }} {{ component.sha256 }}</span
						>
						<span v-if="component.sha256 && component.sha1"> · </span>
						<span v-if="component.sha1"
							>{{ formatMessage(messages.sha1) }} {{ component.sha1 }}</span
						>
					</div>
					<div v-if="component.failureReason" class="text-xs text-red">
						{{ formatMessage(messages.failure) }}: {{ component.failureReason }}
					</div>
				</div>
				<div class="flex shrink-0 items-center gap-1">
					<ButtonStyled circular size="small" type="transparent">
						<button
							v-tooltip="formatMessage(messages.moveUp)"
							:aria-label="formatMessage(messages.moveUp)"
							:disabled="busy || index === 0"
							@click="run(() => move_core_component(instance.id, component.id, -1))"
						>
							<ArrowUpIcon />
						</button>
					</ButtonStyled>
					<ButtonStyled circular size="small" type="transparent">
						<button
							v-tooltip="formatMessage(messages.moveDown)"
							:aria-label="formatMessage(messages.moveDown)"
							:disabled="busy || index === activeComponents.length - 1"
							@click="run(() => move_core_component(instance.id, component.id, 1))"
						>
							<ArrowDownIcon />
						</button>
					</ButtonStyled>
					<ButtonStyled circular color="red" size="small" type="transparent">
						<button
							v-tooltip="formatMessage(messages.remove)"
							:aria-label="formatMessage(messages.remove)"
							:disabled="busy"
							@click="run(() => remove_core_component(instance.id, component.id))"
						>
							<TrashIcon />
						</button>
					</ButtonStyled>
				</div>
			</div>
		</div>

		<div v-if="deletedComponents.length" class="overflow-hidden rounded-lg border border-surface-4">
			<div
				v-for="component in deletedComponents"
				:key="component.id"
				class="flex items-center gap-3 px-3 py-3"
			>
				<div class="min-w-0 flex-1 truncate text-secondary">{{ component.fileName }}</div>
				<ButtonStyled circular size="small" type="transparent">
					<button
						v-tooltip="formatMessage(messages.restore)"
						:aria-label="formatMessage(messages.restore)"
						:disabled="busy"
						@click="run(() => restore_core_component(instance.id, component.id))"
					>
						<RestoreIcon />
					</button>
				</ButtonStyled>
			</div>
		</div>
	</div>
</template>
