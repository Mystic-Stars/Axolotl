<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		max-width="560px"
		scrollable
		actions-divider
	>
		<div class="flex flex-col gap-4" role="radiogroup" :aria-label="formatMessage(messages.header)">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.description, { version: version ?? '' }) }}
			</p>

			<div
				role="radio"
				:aria-checked="selectedMethod === 'builtin'"
				tabindex="0"
				class="rounded-xl border p-4 cursor-pointer transition-colors"
				:class="
					selectedMethod === 'builtin'
						? 'border-brand bg-brand/10'
						: 'border-surface-4 hover:border-contrast/20'
				"
				@click="selectedMethod = 'builtin'"
				@keydown.enter.prevent="selectedMethod = 'builtin'"
				@keydown.space.prevent="selectedMethod = 'builtin'"
			>
				<div class="flex items-center gap-2">
					<div
						class="h-4 w-4 rounded-full border-2 flex items-center justify-center shrink-0"
						:class="
							selectedMethod === 'builtin' ? 'border-brand' : 'border-surface-6'
						"
					>
						<div
							v-if="selectedMethod === 'builtin'"
							class="h-2 w-2 rounded-full bg-brand"
						/>
					</div>
					<span class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.builtinTitle) }}
					</span>
				</div>
				<p class="mt-1 ml-6 text-xs text-secondary">
					{{ formatMessage(messages.builtinDescription) }}
				</p>
			</div>

			<div
				v-if="updateInfo?.packageManager"
				role="radio"
				:aria-checked="selectedMethod === 'packageManager'"
				tabindex="0"
				class="rounded-xl border p-4 cursor-pointer transition-colors"
				:class="
					selectedMethod === 'packageManager'
						? 'border-brand bg-brand/10'
						: 'border-surface-4 hover:border-contrast/20'
				"
				@click="selectedMethod = 'packageManager'"
				@keydown.enter.prevent="selectedMethod = 'packageManager'"
				@keydown.space.prevent="selectedMethod = 'packageManager'"
			>
				<div class="flex items-center gap-2">
					<div
						class="h-4 w-4 rounded-full border-2 flex items-center justify-center shrink-0"
						:class="
							selectedMethod === 'packageManager'
								? 'border-brand'
								: 'border-surface-6'
						"
					>
						<div
							v-if="selectedMethod === 'packageManager'"
							class="h-2 w-2 rounded-full bg-brand"
						/>
					</div>
					<span class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.packageManagerTitle, { manager: updateInfo.packageManagerLabel ?? '' }) }}
					</span>
				</div>
				<p class="mt-1 ml-6 text-xs text-secondary">
					{{ formatMessage(messages.packageManagerDescription) }}
				</p>
				<div
					v-if="selectedMethod === 'packageManager'"
					class="mt-3 ml-6 rounded-lg bg-surface-4 p-3"
				>
					<template v-if="updateInfo.alternateCommand">
						<div
							class="rounded-lg border p-3 cursor-pointer transition-colors mt-2"
							:class="
								selectedAlternatePackage === 'primary'
									? 'border-brand bg-brand/10'
									: 'border-surface-0 hover:border-contrast/20'
							"
							@click="selectedAlternatePackage = 'primary'"
						>
							<div class="flex items-center gap-2">
								<div
									class="h-3.5 w-3.5 rounded-full border-2 flex items-center justify-center shrink-0"
									:class="
										selectedAlternatePackage === 'primary'
											? 'border-brand'
											: 'border-surface-4'
									"
								>
									<div
										v-if="selectedAlternatePackage === 'primary'"
										class="h-1.5 w-1.5 rounded-full bg-brand"
									/>
								</div>
								<code class="text-xs text-contrast break-words leading-relaxed flex-1">
									{{ updateInfo.updateCommand }}
								</code>
								<button
									type="button"
									class="text-xs text-brand hover:underline shrink-0"
									:aria-label="formatMessage(messages.copyCommand)"
									@click.stop="copyCommand(updateInfo.updateCommand)"
								>
									{{ formatMessage(messages.copyCommand) }}
								</button>
							</div>
							<p
								v-if="updateInfo.notes?.[0]"
								class="mt-1 ml-5.5 text-xs text-secondary"
							>
								{{ updateInfo.notes[0] }}
							</p>
						</div>
						<div
							class="rounded-lg border p-3 cursor-pointer transition-colors mt-2"
							:class="
								selectedAlternatePackage === 'alternate'
									? 'border-brand bg-brand/10'
									: 'border-surface-0 hover:border-contrast/20'
							"
							@click="selectedAlternatePackage = 'alternate'"
						>
							<div class="flex items-center gap-2">
								<div
									class="h-3.5 w-3.5 rounded-full border-2 flex items-center justify-center shrink-0"
									:class="
										selectedAlternatePackage === 'alternate'
											? 'border-brand'
											: 'border-surface-4'
									"
								>
									<div
										v-if="selectedAlternatePackage === 'alternate'"
										class="h-1.5 w-1.5 rounded-full bg-brand"
									/>
								</div>
								<code class="text-xs text-contrast break-words leading-relaxed flex-1">
									{{ updateInfo.alternateCommand }}
								</code>
								<button
									type="button"
									class="text-xs text-brand hover:underline shrink-0"
									:aria-label="formatMessage(messages.copyCommand)"
									@click.stop="copyCommand(updateInfo.alternateCommand)"
								>
									{{ formatMessage(messages.copyCommand) }}
								</button>
							</div>
							<p
								v-if="updateInfo.notes?.[1]"
								class="mt-1 ml-5.5 text-xs text-secondary"
							>
								{{ updateInfo.notes[1] }}
							</p>
						</div>
					</template>
					<template v-else>
						<code class="block text-xs text-contrast break-words leading-relaxed">
							{{ updateInfo.updateCommand }}
						</code>
						<button
							type="button"
							class="mt-2 text-xs text-brand hover:underline"
							:aria-label="formatMessage(messages.copyCommand)"
							@click.stop="copyCommand(updateInfo.updateCommand)"
						>
							{{ formatMessage(messages.copyCommand) }}
						</button>
						<ul
							v-if="updateInfo.notes?.length"
							class="mt-3 mb-0 text-xs text-secondary list-disc pl-4"
						>
							<li v-for="note in updateInfo.notes" :key="note">{{ note }}</li>
						</ul>
					</template>
				</div>
			</div>
		</div>

		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<ButtonStyled type="outlined">
					<button type="button" @click="cancel">
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button type="button" @click="confirm">
						{{ formatMessage(messages.confirm) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { ButtonStyled, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

import { copyToClipboard } from '@/helpers/utils.js'

const { formatMessage } = useVIntl()

export interface LinuxUpdateInfo {
	isLinux: boolean
	distribution: string | null
	packageManager: string | null
	packageManagerLabel: string | null
	updateCommand: string | null
	alternateCommand: string | null
	notes: string[]
	needsTerminal: boolean
}

const messages = defineMessages({
	header: {
		id: 'app.smart-update.header',
		defaultMessage: 'Choose Update Method',
	},
	description: {
		id: 'app.smart-update.description',
		defaultMessage:
			'Axolotl Launcher v{version} is available. On Linux, you can update through the built-in updater or your system package manager.',
	},
	builtinTitle: {
		id: 'app.smart-update.builtin-title',
		defaultMessage: 'Built-in Updater (AppImage)',
	},
	builtinDescription: {
		id: 'app.smart-update.builtin-description',
		defaultMessage:
			'Downloads the latest AppImage automatically and installs it when you restart the app. Cryptographic signature verified. No external tools required.',
	},
	packageManagerTitle: {
		id: 'app.smart-update.package-manager-title',
		defaultMessage: 'Update via {manager}',
	},
	packageManagerDescription: {
		id: 'app.smart-update.package-manager-description',
		defaultMessage:
			'Opens a terminal to update via your system package manager. Better integration with your system, managed dependencies, and system-wide updates.',
	},
	copyCommand: {
		id: 'app.smart-update.copy-command',
		defaultMessage: 'Copy',
	},
	alternateCommandLabel: {
		id: 'app.smart-update.alternate-command-label',
		defaultMessage: 'Alternative:',
	},
	confirm: {
		id: 'app.smart-update.confirm',
		defaultMessage: 'Update',
	},
	copied: {
		id: 'app.smart-update.copied',
		defaultMessage: 'Command copied to clipboard',
	},
})

const emit = defineEmits<{
	confirm: [method: 'builtin' | 'packageManager', command: string | null]
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const version = ref<string | null>(null)
const updateInfo = ref<LinuxUpdateInfo | null>(null)
const selectedMethod = ref<'builtin' | 'packageManager'>('builtin')
const selectedAlternatePackage = ref<'primary' | 'alternate'>('primary')

async function show(updateVersion: string) {
	version.value = updateVersion
	selectedMethod.value = 'builtin'
	selectedAlternatePackage.value = 'primary'

	try {
		updateInfo.value = await invoke<LinuxUpdateInfo>('get_linux_update_info')
		if (updateInfo.value?.packageManager) {
			selectedMethod.value = 'packageManager'
		}
	} catch (e) {
		console.warn('Failed to detect Linux package manager:', e)
		updateInfo.value = null
	}

	modal.value?.show()
}

function cancel() {
	modal.value?.hide()
}

function confirm() {
	let command: string | null = null
	if (selectedMethod.value === 'packageManager') {
		if (
			updateInfo.value?.alternateCommand
			&& selectedAlternatePackage.value === 'alternate'
		) {
			command = updateInfo.value.alternateCommand
		} else {
			command = updateInfo.value?.updateCommand ?? null
		}
	}
	emit('confirm', selectedMethod.value, command)
	modal.value?.hide()
}

async function copyCommand(command: string | null | undefined) {
	if (!command) return
	await copyToClipboard(command)
}

defineExpose({ show })
</script>
