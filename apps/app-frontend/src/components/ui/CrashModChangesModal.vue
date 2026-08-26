<script setup lang="ts">
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import type { CrashAnalysisResult } from '@/composables/useCrashAnalysis'
import { refresh_content } from '@/helpers/instance'
import { undo_added_mod } from '@/helpers/logs'

const modal = ref<InstanceType<typeof NewModal>>()
const analysis = ref<CrashAnalysisResult | null>(null)
const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const busy = ref<string | null>(null)

const messages = defineMessages({
	title: {
		id: 'app.minecraft-crash.mod-changes-modal.title',
		defaultMessage: 'Mod changes since the last successful launch',
	},
	description: {
		id: 'app.minecraft-crash.mod-changes-modal.description',
		defaultMessage: 'This comparison does not restore or modify any files.',
	},
	added: { id: 'app.minecraft-crash.mod-changes-modal.added', defaultMessage: 'Added ({count})' },
	removed: {
		id: 'app.minecraft-crash.mod-changes-modal.removed',
		defaultMessage: 'Removed ({count})',
	},
	modified: {
		id: 'app.minecraft-crash.mod-changes-modal.modified',
		defaultMessage: 'Modified ({count})',
	},
	empty: {
		id: 'app.minecraft-crash.mod-changes-modal.empty',
		defaultMessage: 'No Mod file changes were detected.',
	},
	close: { id: 'app.minecraft-crash.mod-changes-modal.close', defaultMessage: 'Close' },
	undo: { id: 'app.minecraft-crash.mod-changes-modal.undo', defaultMessage: 'Undo added Mod' },
	undone: {
		id: 'app.minecraft-crash.mod-changes-modal.undone',
		defaultMessage: 'Added Mod removed',
	},
	undoConfirm: {
		id: 'app.minecraft-crash.mod-changes-modal.undo-confirm',
		defaultMessage: 'Remove {name}? Only this unchanged file will be deleted.',
	},
	undoFailed: {
		id: 'app.minecraft-crash.mod-changes-modal.undo-failed',
		defaultMessage: 'Could not undo this Mod change',
	},
	refreshFailed: {
		id: 'app.minecraft-crash.mod-changes-modal.refresh-failed',
		defaultMessage: 'Mod removed, but the content list could not be refreshed.',
	},
})

const groups = computed(() =>
	(['added', 'removed', 'modified'] as const).map((kind) => ({
		kind,
		items: (analysis.value?.mod_changes ?? []).filter((change) => change.kind === kind),
	})),
)
const groupMessages = {
	added: messages.added,
	removed: messages.removed,
	modified: messages.modified,
} as const

function show(nextAnalysis: CrashAnalysisResult): void {
	analysis.value = nextAnalysis
	modal.value?.show()
}

async function undo(change: (typeof groups.value)[number]['items'][number]): Promise<void> {
	if (change.kind !== 'added' || busy.value) return
	const currentAnalysis = analysis.value
	if (!currentAnalysis || !change.current_sha256) return
	const name = change.project_title || change.filename
	if (!window.confirm(formatMessage(messages.undoConfirm, { name }))) return
	busy.value = change.filename
	let removed = false
	try {
		await undo_added_mod(currentAnalysis.instance_id, change.filename, change.current_sha256)
		removed = true
		currentAnalysis.mod_changes = currentAnalysis.mod_changes.filter(
			(item) => item.filename !== change.filename,
		)
		addNotification({ title: formatMessage(messages.undone), type: 'success' })
	} catch {
		addNotification({ title: formatMessage(messages.undoFailed), type: 'error' })
	} finally {
		busy.value = null
	}
	if (!removed) return
	try {
		await refresh_content(currentAnalysis.instance_id)
	} catch {
		addNotification({ title: formatMessage(messages.refreshFailed), type: 'warning' })
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)" max-width="680px">
		<div class="flex max-h-[65vh] flex-col gap-4 overflow-y-auto">
			<p class="m-0 text-secondary">{{ formatMessage(messages.description) }}</p>
			<p v-if="!analysis?.mod_changes.length" class="m-0 text-secondary">
				{{ formatMessage(messages.empty) }}
			</p>
			<section v-for="group in groups" v-else :key="group.kind" class="flex flex-col gap-2">
				<h3 class="m-0 text-sm font-semibold text-contrast">
					{{ formatMessage(groupMessages[group.kind], { count: group.items.length }) }}
				</h3>
				<ul v-if="group.items.length" class="m-0 flex list-none flex-col gap-1 p-0">
					<li
						v-for="change in group.items"
						:key="`${group.kind}:${change.filename}`"
						class="rounded-md bg-surface-2 px-3 py-2 text-sm text-secondary"
					>
						<div class="flex min-w-0 items-center gap-2">
							<div class="size-8 shrink-0 overflow-hidden rounded bg-surface-3">
								<img
									v-if="change.icon_url"
									:src="change.icon_url"
									:alt="change.project_title || ''"
									class="size-full object-cover"
								/>
							</div>
							<div class="min-w-0">
								<div
									v-if="change.project_title && change.project_title !== change.filename"
									class="truncate font-sans text-sm text-contrast"
								>
									{{ change.project_title || change.filename }}
								</div>
								<div class="truncate text-xs text-secondary">
									{{ change.version_number ? `v${change.version_number} · ` : ''
									}}{{ change.filename }}
								</div>
							</div>
							<ButtonStyled v-if="change.kind === 'added'" type="outlined">
								<button :disabled="busy === change.filename" @click="undo(change)">
									{{ formatMessage(messages.undo) }}
								</button>
							</ButtonStyled>
						</div>
					</li>
				</ul>
			</section>
		</div>
		<template #actions>
			<div class="flex justify-end">
				<ButtonStyled color="brand">
					<button @click="modal?.hide()">{{ formatMessage(messages.close) }}</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
