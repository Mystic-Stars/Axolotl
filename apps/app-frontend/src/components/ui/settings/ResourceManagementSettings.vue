<script setup>
import { BoxIcon, FolderOpenIcon, FolderSearchIcon, PlusIcon, TrashIcon } from '@modrinth/assets'
import {
	Combobox,
	defineMessages,
	IconButton,
	injectNotificationManager,
	Slider,
	StyledInput,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import ConfirmModalWrapper from '@/components/ui/modal/ConfirmModalWrapper.vue'
import { purge_cache_types } from '@/helpers/cache.js'
import { configureCurseForgeManualDownloadWatcher } from '@/helpers/curseforge'
import {
	getMissingContentScannerSettings,
	setMissingContentScannerSettings,
} from '@/helpers/downloads-scanner'
import { get, getProxyConfig, set, setProxyConfig, testProxyConfig } from '@/helpers/settings.ts'
import { showAppDbBackupsFolder } from '@/helpers/utils.js'
import { sync_direct_links } from '@/helpers/instance'
import { useTheming } from '@/store/state'

import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

const props = defineProps({
	scope: {
		type: String,
		default: 'content-downloads',
		validator: (value) =>
			['content-downloads', 'network-multiplayer', 'storage-backups'].includes(value),
	},
})

const { handleError } = injectNotificationManager()
const themeStore = useTheming()
const settings = ref(await get())
const missingContentScannerSettings = ref(getMissingContentScannerSettings())
const minecraftDirectories = ref(loadMinecraftDirectories())
const minecraftDirectoryError = ref(null)
const purgeCacheConfirmModal = ref(null)
const { formatMessage } = useVIntl()

const isPortable = ref(false)

try {
	isPortable.value = await invoke('is_portable_mode')
} catch (error) {
	console.error('Failed to determine portable mode', error)
}

// Proxy settings
/** @type {import('@/helpers/settings.ts').ProxyConfig} */
const proxyConfig = ref({ mode: 'none', url: '', username: '', password: '' })
const proxyTesting = ref(false)
/** @type {import('vue').Ref<string>} */
const proxyTestResult = ref('')

try {
	proxyConfig.value = await getProxyConfig()
} catch (error) {
	console.error('Failed to load proxy configuration', error)
}

async function saveProxyConfig(silent = true) {
	try {
		await setProxyConfig(proxyConfig.value)
	} catch (error) {
		if (!silent) {
			handleError(error)
		}
	}
}

async function testProxy() {
	if (proxyConfig.value.mode === 'custom' && !proxyConfig.value.url?.trim()) {
		proxyTestResult.value = formatMessage(messages.proxyUrlRequired)
		return
	}
	proxyTesting.value = true
	proxyTestResult.value = ''
	try {
		const result = await testProxyConfig(proxyConfig.value)
		if (result.success) {
			proxyTestResult.value =
				result.latency_ms != null
					? formatMessage(messages.proxyTestSuccessWithLatency, { latency: result.latency_ms })
					: formatMessage(messages.proxyTestSuccess)
		} else {
			proxyTestResult.value = result.message || formatMessage(messages.proxyTestFailed)
		}
	} catch (error) {
		const message =
			error instanceof Error
				? error.message
				: typeof error === 'object' && error !== null && 'message' in error
					? String(error.message)
					: typeof error === 'string'
						? error
						: JSON.stringify(error)
		proxyTestResult.value = message
	} finally {
		proxyTesting.value = false
	}
}

const messages = defineMessages({
	selectDirectory: {
		id: 'app.settings.resources.select-directory',
		defaultMessage: 'Select a new app directory',
	},
	appDirectory: { id: 'app.settings.resources.app-directory', defaultMessage: 'App directory' },
	axolotlDataDirectory: {
		id: 'app.settings.resources.axolotl-data-directory',
		defaultMessage: 'Axolotl data directory',
	},
	appDirectoryDescription: {
		id: 'app.settings.resources.app-directory-description',
		defaultMessage:
			'The directory where the launcher stores all of its files. Changes apply after restarting the launcher.',
	},
	appDirectoryDescriptionPortable: {
		id: 'app.settings.resources.app-directory-description-portable',
		defaultMessage:
			'You are currently running in portable mode. The app directory is fixed and cannot be changed.',
	},
	minecraftDirectories: {
		id: 'app.settings.resources.minecraft-directories',
		defaultMessage: 'Minecraft directories',
	},
	minecraftDirectoriesDescription: {
		id: 'app.settings.resources.minecraft-directories-description',
		defaultMessage:
			'Add one or more .minecraft folders for instances that use an external game directory. These folders are kept separate from Axolotl data.',
	},
	addMinecraftDirectory: {
		id: 'app.settings.resources.add-minecraft-directory',
		defaultMessage: 'Add .minecraft directory',
	},
	selectMinecraftDirectory: {
		id: 'app.settings.resources.select-minecraft-directory',
		defaultMessage: 'Select a .minecraft directory',
	},
	removeMinecraftDirectory: {
		id: 'app.settings.resources.remove-minecraft-directory',
		defaultMessage: 'Remove .minecraft directory',
	},
	minecraftDirectoryMustEndWith: {
		id: 'app.settings.resources.minecraft-directory-must-end-with',
		defaultMessage: 'The selected folder must be named .minecraft.',
	},
	purgeConfirmTitle: {
		id: 'app.settings.resources.purge-confirm-title',
		defaultMessage: 'Are you sure you want to purge the cache?',
	},
	purgeConfirmDescription: {
		id: 'app.settings.resources.purge-confirm-description',
		defaultMessage:
			'If you proceed, your entire cache will be purged. This may slow down the app temporarily.',
	},
	appCache: { id: 'app.settings.resources.app-cache', defaultMessage: 'App cache' },
	purgeCache: { id: 'app.settings.resources.purge-cache', defaultMessage: 'Purge cache' },
	appCacheDescription: {
		id: 'app.settings.resources.app-cache-description',
		defaultMessage:
			'Axolotl Launcher caches data to speed up loading. Purging it forces the app to reload data and may temporarily slow the app down.',
	},
	downloadMirrors: {
		id: 'app.settings.resources.download-mirrors',
		defaultMessage: 'Download sources',
	},
	downloadMirrorsDescription: {
		id: 'app.settings.resources.download-mirrors-description',
		defaultMessage:
			'Automatic mode chooses between official and mirror sources based on your local environment and recent connection quality.',
	},
	automaticSource: {
		id: 'app.settings.resources.source.automatic',
		defaultMessage: 'Automatic (recommended)',
	},
	officialPreferredSource: {
		id: 'app.settings.resources.source.official-preferred',
		defaultMessage: 'Prefer official sources',
	},
	officialOnlySource: {
		id: 'app.settings.resources.source.official-only',
		defaultMessage: 'Original sources only (no mirrors)',
	},
	openBmclApiSource: {
		id: 'app.settings.resources.source.open-bmcl-api',
		defaultMessage: 'Prefer OpenBMCLAPI',
	},
	tianpaoSource: {
		id: 'app.settings.resources.source.tianpao',
		defaultMessage: 'Prefer Tianpao',
	},
	minecraftMetadataSource: {
		id: 'app.settings.resources.minecraft-metadata-source',
		defaultMessage: 'Minecraft metadata',
	},
	minecraftMetadataSourceDescription: {
		id: 'app.settings.resources.minecraft-metadata-source-description',
		defaultMessage: 'Version manifests and metadata for Minecraft and supported mod loaders.',
	},
	minecraftFileSource: {
		id: 'app.settings.resources.minecraft-file-source',
		defaultMessage: 'Minecraft files, loaders, and Java',
	},
	minecraftFileSourceDescription: {
		id: 'app.settings.resources.minecraft-file-source-description',
		defaultMessage: 'Game files, assets, libraries, mod loaders, and Java runtimes.',
	},
	modrinthMirror: {
		id: 'app.settings.resources.modrinth-mirror',
		defaultMessage: 'Modrinth',
	},
	modrinthMirrorDescription: {
		id: 'app.settings.resources.modrinth-mirror-description',
		defaultMessage: 'Modrinth file downloads.',
	},
	curseforgeMirror: {
		id: 'app.settings.resources.curseforge-mirror',
		defaultMessage: 'CurseForge',
	},
	curseforgeMirrorDescription: {
		id: 'app.settings.resources.curseforge-mirror-description',
		defaultMessage: 'CurseForge file downloads.',
	},
	curseforgeRestrictionBypass: {
		id: 'app.settings.resources.curseforge-restriction-bypass',
		defaultMessage: 'Automatically download restricted CurseForge files',
	},
	curseforgeRestrictionBypassDescription: {
		id: 'app.settings.resources.curseforge-restriction-bypass-description',
		defaultMessage:
			'When CurseForge does not provide a download address, derive its CDN address and try downloading the file automatically. Disable this to use the manual download workflow.',
	},
	mojangAuthService: {
		id: 'app.settings.resources.mojang-auth-service',
		defaultMessage: 'Mojang authentication service',
	},
	mojangAuthServiceDescription: {
		id: 'app.settings.resources.mojang-auth-service-description',
		defaultMessage:
			'Mojang login, profile, skin, cape, and session verification requests used by the launcher and the game.',
	},
	mojangAuthOfficialPreferred: {
		id: 'app.settings.resources.source.mojang-official-preferred',
		defaultMessage: 'Prefer official source',
	},
	mojangAuthMirrorPreferred: {
		id: 'app.settings.resources.source.mojang-mirror-preferred',
		defaultMessage: 'Prefer Fallen-Proxy',
	},
	mojangAuthOfficialOnly: {
		id: 'app.settings.resources.source.mojang-official-only',
		defaultMessage: 'Official source only',
	},
	maximumDownloads: {
		id: 'app.settings.resources.maximum-downloads',
		defaultMessage: 'Maximum concurrent downloads',
	},
	downloadEngine: {
		id: 'app.settings.resources.download-engine',
		defaultMessage: 'Download engine',
	},
	downloadEngineDescription: {
		id: 'app.settings.resources.download-engine-description',
		defaultMessage: 'Choose which download engine the launcher uses.',
	},
	legacyEngine: {
		id: 'app.settings.resources.download-engine.legacy',
		defaultMessage: 'Native engine',
	},
	xmclEngine: {
		id: 'app.settings.resources.download-engine.xmcl',
		defaultMessage: 'XMCL-compatible',
	},
	maximumDownloadsDescription: {
		id: 'app.settings.resources.maximum-downloads-description',
		defaultMessage:
			'Automatic mode uses 64 concurrent downloads. Manual changes apply immediately.',
	},
	manualConcurrency: {
		id: 'app.settings.resources.concurrency.manual',
		defaultMessage: 'Manual',
	},
	maximumWrites: {
		id: 'app.settings.resources.maximum-writes',
		defaultMessage: 'Maximum concurrent writes',
	},
	maximumWritesDescription: {
		id: 'app.settings.resources.maximum-writes-description',
		defaultMessage:
			'The maximum number of files the launcher can write to disk at once. Use a lower value if you frequently get I/O errors. An app restart is required.',
	},
	proxySettings: {
		id: 'app.settings.resources.proxy-settings',
		defaultMessage: 'Proxy settings',
	},
	proxySettingsDescription: {
		id: 'app.settings.resources.proxy-settings-description',
		defaultMessage:
			'Configure how the launcher connects to the internet. System proxy follows your OS settings. Custom proxy lets you specify a URL with optional authentication.',
	},
	proxyMode: {
		id: 'app.settings.resources.proxy-mode',
		defaultMessage: 'Proxy mode',
	},
	proxyModeNone: {
		id: 'app.settings.resources.proxy-mode.none',
		defaultMessage: 'No proxy (direct connection)',
	},
	proxyModeSystem: {
		id: 'app.settings.resources.proxy-mode.system',
		defaultMessage: 'Use system proxy',
	},
	proxyModeCustom: {
		id: 'app.settings.resources.proxy-mode.custom',
		defaultMessage: 'Custom proxy',
	},
	proxyUrl: {
		id: 'app.settings.resources.proxy-url',
		defaultMessage: 'Proxy URL',
	},
	proxyUrlRequired: {
		id: 'app.settings.resources.proxy-url-required',
		defaultMessage: 'Proxy URL is required',
	},
	proxyUrlPlaceholder: {
		id: 'app.settings.resources.proxy-url-placeholder',
		defaultMessage: 'http/https/socks5://ip:port',
	},
	proxyUsername: {
		id: 'app.settings.resources.proxy-username',
		defaultMessage: 'Username',
	},
	proxyUsernamePlaceholder: {
		id: 'app.settings.resources.proxy-username-placeholder',
		defaultMessage: 'Username (optional)',
	},
	proxyPassword: {
		id: 'app.settings.resources.proxy-password',
		defaultMessage: 'Password',
	},
	proxyPasswordPlaceholder: {
		id: 'app.settings.resources.proxy-password-placeholder',
		defaultMessage: 'Password (optional)',
	},
	proxyTest: {
		id: 'app.settings.resources.proxy-test',
		defaultMessage: 'Test connection',
	},
	proxyTesting: {
		id: 'app.settings.resources.proxy-testing',
		defaultMessage: 'Testing...',
	},
	proxyTestSuccess: {
		id: 'app.settings.resources.proxy-test-success',
		defaultMessage: 'Connection successful',
	},
	proxyTestSuccessWithLatency: {
		id: 'app.settings.resources.proxy-test-success-latency',
		defaultMessage: 'Connection successful ({latency} ms)',
	},
	proxyTestFailed: {
		id: 'app.settings.resources.proxy-test-failed',
		defaultMessage: 'Connection failed',
	},
	missingContentAutoImport: {
		id: 'app.settings.resources.missing-content-auto-import',
		defaultMessage: 'Automatically import missing modpack files',
	},
	missingContentAutoImportDescription: {
		id: 'app.settings.resources.missing-content-auto-import-description',
		defaultMessage:
			'Watch one folder while resolving missing modpack files, then verify and import matching files automatically.',
	},
	missingContentImportDirectory: {
		id: 'app.settings.resources.missing-content-import-directory',
		defaultMessage: 'Monitored import folder',
	},
	missingContentImportDirectoryDescription: {
		id: 'app.settings.resources.missing-content-import-directory-description',
		defaultMessage:
			'The system Downloads folder is used when no custom folder is selected. Subfolders are not scanned.',
	},
	systemDownloadsDirectory: {
		id: 'app.settings.resources.system-downloads-directory',
		defaultMessage: 'System Downloads folder',
	},
	selectImportDirectory: {
		id: 'app.settings.resources.select-import-directory',
		defaultMessage: 'Select monitored folder',
	},
	resetImportDirectory: {
		id: 'app.settings.resources.reset-import-directory',
		defaultMessage: 'Use system Downloads folder',
	},
	databaseBackups: {
		id: 'app.settings.resources.database-backups',
		defaultMessage: 'App database backups',
	},
	openBackupsFolder: {
		id: 'app.settings.resources.open-backups-folder',
		defaultMessage: 'Open backups folder',
	},
	databaseBackupsDescription: {
		id: 'app.settings.resources.database-backups-description',
		defaultMessage:
			'Backups of important app data are stored here in case you need to recover them later.',
	},
})

const MINECRAFT_DIRECTORIES_STORAGE_KEY = 'axolotl-minecraft-directories'

function isMinecraftDirectoryPath(value) {
	const normalized = value.trim().replace(/[\\/]+$/, '')
	return normalized.length > 0 && normalized.split(/[\\/]/).at(-1)?.toLowerCase() === '.minecraft'
}

function loadMinecraftDirectories() {
	try {
		const raw = localStorage.getItem(MINECRAFT_DIRECTORIES_STORAGE_KEY)
		if (!raw) return []
		const parsed = JSON.parse(raw)
		if (!Array.isArray(parsed)) return []
		return [...new Set(parsed.filter((value) => typeof value === 'string'))].filter(
			isMinecraftDirectoryPath,
		)
	} catch {
		return []
	}
}

function persistMinecraftDirectories(values) {
	try {
		const validValues = [
			...new Set(values.map((value) => value.trim()).filter(isMinecraftDirectoryPath)),
		]
		localStorage.setItem(MINECRAFT_DIRECTORIES_STORAGE_KEY, JSON.stringify(validValues))
	} catch {
		// Local storage may be unavailable in an embedded or restricted webview.
	}
}

function downloadSourceModel(setting) {
	return computed({
		get: () => settings.value[setting],
		set: (source) => {
			settings.value[setting] = source
		},
	})
}

const minecraftMetadataSource = downloadSourceModel('minecraft_metadata_source')
const minecraftFileSource = downloadSourceModel('minecraft_file_source')
const modrinthDownloadSource = downloadSourceModel('modrinth_source')
const curseforgeDownloadSource = downloadSourceModel('curseforge_source')
const automaticSourceOption = computed(() => ({
	value: 'auto',
	label: formatMessage(messages.automaticSource),
}))
const officialPreferredSourceOption = computed(() => ({
	value: 'official_preferred',
	label: formatMessage(messages.officialPreferredSource),
}))
const officialOnlySourceOption = computed(() => ({
	value: 'official_only',
	label: formatMessage(messages.officialOnlySource),
}))
const minecraftSourceOptions = computed(() => [
	automaticSourceOption.value,
	officialPreferredSourceOption.value,
	{ value: 'mirror_preferred', label: formatMessage(messages.openBmclApiSource) },
	officialOnlySourceOption.value,
])
const modrinthSourceOptions = computed(() => [
	automaticSourceOption.value,
	officialPreferredSourceOption.value,
	{ value: 'mirror_preferred', label: formatMessage(messages.tianpaoSource) },
	officialOnlySourceOption.value,
])
const curseforgeSourceOptions = computed(() => [
	automaticSourceOption.value,
	officialPreferredSourceOption.value,
	{ value: 'mirror_preferred', label: formatMessage(messages.tianpaoSource) },
	officialOnlySourceOption.value,
])
const mojangAuthSource = downloadSourceModel('mojang_auth_source')
const mojangAuthSourceOptions = computed(() => [
	automaticSourceOption.value,
	{ value: 'official_preferred', label: formatMessage(messages.mojangAuthOfficialPreferred) },
	{ value: 'mirror_preferred', label: formatMessage(messages.mojangAuthMirrorPreferred) },
	{ value: 'official_only', label: formatMessage(messages.mojangAuthOfficialOnly) },
])
const downloadConcurrencyMode = computed({
	get: () => (settings.value.auto_concurrent_downloads ? 'auto' : 'manual'),
	set: (mode) => {
		settings.value.auto_concurrent_downloads = mode === 'auto'
	},
})
const downloadConcurrencyOptions = computed(() => [
	{
		value: 'auto',
		label: formatMessage(messages.automaticSource),
	},
	{
		value: 'manual',
		label: formatMessage(messages.manualConcurrency),
	},
])
const downloadEngine = computed({
	get: () => settings.value.download_engine,
	set: (engine) => {
		settings.value.download_engine = engine
	},
})
const downloadEngineOptions = computed(() => [
	{
		value: 'legacy',
		label: formatMessage(messages.legacyEngine),
	},
	{
		value: 'xmcl',
		label: formatMessage(messages.xmclEngine),
	},
])

const proxyModeOptions = computed(() => [
	{
		value: 'none',
		label: formatMessage(messages.proxyModeNone),
	},
	{
		value: 'system',
		label: formatMessage(messages.proxyModeSystem),
	},
	{
		value: 'custom',
		label: formatMessage(messages.proxyModeCustom),
	},
])

const appDirectoryDescriptionText = computed(() =>
	isPortable.value
		? formatMessage(messages.appDirectoryDescriptionPortable)
		: formatMessage(messages.appDirectoryDescription),
)

watch(
	settings,
	async () => {
		const setSettings = JSON.parse(JSON.stringify(settings.value))

		if (!setSettings.custom_dir) {
			setSettings.custom_dir = null
		}

		await set(setSettings)
	},
	{ deep: true },
)

watch(
	minecraftDirectories,
	(value) => {
		persistMinecraftDirectories(value)
		void syncDirectLinkInstances(true)
	},
	{ deep: true },
)

let directLinkSyncRunning = false
async function syncDirectLinkInstances(allowEmpty = false) {
	if (directLinkSyncRunning || (!allowEmpty && minecraftDirectories.value.length === 0)) return
	directLinkSyncRunning = true
	try {
		await sync_direct_links(minecraftDirectories.value)
		window.dispatchEvent(new Event('axolotl-direct-links-synced'))
	} catch (error) {
		console.warn('Failed to sync external Minecraft instances', error)
	} finally {
		directLinkSyncRunning = false
	}
}

void syncDirectLinkInstances()

watch(
	missingContentScannerSettings,
	(value) => {
		setMissingContentScannerSettings(value)
		void configureCurseForgeManualDownloadWatcher(value.enabled, value.directory).catch(handleError)
	},
	{ deep: true },
)

watch(
	proxyConfig,
	async () => {
		await saveProxyConfig(true)
	},
	{ deep: true },
)

async function purgeCache() {
	await purge_cache_types([
		'project',
		'project_v3',
		'curseforge_project',
		'version',
		'user',
		'team',
		'organization',
		'file',
		'loader_manifest',
		'minecraft_manifest',
		'categories',
		'report_types',
		'loaders',
		'game_versions',
		'donation_platforms',
		'file_hash',
		'file_update',
		'search_results',
		'search_results_v3',
	]).catch(handleError)
}

function handlePurgeCacheClick() {
	if (themeStore.getFeatureFlag('skip_non_essential_warnings')) {
		void purgeCache()
		return
	}

	purgeCacheConfirmModal.value?.show()
}

async function openDbBackupsFolder() {
	await showAppDbBackupsFolder().catch(handleError)
}

async function findLauncherDir() {
	const newDir = await open({
		multiple: false,
		directory: true,
		title: formatMessage(messages.selectDirectory),
	})

	if (newDir) {
		settings.value.custom_dir = newDir
	}
}

async function findMissingContentImportDirectory() {
	const directory = await open({
		multiple: false,
		directory: true,
		title: formatMessage(messages.selectImportDirectory),
	})
	if (typeof directory === 'string') {
		missingContentScannerSettings.value.directory = directory
	}
}

function resetMissingContentImportDirectory() {
	missingContentScannerSettings.value.directory = null
}

async function addMinecraftDirectory() {
	minecraftDirectoryError.value = null
	const directory = await open({
		multiple: false,
		directory: true,
		title: formatMessage(messages.selectMinecraftDirectory),
	})
	if (typeof directory !== 'string') return

	const normalized = directory.trim().replace(/[\\/]+$/, '')
	if (!isMinecraftDirectoryPath(normalized)) {
		minecraftDirectoryError.value = formatMessage(messages.minecraftDirectoryMustEndWith)
		return
	}
	if (!minecraftDirectories.value.includes(normalized)) {
		minecraftDirectories.value.push(normalized)
	}
}

function removeMinecraftDirectory(index) {
	minecraftDirectories.value.splice(index, 1)
	if (minecraftDirectoryError.value) minecraftDirectoryError.value = null
}

function validateMinecraftDirectory(value) {
	minecraftDirectoryError.value =
		value.trim() && !isMinecraftDirectoryPath(value)
			? formatMessage(messages.minecraftDirectoryMustEndWith)
			: null
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<ConfirmModalWrapper
			ref="purgeCacheConfirmModal"
			:title="formatMessage(messages.purgeConfirmTitle)"
			:description="formatMessage(messages.purgeConfirmDescription)"
			:has-to-type="false"
			:proceed-label="formatMessage(messages.purgeCache)"
			:show-ad-on-close="false"
			@proceed="purgeCache"
		/>

		<SettingsSection v-if="props.scope === 'storage-backups'">
			<SettingsRow stacked>
				<template #label>
					<span id="settings-target-storage-app-directory" tabindex="-1">
						{{ formatMessage(messages.axolotlDataDirectory) }}
					</span>
				</template>
				<template #description>{{ appDirectoryDescriptionText }}</template>
				<template #control>
					<StyledInput
						id="appDir"
						v-model="settings.custom_dir"
						:icon="BoxIcon"
						type="text"
						:disabled="isPortable"
						wrapper-class="w-full"
					>
						<template #right>
							<IconButton
								:label="formatMessage(messages.appDirectory)"
								class="ml-1.5"
								:disabled="isPortable"
								@click="findLauncherDir"
							>
								<FolderSearchIcon />
							</IconButton>
						</template>
					</StyledInput>
				</template>
			</SettingsRow>
			<SettingsRow stacked>
				<template #label>
					<span id="settings-target-storage-minecraft-directories" tabindex="-1">
						{{ formatMessage(messages.minecraftDirectories) }}
					</span>
				</template>
				<template #description>{{
					formatMessage(messages.minecraftDirectoriesDescription)
				}}</template>
				<template #control>
					<div class="flex w-full flex-col gap-2">
						<div
							v-for="(directory, index) in minecraftDirectories"
							:key="`${directory}-${index}`"
							class="flex min-w-0 items-center gap-2"
						>
							<StyledInput
								:id="`minecraft-directory-${index}`"
								v-model="minecraftDirectories[index]"
								:icon="BoxIcon"
								type="text"
								wrapper-class="min-w-0 flex-1"
								@change="validateMinecraftDirectory(minecraftDirectories[index])"
							/>
							<IconButton
								:label="formatMessage(messages.removeMinecraftDirectory)"
								@click="removeMinecraftDirectory(index)"
							>
								<TrashIcon />
							</IconButton>
						</div>
						<p v-if="minecraftDirectoryError" class="m-0 text-sm text-red">
							{{ minecraftDirectoryError }}
						</p>
						<button class="btn min-w-max self-start" @click="addMinecraftDirectory">
							<PlusIcon />
							{{ formatMessage(messages.addMinecraftDirectory) }}
						</button>
					</div>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-storage-cache" tabindex="-1">
						{{ formatMessage(messages.appCache) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.appCacheDescription) }}</template>
				<template #control>
					<button id="purge-cache" class="btn min-w-max" @click="handlePurgeCacheClick">
						<TrashIcon />
						{{ formatMessage(messages.purgeCache) }}
					</button>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-resources-database-backups" tabindex="-1">
						{{ formatMessage(messages.databaseBackups) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.databaseBackupsDescription) }}</template>
				<template #control>
					<button id="open-db-backups-folder" class="btn min-w-max" @click="openDbBackupsFolder">
						<FolderOpenIcon />
						{{ formatMessage(messages.openBackupsFolder) }}
					</button>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'content-downloads'">
			<template #header>
				<h2
					id="settings-target-resources-download-mirrors"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.downloadMirrors) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.downloadMirrorsDescription) }}
				</p>
			</template>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.minecraftMetadataSource) }}</template>
				<template #description>{{
					formatMessage(messages.minecraftMetadataSourceDescription)
				}}</template>
				<template #control>
					<div class="w-full">
						<Combobox v-model="minecraftMetadataSource" :options="minecraftSourceOptions" />
					</div>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.minecraftFileSource) }}</template>
				<template #description>{{
					formatMessage(messages.minecraftFileSourceDescription)
				}}</template>
				<template #control>
					<div class="w-full">
						<Combobox v-model="minecraftFileSource" :options="minecraftSourceOptions" />
					</div>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.modrinthMirror) }}</template>
				<template #description>{{ formatMessage(messages.modrinthMirrorDescription) }}</template>
				<template #control>
					<div class="w-full">
						<Combobox v-model="modrinthDownloadSource" :options="modrinthSourceOptions" />
					</div>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.curseforgeMirror) }}</template>
				<template #description>{{ formatMessage(messages.curseforgeMirrorDescription) }}</template>
				<template #control>
					<div class="w-full">
						<Combobox v-model="curseforgeDownloadSource" :options="curseforgeSourceOptions" />
					</div>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.curseforgeRestrictionBypass) }}</template>
				<template #description>
					{{ formatMessage(messages.curseforgeRestrictionBypassDescription) }}
				</template>
				<template #control>
					<Toggle
						id="curseforge-restriction-bypass"
						v-model="settings.bypass_curseforge_download_restrictions"
					/>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'content-downloads'">
			<SettingsRow>
				<template #label>
					<span id="settings-target-resources-download-engine" tabindex="-1">
						{{ formatMessage(messages.downloadEngine) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.downloadEngineDescription) }}</template>
				<template #control>
					<div class="w-full">
						<Combobox v-model="downloadEngine" :options="downloadEngineOptions" />
					</div>
				</template>
			</SettingsRow>
			<SettingsRow stacked>
				<template #label>
					<span id="settings-target-resources-maximum-downloads" tabindex="-1">
						{{ formatMessage(messages.maximumDownloads) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.maximumDownloadsDescription) }}</template>
				<template #control>
					<div class="flex w-full flex-col gap-3">
						<div class="w-48 max-w-full">
							<Combobox v-model="downloadConcurrencyMode" :options="downloadConcurrencyOptions" />
						</div>
						<Slider
							v-if="!settings.auto_concurrent_downloads"
							id="max-downloads"
							v-model="settings.max_concurrent_downloads"
							:min="1"
							:max="256"
							:step="1"
						/>
					</div>
				</template>
			</SettingsRow>
			<SettingsRow stacked>
				<template #label>{{ formatMessage(messages.maximumWrites) }}</template>
				<template #description>{{ formatMessage(messages.maximumWritesDescription) }}</template>
				<template #control>
					<div class="w-full">
						<Slider
							id="max-writes"
							v-model="settings.max_concurrent_writes"
							:min="1"
							:max="50"
							:step="1"
						/>
					</div>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'network-multiplayer'">
			<SettingsRow>
				<template #label>
					<span id="settings-target-network-mojang-auth-source" tabindex="-1">
						{{ formatMessage(messages.mojangAuthService) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.mojangAuthServiceDescription) }}</template>
				<template #control>
					<div class="w-full">
						<Combobox v-model="mojangAuthSource" :options="mojangAuthSourceOptions" />
					</div>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'network-multiplayer'">
			<template #header>
				<h2
					id="settings-target-resources-proxy"
					tabindex="-1"
					class="m-0 text-lg font-semibold text-contrast"
				>
					{{ formatMessage(messages.proxySettings) }}
				</h2>
				<p class="m-0 mt-1 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.proxySettingsDescription) }}
				</p>
			</template>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.proxyMode) }}</template>
				<template #control>
					<div class="w-full">
						<Combobox v-model="proxyConfig.mode" :options="proxyModeOptions" />
					</div>
				</template>
			</SettingsRow>
			<SettingsRow v-if="proxyConfig.mode === 'custom'" stacked>
				<template #label>{{ formatMessage(messages.proxyUrl) }}</template>
				<template #control>
					<StyledInput
						id="proxy-url"
						v-model="proxyConfig.url"
						type="text"
						:placeholder="formatMessage(messages.proxyUrlPlaceholder)"
						wrapper-class="w-full"
						@blur="saveProxyConfig(false)"
					/>
				</template>
			</SettingsRow>
			<SettingsRow v-if="proxyConfig.mode === 'custom'" stacked>
				<template #label>{{ formatMessage(messages.proxyUsername) }}</template>
				<template #control>
					<StyledInput
						id="proxy-username"
						v-model="proxyConfig.username"
						type="text"
						:placeholder="formatMessage(messages.proxyUsernamePlaceholder)"
						wrapper-class="w-full"
						@blur="saveProxyConfig(false)"
					/>
				</template>
			</SettingsRow>
			<SettingsRow v-if="proxyConfig.mode === 'custom'" stacked>
				<template #label>{{ formatMessage(messages.proxyPassword) }}</template>
				<template #control>
					<StyledInput
						id="proxy-password"
						v-model="proxyConfig.password"
						type="password"
						:placeholder="formatMessage(messages.proxyPasswordPlaceholder)"
						wrapper-class="w-full"
						@blur="saveProxyConfig(false)"
					/>
				</template>
			</SettingsRow>
			<SettingsRow compact>
				<template #label>{{ formatMessage(messages.proxyTest) }}</template>
				<template #control>
					<div class="flex flex-wrap items-center justify-end gap-3">
						<span v-if="proxyTestResult" class="text-sm text-secondary">{{ proxyTestResult }}</span>
						<button
							:disabled="
								proxyTesting ||
								proxyConfig.mode === 'none' ||
								(proxyConfig.mode === 'custom' && !proxyConfig.url?.trim())
							"
							class="btn min-w-max"
							@click="testProxy"
						>
							{{ formatMessage(proxyTesting ? messages.proxyTesting : messages.proxyTest) }}
						</button>
					</div>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection v-if="props.scope === 'content-downloads'">
			<SettingsRow>
				<template #label>
					<span id="settings-target-resources-missing-content-import" tabindex="-1">
						{{ formatMessage(messages.missingContentAutoImport) }}
					</span>
				</template>
				<template #description>
					{{ formatMessage(messages.missingContentAutoImportDescription) }}
				</template>
				<template #control>
					<Toggle
						id="missing-content-auto-import"
						v-model="missingContentScannerSettings.enabled"
					/>
				</template>
			</SettingsRow>
			<SettingsRow stacked>
				<template #label>{{ formatMessage(messages.missingContentImportDirectory) }}</template>
				<template #description>
					{{ formatMessage(messages.missingContentImportDirectoryDescription) }}
				</template>
				<template #control>
					<div class="flex w-full flex-wrap items-center gap-2">
						<div class="min-w-0 flex-1">
							<StyledInput
								id="missing-content-import-directory"
								:model-value="
									missingContentScannerSettings.directory ??
									formatMessage(messages.systemDownloadsDirectory)
								"
								:icon="FolderOpenIcon"
								type="text"
								readonly
								wrapper-class="w-full"
							>
								<template #right>
									<IconButton
										type="base"
										:label="formatMessage(messages.selectImportDirectory)"
										class="ml-1.5"
										:disabled="!missingContentScannerSettings.enabled"
										@click="findMissingContentImportDirectory"
									>
										<FolderSearchIcon />
									</IconButton>
								</template>
							</StyledInput>
						</div>
						<button
							v-if="missingContentScannerSettings.directory"
							class="btn min-w-max"
							:disabled="!missingContentScannerSettings.enabled"
							@click="resetMissingContentImportDirectory"
						>
							{{ formatMessage(messages.resetImportDirectory) }}
						</button>
					</div>
				</template>
			</SettingsRow>
		</SettingsSection>
	</div>
</template>
