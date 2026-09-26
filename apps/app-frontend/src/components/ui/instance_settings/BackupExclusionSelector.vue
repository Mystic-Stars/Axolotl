<script setup lang="ts">
import { FileIcon, FolderIcon, MinusIcon, PlusIcon } from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { ref } from 'vue'

import {
	type BackupExclusion,
	type BackupExclusionKind,
	canonicalizeBackupExclusions,
	formatBackupExclusionPath,
	normalizeBackupExclusion,
} from '@/helpers/instance-backup'

const props = defineProps<{
	instanceId: string
	instanceRoot: string
	modelValue: BackupExclusion[]
	disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: BackupExclusion[]] }>()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const picking = ref<BackupExclusionKind | null>(null)

const messages = defineMessages({
	empty: {
		id: 'instance.backups.exclusions.empty',
		defaultMessage: 'No files or folders are excluded.',
	},
	addFile: {
		id: 'instance.backups.exclusions.add-file',
		defaultMessage: 'Add excluded file',
	},
	addFolder: {
		id: 'instance.backups.exclusions.add-folder',
		defaultMessage: 'Add excluded folder',
	},
	remove: {
		id: 'instance.backups.exclusions.remove',
		defaultMessage: 'Remove {path} from exclusions',
	},
})

function pickedPath(result: unknown): string | null {
	if (typeof result === 'string') return result
	if (result && typeof result === 'object' && 'path' in result) {
		const path = (result as { path?: unknown }).path
		return typeof path === 'string' ? path : null
	}
	return null
}

async function addExclusion(kind: BackupExclusionKind) {
	if (props.disabled || picking.value) return
	picking.value = kind
	try {
		const result = await open({
			directory: kind === 'directory',
			multiple: false,
			defaultPath: props.instanceRoot,
		})
		const selectedPath = pickedPath(result)
		if (!selectedPath) return
		const exclusion = await normalizeBackupExclusion(props.instanceId, selectedPath, kind)
		emit('update:modelValue', canonicalizeBackupExclusions([...props.modelValue, exclusion]))
	} catch (error) {
		handleError(error)
	} finally {
		picking.value = null
	}
}

function removeExclusion(path: string) {
	emit(
		'update:modelValue',
		props.modelValue.filter((exclusion) => exclusion.path !== path),
	)
}
</script>

<template>
	<div class="flex flex-col gap-3">
		<p v-if="modelValue.length === 0" class="m-0 text-sm text-[var(--color-text-tertiary)]">
			{{ formatMessage(messages.empty) }}
		</p>
		<div
			v-else
			class="divide-y divide-solid divide-surface-4 rounded-lg border border-solid border-surface-4"
		>
			<div
				v-for="exclusion in modelValue"
				:key="exclusion.path"
				class="flex min-h-11 items-center gap-3 px-3 py-2"
			>
				<FolderIcon
					v-if="exclusion.kind === 'directory'"
					class="size-5 shrink-0 text-[var(--color-text-tertiary)]"
				/>
				<FileIcon v-else class="size-5 shrink-0 text-[var(--color-text-tertiary)]" />
				<code class="min-w-0 flex-1 truncate text-sm text-[var(--color-text-primary)]">{{
					formatBackupExclusionPath(exclusion)
				}}</code>
				<Button
					type="quiet"
					size="2xs"
					circular
					icon-only
					:disabled="disabled || picking !== null"
					:aria-label="
						formatMessage(messages.remove, { path: formatBackupExclusionPath(exclusion) })
					"
					@click="removeExclusion(exclusion.path)"
					><MinusIcon />
				</Button>
			</div>
		</div>
		<div class="flex flex-wrap gap-2">
			<Button type="outlined" :disabled="disabled || picking !== null" @click="addExclusion('file')"
				><PlusIcon />
				{{ formatMessage(messages.addFile) }}
			</Button>
			<Button
				type="outlined"
				:disabled="disabled || picking !== null"
				@click="addExclusion('directory')"
				><PlusIcon />
				{{ formatMessage(messages.addFolder) }}
			</Button>
		</div>
	</div>
</template>
