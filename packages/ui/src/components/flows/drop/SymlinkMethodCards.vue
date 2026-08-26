<template>
	<NewModal
		ref="modal"
		max-width="920px"
		:closable="true"
		hide-header
		scrollable
		@hide="onModalHide"
	>
		<div class="flex flex-col gap-4 p-6">
			<div class="flex flex-col gap-1">
				<span class="text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</span>
				<span class="text-sm text-secondary">
					{{ formatMessage(messages.subtitle, { n: instances.length }) }}
				</span>
			</div>

			<HorizontalRule />

			<div
				class="relative grid grid-cols-1 gap-5 min-[640px]:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] min-[640px]:gap-8"
			>
				<div
					class="pointer-events-none absolute inset-y-0 left-[calc(40%+0.2rem)] z-0 hidden w-px bg-divider min-[640px]:block"
					aria-hidden="true"
				/>

				<div class="relative z-[1] flex min-w-0 flex-col gap-4">
					<div
						ref="methodSectionRef"
						class="flex flex-col gap-2"
						:class="{ 'method-shake': methodShake }"
					>
						<span class="text-sm font-semibold text-contrast">{{
							formatMessage(messages.method)
						}}</span>
						<BigOptionButton
							:icon="CopyIcon"
							:title="formatMessage(messages.copyTitle)"
							:description="formatMessage(messages.copyDesc)"
							:selected="method === 'copy'"
							:no-icon-border="true"
							@click="selectMethod('copy')"
						/>
						<BigOptionButton
							v-if="symlinkAllowed"
							:icon="LinkIcon"
							:title="formatMessage(messages.symlinkTitle)"
							:description="formatMessage(messages.symlinkDesc)"
							:note="symlinkNote"
							:selected="method === 'symlink'"
							:no-icon-border="true"
							@click="selectMethod('symlink')"
						/>
						<span v-if="unsupported" class="text-xs text-danger">
							{{ formatMessage(messages.unsupportedWarning) }}
						</span>
					</div>

					<div class="flex flex-col gap-2">
						<div class="flex items-center gap-2">
							<span class="text-sm font-semibold text-contrast">
								{{ formatMessage(messages.statsLabel) }}
							</span>
						</div>
						<ProgressBar v-if="statsLoading" :progress="0" waiting full-width />
						<div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
							<div
								v-for="row in statRows"
								:key="row.key"
								class="flex items-center justify-between gap-2 rounded-lg bg-surface-2 px-3 py-2"
							>
								<span
									v-tooltip="row.tooltip"
									class="shrink-0 whitespace-nowrap text-sm text-secondary"
								>
									{{ row.label }}
								</span>
								<div class="flex flex-col items-end gap-0.5 whitespace-nowrap">
									<span class="text-sm font-semibold text-contrast">{{ row.size }}</span>
									<span class="text-xs text-secondary">{{ row.files }}</span>
								</div>
							</div>
						</div>
					</div>
				</div>

				<div
					class="relative z-[1] flex min-w-0 flex-col gap-4"
					:class="pageAnim ? 'overflow-hidden' : ''"
				>
					<div
						class="flex min-w-0 flex-col gap-4"
						:class="[pageAnimationClass, pageAnim ? 'overflow-hidden' : '']"
					>
						<div class="flex flex-col gap-2">
							<div class="relative flex items-center gap-2">
								<span class="text-sm font-semibold text-contrast">
									{{ formatMessage(messages.loaderLabel) }}
								</span>
								<TagItem
									v-if="warnings.loaderTypeCustom || warnings.loaderTypeUnrecognized"
									class="shrink-0 border !border-solid border-orange"
									:style="{
										'--_bg-color': 'var(--color-orange-bg)',
										'--_color': 'var(--color-orange)',
									}"
								>
									<span
										v-tooltip="
											warnings.loaderTypeCustom
												? formatMessage(messages.customTooltip)
												: formatMessage(messages.loaderTypeUnknownTooltip)
										"
										class="inline-flex items-center gap-1"
									>
										<CircleAlertIcon />
										{{
											warnings.loaderTypeCustom
												? formatMessage(messages.custom)
												: formatMessage(messages.unrecognized)
										}}
									</span>
								</TagItem>
							</div>
							<Chips
								v-model="loader"
								:items="loaderItems"
								:format-label="formatLoader"
								size="small"
							/>
						</div>

						<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
							<div class="flex flex-col gap-2">
								<div class="relative flex items-center gap-2">
									<span class="text-sm font-semibold text-contrast">
										{{ formatMessage(messages.gameVersionLabel) }}
									</span>
									<TagItem
										v-if="warnings.gameVersionCustom"
										class="absolute right-0 top-1/2 -translate-y-1/2 border !border-solid border-orange"
										:style="{
											'--_bg-color': 'var(--color-orange-bg)',
											'--_color': 'var(--color-orange)',
										}"
									>
										<span
											v-tooltip="formatMessage(messages.customTooltip)"
											class="inline-flex items-center gap-1"
										>
											<CircleAlertIcon />
											{{ formatMessage(messages.custom) }}
										</span>
									</TagItem>
								</div>
								<Combobox
									v-model="gameVersion"
									:options="gameVersionOptions"
									:placeholder="
										statsLoading
											? formatMessage(messages.detecting)
											: formatMessage(messages.selectGameVersion)
									"
									:search-placeholder="formatMessage(messages.searchGameVersion)"
									searchable
									@search-input="handleGameVersionSearch"
								/>
							</div>

							<div class="flex flex-col gap-2">
								<div class="relative flex items-center gap-2">
									<span
										class="text-sm font-semibold"
										:class="loader === 'vanilla' ? 'text-secondary' : 'text-contrast'"
									>
										{{ formatMessage(messages.loaderVersionLabel) }}
									</span>
									<TagItem
										v-if="warnings.loaderVersionCustom || warnings.loaderVersionMissing"
										class="absolute right-0 top-1/2 -translate-y-1/2 border !border-solid border-orange"
										:style="{
											'--_bg-color': 'var(--color-orange-bg)',
											'--_color': 'var(--color-orange)',
										}"
									>
										<span
											v-tooltip="
												warnings.loaderVersionCustom
													? formatMessage(messages.customTooltip)
													: formatMessage(messages.loaderVersionUnknownTooltip)
											"
											class="inline-flex items-center gap-1"
										>
											<CircleAlertIcon />
											{{
												warnings.loaderVersionCustom
													? formatMessage(messages.custom)
													: formatMessage(messages.missing)
											}}
										</span>
									</TagItem>
								</div>
								<Combobox
									v-model="loaderVersion"
									:options="loaderVersionOptions"
									:disabled="loader === 'vanilla'"
									:placeholder="formatMessage(messages.selectLoaderVersion)"
									:search-placeholder="formatMessage(messages.searchLoaderVersion)"
									searchable
									@search-input="handleLoaderVersionSearch"
								/>
							</div>
						</div>

						<div class="flex flex-col gap-3">
							<div class="flex flex-col gap-2">
								<span
									v-tooltip="formatMessage(messages.importPathTooltip)"
									class="text-sm font-semibold text-contrast"
								>
									{{ formatMessage(messages.importPathLabel) }}
								</span>
								<StyledInput
									:model-value="activeInstance?.path || formatMessage(messages.missingPath)"
									input-class="!pr-9"
									wrapper-class="w-full"
									readonly
								>
									<template #right>
										<span
											class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-1.5"
										>
											<button
												type="button"
												class="pointer-events-auto flex h-6 w-6 items-center justify-center rounded-md text-secondary hover:text-contrast disabled:opacity-50"
												:disabled="!activeInstance?.path"
												@click="openVersionPath"
											>
												<FolderOpenIcon class="size-4" />
											</button>
										</span>
									</template>
								</StyledInput>
							</div>

							<div class="flex flex-col gap-2">
								<span
									v-tooltip="formatMessage(messages.runtimeRootTooltip)"
									class="text-sm font-semibold text-contrast"
								>
									{{ formatMessage(messages.runtimeRootLabel) }}
								</span>
								<StyledInput
									:model-value="
										activeSnapshot?.minecraftRoot ||
										(statsLoading ? formatMessage(messages.detecting) : '')
									"
									input-class="!pr-9"
									wrapper-class="w-full"
									readonly
								>
									<template #right>
										<span
											class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-1.5"
										>
											<button
												type="button"
												class="pointer-events-auto flex h-6 w-6 items-center justify-center rounded-md text-secondary hover:text-contrast disabled:opacity-50"
												:disabled="!activeSnapshot?.minecraftRoot"
												@click="openRootInFileManager"
											>
												<FolderOpenIcon class="size-4" />
											</button>
										</span>
									</template>
								</StyledInput>
							</div>
						</div>

						<div v-if="method === 'symlink'" class="flex flex-col gap-2">
							<span class="text-sm font-semibold text-contrast">
								{{ formatMessage(messages.gameDirLabel) }}
							</span>
							<RadioButtons
								v-model="gameDirMode"
								:items="gameDirModeItems"
								@update:model-value="setGameDirMode"
							>
								<template #default="{ item }">
									{{ formatMessage(gameDirModeLabel(item)) }}
								</template>
							</RadioButtons>
							<span v-if="gameDirOverride" class="text-sm text-secondary break-all">
								{{ gameDirOverride }}
							</span>
						</div>

						<div v-if="planError" class="text-xs text-danger">{{ planError }}</div>
					</div>

					<nav
						v-if="instances.length > 1"
						class="mt-auto flex justify-end"
						:aria-label="formatMessage(messages.instance)"
					>
						<div
							ref="instanceNavTrackRef"
							class="instance-selector-track flex max-w-full items-center gap-2 overflow-x-auto"
						>
							<button
								v-for="(instance, index) in instances"
								:key="`${instance.name}-${instance.path ?? index}`"
								type="button"
								class="shrink-0 rounded-full transition-all duration-150"
								:class="
									index === activeIndex
										? `h-2 w-8 ${instanceWarnings[index] ? 'bg-orange' : 'bg-brand'}`
										: `size-2 ${
												instanceWarnings[index] ? 'bg-orange' : 'bg-secondary hover:bg-surface-3'
											}`
								"
								:aria-pressed="index === activeIndex"
								:aria-label="formatMessage(messages.instanceNavLabel, { name: instance.name })"
								:data-active="index === activeIndex"
								@click="goTo(index)"
								@focus="scrollFocusedNavIntoView"
							/>
						</div>
					</nav>
				</div>
			</div>
		</div>

		<template #actions>
			<div class="flex w-full items-center justify-between p-4 pt-0">
				<div class="flex items-center gap-2">
					<ButtonStyled type="transparent">
						<button @click="handleCancel">
							{{ formatMessage(messages.cancel) }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent" :disabled="!canReset">
						<button :disabled="!canReset" @click="resetChanges">
							{{ formatMessage(messages.resetChanges) }}
						</button>
					</ButtonStyled>
				</div>
				<div class="flex items-center gap-2">
					<TagItem
						v-if="warnings.hasWarnings"
						class="border !border-solid border-orange"
						:style="{
							'--_bg-color': 'var(--color-orange-bg)',
							'--_color': 'var(--color-orange)',
						}"
					>
						<CircleAlertIcon />
						{{ formatMessage(messages.custom) }}
					</TagItem>
					<ButtonStyled>
						<button class="flex items-center gap-2" @click="handleConfirm">
							{{ confirmLabel }}
						</button>
					</ButtonStyled>
				</div>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CircleAlertIcon, CopyIcon, FolderOpenIcon, LinkIcon } from '@modrinth/assets'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import BigOptionButton from '#ui/components/base/BigOptionButton.vue'
import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import Chips from '#ui/components/base/Chips.vue'
import Combobox, { type ComboboxOption } from '#ui/components/base/Combobox.vue'
import HorizontalRule from '#ui/components/base/HorizontalRule.vue'
import ProgressBar from '#ui/components/base/ProgressBar.vue'
import RadioButtons from '#ui/components/base/RadioButtons.vue'
import StyledInput from '#ui/components/base/StyledInput.vue'
import TagItem from '#ui/components/base/TagItem.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { useFormatBytes } from '#ui/composables/format-bytes.ts'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import {
	type ImportPlanCounts,
	importPlanDefaultGameVersion,
	importPlanDefaultLoader,
	importPlanDefaultLoaderVersion,
	type ImportPlanSnapshot,
	importPlanWarnings,
	injectInstanceImport,
	injectTags,
	reduceImportPlanSnapshot,
	type SymlinkMethodChoice,
	type SymlinkMethodInstance,
} from '#ui/providers'
import { formatLoaderLabel } from '#ui/utils/loaders'

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const instanceImport = injectInstanceImport()
const tags = injectTags()

const messages = defineMessages({
	title: {
		id: 'drop.symlink_method.title',
		defaultMessage: 'Choose import method',
	},
	subtitle: {
		id: 'drop.symlink_method.subtitle',
		defaultMessage: 'Importing {n} instance(s)',
	},
	method: {
		id: 'drop.symlink_method.method',
		defaultMessage: 'Import method',
	},
	gameDirLabel: {
		id: 'drop.symlink_method.game-dir.label',
		defaultMessage: 'Game directory',
	},
	gameDirIsolated: {
		id: 'drop.symlink_method.game-dir.isolated',
		defaultMessage: 'Version isolated',
	},
	gameDirNotIsolated: {
		id: 'drop.symlink_method.game-dir.not-isolated',
		defaultMessage: 'Version shared (.minecraft/)',
	},
	instance: {
		id: 'drop.symlink_method.instance',
		defaultMessage: 'Instance',
	},
	selectInstance: {
		id: 'drop.symlink_method.select_instance',
		defaultMessage: 'Select instance',
	},
	copyTitle: {
		id: 'drop.symlink_method.copy_title',
		defaultMessage: 'Copy files',
	},
	copyDesc: {
		id: 'drop.symlink_method.copy_desc',
		defaultMessage: 'Copy to Axolotl directory',
	},
	symlinkTitle: {
		id: 'drop.symlink_method.symlink_title',
		defaultMessage: 'Symbolic link',
	},
	symlinkDesc: {
		id: 'drop.symlink_method.symlink_desc',
		defaultMessage: 'Reference original location',
	},
	requiresAdmin: {
		id: 'drop.symlink_method.requires_admin',
		defaultMessage:
			'Administrator authorization (UAC) will be requested once when creating the link',
	},
	unsupportedWarning: {
		id: 'drop.symlink_method.unsupported_warning',
		defaultMessage: 'Symbolic links are not supported on this system',
	},
	gameVersionLabel: {
		id: 'drop.symlink_method.game_version',
		defaultMessage: 'Game version',
	},
	selectGameVersion: {
		id: 'drop.symlink_method.select_game_version',
		defaultMessage: 'Select game version',
	},
	searchGameVersion: {
		id: 'drop.symlink_method.search_game_version',
		defaultMessage: 'Search game versions',
	},
	loaderLabel: {
		id: 'drop.symlink_method.loader',
		defaultMessage: 'Loader',
	},
	loaderVersionLabel: {
		id: 'drop.symlink_method.loader_version',
		defaultMessage: 'Loader version',
	},
	latest: {
		id: 'drop.symlink_method.latest',
		defaultMessage: 'Latest',
	},
	selectLoaderVersion: {
		id: 'drop.symlink_method.select_loader_version',
		defaultMessage: 'Select loader version',
	},
	searchLoaderVersion: {
		id: 'drop.symlink_method.search_loader_version',
		defaultMessage: 'Search loader versions',
	},
	detecting: {
		id: 'drop.symlink_method.detecting',
		defaultMessage: 'Detecting...',
	},
	importPathLabel: {
		id: 'drop.symlink_method.import_path',
		defaultMessage: 'Version path',
	},
	importPathTooltip: {
		id: 'drop.symlink_method.import_path_tooltip',
		defaultMessage: 'Import path',
	},
	runtimeRootLabel: {
		id: 'drop.symlink_method.runtime_root',
		defaultMessage: 'Root',
	},
	runtimeRootTooltip: {
		id: 'drop.symlink_method.runtime_root_tooltip',
		defaultMessage: 'Runtime root',
	},
	statsLabel: {
		id: 'drop.symlink_method.stats',
		defaultMessage: 'Estimate',
	},
	statsCache: {
		id: 'drop.symlink_method.stats_cache',
		defaultMessage: 'Cache',
	},
	statsCacheTooltip: {
		id: 'drop.symlink_method.stats_cache_tooltip',
		defaultMessage: 'Axolotl cache',
	},
	statsLocal: {
		id: 'drop.symlink_method.stats_local',
		defaultMessage: 'Reuse',
	},
	statsLocalTooltip: {
		id: 'drop.symlink_method.stats_local_tooltip',
		defaultMessage: 'Local reuse',
	},
	statsNetwork: {
		id: 'drop.symlink_method.stats_network',
		defaultMessage: 'Download',
	},
	statsNetworkTooltip: {
		id: 'drop.symlink_method.stats_network_tooltip',
		defaultMessage: 'Needs download',
	},
	statsMigrate: {
		id: 'drop.symlink_method.stats_migrate',
		defaultMessage: 'Migrate',
	},
	statsMigrateTooltip: {
		id: 'drop.symlink_method.stats_migrate_tooltip',
		defaultMessage: 'Needs migration',
	},
	statsFiles: {
		id: 'drop.symlink_method.stats_files',
		defaultMessage: '{files} files',
	},
	notAvailable: {
		id: 'drop.symlink_method.not_available',
		defaultMessage: 'Unavailable',
	},
	loaderVersionUnknownTooltip: {
		id: 'drop.symlink_method.loader_version_unknown_tooltip',
		defaultMessage: 'Loader version not detected. A custom version will be used.',
	},
	loaderTypeUnknownTooltip: {
		id: 'drop.symlink_method.loader_type_unknown_tooltip',
		defaultMessage: 'Loader not recognized. Mods may not load correctly.',
	},
	missing: {
		id: 'drop.symlink_method.missing',
		defaultMessage: 'Missing',
	},
	unrecognized: {
		id: 'drop.symlink_method.unrecognized',
		defaultMessage: 'Unrecognized',
	},
	custom: {
		id: 'drop.symlink_method.custom',
		defaultMessage: 'Custom',
	},
	customTooltip: {
		id: 'drop.symlink_method.custom_tooltip',
		defaultMessage:
			'Usually you should not customize it, unless you are sure it was detected incorrectly.',
	},
	missingPath: {
		id: 'drop.symlink_method.missing_path',
		defaultMessage: 'No import path available',
	},
	planFailed: {
		id: 'drop.symlink_method.plan_failed',
		defaultMessage: 'Import estimate failed',
	},
	cancel: {
		id: 'drop.symlink_method.cancel',
		defaultMessage: 'Cancel',
	},
	resetChanges: {
		id: 'drop.symlink_method.reset_changes',
		defaultMessage: 'Reset changes',
	},
	confirm: {
		id: 'drop.symlink_method.confirm',
		defaultMessage: 'Confirm',
	},
	next: {
		id: 'drop.symlink_method.next',
		defaultMessage: 'Next',
	},
	instanceNavLabel: {
		id: 'drop.symlink_method.instance_nav_label',
		defaultMessage: 'Switch to {name}',
	},
})

const emit = defineEmits<{
	(e: 'confirm', choices: SymlinkMethodChoice[]): void
	(e: 'cancel'): void
}>()

interface InstanceChoice {
	gameVersion: string
	loader: string
	loaderVersion: string
	touched: {
		gameVersion: boolean
		loader: boolean
		loaderVersion: boolean
	}
}

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const isOpen = ref(false)
const instances = ref<SymlinkMethodInstance[]>([])
const symlinkCapable = ref<'supported' | 'requires_admin' | 'unsupported'>('supported')
const activeIndex = ref(0)
const method = ref<'copy' | 'symlink' | null>(null)
const gameDirMode = ref<'isolated' | 'not-isolated'>('isolated')
const methodSectionRef = ref<HTMLElement | null>(null)
const methodShake = ref(false)
const gameVersion = ref('')
const loader = ref('vanilla')
const loaderVersion = ref('latest')
const loaderVersions = ref<string[]>([])
let loaderVersionRequest = 0
const touched = ref({
	gameVersion: false,
	loader: false,
	loaderVersion: false,
})
const internalUpdating = ref(false)
const snapshots = ref<Record<number, ImportPlanSnapshot | null>>({})
const detectedByInstance = ref<
	Record<number, { gameVersion: string; loader: string; loaderVersion: string; modCount: number }>
>({})
const requestIds = ref<Record<number, string | null>>({})
const scanning = ref<Record<number, boolean>>({})
const planErrors = ref<Record<number, string | null>>({})
const pageAnim = ref<'out-next' | 'out-prev' | 'in-next' | 'in-prev' | null>(null)
const instanceNavTrackRef = ref<HTMLElement | null>(null)
const instanceChoices = ref<Record<number, InstanceChoice>>({})
let unlisten: (() => void) | null = null
let listeningPromise: Promise<void> | null = null
let rescanTimer: number | null = null

const activeInstance = computed(() => instances.value[activeIndex.value])
const activeSnapshot = computed(() => snapshots.value[activeIndex.value] ?? null)
// The `.minecraft` root for the active instance: computed so the game-dir
// override can be derived (isolated -> <root>/versions/<name>, shared -> <root>).
const activeGameRoot = computed(
	() =>
		activeSnapshot.value?.minecraftRoot ||
		activeInstance.value?.basePath ||
		activeInstance.value?.path ||
		null,
)
const gameDirOverride = computed(() => {
	const root = activeGameRoot.value
	if (method.value !== 'symlink' || !root) return null
	return gameDirMode.value === 'isolated'
		? `${root}/versions/${activeInstance.value?.name ?? ''}`
		: root
})
const statsLoading = computed(() => Object.values(scanning.value).some(Boolean))
const planError = computed(() => planErrors.value[activeIndex.value] ?? null)
const pageAnimationClass = computed(() => (pageAnim.value ? `page-${pageAnim.value}` : ''))
const symlinkAllowed = computed(() => symlinkCapable.value !== 'unsupported')
const requiresAdmin = computed(() => symlinkCapable.value === 'requires_admin')
const unsupported = computed(() => symlinkCapable.value === 'unsupported')
const symlinkNote = computed(() =>
	requiresAdmin.value ? formatMessage(messages.requiresAdmin) : undefined,
)
const loaderItems = ['vanilla', 'fabric', 'forge', 'neoforge', 'quilt']
const formatLoader = (item: string) => formatLoaderLabel(item, formatMessage)
const loaderApiName = computed(() => (loader.value === 'neoforge' ? 'neo' : loader.value))

const gameVersionOptions = computed<ComboboxOption<string>[]>(() => {
	const versions = new Set(tags.gameVersions.value.map((version) => version.version))
	if (gameVersion.value) versions.add(gameVersion.value)
	return [...versions].map((version) => ({ value: version, label: version }))
})

const loaderVersionOptions = computed<ComboboxOption<string>[]>(() => {
	const versions = new Set(['latest'])
	if (loaderVersion.value && loaderVersion.value !== 'latest') {
		versions.add(loaderVersion.value)
	}
	for (const version of loaderVersions.value) {
		versions.add(version)
	}
	return [...versions].map((version) => ({
		value: version,
		label: version === 'latest' ? formatMessage(messages.latest) : version,
	}))
})

async function loadLoaderVersions() {
	const apiLoader = loaderApiName.value
	const version = gameVersion.value
	if (loader.value === 'vanilla' || !apiLoader || !version) {
		loaderVersions.value = []
		return
	}

	const request = ++loaderVersionRequest
	try {
		const versions = await instanceImport.getLoaderVersions(apiLoader, version)
		console.debug(
			'[SymlinkMethodCards] loader versions loaded',
			apiLoader,
			version,
			versions.length,
		)
		if (request === loaderVersionRequest) {
			loaderVersions.value = versions
		}
	} catch (error) {
		console.error('[SymlinkMethodCards] load loader versions failed', error)
		if (request === loaderVersionRequest) {
			loaderVersions.value = []
		}
	}
}

const warnings = computed(() =>
	importPlanWarnings(activeSnapshot.value, {
		gameVersion: gameVersion.value,
		loader: loader.value,
		loaderVersion: loaderVersion.value,
		detectedGameVersion: detectedByInstance.value[activeIndex.value]?.gameVersion,
		detectedLoader: detectedByInstance.value[activeIndex.value]?.loader,
		detectedLoaderVersion: detectedByInstance.value[activeIndex.value]?.loaderVersion,
		detectedModCount: detectedByInstance.value[activeIndex.value]?.modCount,
		gameVersionTouched: touched.value.gameVersion,
		loaderTouched: touched.value.loader,
		loaderVersionTouched: touched.value.loaderVersion,
	}),
)

const instanceWarnings = computed(() => {
	const result: Record<number, boolean> = {}
	for (let index = 0; index < instances.value.length; index++) {
		if (index === activeIndex.value) {
			result[index] = warnings.value.hasWarnings
			continue
		}
		const snapshot = snapshots.value[index] ?? null
		const saved = instanceChoices.value[index]
		const detected = detectedByInstance.value[index]
		result[index] = importPlanWarnings(snapshot, {
			gameVersion:
				saved?.gameVersion ??
				detected?.gameVersion ??
				importPlanDefaultGameVersion(snapshot?.gameVersion),
			loader: saved?.loader ?? detected?.loader ?? importPlanDefaultLoader(snapshot?.loader),
			loaderVersion:
				saved?.loaderVersion ??
				detected?.loaderVersion ??
				importPlanDefaultLoaderVersion(snapshot?.loaderVersion),
			detectedGameVersion: detected?.gameVersion,
			detectedLoader: detected?.loader,
			detectedLoaderVersion: detected?.loaderVersion,
			detectedModCount: detected?.modCount,
		}).hasWarnings
	}
	return result
})

const canReset = computed(
	() => touched.value.gameVersion || touched.value.loader || touched.value.loaderVersion,
)

function selectMethod(value: 'copy' | 'symlink') {
	method.value = value
}

const gameDirModeItems = ['isolated', 'not-isolated'] as const

function gameDirModeLabel(mode: (typeof gameDirModeItems)[number]) {
	switch (mode) {
		case 'isolated':
			return messages.gameDirIsolated
		default:
			return messages.gameDirNotIsolated
	}
}

function setGameDirMode(mode: (typeof gameDirModeItems)[number]) {
	gameDirMode.value = mode
}

function resetChanges() {
	if (!canReset.value) return
	const snapshot = activeSnapshot.value
	const detected = detectedByInstance.value[activeIndex.value]
	internalUpdating.value = true
	touched.value = {
		gameVersion: false,
		loader: false,
		loaderVersion: false,
	}
	gameVersion.value = detected?.gameVersion ?? importPlanDefaultGameVersion(snapshot?.gameVersion)
	loader.value = detected?.loader ?? importPlanDefaultLoader(snapshot?.loader)
	loaderVersion.value =
		detected?.loaderVersion ?? importPlanDefaultLoaderVersion(snapshot?.loaderVersion)
	internalUpdating.value = false
	scheduleRescan()
}

async function openRootInFileManager() {
	const path = activeSnapshot.value?.minecraftRoot
	if (!path) return
	try {
		await instanceImport.openPath(path)
	} catch (error) {
		console.error('[SymlinkMethodCards] open root failed', error)
	}
}

async function openVersionPath() {
	const path = activeInstance.value?.path
	if (!path) return
	try {
		await instanceImport.openPath(path)
	} catch (error) {
		console.error('[SymlinkMethodCards] open version path failed', error)
	}
}

function formatCounts(counts?: ImportPlanCounts | null) {
	if (!counts) return formatMessage(messages.notAvailable)
	return formatBytes(counts.bytes)
}

function formatFiles(count: number): string {
	return formatMessage(messages.statsFiles, { files: count })
}

const totalCounts = computed(() => {
	let hasAny = false
	const totals = {
		cache: { files: 0, bytes: 0 },
		local: { files: 0, bytes: 0 },
		network: { files: 0, bytes: 0 },
		migrate: { files: 0, bytes: 0 },
	}
	for (const snapshot of Object.values(snapshots.value)) {
		if (!snapshot) continue
		hasAny = true
		totals.cache.files += snapshot.cache.files
		totals.cache.bytes += snapshot.cache.bytes
		totals.local.files += snapshot.local.files
		totals.local.bytes += snapshot.local.bytes
		totals.network.files += snapshot.network.files
		totals.network.bytes += snapshot.network.bytes
		totals.migrate.files += snapshot.migrate.files
		totals.migrate.bytes += snapshot.migrate.bytes
	}
	return hasAny ? totals : null
})

const statRows = computed(() =>
	[
		{
			key: 'cache',
			label: formatMessage(messages.statsCache),
			tooltip: formatMessage(messages.statsCacheTooltip),
			counts: totalCounts.value?.cache,
		},
		{
			key: 'local',
			label: formatMessage(messages.statsLocal),
			tooltip: formatMessage(messages.statsLocalTooltip),
			counts: totalCounts.value?.local,
		},
		{
			key: 'network',
			label: formatMessage(messages.statsNetwork),
			tooltip: formatMessage(messages.statsNetworkTooltip),
			counts: totalCounts.value?.network,
		},
		{
			key: 'migrate',
			label: formatMessage(messages.statsMigrate),
			tooltip: formatMessage(messages.statsMigrateTooltip),
			counts: totalCounts.value?.migrate,
		},
	].map((row) => ({
		...row,
		size: formatCounts(row.counts),
		files: row.counts ? formatFiles(row.counts.files) : formatMessage(messages.notAvailable),
	})),
)

const confirmLabel = computed(() =>
	instances.value.length > 1 && activeIndex.value < instances.value.length - 1
		? formatMessage(messages.next)
		: formatMessage(messages.confirm),
)

function cancelPlan(index: number) {
	const requestId = requestIds.value[index]
	requestIds.value[index] = null
	scanning.value[index] = false
	if (requestId) {
		console.debug('[SymlinkMethodCards] cancel import plan', index, requestId)
		void instanceImport.cancelImportPlan(requestId).catch((error) => {
			console.error('[SymlinkMethodCards] cancel import plan failed', index, requestId, error)
		})
	}
}

function scheduleRescan(index = activeIndex.value) {
	if (!isOpen.value || internalUpdating.value) return
	if (rescanTimer !== null) window.clearTimeout(rescanTimer)
	rescanTimer = window.setTimeout(() => {
		rescanTimer = null
		if (!isOpen.value) return
		if (index === activeIndex.value) saveCurrentChoices()
		void startPlan(index)
	}, 500)
}

async function ensureListening() {
	if (unlisten) return
	if (listeningPromise) return listeningPromise
	listeningPromise = (async () => {
		try {
			unlisten = await instanceImport.listenImportPlan(handleSnapshot)
			console.debug('[SymlinkMethodCards] import plan listener ready')
		} catch (error) {
			console.error('[SymlinkMethodCards] listenImportPlan failed', error)
			planErrors.value[activeIndex.value] = String(error)
		} finally {
			listeningPromise = null
		}
	})()
	return listeningPromise
}

function stopListening() {
	unlisten?.()
	unlisten = null
	if (listeningPromise) {
		listeningPromise = null
	}
}

function saveCurrentChoices() {
	instanceChoices.value[activeIndex.value] = {
		gameVersion: gameVersion.value,
		loader: loader.value,
		loaderVersion: loaderVersion.value,
		touched: { ...touched.value },
	}
}

function resetActiveFields() {
	method.value = null
	gameVersion.value = ''
	loader.value = 'vanilla'
	loaderVersion.value = 'latest'
	touched.value = {
		gameVersion: false,
		loader: false,
		loaderVersion: false,
	}
}

async function startPlan(index = activeIndex.value) {
	if (!isOpen.value) return
	cancelPlan(index)
	const instance = instances.value[index]
	if (!instance?.path) {
		console.warn('[SymlinkMethodCards] no import path', index, instance)
		snapshots.value[index] = null
		scanning.value[index] = false
		planErrors.value[index] = formatMessage(messages.missingPath)
		return
	}

	const requestId = crypto.randomUUID()
	requestIds.value[index] = requestId
	snapshots.value[index] = null
	scanning.value[index] = true
	planErrors.value[index] = null

	try {
		await ensureListening()
		const saved = instanceChoices.value[index]
		const request = {
			requestId,
			launcherType: instance.launcherType ?? 'Generic',
			basePath: instance.basePath ?? '',
			instanceFolder: instance.name,
			instancePath: instance.path,
			gameVersion: saved?.touched.gameVersion ? saved.gameVersion : null,
			loader: saved?.touched.loader ? saved.loader : null,
			loaderVersion: saved?.touched.loaderVersion ? saved.loaderVersion : null,
		}
		console.debug('[SymlinkMethodCards] start import plan', index, request)
		await instanceImport.startImportPlan(request)
		console.debug('[SymlinkMethodCards] import plan started', index, requestId)
	} catch (error) {
		console.error('[SymlinkMethodCards] start import plan failed', error)
		if (requestIds.value[index] !== requestId) return
		requestIds.value[index] = null
		scanning.value[index] = false
		planErrors.value[index] = String(error)
	}
}

async function startAllPlans() {
	// TODO(B2): re-estimate cache/local/network incrementally when overrides
	// change, without rescanning the whole .minecraft folder.
	for (let index = 0; index < instances.value.length; index++) {
		if (!isOpen.value) return
		await startPlan(index)
	}
}

const pendingSnapshots = new Map<number, ImportPlanSnapshot>()
let snapshotFlushPending = false

function handleSnapshot(snapshot: ImportPlanSnapshot) {
	let index = Object.entries(requestIds.value).findIndex(
		([, requestId]) => requestId === snapshot.requestId,
	)
	if (index === -1) {
		index = instances.value.findIndex(
			(instance) => instance.path && snapshot.importPath === instance.path,
		)
	}
	if (index === -1 || requestIds.value[index] !== snapshot.requestId) return

	pendingSnapshots.set(index, snapshot)
	if (!snapshotFlushPending) {
		snapshotFlushPending = true
		requestAnimationFrame(() => {
			snapshotFlushPending = false
			const batch = [...pendingSnapshots.entries()]
			pendingSnapshots.clear()
			for (const [batchIndex, batchSnapshot] of batch) {
				applyImportPlanSnapshot(batchIndex, batchSnapshot)
			}
		})
	}
}

function applyImportPlanSnapshot(index: number, snapshot: ImportPlanSnapshot) {
	if (requestIds.value[index] !== snapshot.requestId) return
	if (snapshot.stage === 'done' || snapshot.stage === 'error') {
		console.debug('[SymlinkMethodCards] import plan final snapshot', index, snapshot)
	}
	const detected = detectedByInstance.value[index]
	if (snapshot.gameVersion) {
		if (!detected) {
			detectedByInstance.value[index] = {
				gameVersion: importPlanDefaultGameVersion(snapshot.gameVersion),
				loader: importPlanDefaultLoader(snapshot.loader),
				loaderVersion: importPlanDefaultLoaderVersion(snapshot.loaderVersion),
				modCount: snapshot.modCount,
			}
		} else if (snapshot.modCount > detected.modCount) {
			detected.modCount = snapshot.modCount
		}
	}

	const merged = reduceImportPlanSnapshot(
		snapshots.value[index] ?? null,
		snapshot,
		snapshot.requestId,
	)
	if (merged) snapshots.value[index] = merged
	scanning.value[index] = snapshot.stage === 'resolving' || snapshot.stage === 'scanning'
	planErrors.value[index] =
		snapshot.stage === 'error' ? (snapshot.error ?? formatMessage(messages.planFailed)) : null

	internalUpdating.value = true
	if (index === activeIndex.value) {
		if (!touched.value.gameVersion && snapshot.gameVersion) {
			gameVersion.value = importPlanDefaultGameVersion(snapshot.gameVersion)
		}
		if (!touched.value.loader && snapshot.loader) {
			loader.value = importPlanDefaultLoader(snapshot.loader)
		}
		if (!touched.value.loaderVersion && snapshot.loaderVersion) {
			loaderVersion.value = importPlanDefaultLoaderVersion(snapshot.loaderVersion)
		}
	}
	internalUpdating.value = false

	if (index === activeIndex.value && snapshot.stage === 'done') {
		void loadLoaderVersions()
	}
}

async function goTo(index: number) {
	if (
		pageAnim.value ||
		index === activeIndex.value ||
		index < 0 ||
		index >= instances.value.length
	) {
		return
	}
	saveCurrentChoices()
	const direction = index > activeIndex.value ? 'next' : 'prev'
	pageAnim.value = `out-${direction}`
	await wait(150)
	if (!isOpen.value) return

	internalUpdating.value = true
	activeIndex.value = index
	loaderVersions.value = []
	const saved = instanceChoices.value[index]
	const snapshot = snapshots.value[index] ?? null
	const detected = detectedByInstance.value[index]
	gameVersion.value = saved?.touched.gameVersion
		? saved.gameVersion
		: (detected?.gameVersion ?? importPlanDefaultGameVersion(snapshot?.gameVersion))
	loader.value = saved?.touched.loader
		? saved.loader
		: (detected?.loader ?? importPlanDefaultLoader(snapshot?.loader))
	loaderVersion.value = saved?.touched.loaderVersion
		? saved.loaderVersion
		: (detected?.loaderVersion ?? importPlanDefaultLoaderVersion(snapshot?.loaderVersion))
	touched.value = saved?.touched ?? { gameVersion: false, loader: false, loaderVersion: false }
	internalUpdating.value = false

	await nextTick()
	if (!isOpen.value) return
	pageAnim.value = `in-${direction}`
	await wait(160)
	pageAnim.value = null
	scrollActiveNavIntoView()
}

function wait(ms: number) {
	return new Promise<void>((resolve) => window.setTimeout(resolve, ms))
}

function scrollActiveNavIntoView() {
	const track = instanceNavTrackRef.value
	if (!track) return
	const active = track.querySelector<HTMLElement>('[data-active="true"]')
	if (active) scrollNavElementIntoView(active)
}

function scrollFocusedNavIntoView(event: FocusEvent) {
	if (event.currentTarget instanceof HTMLElement) {
		scrollNavElementIntoView(event.currentTarget)
	}
}

function scrollNavElementIntoView(element: HTMLElement) {
	const track = instanceNavTrackRef.value
	if (!track) return
	const trackRect = track.getBoundingClientRect()
	const elementRect = element.getBoundingClientRect()
	if (elementRect.left < trackRect.left) {
		track.scrollLeft -= trackRect.left - elementRect.left
	} else if (elementRect.right > trackRect.right) {
		track.scrollLeft += elementRect.right - trackRect.right
	}
}

function handleGameVersionSearch(query: string) {
	touched.value.gameVersion = true
	gameVersion.value = query.trim()
}

function handleLoaderVersionSearch(query: string) {
	touched.value.loaderVersion = true
	loaderVersion.value = query.trim()
}

function handleConfirm() {
	if (instances.value.length === 0) return
	if (instances.value.length > 1 && activeIndex.value < instances.value.length - 1) {
		void goTo(activeIndex.value + 1)
		return
	}
	if (!method.value) {
		triggerMethodShake()
		return
	}
	saveCurrentChoices()
	const choices: SymlinkMethodChoice[] = instances.value.map((instance, index) => {
		const saved = instanceChoices.value[index]
		const snapshot = snapshots.value[index] ?? null
		const root = snapshot?.minecraftRoot || instance.basePath || instance.path || null
		return {
			instanceName: instance.name,
			instancePath: instance.path,
			symlink: method.value === 'symlink',
			gameVersion:
				saved?.gameVersion || importPlanDefaultGameVersion(snapshot?.gameVersion) || null,
			loader: saved?.loader || importPlanDefaultLoader(snapshot?.loader) || null,
			loaderVersion:
				saved?.loaderVersion || importPlanDefaultLoaderVersion(snapshot?.loaderVersion) || null,
			gameDirOverride:
				method.value === 'symlink' && root
					? gameDirMode.value === 'isolated'
						? `${root}/versions/${instance.name}`
						: root
					: null,
		}
	})

	stopAllPlans()
	isOpen.value = false
	modal.value?.hide()
	emit('confirm', choices)
}

function triggerMethodShake() {
	methodShake.value = false
	requestAnimationFrame(() => {
		methodShake.value = true
		window.setTimeout(() => {
			methodShake.value = false
		}, 450)
	})
}

function handleCancel() {
	stopAllPlans()
	isOpen.value = false
	modal.value?.hide()
	emit('cancel')
}

function onModalHide() {
	if (!isOpen.value) return
	stopAllPlans()
	isOpen.value = false
	emit('cancel')
}

function show(options: {
	instances: SymlinkMethodInstance[]
	symlinkCapable: 'supported' | 'requires_admin' | 'unsupported'
}) {
	console.debug('[SymlinkMethodCards] show', options)
	if (rescanTimer !== null) {
		window.clearTimeout(rescanTimer)
		rescanTimer = null
	}
	pendingSnapshots.clear()
	snapshotFlushPending = false
	instances.value = options.instances
	symlinkCapable.value = options.symlinkCapable
	instanceChoices.value = {}
	snapshots.value = {}
	detectedByInstance.value = {}
	requestIds.value = {}
	scanning.value = {}
	planErrors.value = {}
	pageAnim.value = null
	internalUpdating.value = true
	activeIndex.value = 0
	gameDirMode.value = 'isolated'
	resetActiveFields()
	internalUpdating.value = false
	isOpen.value = true
	modal.value?.show()
	void startAllPlans()
}

function hide() {
	if (!isOpen.value) return
	stopAllPlans()
	isOpen.value = false
	modal.value?.hide()
}

function stopAllPlans() {
	if (rescanTimer !== null) {
		window.clearTimeout(rescanTimer)
		rescanTimer = null
	}
	for (let index = 0; index < instances.value.length; index++) {
		cancelPlan(index)
	}
	stopListening()
	scanning.value = {}
}

watch(
	loader,
	() => {
		if (internalUpdating.value || !isOpen.value) return
		touched.value.loader = true
		touched.value.loaderVersion = true
		loaderVersion.value = 'latest'
		scheduleRescan()
	},
	{ flush: 'sync' },
)

watch(
	gameVersion,
	() => {
		if (internalUpdating.value || !isOpen.value) return
		touched.value.gameVersion = true
		scheduleRescan()
	},
	{ flush: 'sync' },
)

watch(
	loaderVersion,
	() => {
		if (internalUpdating.value || !isOpen.value) return
		touched.value.loaderVersion = true
		scheduleRescan()
	},
	{ flush: 'sync' },
)

watch([loader, gameVersion], () => {
	if (!isOpen.value) return
	void loadLoaderVersions()
})

onMounted(async () => {
	await ensureListening()
})

onUnmounted(() => {
	stopAllPlans()
})

defineExpose({ show, hide })
</script>

<style scoped>
.method-shake {
	animation: method-shake 0.45s ease-in-out;
}

@keyframes method-shake {
	0%,
	100% {
		transform: translateX(0);
	}
	20% {
		transform: translateX(-6px);
	}
	40% {
		transform: translateX(6px);
	}
	60% {
		transform: translateX(-4px);
	}
	80% {
		transform: translateX(4px);
	}
}

.page-out-next {
	animation: page-out-next 150ms cubic-bezier(0.4, 0, 1, 1) both;
}

.page-out-prev {
	animation: page-out-prev 150ms cubic-bezier(0.4, 0, 1, 1) both;
}

.page-in-next {
	animation: page-in-next 170ms cubic-bezier(0, 0, 0.2, 1) both;
}

.page-in-prev {
	animation: page-in-prev 170ms cubic-bezier(0, 0, 0.2, 1) both;
}

.instance-selector-track {
	scrollbar-width: none;
}

.instance-selector-track::-webkit-scrollbar {
	display: none;
}

@keyframes page-out-next {
	from {
		transform: translateX(0);
		opacity: 1;
	}
	to {
		transform: translateX(-100%);
		opacity: 0;
	}
}

@keyframes page-out-prev {
	from {
		transform: translateX(0);
		opacity: 1;
	}
	to {
		transform: translateX(100%);
		opacity: 0;
	}
}

@keyframes page-in-next {
	from {
		transform: translateX(100%);
		opacity: 0;
	}
	to {
		transform: translateX(0);
		opacity: 1;
	}
}

@keyframes page-in-prev {
	from {
		transform: translateX(-100%);
		opacity: 0;
	}
	to {
		transform: translateX(0);
		opacity: 1;
	}
}
</style>
