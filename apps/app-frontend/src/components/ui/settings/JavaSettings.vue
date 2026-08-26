<script setup>
import { DownloadIcon, FolderSearchIcon, ListIcon, ScanEyeIcon, SearchIcon } from '@modrinth/assets'
import {
	Checkbox,
	defineMessages,
	injectNotificationManager,
	NewButton as Button,
	Slider,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { platform } from '@tauri-apps/plugin-os'
import { ref, watch } from 'vue'

import JavaArgumentsInput from '@/components/ui/JavaArgumentsInput.vue'
import JavaSelector from '@/components/ui/JavaSelector.vue'
import MemoryAllocationDisplay from '@/components/ui/MemoryAllocationDisplay.vue'
import DownloadJavaModal from '@/components/ui/settings/DownloadJavaModal.vue'
import InstalledJavaModal from '@/components/ui/settings/InstalledJavaModal.vue'
import useMemorySlider from '@/composables/useMemorySlider'
import { trackEvent } from '@/helpers/analytics'
import { collectGcContext } from '@/helpers/gc/context'
import { wait_for_install_job } from '@/helpers/install'
import { getJavaArgumentPresets } from '@/helpers/java-argument-presets'
import {
	find_filtered_jres,
	get_java_default_versions,
	get_jre,
	remove_java_default_version,
	set_java_default_version,
	set_java_version,
} from '@/helpers/jre'
import { get, set } from '@/helpers/settings.ts'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	javaLocation: {
		id: 'app.settings.java.location',
		defaultMessage: 'Java {version} location',
	},
	findJava: {
		id: 'app.settings.java.find-java',
		defaultMessage: 'Find Java',
	},
	deepScan: {
		id: 'app.settings.java.deep-scan',
		defaultMessage: 'Deep Scan',
	},
	manualAdd: {
		id: 'app.settings.java.manual-add',
		defaultMessage: 'Manual Add',
	},
	downloadJava: {
		id: 'app.settings.java.download-java',
		defaultMessage: 'Download Java',
	},
	viewInstalled: {
		id: 'app.settings.java.view-installed',
		defaultMessage: 'View installed Java',
	},
	autoHighPerformanceMode: {
		id: 'app.settings.java.auto-high-performance-mode',
		defaultMessage: 'Automatically use high-performance GPU for Java',
	},
	autoHighPerformanceModeDescription: {
		id: 'app.settings.java.auto-high-performance-mode-description',
		defaultMessage:
			'Uses the high-performance GPU for Minecraft when it launches. Supported on Windows and Linux.',
	},
	scanning: {
		id: 'app.settings.java.scanning',
		defaultMessage: 'Scanning...',
	},
	deepScanConfirm: {
		id: 'app.settings.java.deep-scan-confirm',
		defaultMessage: 'This will scan ALL directories on ALL drives. May take several minutes.',
	},
	scanAnyway: {
		id: 'app.settings.java.scan-anyway',
		defaultMessage: 'Scan Anyway',
	},
	cancel: {
		id: 'app.settings.java.cancel',
		defaultMessage: 'Cancel',
	},
	memory: {
		id: 'app.settings.defaults.memory',
		defaultMessage: 'Memory allocated',
	},
	memoryDescription: {
		id: 'app.settings.defaults.memory-description',
		defaultMessage: 'The memory allocated to each instance when it is run.',
	},
	automaticMemory: {
		id: 'app.settings.defaults.automatic-memory',
		defaultMessage: 'Automatically allocate memory at launch',
	},
	automaticMemoryDescription: {
		id: 'app.settings.defaults.automatic-memory-description',
		defaultMessage: 'Adjusts memory for each launch based on available RAM and installed mods.',
	},
	optimizeMemoryBeforeLaunch: {
		id: 'app.settings.defaults.optimize-memory-before-launch',
		defaultMessage: 'Optimize memory before launching the game',
	},
	optimizeMemoryBeforeLaunchDescription: {
		id: 'app.settings.defaults.optimize-memory-before-launch-description',
		defaultMessage: 'Waits for Windows memory optimization to finish before starting the game.',
	},
	javaArguments: {
		id: 'app.settings.defaults.java-arguments',
		defaultMessage: 'Java arguments',
	},
	javaArgumentsPlaceholder: {
		id: 'app.settings.defaults.java-arguments-placeholder',
		defaultMessage: 'Enter Java arguments...',
	},
})

const supportedJavaVersions = [25, 21, 17, 8]
const javaDefaults = ref({})
const scanning = ref(false)
const scanMode = ref('')
const showDeepScanConfirm = ref(false)
const downloadJavaModal = ref(null)
const installedJavaModal = ref(null)
const defaultSaveQueues = new Map()

const currentPlatform = await platform()
const supportsHighPerformanceMode = ['windows', 'linux'].includes(currentPlatform)
const supportsMemoryOptimization = currentPlatform === 'windows'
const settings = ref(await get().catch(handleError))
const autoHighPerformanceMode = ref(settings.value?.auto_set_java_high_performance_mode ?? false)

const javaArgs = ref((settings.value?.extra_launch_args ?? []).join(' '))

const memory = ref(
	settings.value?.memory
		? { optimize_before_launch: false, ...settings.value.memory }
		: { maximum: 2048, automatic: true, optimize_before_launch: false },
)

let shouldApplyDefaultAuto = (settings.value?.extra_launch_args?.length ?? 0) === 0

const memorySlider = await useMemorySlider().catch(handleError)
const maxMemory = memorySlider?.maxMemory ?? 4096
const snapPoints = memorySlider?.snapPoints ?? []

const gcContext = ref(undefined)

async function updateGcContext() {
	gcContext.value = await collectGcContext(memory.value.maximum, null, null, 0)

	if (shouldApplyDefaultAuto) {
		const autoPreset = getJavaArgumentPresets(gcContext.value).find(
			(preset) => preset.id === 'gc-auto',
		)

		if (autoPreset) {
			javaArgs.value = autoPreset.resolveArgs
				? autoPreset.resolveArgs(gcContext.value)
				: autoPreset.args
		}

		shouldApplyDefaultAuto = false
	}
}

await updateGcContext()

watch(() => memory.value.maximum, updateGcContext)

watch(
	[autoHighPerformanceMode, memory, javaArgs],
	async () => {
		if (!settings.value) return

		settings.value = {
			...settings.value,
			auto_set_java_high_performance_mode: autoHighPerformanceMode.value,
			memory: memory.value,
			extra_launch_args: javaArgs.value.trim().split(/\s+/).filter(Boolean),
		}

		await set(settings.value).catch(handleError)
	},
	{ deep: true },
)

async function reloadDefaults() {
	const defaults = await get_java_default_versions().catch(handleError)
	if (!defaults) return

	javaDefaults.value = Object.fromEntries(
		defaults.map((javaVersion) => [javaVersion.parsed_version, javaVersion]),
	)
}

await reloadDefaults()

async function persistDefault(majorVersion, javaVersion) {
	const path = javaVersion?.path?.trim()
	if (!path) {
		const removed = await remove_java_default_version(majorVersion)
			.then(() => true)
			.catch((error) => {
				handleError(error)
				return false
			})
		if (removed) {
			javaDefaults.value[majorVersion] = undefined
		} else {
			await reloadDefaults()
		}
		return
	}

	const validated = await set_java_default_version(majorVersion, path).catch((error) => {
		handleError(error)
		return null
	})
	if (validated) {
		javaDefaults.value[majorVersion] = validated
	} else {
		await reloadDefaults()
	}
}

function saveDefault(majorVersion, javaVersion) {
	const previous = defaultSaveQueues.get(majorVersion) ?? Promise.resolve()
	const operation = previous.then(() => persistDefault(majorVersion, javaVersion))
	defaultSaveQueues.set(majorVersion, operation)

	return operation.finally(() => {
		if (defaultSaveQueues.get(majorVersion) === operation) {
			defaultSaveQueues.delete(majorVersion)
		}
	})
}

async function runScan(exhaustive) {
	if (exhaustive) {
		showDeepScanConfirm.value = true
		return
	}

	scanning.value = true
	scanMode.value = 'quick'
	trackEvent('JavaQuickScan', { source: 'settings' })
	try {
		await find_filtered_jres(null, false, true, false).catch(handleError)
	} finally {
		scanning.value = false
		scanMode.value = ''
	}
}

async function confirmDeepScan() {
	showDeepScanConfirm.value = false
	scanning.value = true
	scanMode.value = 'deep'
	trackEvent('JavaDeepScan', { source: 'settings' })
	try {
		await find_filtered_jres(null, true, true, true).catch(handleError)
	} finally {
		scanning.value = false
		scanMode.value = ''
	}
}

async function handleManualAdd() {
	const result = await open({ multiple: false })
	if (!result) return

	const filePath = result.path ?? result
	const javaInfo = await get_jre(filePath).catch(handleError)
	if (!javaInfo) return

	await set_java_version(javaInfo).catch(handleError)
	trackEvent('JavaManualSelect', { path: filePath })
}

async function onJavaDownloaded(job) {
	if (job?.job_id) {
		await wait_for_install_job(job.job_id).catch(handleError)
	}
	await reloadDefaults()
}
</script>

<template>
	<DownloadJavaModal ref="downloadJavaModal" @downloaded="onJavaDownloaded" />
	<InstalledJavaModal ref="installedJavaModal" @changed="reloadDefaults" />

	<div class="settings-page flex flex-col gap-6">
		<div
			v-for="(javaVersion, index) in supportedJavaVersions"
			:key="`java-${javaVersion}`"
			class="flex flex-col gap-2.5"
		>
			<h2 class="m-0 text-lg font-semibold text-contrast" :class="{ 'mt-2': index !== 0 }">
				{{ formatMessage(messages.javaLocation, { version: javaVersion }) }}
			</h2>
			<JavaSelector
				:id="`java-selector-${javaVersion}`"
				v-model="javaDefaults[javaVersion]"
				:version="javaVersion"
				@commit="saveDefault(javaVersion, $event)"
			/>
		</div>

		<div class="flex flex-wrap gap-2 border-0 border-t border-solid border-button-border pt-5">
			<Button
				type="base"
				native-type="button"
				class="!shadow-none"
				:disabled="scanning"
				@click="runScan(false)"
			>
				<SearchIcon aria-hidden="true" />
				{{
					scanning && scanMode === 'quick'
						? formatMessage(messages.scanning)
						: formatMessage(messages.findJava)
				}}
			</Button>
			<Button
				type="base"
				native-type="button"
				class="!shadow-none"
				:disabled="scanning"
				@click="runScan(true)"
			>
				<ScanEyeIcon aria-hidden="true" />
				{{
					scanning && scanMode === 'deep'
						? formatMessage(messages.scanning)
						: formatMessage(messages.deepScan)
				}}
			</Button>
			<Button
				type="base"
				native-type="button"
				class="!shadow-none"
				:disabled="scanning"
				@click="handleManualAdd"
			>
				<FolderSearchIcon aria-hidden="true" />
				{{ formatMessage(messages.manualAdd) }}
			</Button>
			<Button
				type="base"
				native-type="button"
				class="!shadow-none"
				:disabled="scanning"
				@click="downloadJavaModal?.show()"
			>
				<DownloadIcon aria-hidden="true" />
				{{ formatMessage(messages.downloadJava) }}
			</Button>
			<Button
				type="base"
				native-type="button"
				class="!shadow-none"
				@click="installedJavaModal?.show()"
			>
				<ListIcon aria-hidden="true" />
				{{ formatMessage(messages.viewInstalled) }}
			</Button>
		</div>

		<div
			v-if="showDeepScanConfirm"
			class="flex flex-col gap-2 rounded-lg border border-warning bg-warning/10 p-3 text-sm"
		>
			<span>{{ formatMessage(messages.deepScanConfirm) }}</span>
			<div class="flex flex-wrap gap-2">
				<Button type="colored" color="red" native-type="button" @click="confirmDeepScan">
					{{ formatMessage(messages.scanAnyway) }}
				</Button>
				<Button type="outlined" native-type="button" @click="showDeepScanConfirm = false">
					{{ formatMessage(messages.cancel) }}
				</Button>
			</div>
		</div>

		<div
			v-if="supportsHighPerformanceMode"
			class="border-0 border-t border-solid border-button-border pt-5"
		>
			<div class="flex items-center justify-between gap-4">
				<div class="flex min-w-0 flex-col gap-1">
					<span class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.autoHighPerformanceMode) }}
					</span>
					<span class="text-xs text-secondary">
						{{ formatMessage(messages.autoHighPerformanceModeDescription) }}
					</span>
				</div>
				<Toggle id="auto-java-high-performance-mode" v-model="autoHighPerformanceMode" />
			</div>
		</div>

		<div class="flex flex-col gap-6 border-0 border-t border-solid border-button-border pt-5">
			<div class="flex flex-col gap-2.5">
				<h2
					id="settings-target-java-memory"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.memory) }}
				</h2>

				<Checkbox v-model="memory.automatic" :label="formatMessage(messages.automaticMemory)" />
				<div v-if="supportsMemoryOptimization" class="flex flex-col gap-1">
					<Checkbox
						v-model="memory.optimize_before_launch"
						:label="formatMessage(messages.optimizeMemoryBeforeLaunch)"
					/>
					<p class="m-0 text-xs leading-tight text-secondary">
						{{ formatMessage(messages.optimizeMemoryBeforeLaunchDescription) }}
					</p>
				</div>

				<Slider
					id="max-memory"
					v-model="memory.maximum"
					:disabled="memory.automatic"
					:min="512"
					:max="maxMemory"
					:step="64"
					:snap-points="snapPoints"
					:snap-range="512"
					unit="MB"
				/>

				<p class="m-0 mt-1 leading-tight">
					{{
						formatMessage(
							memory.automatic ? messages.automaticMemoryDescription : messages.memoryDescription,
						)
					}}
				</p>

				<MemoryAllocationDisplay :memory="memory" show-optimize-button />
			</div>

			<div class="flex flex-col gap-2.5">
				<h2
					id="settings-target-java-arguments"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.javaArguments) }}
				</h2>

				<JavaArgumentsInput
					id="java-args"
					v-model="javaArgs"
					:gc-context="gcContext"
					:placeholder="formatMessage(messages.javaArgumentsPlaceholder)"
				/>
			</div>
		</div>
	</div>
</template>

<style scoped>
.settings-page > div {
	padding: var(--gap-lg);
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
}

.settings-page > div:has(.border-warning) {
	padding: var(--gap-md);
	background: var(--color-orange-bg);
}

@media (max-width: 700px) {
	.settings-page :deep(.flex.items-center.justify-between) {
		align-items: flex-start;
		flex-direction: column;
	}
}
</style>
