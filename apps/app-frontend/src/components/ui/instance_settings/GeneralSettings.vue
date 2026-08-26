<script setup lang="ts">
import { CopyIcon, EditIcon, SpinnerIcon, TrashIcon, UploadIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Chips,
	defineMessages,
	injectFilePicker,
	injectNotificationManager,
	OverflowMenu,
	RadioButtons,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { computed, type Ref, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'
import { trackEvent } from '@/helpers/analytics'
import { install_duplicate_instance } from '@/helpers/install'
import { edit, edit_icon, remove } from '@/helpers/instance'
import { injectInstanceSettings } from '@/providers/instance-settings'

import type { GameInstance } from '../../../helpers/types'

const { handleError } = injectNotificationManager()
const filePicker = injectFilePicker()
const { formatMessage } = useVIntl()
const router = useRouter()
const queryClient = useQueryClient()

const deleteConfirmModal = ref()

const { instance } = injectInstanceSettings()
type ReleaseChannel = GameInstance['update_channel']
const releaseChannelOptions: ReleaseChannel[] = ['release', 'beta', 'alpha']

const title = ref(instance.value.name)
const icon: Ref<string | undefined> = ref(instance.value.icon_path)
const savingReleaseChannel = ref(false)
const selectedReleaseChannel = ref<ReleaseChannel>(instance.value.update_channel)
const releaseChannelDisabledItems = computed<ReleaseChannel[]>(() =>
	savingReleaseChannel.value ? [...releaseChannelOptions] : [],
)

const installing = computed(() => instance.value.install_stage !== 'installed')

async function duplicateInstance() {
	await install_duplicate_instance(instance.value.id).catch(handleError)
	trackEvent('InstanceDuplicate', {
		loader: instance.value.loader,
		game_version: instance.value.game_version,
	})
}

function formatReleaseChannelLabel(channel: ReleaseChannel) {
	switch (channel) {
		case 'release':
			return formatMessage(messages.updateChannelRelease)
		case 'beta':
			return formatMessage(messages.updateChannelBeta)
		case 'alpha':
			return formatMessage(messages.updateChannelAlpha)
	}
}

function formatReleaseChannelDescription(channel: ReleaseChannel) {
	switch (channel) {
		case 'release':
			return formatMessage(messages.updateChannelReleaseDescription)
		case 'beta':
			return formatMessage(messages.updateChannelBetaDescription)
		case 'alpha':
			return formatMessage(messages.updateChannelAlphaDescription)
	}
}

watch(
	() => [instance.value.id, instance.value.update_channel] as const,
	() => {
		if (!savingReleaseChannel.value) {
			selectedReleaseChannel.value = instance.value.update_channel
		}
	},
)

watch(selectedReleaseChannel, async (channel, previousChannel) => {
	const previousReleaseChannel = previousChannel ?? instance.value.update_channel
	if (channel === instance.value.update_channel) return

	savingReleaseChannel.value = true
	const instanceId = instance.value.id
	await edit(instanceId, { update_channel: channel })
		.then(() => queryClient.invalidateQueries({ queryKey: ['linkedModpackInfo', instanceId] }))
		.catch((error) => {
			selectedReleaseChannel.value = previousReleaseChannel
			handleError(error)
		})
	savingReleaseChannel.value = false
})

async function resetIcon() {
	icon.value = undefined
	await edit_icon(instance.value.id, null).catch(handleError)
	trackEvent('InstanceRemoveIcon')
}

async function setIcon() {
	try {
		const picked = await (filePicker.pickInstanceIcon?.() ?? filePicker.pickImage())
		if (!picked?.path) return

		const previousIcon = icon.value
		icon.value = picked.path
		try {
			await edit_icon(instance.value.id, picked.path)
			trackEvent('InstanceSetIcon')
		} catch (error) {
			icon.value = previousIcon
			handleError(error)
		}
	} catch (error) {
		handleError(error)
	}
}

const gameDirOverride = ref(instance.value.game_dir_override)
const savingGameDir = ref(false)

watch(
	() => instance.value.game_dir_override,
	(path) => {
		gameDirOverride.value = path
	},
)

// An external game dir is stored as a single path. Whether it is version
// isolated is encoded in the path: `<root>/versions/<name>` vs the `.minecraft`
// root itself. `isExternal` is false for built-in (managed) instances, which
// expose no isolation option.
const isExternal = computed(() => !!gameDirOverride.value)

const gameDirInfo = computed(() => {
	const path = gameDirOverride.value
	if (!path) return { isolated: false, baseRoot: null }
	const normalized = path.replace(/\\/g, '/')
	const segments = normalized.split('/').filter(Boolean)
	if (segments.length >= 2 && segments[segments.length - 2] === 'versions') {
		return { isolated: true, baseRoot: segments.slice(0, -2).join('/') }
	}
	return { isolated: false, baseRoot: path }
})

type GameDirMode = 'isolated' | 'not-isolated'
const gameDirMode = computed<GameDirMode>({
	get: () => (gameDirInfo.value.isolated ? 'isolated' : 'not-isolated'),
	set: (mode) => void setGameDirMode(mode),
})
const gameDirModeItems: GameDirMode[] = ['isolated', 'not-isolated']

function gameDirModeLabel(mode: GameDirMode) {
	return mode === 'isolated' ? messages.gameDirIsolated : messages.gameDirNotIsolated
}

async function setGameDirMode(mode: GameDirMode) {
	const baseRoot = gameDirInfo.value.baseRoot
	if (!baseRoot) return
	const nextPath = mode === 'isolated' ? `${baseRoot}/versions/${instance.value.name}` : baseRoot
	if (nextPath === gameDirOverride.value) return

	// The launcher only records the new override path; the user is responsible
	// for actually moving the mods/saves/config folders to match.
	const previous = gameDirOverride.value
	gameDirOverride.value = nextPath
	savingGameDir.value = true
	try {
		await edit(instance.value.id, { game_dir_override: nextPath })
	} catch (error) {
		gameDirOverride.value = previous
		handleError(error)
	} finally {
		savingGameDir.value = false
	}
}

const editInstanceObject = computed(() => ({
	name: title.value.trim().substring(0, 32) ?? 'Instance',
}))

watch(
	title,
	async () => {
		if (removing.value) return
		await edit(instance.value.id, editInstanceObject.value).catch(handleError)
	},
	{ deep: true },
)

const removing = ref(false)
async function removeInstance() {
	removing.value = true
	const path = instance.value.id

	trackEvent('InstanceRemove', {
		loader: instance.value.loader,
		game_version: instance.value.game_version,
	})

	await router.push({ path: '/' })
	await remove(path).catch(handleError)
}

const messages = defineMessages({
	icon: {
		id: 'instance.settings.tabs.general.icon',
		defaultMessage: 'Icon',
	},
	name: {
		id: 'instance.settings.tabs.general.name',
		defaultMessage: 'Name',
	},
	editIcon: {
		id: 'instance.settings.tabs.general.edit-icon',
		defaultMessage: 'Edit icon',
	},
	selectIcon: {
		id: 'instance.settings.tabs.general.edit-icon.select',
		defaultMessage: 'Select icon',
	},
	replaceIcon: {
		id: 'instance.settings.tabs.general.edit-icon.replace',
		defaultMessage: 'Replace icon',
	},
	removeIcon: {
		id: 'instance.settings.tabs.general.edit-icon.remove',
		defaultMessage: 'Remove icon',
	},
	duplicateInstance: {
		id: 'instance.settings.tabs.general.duplicate-instance',
		defaultMessage: 'Duplicate instance',
	},
	duplicateInstanceDescription: {
		id: 'instance.settings.tabs.general.duplicate-instance.description',
		defaultMessage: 'Creates a copy of this instance, including worlds, configs, mods, etc.',
	},
	duplicateButtonTooltipInstalling: {
		id: 'instance.settings.tabs.general.duplicate-button.tooltip.installing',
		defaultMessage: 'Cannot duplicate while installing.',
	},
	duplicateButton: {
		id: 'instance.settings.tabs.general.duplicate-button',
		defaultMessage: 'Duplicate',
	},
	gameDir: {
		id: 'instance.settings.tabs.general.game-dir',
		defaultMessage: 'Game directory',
	},
	gameDirDescription: {
		id: 'instance.settings.tabs.general.game-dir.description',
		defaultMessage:
			'Uses a separate folder as the working directory for this instance. The game reads mods, saves, configs, and resource packs from that folder instead of the managed instance folder.',
	},
	gameDirCurrent: {
		id: 'instance.settings.tabs.general.game-dir.current',
		defaultMessage: 'Current directory',
	},
	gameDirIsolated: {
		id: 'instance.settings.tabs.general.game-dir.isolated',
		defaultMessage: 'Version isolated (stored in versions/)',
	},
	gameDirNotIsolated: {
		id: 'instance.settings.tabs.general.game-dir.not-isolated',
		defaultMessage: 'Version shared (.minecraft/)',
	},
	gameDirMoveNote: {
		id: 'instance.settings.tabs.general.game-dir.move-note',
		defaultMessage:
			'Switching isolation only updates the launcher path. Move the mods, saves, and config folders yourself to match.',
	},
	gameDirManagedNote: {
		id: 'instance.settings.tabs.general.game-dir.managed-note',
		defaultMessage: 'This instance uses the Axolotl-managed folder.',
	},
	updateChannel: {
		id: 'instance.settings.tabs.general.update-channel',
		defaultMessage: 'Update channel',
	},
	updateChannelReleaseDescription: {
		id: 'instance.settings.tabs.general.update-channel.release.description',
		defaultMessage: 'Only release versions will be shown as available updates.',
	},
	updateChannelBetaDescription: {
		id: 'instance.settings.tabs.general.update-channel.beta.description',
		defaultMessage: 'Release and beta versions will be shown as available updates.',
	},
	updateChannelAlphaDescription: {
		id: 'instance.settings.tabs.general.update-channel.alpha.description',
		defaultMessage: 'Release, beta, and alpha versions will be shown as available updates.',
	},
	updateChannelRelease: {
		id: 'instance.settings.tabs.general.update-channel.release',
		defaultMessage: 'Release',
	},
	updateChannelBeta: {
		id: 'instance.settings.tabs.general.update-channel.beta',
		defaultMessage: 'Beta',
	},
	updateChannelAlpha: {
		id: 'instance.settings.tabs.general.update-channel.alpha',
		defaultMessage: 'Alpha',
	},
	selectUpdateChannelAriaLabel: {
		id: 'instance.settings.tabs.general.update-channel.select',
		defaultMessage: 'Select update channel',
	},
	deleteInstance: {
		id: 'instance.settings.tabs.general.delete',
		defaultMessage: 'Delete instance',
	},
	deleteInstanceDescription: {
		id: 'instance.settings.tabs.general.delete.description',
		defaultMessage:
			'Permanently deletes an instance from your device, including your worlds, configs, and all installed content. Be careful, as once you delete a instance there is no way to recover it.',
	},
	deleteInstanceButton: {
		id: 'instance.settings.tabs.general.delete.button',
		defaultMessage: 'Delete instance',
	},
	deletingInstanceButton: {
		id: 'instance.settings.tabs.general.deleting.button',
		defaultMessage: 'Deleting...',
	},
})
</script>

<template>
	<ConfirmDeleteInstanceModal
		ref="deleteConfirmModal"
		:symlink-target="instance.symlink_target"
		@delete="removeInstance"
	/>
	<div class="block">
		<div class="float-end ml-10 relative group w-fit">
			<div class="flex flex-col gap-1">
				<span class="text-lg font-semibold text-contrast">
					{{ formatMessage(messages.icon) }}
				</span>
				<div class="group relative w-fit">
					<OverflowMenu
						v-tooltip="formatMessage(messages.editIcon)"
						class="bg-transparent border-none appearance-none p-0 m-0 cursor-pointer group-active:scale-95 transition-transform"
						:options="[
							{
								id: 'select',
								action: () => setIcon(),
							},
							{
								id: 'remove',
								color: 'danger',
								action: () => resetIcon(),
								shown: !!icon,
							},
						]"
					>
						<InstanceIcon
							:icon-path="icon"
							:instance-id="instance.id"
							:loader="instance.loader"
							size="108px"
							class="transition-[filter] group-hover:brightness-75"
							no-shadow
						/>
						<div
							class="absolute top-0 h-full w-full flex items-center justify-center opacity-0 transition-all group-hover:opacity-100"
						>
							<EditIcon aria-hidden="true" class="h-10 w-10 text-primary" />
						</div>
						<template #select>
							<UploadIcon />
							{{ icon ? formatMessage(messages.replaceIcon) : formatMessage(messages.selectIcon) }}
						</template>
						<template #remove> <TrashIcon /> {{ formatMessage(messages.removeIcon) }} </template>
					</OverflowMenu>
				</div>
			</div>
		</div>
		<label for="instance-name" class="m-0 text-lg font-semibold text-contrast block">
			{{ formatMessage(messages.name) }}
		</label>
		<div class="flex">
			<StyledInput
				id="instance-name"
				v-model="title"
				autocomplete="off"
				:maxlength="80"
				wrapper-class="flex-grow"
			/>
		</div>
		<template v-if="instance.install_stage == 'installed'">
			<div class="flex flex-col gap-2.5 mt-6">
				<h2 id="duplicate-instance-label" class="m-0 text-lg font-semibold text-contrast block">
					{{ formatMessage(messages.duplicateInstance) }}
				</h2>
				<ButtonStyled>
					<button
						v-tooltip="installing ? formatMessage(messages.duplicateButtonTooltipInstalling) : null"
						aria-labelledby="duplicate-instance-label"
						:disabled="installing"
						class="w-max !shadow-none"
						@click="duplicateInstance"
					>
						<CopyIcon /> {{ formatMessage(messages.duplicateButton) }}
					</button>
				</ButtonStyled>
				<p class="m-0">
					{{ formatMessage(messages.duplicateInstanceDescription) }}
				</p>
			</div>
		</template>
		<div class="flex flex-col gap-2.5 mt-6">
			<h2 class="m-0 text-lg font-semibold text-contrast block">
				{{ formatMessage(messages.gameDir) }}
			</h2>
			<p class="m-0">
				{{ formatMessage(messages.gameDirDescription) }}
			</p>
			<template v-if="isExternal">
				<div class="flex flex-col gap-1.5">
					<RadioButtons v-model="gameDirMode" :items="gameDirModeItems" force-selection>
						<template #default="{ item }">
							{{ formatMessage(gameDirModeLabel(item)) }}
						</template>
					</RadioButtons>
				</div>
				<p v-if="gameDirOverride" class="m-0 text-secondary break-all">
					{{ formatMessage(messages.gameDirCurrent) }}:
					<code>{{ gameDirOverride }}</code>
				</p>
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.gameDirMoveNote) }}
				</p>
			</template>
			<p v-else class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.gameDirManagedNote) }}
			</p>
		</div>
		<div class="flex flex-col gap-2.5 mt-6">
			<h2 class="m-0 text-lg font-semibold text-contrast block">
				{{ formatMessage(messages.updateChannel) }}
			</h2>
			<Chips
				v-model="selectedReleaseChannel"
				:items="releaseChannelOptions"
				:format-label="formatReleaseChannelLabel"
				:capitalize="false"
				:disabled-items="releaseChannelDisabledItems"
				:aria-label="formatMessage(messages.selectUpdateChannelAriaLabel)"
			/>
			<p class="m-0">
				{{ formatReleaseChannelDescription(selectedReleaseChannel) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5 mt-6">
			<h2 id="delete-instance-label" class="m-0 text-lg font-semibold text-contrast block">
				{{ formatMessage(messages.deleteInstance) }}
			</h2>
			<ButtonStyled color="red">
				<button
					aria-labelledby="delete-instance-label"
					:disabled="removing"
					class="w-fit !shadow-none"
					@click="deleteConfirmModal.show()"
				>
					<SpinnerIcon v-if="removing" class="animate-spin" />
					<TrashIcon v-else />
					{{
						removing
							? formatMessage(messages.deletingInstanceButton)
							: formatMessage(messages.deleteInstanceButton)
					}}
				</button>
			</ButtonStyled>
			<p class="m-0">
				{{ formatMessage(messages.deleteInstanceDescription) }}
			</p>
		</div>
	</div>
</template>
<style scoped lang="scss">
.hovering-icon-shadow {
	box-shadow: var(--shadow-inset-sm), var(--shadow-raised);
}
</style>
