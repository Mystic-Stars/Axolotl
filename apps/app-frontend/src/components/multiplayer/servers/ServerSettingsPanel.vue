<script setup lang="ts">
import { ImageIcon, SaveIcon, SpinnerIcon, TrashIcon, XIcon } from '@modrinth/assets'
import { requiredJavaMajorVersion } from '@modrinth/server'
import {
	Admonition,
	ButtonStyled,
	Card,
	ConfirmModal,
	defineMessages,
	injectFilePicker,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import ServerIcon from '@/components/multiplayer/servers/ServerIcon.vue'
import ServerPropertiesEditor from '@/components/multiplayer/servers/ServerPropertiesEditor.vue'
import JavaSelector from '@/components/ui/JavaSelector.vue'
import { type ServerView, useServers } from '@/composables/useServers'
import { get_jre } from '@/helpers/jre'
import { servers as serversApi } from '@/helpers/servers'

const props = defineProps<{
	server: ServerView
}>()

const emit = defineEmits<{
	deleted: []
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	general: { id: 'app.servers.settings.general', defaultMessage: 'General' },
	name: { id: 'app.servers.settings.name', defaultMessage: 'Server name' },
	icon: { id: 'app.servers.settings.icon', defaultMessage: 'Icon' },
	selectIcon: { id: 'app.servers.icon.select', defaultMessage: 'Select icon' },
	changeIcon: { id: 'app.servers.icon.change', defaultMessage: 'Change icon' },
	removeIcon: { id: 'app.servers.icon.remove', defaultMessage: 'Remove icon' },
	java: { id: 'app.servers.settings.java', defaultMessage: 'Java' },
	memory: { id: 'app.servers.settings.memory', defaultMessage: 'Memory (MB)' },
	jvmArgs: { id: 'app.servers.settings.jvm-args', defaultMessage: 'JVM arguments' },
	jvmArgsHint: {
		id: 'app.servers.settings.jvm-args-hint',
		defaultMessage: 'Space-separated arguments, e.g. -XX:+UseG1GC',
	},
	save: { id: 'app.servers.settings.save', defaultMessage: 'Save changes' },
	saved: { id: 'app.servers.settings.saved', defaultMessage: 'Server settings saved' },
	cancel: { id: 'app.servers.settings.cancel', defaultMessage: 'Cancel' },
	deleteTitle: { id: 'app.servers.settings.delete', defaultMessage: 'Delete server' },
	deleteHint: {
		id: 'app.servers.settings.delete-hint',
		defaultMessage: 'Permanently remove this server and all of its files.',
	},
	deleteConfirm: {
		id: 'app.servers.settings.delete-confirm',
		defaultMessage: 'Delete {name} and all of its files? This cannot be undone.',
	},
	deleteProceed: { id: 'app.servers.settings.delete-proceed', defaultMessage: 'Delete' },
	configFiles: { id: 'app.servers.settings.config', defaultMessage: 'Configuration' },
	runningTitle: {
		id: 'app.servers.settings.running-title',
		defaultMessage: 'Server is running',
	},
	runningHint: {
		id: 'app.servers.settings.running-hint',
		defaultMessage: 'Your changes will take effect the next time the server starts.',
	},
})

const { deleteServer, refresh } = useServers()
const { addNotification, handleError } = injectNotificationManager()
const filePicker = injectFilePicker()

const name = ref(props.server.name)
const iconPath = ref<string | null>(props.server.iconPath ?? null)
const javaSelection = ref<{ path: string; version: string }>({
	path: props.server.javaPath ?? '',
	version: '',
})
const memoryMb = ref(props.server.memoryMb ?? 2048)
const jvmArgsText = ref((props.server.jvmArgs ?? []).join(' '))
const isSaving = ref(false)
const deleteModal = useTemplateRef<ComponentExposed<typeof ConfirmModal>>('deleteModal')
const editor = useTemplateRef<ComponentExposed<typeof ServerPropertiesEditor>>('editor')

const requiredJava = computed(() => requiredJavaMajorVersion(props.server.gameVersion))

// Same busy boundary as the files panel: a running server holds its
// configuration in memory and may overwrite external edits, and
// manifest changes only take effect after a restart anyway.
const isRunning = computed(() => props.server.running)

const baseline = ref({
	name: props.server.name,
	iconPath: props.server.iconPath ?? null,
	javaPath: props.server.javaPath ?? '',
	javaVersion: '',
	memoryMb: props.server.memoryMb ?? 2048,
	jvmArgs: (props.server.jvmArgs ?? []).join(' '),
})

onMounted(async () => {
	if (!javaSelection.value.path) return
	try {
		const jre = await get_jre(javaSelection.value.path)
		if (jre) {
			javaSelection.value.version = jre.version
			baseline.value.javaVersion = jre.version
		}
	} catch {
		// Keep the path; the selector validates against the required major version.
	}
})

watch(
	() => props.server.iconPath,
	(value) => {
		const synced = value ?? null
		iconPath.value = synced
		baseline.value.iconPath = synced
	},
)

const generalDirty = computed(
	() =>
		name.value !== baseline.value.name ||
		iconPath.value !== baseline.value.iconPath ||
		javaSelection.value.path !== baseline.value.javaPath ||
		javaSelection.value.version !== baseline.value.javaVersion ||
		memoryMb.value !== baseline.value.memoryMb ||
		jvmArgsText.value !== baseline.value.jvmArgs,
)

const isDirty = computed(() => generalDirty.value || (editor.value?.isDirty ?? false))

async function save() {
	isSaving.value = true
	try {
		const jvmArgs = jvmArgsText.value.trim().split(/\s+/).filter(Boolean)
		const parsedMemory = Number(memoryMb.value)
		const memoryMbValue =
			Number.isFinite(parsedMemory) && parsedMemory > 0 ? parsedMemory : baseline.value.memoryMb
		await serversApi.updateSettings(props.server.id, {
			name: name.value.trim(),
			javaPath: javaSelection.value.path,
			memoryMb: memoryMbValue,
			jvmArgs,
		})
		if (iconPath.value !== baseline.value.iconPath) {
			await serversApi.setIcon(props.server.id, iconPath.value)
		}
		const propsSaved = (await editor.value?.save()) ?? true
		if (!propsSaved) return
		name.value = name.value.trim()
		memoryMb.value = memoryMbValue
		jvmArgsText.value = jvmArgs.join(' ')
		baseline.value = {
			name: name.value,
			iconPath: iconPath.value,
			javaPath: javaSelection.value.path,
			javaVersion: javaSelection.value.version,
			memoryMb: memoryMbValue,
			jvmArgs: jvmArgsText.value,
		}
		await refresh()
		addNotification({ type: 'success', title: formatMessage(messages.saved) })
	} catch (error) {
		handleError(error)
	} finally {
		isSaving.value = false
	}
}

function cancel() {
	name.value = baseline.value.name
	iconPath.value = baseline.value.iconPath
	javaSelection.value = { path: baseline.value.javaPath, version: baseline.value.javaVersion }
	memoryMb.value = baseline.value.memoryMb
	jvmArgsText.value = baseline.value.jvmArgs
	editor.value?.cancel()
}

async function pickIcon() {
	try {
		const picked = await (filePicker.pickInstanceIcon?.() ?? filePicker.pickImage())
		if (picked?.path) iconPath.value = picked.path
	} catch (error) {
		handleError(error)
	}
}

async function confirmDelete() {
	const ok = await deleteServer(props.server.id)
	if (ok) emit('deleted')
}
</script>

<template>
	<div class="flex min-h-full flex-col">
		<div class="flex flex-col gap-6 pb-20">
			<Admonition v-if="isRunning" type="warning" :header="formatMessage(messages.runningTitle)">
				{{ formatMessage(messages.runningHint) }}
			</Admonition>

			<Card data-onboarding-id="server-settings" class="!m-0">
				<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
					<h3 class="m-0 col-span-full text-base font-semibold text-contrast">
						{{ formatMessage(messages.general) }}
					</h3>

					<div class="flex min-w-0 items-center gap-3 sm:col-span-2 xl:col-span-4">
						<ServerIcon
							:icon-path="iconPath"
							:server-type="server.serverType"
							:server-id="server.id"
							size="48px"
						/>
						<div class="flex flex-col gap-1">
							<span class="font-semibold text-contrast">{{ formatMessage(messages.icon) }}</span>
							<div class="flex gap-2">
								<ButtonStyled type="outlined" size="small">
									<button type="button" @click="pickIcon">
										<ImageIcon />
										{{
											iconPath
												? formatMessage(messages.changeIcon)
												: formatMessage(messages.selectIcon)
										}}
									</button>
								</ButtonStyled>
								<ButtonStyled v-if="iconPath" color="red" type="outlined" size="small">
									<button type="button" @click="iconPath = null">
										<TrashIcon />
										{{ formatMessage(messages.removeIcon) }}
									</button>
								</ButtonStyled>
							</div>
						</div>
					</div>

					<label class="flex min-w-0 flex-col gap-2" for="server-settings-name">
						<span class="font-semibold text-contrast">{{ formatMessage(messages.name) }}</span>
						<StyledInput id="server-settings-name" v-model="name" />
					</label>

					<label class="flex min-w-0 flex-col gap-2" for="server-settings-memory">
						<span class="font-semibold text-contrast">{{ formatMessage(messages.memory) }}</span>
						<StyledInput
							id="server-settings-memory"
							v-model="memoryMb"
							inputmode="numeric"
							wrapper-class="max-w-40"
						/>
					</label>

					<div class="flex min-w-0 flex-col gap-2 sm:col-span-2 xl:col-span-2">
						<span class="font-semibold text-contrast">{{ formatMessage(messages.java) }}</span>
						<JavaSelector
							id="server-settings-java"
							v-model="javaSelection"
							:version="requiredJava"
							select-all-versions
						/>
					</div>

					<label
						class="flex min-w-0 flex-col gap-2 sm:col-span-2 xl:col-span-4"
						for="server-settings-jvm"
					>
						<span class="font-semibold text-contrast">{{ formatMessage(messages.jvmArgs) }}</span>
						<StyledInput id="server-settings-jvm" v-model="jvmArgsText" />
						<span class="text-xs text-secondary">{{ formatMessage(messages.jvmArgsHint) }}</span>
					</label>
				</div>
			</Card>

			<Card class="!m-0">
				<ServerPropertiesEditor ref="editor" :server-id="server.id" />
			</Card>

			<Card class="!m-0">
				<div class="flex flex-wrap items-center justify-between gap-3">
					<div class="flex min-w-0 items-start gap-3">
						<div
							class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-red-highlight text-red"
						>
							<TrashIcon class="size-4" />
						</div>
						<div class="min-w-0">
							<h3 class="m-0 text-base font-semibold text-contrast">
								{{ formatMessage(messages.deleteTitle) }}
							</h3>
							<p class="mb-0 mt-1 text-sm text-secondary">
								{{ formatMessage(messages.deleteHint) }}
							</p>
						</div>
					</div>
					<ButtonStyled color="red" type="outlined">
						<button type="button" :disabled="server.running" @click="deleteModal?.show()">
							<TrashIcon />
							{{ formatMessage(messages.deleteTitle) }}
						</button>
					</ButtonStyled>
				</div>
			</Card>
		</div>

		<div
			v-if="isDirty"
			class="fixed bottom-4 z-50 flex"
			:style="{
				left: 'calc(var(--left-bar-width) + 1.5rem)',
				width: 'calc(100% - var(--left-bar-width) - var(--right-bar-width) - 3rem)',
			}"
		>
			<div class="flex w-full items-center justify-end">
				<div
					class="flex items-center gap-2 rounded-xl border border-solid border-button-border bg-bg-raised px-3 py-2 shadow-lg"
				>
					<ButtonStyled type="outlined">
						<button type="button" :disabled="isSaving" @click="cancel">
							<XIcon />
							{{ formatMessage(messages.cancel) }}
						</button>
					</ButtonStyled>
					<ButtonStyled color="brand">
						<button type="button" :disabled="isSaving || isRunning" @click="save">
							<SpinnerIcon v-if="isSaving" class="animate-spin" />
							<SaveIcon v-else />
							{{ formatMessage(messages.save) }}
						</button>
					</ButtonStyled>
				</div>
			</div>
		</div>

		<ConfirmModal
			ref="deleteModal"
			:title="formatMessage(messages.deleteTitle)"
			:description="formatMessage(messages.deleteConfirm, { name: server.name })"
			:proceed-label="formatMessage(messages.deleteProceed)"
			@proceed="confirmDelete"
		/>
	</div>
</template>
