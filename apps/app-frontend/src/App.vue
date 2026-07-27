<script setup lang="ts">
import { AuthFeature, TauriModrinthClient, VerboseLoggingFeature } from '@modrinth/api-client'
import {
	ChangeSkinIcon,
	CompassIcon,
	DownloadIcon,
	ExternalIcon,
	HomeIcon,
	LeftArrowIcon,
	LibraryIcon,
	LogInIcon,
	LogOutIcon,
	PlusIcon,
	RefreshCwIcon,
	RightArrowIcon,
	SettingsIcon,
	SpinnerIcon,
	UserIcon,
	WorldIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Avatar,
	ButtonStyled,
	commonMessages,
	ContentInstallModal,
	ContentUpdaterModal,
	CreationFlowModal,
	defineMessages,
	I18nDebugPanel,
	LoadingBar,
	NewModal,
	NotificationPanel,
	OverflowMenu,
	PopupNotificationPanel,
	provideModalBehavior,
	provideModrinthClient,
	provideNotificationManager,
	providePageContext,
	providePopupNotificationManager,
	useDebugLogger,
	useFormatBytes,
	useGlobalDrop,
	useVIntl,
} from '@modrinth/ui'
import ConfirmDropTypeModal from '@modrinth/ui/src/components/flows/drop/ConfirmDropTypeModal.vue'
import GenericContentInstallModal from '@modrinth/ui/src/components/flows/drop/GenericContentInstallModal.vue'
import LauncherImportModal from '@modrinth/ui/src/components/flows/drop/LauncherImportModal.vue'
import SymlinkMethodCards from '@modrinth/ui/src/components/flows/drop/SymlinkMethodCards.vue'
import { useInstanceContext } from '@modrinth/ui/src/composables/use-instance-context'
import { useQuery } from '@tanstack/vue-query'
import { getVersion } from '@tauri-apps/api/app'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { Effect, getCurrentWindow } from '@tauri-apps/api/window'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import { openUrl } from '@tauri-apps/plugin-opener'
import { type as getOsType } from '@tauri-apps/plugin-os'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
import { computed, nextTick, onMounted, onUnmounted, provide, ref, watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { getAnnouncementByVersion } from '@/announcements/catalog'
import AccountsCard from '@/components/ui/AccountsCard.vue'
import UpdateAnnouncementModal from '@/components/ui/announcement/UpdateAnnouncementModal.vue'
import AppActionBar from '@/components/ui/AppActionBar.vue'
import AxolotlLogo from '@/components/ui/AxolotlLogo.vue'
import Breadcrumbs from '@/components/ui/Breadcrumbs.vue'
import ErrorModal from '@/components/ui/ErrorModal.vue'
import AddServerToInstanceModal from '@/components/ui/install_flow/AddServerToInstanceModal.vue'
import UnknownPackWarningModal from '@/components/ui/install_flow/UnknownPackWarningModal.vue'
import MinecraftAuthErrorModal from '@/components/ui/minecraft-auth-error-modal/MinecraftAuthErrorModal.vue'
import MinecraftCrashModal from '@/components/ui/MinecraftCrashModal.vue'
import AppSettingsModal from '@/components/ui/modal/AppSettingsModal.vue'
import AuthGrantFlowWaitModal from '@/components/ui/modal/AuthGrantFlowWaitModal.vue'
import CommunityAnnouncementModal from '@/components/ui/modal/CommunityAnnouncementModal.vue'
import CurseForgeManualDownloadsModal from '@/components/ui/modal/CurseForgeManualDownloadsModal.vue'
import InstallToPlayModal from '@/components/ui/modal/InstallToPlayModal.vue'
import InstanceIconPickerModal from '@/components/ui/modal/InstanceIconPickerModal.vue'
import ModpackAlreadyInstalledModal from '@/components/ui/modal/ModpackAlreadyInstalledModal.vue'
import UpdateToPlayModal from '@/components/ui/modal/UpdateToPlayModal.vue'
import NavButton from '@/components/ui/NavButton.vue'
import NavRail from '@/components/ui/NavRail.vue'
import OnboardingOverlay from '@/components/ui/onboarding/OnboardingOverlay.vue'
import QuickInstanceSwitcher from '@/components/ui/QuickInstanceSwitcher.vue'
import SplashScreen from '@/components/ui/SplashScreen.vue'
import WindowControls from '@/components/ui/WindowControls.vue'
import { useCheckDisableMouseover } from '@/composables/macCssFix.js'
import { minecraftLaunchErrorKey } from '@/composables/useMinecraftLaunchError'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { AxolotlBrandConfig, config, getOfficialLabrinthBaseUrl } from '@/config'
import { debugAnalytics, initAnalytics, trackEvent } from '@/helpers/analytics'
import { check_reachable } from '@/helpers/auth.js'
import { get_user, get_version } from '@/helpers/cache.js'
import {
	type ClassificationResult,
	classifyDroppedItem,
	classifyDroppedItemWithExtraction,
	detectFileLock,
	extractModMetadata,
	lookupModHash,
	type ModrinthLookupResult,
	scanLauncherInstances,
	type ScanResult,
} from '@/helpers/drop'
import { isVersionInRange, areLoadersCompatible } from '@/helpers/version-compatibility'
import { command_listener, install_job_listener, warning_listener } from '@/helpers/events.js'
import { import_instance } from '@/helpers/import.js'
import {
	type InstallJobSnapshot,
	install_create_modpack_instance,
	install_get_modpack_preview,
	wait_for_install_job,
} from '@/helpers/install'
import {
	add_project_from_path,
	check_symlink_capability,
	get as getInstance,
	import_world_save,
	list as listInstances,
	run,
} from '@/helpers/instance'
import { cancelLogin, get as getCreds, login, logout } from '@/helpers/mr_auth.ts'
import { mergeUrlQuery, parseModrinthLink } from '@/helpers/project-links.ts'
import { get as getSettings, getUpdateSource, set as setSettings } from '@/helpers/settings.ts'
import { get_opening_command, initialize_state } from '@/helpers/state'
import {
	areUpdatesEnabled,
	checkAppUpdate,
	enqueueUpdateForInstallation,
	exportErrorLogs,
	getOS,
	getUpdateSize,
	isDev,
	isNetworkMetered,
	setRestartAfterPendingUpdate,
} from '@/helpers/utils.js'
import { start_join_server, start_join_singleplayer_world } from '@/helpers/worlds.ts'
import i18n, { resolveInitialLocale } from '@/i18n.config'
import {
	appUpdateState,
	downloadAvailableAppUpdate,
	getNextAppUpdatePopupTime,
	installAvailableAppUpdate,
	markAppUpdateActionable,
	markAppUpdatePopupShown,
	openAppUpdateChangelog,
	setAppUpdateActions,
} from '@/providers/app-update.ts'
import { createContentInstall, provideContentInstall } from '@/providers/content-install'
import { createDownloadManager, provideDownloadManager } from '@/providers/download-manager'
import {
	provideAppUpdateDownloadProgress,
	subscribeToDownloadProgress,
} from '@/providers/download-progress.ts'
import { createServerInstall, provideServerInstall } from '@/providers/server-install'
import { setupProviders } from '@/providers/setup'
import { setupAuthProvider } from '@/providers/setup/auth'
import { setupLoadingStateProvider } from '@/providers/setup/loading-state'
import { useError } from '@/store/error.js'
import { useTheming } from '@/store/state'

import { generateSkinPreviews } from './helpers/rendering/batch-skin-renderer'
import { get_available_capes, get_available_skins } from './helpers/skins'
import { AppNotificationManager } from './providers/app-notifications'
import { AppPopupNotificationManager } from './providers/app-popup-notifications'
import { ModrinthMirrorFallbackFeature } from './providers/modrinth-mirror-fallback'

const themeStore = useTheming()
const router = useRouter()
const route = useRoute()
const onSkinsPage = computed(() => route.path === '/skins')
const APP_LEFT_NAV_WIDTH = '4rem'
const APP_SIDEBAR_WIDTH = 300
const credentials = ref()
const sidebarToggled = ref(true)
const unsubscribeSidebarToggle = themeStore.$subscribe(() => {
	sidebarToggled.value = !themeStore.toggleSidebar
})
const forceSidebar = computed(
	() => route.path.startsWith('/browse') || route.path.startsWith('/project'),
)
const sidebarVisible = computed(() => sidebarToggled.value || forceSidebar.value)
const customBackgroundStyle = computed(() => {
	// A custom image would sit between the desktop and the UI, defeating the
	// transparent window entirely, so the two are mutually exclusive.
	if (themeStore.transparentBackground || !themeStore.customBackgroundPath) return undefined

	return {
		backgroundImage: `url("${convertFileSrc(themeStore.customBackgroundPath)}")`,
		filter: `blur(${themeStore.customBackgroundBlur}px)`,
		opacity: themeStore.customBackgroundOpacity / 100,
	}
})

const notificationManager = new AppNotificationManager()
provideNotificationManager(notificationManager)
const { handleError, addNotification } = notificationManager
const downloadManager = createDownloadManager(handleError)
provideDownloadManager(downloadManager)

const popupNotificationManager = new AppPopupNotificationManager()
providePopupNotificationManager(popupNotificationManager)
const { addPopupNotification } = popupNotificationManager

const appVersion = getVersion()
const tauriApiClient = new TauriModrinthClient({
	userAgent: async () => AxolotlBrandConfig.userAgent(await appVersion, await getOsType()),
	labrinthBaseUrl: config.labrinthBaseUrl,
	features: [
		...(AxolotlBrandConfig.capabilities.privateModrinthServices
			? [
					new AuthFeature({
						token: async () => (await getCreds())?.session,
					}),
				]
			: []),
		new ModrinthMirrorFallbackFeature(),
		new VerboseLoggingFeature(),
	],
})
provideModrinthClient(tauriApiClient)
providePageContext({
	hierarchicalSidebarAvailable: ref(true),
	showAds: ref(false),
	floatingActionBarOffsets: {
		left: ref(APP_LEFT_NAV_WIDTH),
		right: computed(() => (sidebarVisible.value ? `${APP_SIDEBAR_WIDTH}px` : '0px')),
	},
	featureFlags: {
		serverRamAsBytesAlwaysOn: computed(() =>
			themeStore.getFeatureFlag('server_ram_as_bytes_always_on'),
		),
	},
	openExternalUrl: (url) => openUrl(url),
})
provideModalBehavior({
	noblur: computed(() => !themeStore.advancedRendering),
})

const {
	instanceIconPickerModal,
	installationModal,
	unknownPackWarningModal,
	fetchExistingInstanceNames,
	handleCreate,
	handleBrowseModpacks,
	searchModpacks,
	getProjectVersions,
	getLoaderManifest,
	setModpackAlreadyInstalledModal,
	handleModpackDuplicateCreateAnyway,
	handleModpackDuplicateGoToInstance,
	fileDrop,
} = setupProviders(notificationManager, popupNotificationManager)

const { browserOffline, offline, setNetworkReachable } = useNetworkStatus()

const showOnboarding = ref(false)
const onboardingMode = ref('main')
const onboardingSettings = ref(null)
const onboardingReplay = ref(false)
const settingsModal = ref(null)
const nativeDecorations = ref(false)

const os = ref('')
const isDevEnvironment = ref(false)

/**
 * Acrylic is rendered by the Windows compositor behind the webview, so CSS
 * cannot clip it. Keep the native rounded frame and hide its border while the
 * CSS-drawn transparent-window border is active.
 */
async function applyWindowFrame() {
	if (os.value !== 'Windows') return

	try {
		await invoke('set_transparent_window_frame', {
			enabled: themeStore.transparentBackground,
		})
	} catch (error) {
		console.warn('Failed to update transparent window frame', error)
	}
}

watch(() => themeStore.transparentBackground, applyWindowFrame)

/**
 * The frosted glass has to come from the compositor: a webview cannot reach the
 * pixels behind its own window, so `backdrop-filter` can never blur the desktop.
 * Acrylic blurs whatever sits behind the window, matching what the transparency
 * already reveals; Mica would only sample the wallpaper and ignore other
 * windows. Linux exposes no window effects at all.
 */
async function applyWindowEffects() {
	if (os.value === 'Linux') return

	try {
		const window = getCurrentWindow()
		if (!themeStore.transparentBackground || !themeStore.transparentBackgroundBlur) {
			await window.clearEffects()
			return
		}

		await window.setEffects({
			effects: [os.value === 'MacOS' ? Effect.UnderWindowBackground : Effect.Acrylic],
		})
	} catch (error) {
		console.warn('Failed to update window effects', error)
	}
}

watch(
	() => [themeStore.transparentBackground, themeStore.transparentBackgroundBlur],
	applyWindowEffects,
)

const stateInitialized = ref(false)
const communityAnnouncementModal = ref()
const updateAnnouncementModal = ref()
const minecraftCrashModal = ref()
const pendingUpdateAnnouncementVersion = ref(null)
const updateAnnouncementShowing = ref(false)

const isMaximized = ref(false)

const authUnreachableDebug = useDebugLogger('AuthReachableChecker')
const authServerQuery = useQuery({
	queryKey: ['authServerReachability'],
	enabled: computed(() => !browserOffline.value),
	queryFn: async () => {
		try {
			await check_reachable()
			setNetworkReachable(true)
			authUnreachableDebug('Auth servers are reachable')
			return true
		} catch (error) {
			setNetworkReachable(false)
			throw error
		}
	},
	refetchInterval: 5 * 60 * 1000, // 5 minutes
	retry: false,
	refetchOnWindowFocus: false,
})

const authUnreachable = computed(() => {
	if (!offline.value && authServerQuery.isError.value && !authServerQuery.isLoading.value) {
		console.warn('Failed to reach auth servers', authServerQuery.error.value)
		return true
	}
	return false
})

onMounted(async () => {
	await useCheckDisableMouseover()

	document.querySelector('body').addEventListener('click', handleClick)
	document.querySelector('body').addEventListener('auxclick', handleAuxClick)

	checkUpdates()
})

onUnmounted(async () => {
	document.querySelector('body').removeEventListener('click', handleClick)
	document.querySelector('body').removeEventListener('auxclick', handleAuxClick)
	unsubscribeSidebarToggle()
	clearDelayedUpdatePopup()
	await unlistenUpdateDownload?.()
	downloadManager.dispose()
})

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()

async function onImportFileReceived({
	file: _file,
	filePath,
	source: _source,
}: {
	file: File | null
	filePath: string | null
	source: 'file-picker' | 'drag-drop'
}) {
	if (!filePath) return

	const fileName = filePath.split(/[/\\]/).pop() || 'file'

	// ── Hide creation modal first ──
	installationModal.value?.hide()

	// ── Show "Processing..." (matches drag-drop behavior) ──
	const processingNotify = addNotification({
		title: formatMessage(messages.dropProcessing, { name: fileName }),
		type: 'info',
		autoCloseMs: null,
	})

	try {
		// ── Classify the file (same entry point as drag-drop) ──
		const classification = await classifyDroppedItem(filePath)
		clearDropProcessingNotification()
		notificationManager.removeNotification(processingNotify.id)

		// ── Set drop state so handleDropConfirm can read it ──
		dropClassification.value = classification
		dropFilePath.value = classification.file_path ?? classification.base_path ?? ''
		dropFileName.value = fileName

		// ── Unknown + extraction → force analysis prompt ──
		if (
			classification.item_type === 'unknown' &&
			classification.reason?.toLowerCase().includes('extraction')
		) {
			showForceAnalysisPrompt(classification)
			return
		}

		// ── Unknown (no extraction) → error ──
		if (classification.item_type === 'unknown') {
			addNotification({
				title: formatMessage(messages.dropUnknownTitle),
				text: classification.reason
					? classification.reason
					: formatMessage(messages.dropUnknownText),
				type: 'error',
			})
			return
		}

		// ── Known types → show the same confirm modal as drag-drop ──
		confirmDropModal.value?.show()
	} catch (e) {
		notificationManager.removeNotification(processingNotify?.id)
		addNotification({
			title: 'Failed to process file',
			text: e instanceof Error ? e.message : String(e),
			type: 'error',
		})
	}
}

const messages = defineMessages({
	updateInstalledToastTitle: {
		id: 'app.update.complete-toast.title',
		defaultMessage: 'Version {version} was successfully installed!',
	},
	updateInstalledToastText: {
		id: 'app.update.complete-toast.text',
		defaultMessage: 'Click here to view the changelog.',
	},
	authUnreachableHeader: {
		id: 'app.auth-servers.unreachable.header',
		defaultMessage: 'Cannot reach authentication servers',
	},
	authUnreachableBody: {
		id: 'app.auth-servers.unreachable.body',
		defaultMessage:
			'Minecraft authentication servers may be down right now. Check your internet connection and try again later.',
	},
	restarting: {
		id: 'app.restarting',
		defaultMessage: 'Restarting...',
	},
	home: {
		id: 'app.navigation.home',
		defaultMessage: 'Home',
	},
	worlds: {
		id: 'app.navigation.worlds',
		defaultMessage: 'Worlds',
	},
	discoverContent: {
		id: 'app.navigation.discover-content',
		defaultMessage: 'Discover content',
	},
	skinSelector: {
		id: 'app.navigation.skin-selector',
		defaultMessage: 'Skin selector',
	},
	library: {
		id: 'app.navigation.library',
		defaultMessage: 'Library',
	},
	downloads: {
		id: 'app.navigation.downloads',
		defaultMessage: 'Downloads',
	},
	createInstance: {
		id: 'app.navigation.create-instance',
		defaultMessage: 'Create new instance',
	},
	signedInAs: {
		id: 'app.account.signed-in-as',
		defaultMessage: 'Signed in as',
	},
	playingAs: {
		id: 'app.minecraft.playing-as',
		defaultMessage: 'Playing as',
	},
	warning: {
		id: 'app.notification.warning',
		defaultMessage: 'Warning',
	},
	exportErrorLogs: {
		id: 'app.notification.export-error-logs',
		defaultMessage: 'Export error logs',
	},

	// ── Drop / import notification messages ──
	dropOverlayTitle: {
		id: 'app.drop.overlay-title',
		defaultMessage: 'Drop to import',
	},
	dropOverlaySubtitle: {
		id: 'app.drop.overlay-subtitle',
		defaultMessage: 'Release to analyze',
	},
	dropProcessing: {
		id: 'app.drop.processing',
		defaultMessage: 'Processing {name}...',
	},
	dropMultipleFilesTitle: {
		id: 'app.drop.error.multiple-files-title',
		defaultMessage: 'Cannot import multiple files',
	},
	dropMultipleFilesText: {
		id: 'app.drop.error.multiple-files-text',
		defaultMessage: 'Please drop one file at a time.',
	},
	dropShortcutFailedTitle: {
		id: 'app.drop.error.shortcut-title',
		defaultMessage: 'Shortcut resolution failed',
	},
	dropShortcutFailedText: {
		id: 'app.drop.error.shortcut-text',
		defaultMessage: 'Could not resolve the shortcut target.',
	},
	dropUnknownTitle: {
		id: 'app.drop.error.unknown-title',
		defaultMessage: 'Unknown file type',
	},
	dropUnknownText: {
		id: 'app.drop.error.unknown-text',
		defaultMessage: 'Could not determine what kind of file this is.',
	},
	dropErrorTitle: {
		id: 'app.drop.error.title',
		defaultMessage: 'Drop error',
	},
	dropWorldImportedTitle: {
		id: 'app.drop.world-imported-title',
		defaultMessage: 'World imported',
	},
	dropWorldImportedText: {
		id: 'app.drop.world-imported-text',
		defaultMessage: 'World save has been imported successfully.',
	},
	dropContentInstalledTitle: {
		id: 'app.drop.content-installed-title',
		defaultMessage: 'Content installed',
	},
	dropContentInstalledText: {
		id: 'app.drop.content-installed-text',
		defaultMessage: 'File has been installed to the instance.',
	},
	dropInstallFailedTitle: {
		id: 'app.drop.install-failed-title',
		defaultMessage: 'Installation failed',
	},
	dropInstanceImportedTitle: {
		id: 'app.drop.instance-imported-title',
		defaultMessage: 'Instance imported',
	},
	dropInstanceImportedText: {
		id: 'app.drop.instance-imported-text',
		defaultMessage: '{name} imported successfully.',
	},
	dropImportFailedTitle: {
		id: 'app.drop.import-failed-title',
		defaultMessage: 'Import failed',
	},
	dropImportFailedText: {
		id: 'app.drop.import-failed-text',
		defaultMessage: 'Failed to import {name}: {error}',
	},
	dropNoInstances: {
		id: 'app.drop.no-instances',
		defaultMessage: 'No instances found',
	},
	dropImportProgressTitle: {
		id: 'app.drop.import-progress-title',
		defaultMessage: 'Importing instances…',
	},
	dropImportProgressText: {
		id: 'app.drop.import-progress-text',
		defaultMessage: '{current} / {total} instances imported',
	},
	dropImportCompletedTitle: {
		id: 'app.drop.import-completed-title',
		defaultMessage: 'Import completed',
	},
	dropImportCompletedText: {
		id: 'app.drop.import-completed-text',
		defaultMessage: 'Successfully imported {count} instances',
	},
	dropImportCompletedPartialText: {
		id: 'app.drop.import-completed-partial-text',
		defaultMessage: 'Imported {completed} of {total} instances ({failed} failed)',
	},

	dropModpackInstalling: {
		id: 'app.drop.modpack-installing',
		defaultMessage: 'Installing modpack...',
	},
	dropModpackInstalledSuccess: {
		id: 'app.drop.modpack-installed-success',
		defaultMessage: 'Modpack installed successfully',
	},
	dropModpackInstallFailed: {
		id: 'app.drop.modpack-install-failed',
		defaultMessage: 'Failed to install modpack',
	},

	dropUnknownForceAnalysisTitle: {
		id: 'app.drop.unknown-force-analysis-title',
		defaultMessage: 'Unable to identify file type',
	},
	dropUnknownForceAnalysisText: {
		id: 'app.drop.unknown-force-analysis-text',
		defaultMessage:
			'This archive needs to be extracted and deeply analyzed to determine its content type. This may take a while. Force analysis?',
	},
	dropUnknownForceAnalysisButton: {
		id: 'app.drop.unknown-force-analysis-button',
		defaultMessage: 'Force analysis',
	},
	dropUnknownForceAnalyzing: {
		id: 'app.drop.unknown-force-analyzing',
		defaultMessage: 'Force analyzing archive...',
	},
	dropUnknownForceAnalysisFailedTitle: {
		id: 'app.drop.unknown-force-analysis-failed-title',
		defaultMessage: 'Analysis failed',
	},
	dropUnknownForceAnalysisFailedText: {
		id: 'app.drop.unknown-force-analysis-failed-text',
		defaultMessage: 'Could not identify the file type even after deep analysis.',
	},

	dropInstallModTitle: {
		id: 'app.drop.mod-compatibility-title',
		defaultMessage: 'Version Mismatch',
	},
	dropInstallModWarning: {
		id: 'app.drop.mod-compatibility-warning',
		defaultMessage:
			'This mod targets {modVersion} ({modLoader}), but the instance is {instVersion} ({instLoader}).',
	},
})

function getErrorNotificationDetails(notification) {
	const details = [notification.title, notification.text, notification.errorCode].filter(Boolean)
	if (notification.supportData) {
		details.push(JSON.stringify(notification.supportData, null, 2))
	}
	return details.join('\n\n')
}

async function exportNotificationErrorLogs(notification) {
	try {
		await exportErrorLogs(getErrorNotificationDetails(notification))
	} catch (error) {
		handleError(error)
	}
}

async function setupApp() {
	const initialSettings = await getSettings()
	await downloadManager.start()
	const {
		native_decorations,
		theme,
		accent_color,
		locale,
		telemetry,
		collapsed_navigation,
		hide_nametag_skins_page,
		advanced_rendering,
		onboarded,
		default_page,
		toggle_sidebar,
		custom_background_path,
		custom_background_blur,
		custom_background_opacity,
		transparent_background,
		transparent_background_opacity,
		transparent_background_blur,
		sidebar_instance_count,
		developer_mode,
		feature_flags,
		pending_update_toast_for_version,
	} = initialSettings

	// Initialize locale from saved settings
	if (locale) {
		i18n.global.locale.value = locale
	} else {
		const resolvedLocale = resolveInitialLocale(navigator.languages)
		i18n.global.locale.value = resolvedLocale
		initialSettings.locale = resolvedLocale
		await setSettings(initialSettings)
	}

	const defaultPageRoutes = {
		Home: '/',
		DiscoverContent: '/browse/modpack',
		Library: '/library',
	}
	const defaultPageRoute = offline.value ? '/library' : defaultPageRoutes[default_page]
	if (defaultPageRoute && defaultPageRoute !== '/') await router.push(defaultPageRoute)

	os.value = await getOS()
	const dev = await isDev()
	isDevEnvironment.value = dev
	const version = await getVersion()
	pendingUpdateAnnouncementVersion.value = pending_update_toast_for_version
	if (!onboarded && route.path !== '/') await router.replace('/')
	showOnboarding.value = !onboarded
	onboardingSettings.value = initialSettings

	nativeDecorations.value = native_decorations
	if (os.value !== 'MacOS') await getCurrentWindow().setDecorations(native_decorations)

	themeStore.setThemeState(theme)
	themeStore.setAccentColor(accent_color)
	themeStore.collapsedNavigation = collapsed_navigation
	themeStore.advancedRendering = advanced_rendering
	themeStore.hideNametagSkinsPage = hide_nametag_skins_page
	themeStore.toggleSidebar = toggle_sidebar
	themeStore.customBackgroundPath = custom_background_path
	themeStore.customBackgroundBlur = custom_background_blur
	themeStore.customBackgroundOpacity = custom_background_opacity
	themeStore.transparentBackground = transparent_background
	themeStore.transparentBackgroundOpacity = transparent_background_opacity
	themeStore.transparentBackgroundBlur = transparent_background_blur
	themeStore.setTransparentBackgroundClass()
	await applyWindowFrame()
	await applyWindowEffects()
	themeStore.sidebarInstanceCount = sidebar_instance_count
	themeStore.devMode = developer_mode
	themeStore.featureFlags = feature_flags
	stateInitialized.value = true

	isMaximized.value = await getCurrentWindow().isMaximized()

	await getCurrentWindow().onResized(async () => {
		isMaximized.value = await getCurrentWindow().isMaximized()
	})

	if (telemetry) {
		initAnalytics()
		if (dev) debugAnalytics()
		trackEvent('Launched', { version, dev, onboarded })
	}

	if (!dev) document.addEventListener('contextmenu', (event) => event.preventDefault())

	const osType = await getOsType()
	if (osType === 'macos') {
		document.getElementsByTagName('html')[0].classList.add('mac')
	} else {
		document.getElementsByTagName('html')[0].classList.add('windows')
	}

	await warning_listener(async (e) => {
		if (e.kind === 'minecraft_crash') {
			await minecraftCrashModal.value?.handleWarning(e)
			return
		}

		addNotification({
			title: formatMessage(messages.warning),
			text: e.message,
			type: 'warn',
		})
	})

	get_opening_command().then(handleCommand)
	fetchCredentials()

	try {
		const skins = (await get_available_skins()) ?? []
		const capes = (await get_available_capes()) ?? []
		generateSkinPreviews(skins, capes)
	} catch (error) {
		console.warn('Failed to generate skin previews in app setup.', error)
	}
}

function startOnboarding(mode = 'main') {
	onboardingReplay.value = false
	onboardingMode.value = mode
	showOnboarding.value = true
}

async function replayOnboarding(mode) {
	onboardingReplay.value = true
	onboardingMode.value = mode
	settingsModal.value?.hide()
	if (mode === 'main') await router.replace('/')
	showOnboarding.value = true
}

async function finishOnboarding() {
	const wasReplay = onboardingReplay.value
	const settings = onboardingSettings.value ?? (await getSettings())
	if (!onboardingReplay.value) {
		if (onboardingMode.value === 'instance') {
			settings.onboarding_instance_tour_completed = true
		} else {
			settings.onboarded = true
			settings.onboarding_version = 1
		}
		await setSettings(settings)
		onboardingSettings.value = settings
	}
	showOnboarding.value = false
	onboardingReplay.value = false
	if (!wasReplay) await scheduleStartupDialogs()
}

async function skipOnboarding() {
	await finishOnboarding()
}

function closeOnboardingSettings() {
	settingsModal.value?.hide()
}

async function handleUpdateAnnouncementClosed(version) {
	if (pendingUpdateAnnouncementVersion.value !== version) return

	const settings = await getSettings()
	if (settings.pending_update_toast_for_version === version) {
		settings.pending_update_toast_for_version = null
		await setSettings(settings)
	}
	pendingUpdateAnnouncementVersion.value = null
	updateAnnouncementShowing.value = false
	await new Promise((resolve) => setTimeout(resolve, 350))
	await scheduleStartupDialogs()
}

async function scheduleStartupDialogs() {
	if (!stateInitialized.value || showOnboarding.value || updateAnnouncementShowing.value) return

	if (pendingUpdateAnnouncementVersion.value && updateAnnouncementModal.value) {
		updateAnnouncementShowing.value = true
		settingsModal.value?.hide()
		await nextTick()
		updateAnnouncementModal.value.show(pendingUpdateAnnouncementVersion.value)
		return
	}

	communityAnnouncementModal.value?.showIfNeeded()
}

provide('replayOnboarding', replayOnboarding)
provide(
	minecraftLaunchErrorKey,
	async (launchError, payload) =>
		(await minecraftCrashModal.value?.handleLaunchError(launchError, payload)) ?? false,
)
provide('previewMinecraftCrashModal', () => minecraftCrashModal.value?.showPreview())
provide('chooseImportMethod', chooseImportMethod)
provide('previewUpdateAnnouncement', (version = null) => {
	const previewVersion = version ?? pendingUpdateAnnouncementVersion.value
	if (previewVersion) updateAnnouncementModal.value?.show(previewVersion)
})

const stateFailed = ref(false)
initialize_state()
	.then(() => {
		setupApp().catch((err) => {
			stateFailed.value = true
			console.error(err)
			error.showError(err, null, false, 'state_init')
		})
	})
	.catch((err) => {
		stateFailed.value = true
		console.error('Failed to initialize app', err)
		error.showError(err, null, false, 'state_init')
	})

const handleClose = async () => {
	await saveWindowState(StateFlags.ALL)
	await getCurrentWindow().close()
}

const loading = setupLoadingStateProvider()
loading.setEnabled(false)
let initialLoadToken = loading.begin()
let routerToken = null
let suspenseToken = null

let suspensePending = false

const sidebarOverlayScrollbarsOptions = Object.freeze({
	overflow: {
		x: 'hidden',
		y: 'scroll',
	},
})

router.beforeEach(() => {
	suspensePending = false
	if (routerToken) loading.end(routerToken)
	routerToken = loading.begin()
})
router.afterEach((to, from, failure) => {
	trackEvent('PageView', {
		path: to.path,
		fromPath: from.path,
		failed: failure,
	})
	setTimeout(() => {
		if (!suspensePending && stateInitialized.value) {
			if (initialLoadToken) {
				loading.end(initialLoadToken)
				initialLoadToken = null
			}
			if (routerToken) {
				loading.end(routerToken)
				routerToken = null
			}
		}
	}, 100)
})

function onSuspensePending() {
	suspensePending = true
	if (suspenseToken) loading.end(suspenseToken)
	suspenseToken = loading.begin()
}

function onSuspenseResolve() {
	if (suspenseToken) {
		loading.end(suspenseToken)
		suspenseToken = null
	}
	if (routerToken) {
		loading.end(routerToken)
		routerToken = null
	}
}

watch(
	stateInitialized,
	(ready) => {
		if (ready) {
			if (initialLoadToken) {
				loading.end(initialLoadToken)
				initialLoadToken = null
			}
			if (routerToken) {
				loading.end(routerToken)
				routerToken = null
			}
			void scheduleStartupDialogs()
		}
	},
	{ flush: 'post' },
)

watch(offline, (isOffline) => {
	if (isOffline && (route.path.startsWith('/browse') || route.path.startsWith('/project'))) {
		void router.push('/library')
	}
})

watch(
	() => route.path,
	(path) => {
		if (
			path.startsWith('/instance/') &&
			onboardingSettings.value?.onboarded &&
			!onboardingSettings.value?.onboarding_instance_tour_completed &&
			!showOnboarding.value
		) {
			startOnboarding('instance')
		}
	},
)

const error = useError()
error.setMinecraftLaunchErrorHandler((launchError, context) => {
	if (!minecraftCrashModal.value?.isLaunchFailure(launchError) || !context?.instanceId) return false
	void minecraftCrashModal.value.handleLaunchError(launchError, {
		instance_id: context.instanceId,
		instance_name: 'Minecraft',
	})
	return true
})
const errorModal = ref()
const minecraftAuthErrorModal = ref()

const contentInstall = createContentInstall({ router, handleError, addNotification })
provideContentInstall(contentInstall)
const {
	instances: contentInstallInstances,
	compatibleLoaders: contentInstallLoaders,
	gameVersions: contentInstallGameVersions,
	loading: contentInstallLoading,
	defaultTab: contentInstallDefaultTab,
	preferredLoader: contentInstallPreferredLoader,
	preferredGameVersion: contentInstallPreferredGameVersion,
	releaseGameVersions: contentInstallReleaseGameVersions,
	projectInfo: contentInstallProjectInfo,
	symlinkTarget: contentInstallSymlinkTarget,
	handleInstallToInstance,
	handleCreateAndInstall,
	handleNavigate: handleContentInstallNavigate,
	handleCancel: handleContentInstallCancel,
	setContentInstallModal,
	setModpackAlreadyInstalledModal: setContentInstallModpackAlreadyInstalledModal,
	handleModpackDuplicateCreateAnyway: handleContentInstallModpackDuplicateCreateAnyway,
	handleModpackDuplicateGoToInstance: handleContentInstallModpackDuplicateGoToInstance,
	setCurseForgeManualDownloadsModal: setContentInstallCurseForgeManualDownloadsModal,
	setIncompatibilityWarningModal: setContentIncompatibilityWarningModal,
	incompatibilityWarningVersions: contentInstallIncompatibilityWarningVersions,
	incompatibilityWarningCurrentGameVersion: contentInstallIncompatibilityWarningCurrentGameVersion,
	incompatibilityWarningCurrentLoader: contentInstallIncompatibilityWarningCurrentLoader,
	incompatibilityWarningProjectType: contentInstallIncompatibilityWarningProjectType,
	incompatibilityWarningProjectIconUrl: contentInstallIncompatibilityWarningProjectIconUrl,
	incompatibilityWarningProjectName: contentInstallIncompatibilityWarningProjectName,
	incompatibilityWarningMessage: contentInstallIncompatibilityWarningMessage,
	incompatibilityWarningInstalling: contentInstallIncompatibilityWarningInstalling,
	handleIncompatibilityWarningInstall: handleContentInstallIncompatibilityWarningInstall,
	handleIncompatibilityWarningCancel: handleContentInstallIncompatibilityWarningCancel,
} = contentInstall

/**
 * Handles @update from ContentUpdaterModal (incompatibility-warning mode).
 * In drag & drop mode (pendingDropIncompatibility set), installs the local file.
 * In normal content-install mode, delegates to the provider handler.
 */
async function handleIncompatibilityWarningUpdate(
	version: Labrinth.Versions.v2.Version,
	event: MouseEvent,
) {
	const pending = pendingDropIncompatibility.value
	if (pending) {
		pendingDropIncompatibility.value = null
		const projectTypeMap: Record<string, ContentFileProjectType | undefined> = {
			mod: 'mod',
			resource_pack: 'resourcepack',
			shader_pack: 'shaderpack',
			litematic: 'schematic',
			schematic: 'schematic',
		}
		const projectType = projectTypeMap[pending.type]
		try {
			await add_project_from_path(pending.instId, pending.filePath, projectType)
			addNotification({
				title: formatMessage(messages.dropContentInstalledTitle),
				text: formatMessage(messages.dropContentInstalledText),
				type: 'success',
			})
		} catch (e) {
			addNotification({
				title: formatMessage(messages.dropInstallFailedTitle),
				text: e instanceof Error ? e.message : typeof e === 'string' ? e : JSON.stringify(e),
				type: 'error',
			})
		}
		return
	}
	await handleContentInstallIncompatibilityWarningInstall(version, event)
}

/**
 * Handles @cancel from ContentUpdaterModal. Clears drag & drop state if set.
 */
function handleIncompatibilityWarningCancel() {
	pendingDropIncompatibility.value = null
	handleContentInstallIncompatibilityWarningCancel()
}

/**
 * Handles @searchCompat from ContentUpdaterModal simplified warning mode.
 * Navigates to the Modrinth project page or browse/search page.
 */
function handleDropInstallSearchCompat() {
	const pending = pendingDropIncompatibility.value
	if (!pending) return
	const searchName = pending.meta?.name ?? pending.meta?.mod_id ?? 'mod'
	const searchUrl = pending.modrinthLookup
		? `/project/${pending.modrinthLookup.project_id}`
		: `/browse/mod?q=${encodeURIComponent(searchName)}&i=${pending.instId}`
	pendingDropIncompatibility.value = null
	router.push(searchUrl)
}

const serverInstall = createServerInstall({ router, handleError, popupNotificationManager })
provideServerInstall(serverInstall)
const {
	setInstallToPlayModal: setServerInstallToPlayModal,
	setUpdateToPlayModal: setServerUpdateToPlayModal,
	setAddServerToInstanceModal: setServerAddServerToInstanceModal,
	playServerProject,
	symlinkTarget: addServerSymlinkTarget,
} = serverInstall

const modInstallModal = ref()
const modpackAlreadyInstalledModal = ref()
const contentInstallModpackAlreadyInstalledModal = ref()
const contentInstallCurseForgeManualDownloadsModal = ref()
const addServerToInstanceModal = ref()
const incompatibilityWarningModal = ref()
const installToPlayModal = ref()
const updateToPlayModal = ref()

const modrinthLoginFlowWaitModal = ref()

const confirmDropModal = ref<InstanceType<typeof ConfirmDropTypeModal> | null>(null)
const dropClassification = ref<ClassificationResult | null>(null)
const dropFileName = ref('')
const dropFilePath = ref('')

const { isInInstance, instanceId } = useInstanceContext()
const genericInstallModal = ref<InstanceType<typeof GenericContentInstallModal> | null>(null)
const launcherImportModal = ref<InstanceType<typeof LauncherImportModal> | null>(null)
const symlinkCardsModal = ref<InstanceType<typeof SymlinkMethodCards> | null>(null)
const scanningInstances = ref(false)
const pendingInstall = ref<{ type: string; filePath: string } | null>(null)
const pendingDropIncompatibility = ref<{
	filePath: string
	instId: string
	type: string
	instVersion: string | undefined
	instLoader: string | undefined
	meta: { name?: string; mod_id?: string } | null
	modrinthLookup: ModrinthLookupResult | null
} | null>(null)
const selectedInstances = ref<
	Array<{ launcherType: string; basePath: string; name: string; path: string }>
>([])
const currentImportContext = ref<{ launcherType: string; basePath: string } | null>(null)

const dropDebug = useDebugLogger('DropFlow')

const dropProcessingNotificationId = ref<number | null>(null)

const { isDragging, isProcessing } = useGlobalDrop(
	{
		classifyFile: async (path) => {
			if (onSkinsPage.value) {
				return { item_type: 'unknown' as const, file_path: path, reason: 'skipped' }
			}
			return classifyDroppedItem(path)
		},
		onClassifyStart: (fileName) => {
			if (onSkinsPage.value) return
			// Immediate feedback when a file is dropped — show a notification
			// with the file name before classification even begins.
			dropProcessingNotificationId.value = addNotification({
				title: formatMessage(messages.dropProcessing, { name: fileName }),
				type: 'info',
				autoCloseMs: null,
			}).id
		},
		onImportStart: (type, classification) => {
			if (type === 'unknown' && classification?.reason === 'skipped') return
			dropClassification.value = classification
			dropFilePath.value = classification.file_path ?? classification.base_path ?? ''
			dropFileName.value =
				classification.file_path?.split('/').pop() ??
				classification.base_path?.split(/[/\\]/).pop() ??
				'file'

			if (type === 'unknown' && classification?.reason?.toLowerCase().includes('extraction')) {
				clearDropProcessingNotification()
				showForceAnalysisPrompt(classification)
				return
			}

			if (type === 'unknown') {
				clearDropProcessingNotification()
				const unknownFile =
					classification?.file_path?.split(/[/\\]/).pop() ??
					classification?.base_path?.split(/[/\\]/).pop() ??
					''

				// .tmp files are OS-level temp copies from drag-and-drop (browser, archive, etc.)
				const isTempFile = unknownFile.startsWith('.tmp') || unknownFile.startsWith('tmp')
				if (isTempFile) {
					addNotification({
						title: 'Temporary file detected',
						text:
							`The file "${unknownFile}" appears to be a temporary copy. ` +
							`Try dragging the file from its original folder instead of from a browser, ` +
							`archive, or cloud storage.`,
						type: 'warning',
					})
				} else {
					addNotification({
						title: formatMessage(messages.dropUnknownTitle),
						text: classification?.reason
							? classification.reason
							: formatMessage(messages.dropUnknownText),
						type: 'error',
					})
				}
				return
			}

			confirmDropModal.value?.show()
		},
		onImportEnd: () => {},
		onError: (reason) => {
			clearDropProcessingNotification()

			if (reason === 'multiple-files') {
				addNotification({
					title: formatMessage(messages.dropMultipleFilesTitle),
					text: formatMessage(messages.dropMultipleFilesText),
					type: 'error',
				})
			} else if (reason === 'shortcut-exceeded') {
				addNotification({
					title: formatMessage(messages.dropShortcutFailedTitle),
					text: formatMessage(messages.dropShortcutFailedText),
					type: 'error',
				})
			} else if (reason === 'unknown') {
				addNotification({
					title: formatMessage(messages.dropUnknownTitle),
					text: formatMessage(messages.dropUnknownText),
					type: 'error',
				})
			} else {
				addNotification({
					title: formatMessage(messages.dropErrorTitle),
					text: reason,
					type: 'error',
				})
			}
		},
	},
	fileDrop,
)

function clearDropProcessingNotification() {
	if (dropProcessingNotificationId.value !== null) {
		notificationManager.removeNotification(dropProcessingNotificationId.value)
		dropProcessingNotificationId.value = null
	}
}

function handleDropCancel() {
	clearDropProcessingNotification()
	dropClassification.value = null
}

async function handleDropConfirm(type: string) {
	const classification = dropClassification.value
	dropClassification.value = null
	confirmDropModal.value?.hide()

	dropDebug('handleDropConfirm: entry', {
		type,
		classification_item_type: classification?.item_type,
		file_path: classification?.file_path,
	})

	const isLauncherImport =
		classification?.item_type === 'launcher' || classification?.item_type === 'hmcl_launcher'

	if (!isLauncherImport && !classification?.file_path && !dropFilePath.value) {
		dropDebug(
			'handleDropConfirm: no filePath available (classification and dropFilePath both empty), aborting',
		)
		return
	}

	const filePath = classification?.file_path ?? dropFilePath.value
	const fileName =
		filePath?.split('/').pop() ?? classification.base_path?.split(/[/\\]/).pop() ?? 'file'
	dropDebug('handleDropConfirm: routing decision', {
		type,
		isLauncherImport,
		item_type: classification?.item_type,
	})

	if (type === 'dot_minecraft') {
		dropDebug('handleDropConfirm: .minecraft folder branch', {
			dropFilePath: dropFilePath.value,
		})
		if (!dropFilePath.value) {
			dropDebug('handleDropConfirm: dot_minecraft — no dropFilePath, aborting')
			return
		}
		// Treat the .minecraft folder path as a vanilla-style instance source
		// and scan it for importable instances
		currentImportContext.value = { launcherType: 'Generic', basePath: dropFilePath.value }
		scanningInstances.value = true
		const results = await scanLauncherInstances('Generic', dropFilePath.value)
		scanningInstances.value = false
		const totalInstances = results.reduce((s, r) => s + r.instances.length, 0)
		dropDebug('handleDropConfirm: .minecraft scan result', { totalInstances, results })

		if (totalInstances === 0) {
			currentImportContext.value = null
			scanningInstances.value = false
			dropDebug('handleDropConfirm: no instances found in .minecraft folder')
			addNotification({ title: formatMessage(messages.dropNoInstances), type: 'warning' })
			return
		}

		if (totalInstances === 1 && results[0]?.instances[0]) {
			const single = results[0].instances[0]
			dropDebug('handleDropConfirm: single instance from .minecraft, showing symlink modal', {
				name: single.name,
				path: single.path,
			})
			selectedInstances.value = [
				{
					launcherType: 'Generic',
					basePath: dropFilePath.value,
					name: single.name,
					path: single.path,
				},
			]
			const cap = await check_symlink_capability()
			symlinkCardsModal.value?.show({
				instanceNames: [single.name],
				symlinkCapable: cap,
			})
			return
		}

		// Multiple instances → show selection modal
		dropDebug(
			'handleDropConfirm: multiple instances from .minecraft, showing launcher import modal',
		)
		launcherImportModal.value?.show(results)
		return
	}

	if (isLauncherImport && type === 'instance') {
		const launcherType =
			classification!.item_type === 'hmcl_launcher' ? 'HMCL' : classification!.launcher_type!
		const basePath =
			classification!.item_type === 'hmcl_launcher'
				? classification!.launcher_dir!
				: classification!.base_path!
		dropDebug('handleDropConfirm: launcher import branch', { launcherType, basePath })

		currentImportContext.value = { launcherType, basePath }
		scanningInstances.value = true
		const results = await scanLauncherInstances(launcherType, basePath)
		scanningInstances.value = false
		const totalInstances = results.reduce((s, r) => s + r.instances.length, 0)
		dropDebug('handleDropConfirm: launcher scan result', { totalInstances, results })

		if (totalInstances === 0) {
			currentImportContext.value = null
			scanningInstances.value = false
			dropDebug('handleDropConfirm: no instances found')
			addNotification({ title: formatMessage(messages.dropNoInstances), type: 'warning' })
			return
		}

		if (totalInstances === 1 && results[0]?.instances[0]) {
			// Single instance → go directly to symlink method selection
			const single = results[0].instances[0]
			dropDebug('handleDropConfirm: single instance, showing symlink modal', {
				name: single.name,
				path: single.path,
			})
			selectedInstances.value = [{ launcherType, basePath, name: single.name, path: single.path }]
			const cap = await check_symlink_capability()
			symlinkCardsModal.value?.show({
				instanceNames: [single.name],
				symlinkCapable: cap,
			})
			return
		}

		// Multiple instances → show selection modal
		dropDebug('handleDropConfirm: multiple instances, showing launcher import modal')
		launcherImportModal.value?.show(results)
		return
	}

	if (type === 'modpack') {
		dropDebug('handleDropConfirm: modpack branch', { filePath, fileName })

		if (!filePath) {
			dropDebug('handleDropConfirm: modpack — no filePath, aborting')
			addNotification({ title: formatMessage(messages.dropModpackInstallFailed), type: 'error' })
			return
		}

		// ── Replace "Processing..." with "Installing..." immediately (pure frontend) ──
		clearDropProcessingNotification()
		let installingNotify = addNotification({
			title: formatMessage(messages.dropModpackInstalling),
			type: 'info',
			autoCloseMs: null,
		})

		const isMrpack = !!fileName?.toLowerCase().endsWith('.mrpack')
		const location = { type: 'fromFile' as const, path: filePath }

		const doInstall = async () => {
			const job = await install_create_modpack_instance(location).catch(handleError)
			if (!job) {
				notificationManager.removeNotification(installingNotify.id)
				return
			}

			// Single-use listener that auto-cleans up when the job reaches a terminal state
			const unlisten = await install_job_listener((updatedJob: InstallJobSnapshot) => {
				if (updatedJob.job_id !== job.job_id) return

				if (updatedJob.status === 'succeeded') {
					notificationManager.removeNotification(installingNotify.id)
					addNotification({
						title: formatMessage(messages.dropModpackInstalledSuccess),
						type: 'success',
					})

					unlisten()
				} else if (['failed', 'canceled', 'interrupted'].includes(updatedJob.status)) {
					notificationManager.removeNotification(installingNotify.id)
					unlisten()
				}
			})
		}

		// .mrpack is a well-defined standard — skip preview, install directly.
		// .zip needs a preview pass to scan entry names and determine what we're dealing with.
		if (!isMrpack) {
			const preview = await install_get_modpack_preview(location).catch((e) => {
				dropDebug('handleDropConfirm: modpack preview failed', { error: e })
				notificationManager.removeNotification(installingNotify.id)
				handleError(e)
				return null
			})
			if (!preview) return

			if (preview.unknownFile) {
				// Clear "Installing..." — warning modal will handle re-showing on confirm
				notificationManager.removeNotification(installingNotify.id)
				unknownPackWarningModal.value?.show(async () => {
					installingNotify = addNotification({
						title: formatMessage(messages.dropModpackInstalling),
						type: 'info',
						autoCloseMs: null,
					})
					await doInstall()
				}, fileName)
				trackEvent('InstanceCreate', { source: 'DropConfirmModpack' })
				await router.push('/library')
				return
			}
		}

		await doInstall()
		trackEvent('InstanceCreate', { source: 'DropConfirmModpack' })
		await router.push('/library')
		return
	}

	// Content types that can be installed
	const contentTypes = [
		'mod',
		'resource_pack',
		'shader_pack',
		'world_save',
		'litematic',
		'schematic',
	]
	if (!contentTypes.includes(type)) {
		dropDebug('handleDropConfirm: type not in contentTypes — FALLTHROUGH, no handler!', {
			type,
			contentTypes,
		})
		return
	}

	dropDebug('handleDropConfirm: content install branch', {
		type,
		isInInstance: isInInstance.value,
		hasInstanceId: !!instanceId.value,
	})

	if (isInInstance.value && instanceId.value) {
		dropDebug('handleDropConfirm: installing directly to current instance', {
			instanceId: instanceId.value,
		})
		await installContentDirectly(type, filePath, instanceId.value)
	} else {
		// Store pending install info for when an instance is selected
		dropDebug('handleDropConfirm: storing pending install, showing instance selection modal')
		pendingInstall.value = { type, filePath }

		// Load all instances for the selection modal
		let instances: {
			id: string
			name: string
			iconUrl?: string | null
			gameVersion?: string | null
			loader?: string | null
		}[] = []
		try {
			const allInstances = await listInstances()
			instances = allInstances.map((inst) => ({
				id: inst.id,
				name: inst.name,
				iconUrl: inst.icon_path ? convertFileSrc(inst.icon_path) : null,
				gameVersion: inst.game_version || null,
				loader: inst.loader || null,
			}))
		} catch {
			// If listing fails, show empty list
		}
		genericInstallModal.value?.show({
			contentType: type,
			filePath,
			fileName,
			instances,
		})
	}
}

async function installContentDirectly(type: string, filePath: string, instId: string) {
	try {
		if (type === 'world_save') {
			await import_world_save(instId, filePath)
			addNotification({
				title: formatMessage(messages.dropWorldImportedTitle),
				text: formatMessage(messages.dropWorldImportedText),
				type: 'success',
			})
			return
		}

		if (type === 'mod') {
			let meta: {
				minecraft_version?: string
				loader?: string
				name?: string
				mod_id?: string
			} | null = null
			let modrinthLookup: ModrinthLookupResult | null = null

			const metaStr = await extractModMetadata(filePath)
			dropDebug('installContentDirectly: mod metadata extraction', { filePath, hasMeta: !!metaStr })

			if (metaStr) {
				try {
					meta = JSON.parse(metaStr)
					dropDebug('installContentDirectly: parsed mod metadata', { meta })
				} catch (e) {
					dropDebug('installContentDirectly: failed to parse mod metadata', { error: e })
				}
			}

			try {
				modrinthLookup = await lookupModHash(filePath)
				dropDebug('installContentDirectly: modrinth hash lookup', { found: !!modrinthLookup })
			} catch (e) {
				dropDebug('installContentDirectly: hash lookup failed', { error: e })
			}

			const inst = await getInstance(instId)
			dropDebug('installContentDirectly: instance details', {
				inst: inst?.id,
				game_version: inst?.game_version,
				loader: inst?.loader,
			})

			if (inst && meta?.minecraft_version) {
				const instVersion = inst.game_version
				const instLoader = inst.loader
				const modMcVersion = meta.minecraft_version
				const modLoader = meta.loader

				let versionMismatch = false
				if (modMcVersion && instVersion) {
					versionMismatch = !isVersionInRange(instVersion, modMcVersion)
				}

				let loaderMismatch = false
				if (modLoader && instLoader) {
					loaderMismatch = !areLoadersCompatible(modLoader, instLoader)
				}

				dropDebug('installContentDirectly: compatibility check', {
					versionMismatch,
					loaderMismatch,
					modMcVersion,
					instVersion,
					modLoader,
					instLoader,
				})

				if (versionMismatch || loaderMismatch) {
					pendingDropIncompatibility.value = {
						filePath,
						instId,
						type,
						instVersion,
						instLoader,
						meta,
						modrinthLookup,
					}
					const warning = formatMessage(messages.dropInstallModWarning, {
						modVersion: modMcVersion ?? 'any',
						modLoader: modLoader ?? 'any',
						instVersion: instVersion ?? 'any',
						instLoader: instLoader ?? 'none',
					})
					contentInstallIncompatibilityWarningVersions.value = []
					contentInstallIncompatibilityWarningCurrentGameVersion.value = instVersion ?? ''
					contentInstallIncompatibilityWarningCurrentLoader.value = instLoader ?? ''
					contentInstallIncompatibilityWarningProjectType.value = 'mod'
					contentInstallIncompatibilityWarningProjectName.value = meta?.name ?? 'Mod'
					contentInstallIncompatibilityWarningMessage.value = warning
					contentInstallIncompatibilityWarningInstalling.value = false
					await nextTick()
					incompatibilityWarningModal.value?.show()
					return
				}
			} else {
				dropDebug('installContentDirectly: skipping version check', {
					hasInstance: !!inst,
					hasModVersion: !!meta?.minecraft_version,
				})
			}
		}

		const projectTypeMap: Record<string, ContentFileProjectType | undefined> = {
			mod: 'mod',
			resource_pack: 'resourcepack',
			shader_pack: 'shaderpack',
			litematic: 'schematic',
			schematic: 'schematic',
		}
		const projectType = projectTypeMap[type]
		await add_project_from_path(instId, filePath, projectType)
		addNotification({
			title: formatMessage(messages.dropContentInstalledTitle),
			text: formatMessage(messages.dropContentInstalledText),
			type: 'success',
		})
	} catch (e) {
		let errMsg = e instanceof Error ? e.message : typeof e === 'string' ? e : JSON.stringify(e)
		try {
			const lockInfo = await detectFileLock(filePath)
			if (lockInfo.length > 0) {
				const lockLines = lockInfo.map((p) => `  PID ${p.pid}: ${p.name} (${p.path})`).join('\n')
				errMsg += `\n\nFile locked by:\n${lockLines}`
			}
		} catch {
			// Lock detection is best-effort
		}
		addNotification({
			title: formatMessage(messages.dropInstallFailedTitle),
			text: errMsg,
			type: 'error',
		})
	}
}

/**
 * Show a popup notification asking the user to confirm force-analysis
 * (extraction + classification) of a ZIP archive that couldn't be identified
 * from entry names alone.
 */
function showForceAnalysisPrompt(classification: ClassificationResult) {
	const filePath = dropFilePath.value
	if (!filePath) return

	dropDebug('showForceAnalysisPrompt: showing force-analysis prompt', {
		reason: classification.reason,
		filePath,
	})

	addPopupNotification({
		title: formatMessage(messages.dropUnknownForceAnalysisTitle),
		text: formatMessage(messages.dropUnknownForceAnalysisText),
		type: 'info',
		autoCloseMs: null,
		buttons: [
			{
				label: formatMessage(messages.dropUnknownForceAnalysisButton),
				action: async () => {
					const analyzingNotification = addNotification({
						title: formatMessage(messages.dropUnknownForceAnalyzing),
						type: 'info',
						autoCloseMs: null,
					})

					try {
						const result = await classifyDroppedItemWithExtraction(filePath)
						notificationManager.removeNotification(analyzingNotification.id)

						if (result.item_type === 'unknown') {
							addNotification({
								title: formatMessage(messages.dropUnknownForceAnalysisFailedTitle),
								text: formatMessage(messages.dropUnknownForceAnalysisFailedText),
								type: 'error',
							})
							return
						}

						// Success
						// user already confirmed
						dropClassification.value = result
						switch (result.item_type) {
							case 'modpack':
								await handleDropConfirm('modpack')
								break
							case 'world_save':
								await handleDropConfirm('world_save')
								break
							case 'launcher':
							case 'hmcl_launcher':
								await handleDropConfirm('instance')
								break
							default:
								// mod, resource_pack, shader_pack, litematic to content install
								await handleDropConfirm(result.item_type)
								break
						}
					} catch (e) {
						notificationManager.removeNotification(analyzingNotification.id)
						addNotification({
							title: formatMessage(messages.dropUnknownForceAnalysisFailedTitle),
							text: e instanceof Error ? e.message : String(e),
							type: 'error',
						})
					}
				},
				color: 'brand',
			},
		],
	})
}

async function handleGenericInstall(instanceId: string) {
	genericInstallModal.value?.hide()
	const pending = pendingInstall.value
	pendingInstall.value = null
	if (!pending) return

	await installContentDirectly(pending.type, pending.filePath, instanceId)
}

async function handleGenericInstallNavigateCreate() {
	genericInstallModal.value?.hide()
	router.push('/create')
}

let symlinkChoiceResolve: ((symlink: boolean) => void) | null = null

function chooseImportMethod(options: {
	instanceNames: string[]
	symlinkCapable: 'supported' | 'requires_admin' | 'unsupported'
}): Promise<boolean> {
	return new Promise((resolve) => {
		symlinkChoiceResolve = resolve
		symlinkCardsModal.value?.show(options)
	})
}

async function onImportSelected(
	selections: Array<{
		launcherType: string
		launcherName: string
		instances: Array<{ name: string; path: string }>
	}>,
) {
	const allSelected: Array<{ launcherType: string; basePath: string; name: string; path: string }> =
		[]
	for (const sel of selections) {
		for (const inst of sel.instances) {
			allSelected.push({
				launcherType: sel.launcherType,
				basePath: '',
				name: inst.name,
				path: inst.path,
			})
		}
	}
	if (allSelected.length === 0) return
	selectedInstances.value = allSelected

	const cap = await check_symlink_capability()
	symlinkCardsModal.value?.show({
		instanceNames: allSelected.map((i) => i.name),
		symlinkCapable: cap,
	})
}

function onSymlinkMethodCancelled() {
	if (symlinkChoiceResolve) {
		symlinkChoiceResolve(false)
		symlinkChoiceResolve = null
	}
	symlinkCardsModal.value?.hide()
}

async function onSymlinkMethodConfirmed(symlink: boolean) {
	// Resolve the promise-based chooser first (if called from creation-modal flow)
	if (symlinkChoiceResolve) {
		symlinkChoiceResolve(symlink)
		symlinkChoiceResolve = null
		return
	}

	// Otherwise handle the drop import flow directly
	const instances = selectedInstances.value
	selectedInstances.value = []
	const ctx = currentImportContext.value
	currentImportContext.value = null
	if (instances.length === 0) return

	// Single instance: simple notification (no progress overlay needed)
	if (instances.length === 1) {
		const inst = instances[0]
		try {
			const job = await import_instance(
				ctx?.launcherType ?? inst.launcherType,
				ctx?.basePath ?? inst.path,
				inst.name,
				symlink,
			)
			await wait_for_install_job(job.job_id)
			addNotification({
				title: formatMessage(messages.dropInstanceImportedTitle),
				text: formatMessage(messages.dropInstanceImportedText, { name: inst.name }),
				type: 'success',
			})
		} catch (e) {
			addNotification({
				title: formatMessage(messages.dropImportFailedTitle),
				text: formatMessage(messages.dropImportFailedText, { name: inst.name, error: String(e) }),
				type: 'error',
			})
		}
		return
	}

	// Multiple instances: show cumulative progress
	const total = instances.length
	let completed = 0
	let failedCount = 0

	let progressNotif = addNotification({
		title: formatMessage(messages.dropImportProgressTitle),
		text: formatMessage(messages.dropImportProgressText, { current: 0, total }),
		type: 'info',
		autoCloseMs: null,
	})

	for (let i = 0; i < instances.length; i++) {
		const inst = instances[i]

		// Update progress notification
		notificationManager.removeNotification(progressNotif.id)
		progressNotif = addNotification({
			title: formatMessage(messages.dropImportProgressTitle),
			text: formatMessage(messages.dropImportProgressText, {
				current: i + 1,
				total,
			}),
			type: 'info',
			autoCloseMs: null,
		})

		try {
			const job = await import_instance(
				ctx?.launcherType ?? inst.launcherType,
				ctx?.basePath ?? inst.path,
				inst.name,
				symlink,
			)
			await wait_for_install_job(job.job_id)
			completed++
		} catch (e) {
			failedCount++
			addNotification({
				title: formatMessage(messages.dropImportFailedTitle),
				text: formatMessage(messages.dropImportFailedText, { name: inst.name, error: String(e) }),
				type: 'error',
			})
		}
	}

	// Final summary — replace progress notification
	notificationManager.removeNotification(progressNotif.id)
	if (failedCount === 0) {
		addNotification({
			title: formatMessage(messages.dropImportCompletedTitle),
			text: formatMessage(messages.dropImportCompletedText, { count: total }),
			type: 'success',
		})
	} else {
		addNotification({
			title: formatMessage(messages.dropImportCompletedTitle),
			text: formatMessage(messages.dropImportCompletedPartialText, {
				completed,
				failed: failedCount,
				total,
			}),
			type: 'warning',
		})
	}
}

function handleDropHelp() {
	dropClassification.value = null
	confirmDropModal.value?.hide()
	router.push('/help/drop')
}

watch(incompatibilityWarningModal, (modal) => {
	if (modal) {
		setContentIncompatibilityWarningModal(modal)
	}
})

setupAuthProvider(credentials, async (_redirectPath) => {
	if (AxolotlBrandConfig.capabilities.privateModrinthServices) await signIn()
})

async function validateSession(sessionToken) {
	try {
		const response = await tauriFetch(`${getOfficialLabrinthBaseUrl()}/v2/user`, {
			method: 'GET',
			headers: { Authorization: sessionToken },
		})
		if (response.status === 401) return false
		return true
	} catch {
		return true
	}
}

async function fetchCredentials() {
	if (!AxolotlBrandConfig.capabilities.privateModrinthServices) {
		credentials.value = null
		return
	}
	const creds = await getCreds().catch(handleError)
	if (creds && creds.user_id) {
		if (creds.session && !(await validateSession(creds.session))) {
			await logout().catch(handleError)
			credentials.value = null
			return
		}
		creds.user = await get_user(creds.user_id, 'bypass').catch(handleError)
	}
	credentials.value = creds ?? null
}

async function signIn() {
	modrinthLoginFlowWaitModal.value.show()

	try {
		await login()
		await fetchCredentials()
	} catch (error) {
		if (
			typeof error === 'object' &&
			typeof error['message'] === 'string' &&
			error.message.includes('Login canceled')
		) {
			// Not really an error due to being a result of user interaction, show nothing
		} else {
			handleError(error)
		}
	} finally {
		modrinthLoginFlowWaitModal.value.hide()
	}
}

async function logOut() {
	await logout().catch(handleError)
	await fetchCredentials()
}

onMounted(() => {
	invoke('show_window')

	error.setErrorModal(errorModal.value)
	error.setMinecraftAuthErrorModal(minecraftAuthErrorModal.value)

	setContentIncompatibilityWarningModal(incompatibilityWarningModal.value)
	setContentInstallModal(modInstallModal.value)
	setContentInstallModpackAlreadyInstalledModal(contentInstallModpackAlreadyInstalledModal.value)
	setContentInstallCurseForgeManualDownloadsModal(
		contentInstallCurseForgeManualDownloadsModal.value,
	)
	setModpackAlreadyInstalledModal(modpackAlreadyInstalledModal.value)
	setServerAddServerToInstanceModal(addServerToInstanceModal.value)
	setServerInstallToPlayModal(installToPlayModal.value)
	setServerUpdateToPlayModal(updateToPlayModal.value)
})

const accounts = ref(null)
provide('accountsCard', accounts)

command_listener(handleCommand)

async function handleCommand(e) {
	if (!e) return
	if (offline.value && e.event !== 'LaunchInstance') {
		await router.push('/library')
		return
	}

	if (e.event === 'RunMRPack') {
		// RunMRPack should directly install a local modpack file given a path;
		// non-mrpack archives (CurseForge/MCBBS/HMCL/MultiMC zips) are format-sniffed by the backend
		if (e.path.endsWith('.mrpack') || e.path.endsWith('.zip')) {
			const location = { type: 'fromFile', path: e.path }
			const preview = await install_get_modpack_preview(location).catch(handleError)
			if (preview?.unknownFile) {
				const splitPath = e.path.split(/[\\/]/)
				const fileName = splitPath ? splitPath[splitPath.length - 1] : e.path
				unknownPackWarningModal.value?.show(
					() => install_create_modpack_instance(location).then(() => undefined),
					fileName,
				)
			} else {
				await install_create_modpack_instance(location).catch(handleError)
			}
			trackEvent('InstanceCreate', {
				source: 'CreationModalFileDrop',
			})
		}
	} else if (e.event === 'LaunchInstance') {
		const instance = await getInstance(e.id).catch(() => null)
		const handleLaunchCommandError = async (launchError) => {
			const handled =
				(await minecraftCrashModal.value?.handleLaunchError(launchError, {
					instance_id: e.id,
					instance_name: instance?.name || 'Minecraft',
				})) ?? false
			if (!handled) handleError(launchError)
		}
		if (e.server) {
			await start_join_server(e.id, e.server).catch(handleLaunchCommandError)
		} else if (e.singleplayer_world) {
			await start_join_singleplayer_world(e.id, e.singleplayer_world).catch(
				handleLaunchCommandError,
			)
		} else {
			await run(e.id).catch(handleLaunchCommandError)
		}
	} else if (e.event === 'InstallServer') {
		await router.push(`/project/${e.id}`)
		await playServerProject(e.id).catch(handleError)
	} else if (e.event === 'InstallVersion') {
		const version = await get_version(e.id, 'must_revalidate').catch(handleError)
		if (version) {
			await contentInstall
				.install(version.project_id, version.id, null, 'URLConfirmModal', undefined, undefined, {
					showProjectInfo: true,
				})
				.catch(handleError)
		}
	} else {
		await contentInstall
			.install(e.id, null, null, 'URLConfirmModal', undefined, undefined, { showProjectInfo: true })
			.catch(handleError)
	}
}

const appUpdateDownload = {
	progress: appUpdateState.progress,
	version: ref(),
}
let unlistenUpdateDownload

const {
	metered,
	finishedDownloading,
	downloading,
	restarting,
	availableUpdate,
	updateSize,
	updatesEnabled,
} = appUpdateState
let delayedUpdatePopupTimeout = null

const updatePopupMessages = defineMessages({
	updateAvailable: {
		id: 'app.update-popup.title',
		defaultMessage: 'Update available',
	},
	downloadComplete: {
		id: 'app.update-popup.download-complete',
		defaultMessage: 'Download complete',
	},
	meteredBody: {
		id: 'app.update-popup.body.metered',
		defaultMessage: `Axolotl Launcher v{version} is available now! Since you're on a metered network, we didn't automatically download it.`,
	},
	downloadedBody: {
		id: 'app.update-popup.body.download-complete',
		defaultMessage: `Axolotl Launcher v{version} has finished downloading. Reload to update now, or automatically when you close Axolotl Launcher.`,
	},
	linuxBody: {
		id: 'app.update-popup.body.linux',
		defaultMessage:
			"Axolotl Launcher v{version} is available. We didn't automatically download it \u2014 use the Update button to choose between the built-in updater or your package manager.",
	},
	reload: {
		id: 'app.update-popup.reload',
		defaultMessage: 'Reload to update',
	},
	download: {
		id: 'app.update-popup.download',
		defaultMessage: 'Download ({size})',
	},
	changelog: {
		id: 'app.update-popup.changelog',
		defaultMessage: 'Changelog',
	},
})

function clearDelayedUpdatePopup() {
	if (delayedUpdatePopupTimeout !== null) {
		clearTimeout(delayedUpdatePopupTimeout)
		delayedUpdatePopupTimeout = null
	}
}

function getCurrentUpdatePromptStage() {
	return finishedDownloading.value ? 'downloaded' : 'available'
}

function scheduleDelayedUpdatePopup() {
	clearDelayedUpdatePopup()

	const version = availableUpdate.value?.version
	if (!version) {
		return
	}

	const nextPopupTime = getNextAppUpdatePopupTime(version, getCurrentUpdatePromptStage())
	if (nextPopupTime === null) {
		return
	}

	const delay = nextPopupTime - Date.now()
	if (delay <= 0) {
		showDelayedUpdatePopup()
		return
	}

	delayedUpdatePopupTimeout = setTimeout(showDelayedUpdatePopup, Math.min(delay, 2_147_483_647))
}

function showDelayedUpdatePopup() {
	const update = availableUpdate.value
	if (!update) {
		return
	}

	const stage = getCurrentUpdatePromptStage()
	const nextPopupTime = getNextAppUpdatePopupTime(update.version, stage)
	if (nextPopupTime === null) {
		return
	}

	if (Date.now() < nextPopupTime) {
		scheduleDelayedUpdatePopup()
		return
	}

	if (os.value === 'Linux' && !finishedDownloading.value) {
		addPopupNotification({
			title: formatMessage(updatePopupMessages.updateAvailable),
			text: formatMessage(updatePopupMessages.linuxBody, { version: update.version }),
			type: 'info',
			autoCloseMs: null,
			buttons: [
				{
					label: formatMessage(updatePopupMessages.changelog),
					action: () => openAppUpdateChangelog(),
					color: 'brand',
					keepOpen: true,
				},
			],
		})
	} else if (metered.value && !finishedDownloading.value) {
		addPopupNotification({
			title: formatMessage(updatePopupMessages.updateAvailable),
			text: formatMessage(updatePopupMessages.meteredBody, { version: update.version }),
			type: 'info',
			autoCloseMs: null,
			buttons: [
				{
					label: formatMessage(updatePopupMessages.download, {
						size: formatBytes(updateSize.value ?? 0),
					}),
					action: () => downloadAvailableAppUpdate(),
					color: 'brand',
				},
				{
					label: formatMessage(updatePopupMessages.changelog),
					action: () => openAppUpdateChangelog(),
					keepOpen: true,
				},
			],
		})
	} else if (finishedDownloading.value) {
		addPopupNotification({
			title: formatMessage(updatePopupMessages.downloadComplete),
			text: formatMessage(updatePopupMessages.downloadedBody, {
				version: update.version,
			}),
			type: 'success',
			autoCloseMs: null,
			buttons: [
				{
					label: formatMessage(updatePopupMessages.reload),
					action: () => installAvailableAppUpdate(),
					color: 'brand',
				},
				{
					label: formatMessage(updatePopupMessages.changelog),
					action: () => openAppUpdateChangelog(),
					keepOpen: true,
				},
			],
		})
	} else {
		scheduleDelayedUpdatePopup()
		return
	}

	markAppUpdatePopupShown(update.version, stage)
}

let lastUpdateSource = 'cnb'

async function performUpdateCheck() {
	const source = getUpdateSource()
	if (source !== lastUpdateSource) {
		availableUpdate.value = null
		updateSize.value = null
		appUpdateDownload.progress.value = 0
		finishedDownloading.value = false
		downloading.value = false
		lastUpdateSource = source
	}

	const update = await checkAppUpdate(source)
	if (!update) {
		console.log('No update available')
		return 'up-to-date'
	}

	const isExistingUpdate = update.version === availableUpdate.value?.version

	if (isExistingUpdate) {
		console.log('Update is already known')
		scheduleDelayedUpdatePopup()
		return 'available'
	}

	appUpdateDownload.progress.value = 0
	finishedDownloading.value = false
	downloading.value = false
	updateSize.value = null
	availableUpdate.value = update

	console.log(`Update ${update.version} is available.`)

	metered.value = await isNetworkMetered()
	if (!metered.value && os.value !== 'Linux') {
		console.log('Starting download of update')
		downloadUpdate(update)
	} else {
		if (os.value === 'Linux') {
			console.log('Linux detected, deferring update to smart update flow.')
		} else {
			console.log(
				`Metered connection detected, not auto-downloading update.`,
			)
		}
		markAppUpdateActionable(update.version)
		scheduleDelayedUpdatePopup()
	}

	getUpdateSize(update.rid).then((size) => (updateSize.value = size))
	return 'available'
}

async function manualUpdateCheck() {
	if (!(await areUpdatesEnabled())) {
		updatesEnabled.value = false
		return 'disabled'
	}

	updatesEnabled.value = true
	if (offline.value) {
		return 'offline'
	}

	return await performUpdateCheck()
}

async function checkUpdates() {
	if (!(await areUpdatesEnabled())) {
		console.log('Skipping update check as updates are disabled in this build or environment')
		updatesEnabled.value = false

		return
	}

	updatesEnabled.value = true
	if (!offline.value) {
		await performUpdateCheck().catch((error) => {
			console.warn('Failed to check for launcher updates', error)
		})
	}
	setTimeout(
		() => {
			checkUpdates()
		},
		5 /* min */ * 60 /* sec */ * 1000 /* ms */,
	)
}

async function downloadAvailableUpdate() {
	return downloadUpdate(availableUpdate.value)
}

async function downloadUpdate(versionToDownload) {
	if (!versionToDownload) {
		handleError(`Failed to download update: no version available`)
		return
	}

	if (downloading.value || appUpdateDownload.progress.value !== 0) {
		console.error(`Update ${versionToDownload.version} already downloading`)
		return
	}

	console.log(`Downloading update ${versionToDownload.version}`)
	downloading.value = true

	try {
		enqueueUpdateForInstallation(versionToDownload.rid)
			.then(() => {
				downloading.value = false
				finishedDownloading.value = true
				unlistenUpdateDownload?.().then(() => {
					unlistenUpdateDownload = null
				})
				console.log('Finished downloading!')
				markAppUpdateActionable(versionToDownload.version, 'downloaded')
				scheduleDelayedUpdatePopup()
			})
			.catch((e) => {
				downloading.value = false
				appUpdateDownload.progress.value = 0
				handleError(e)
			})
		unlistenUpdateDownload = await subscribeToDownloadProgress(
			appUpdateDownload,
			versionToDownload.version,
		)
	} catch (e) {
		downloading.value = false
		appUpdateDownload.progress.value = 0
		handleError(e)
	}
}

async function installUpdate() {
	restarting.value = true

	try {
		await setRestartAfterPendingUpdate(true)
	} catch (e) {
		restarting.value = false
		handleError(e)
		return
	}
	setTimeout(async () => {
		await handleClose()
	}, 250)
}

setAppUpdateActions({
	check: manualUpdateCheck,
	download: downloadAvailableUpdate,
	install: installUpdate,
	changelog: (version) => {
		if (version && getAnnouncementByVersion(version)) {
			updateAnnouncementModal.value?.show(version)
		} else {
			openUrl(AxolotlBrandConfig.website)
		}
	},
})

async function openModrinthProjectLinkInApp(parsed) {
	const { slug, pathSuffix, url } = parsed
	const loadToken = loading.begin()
	try {
		const { id } = await tauriApiClient.labrinth.projects_v2.check(slug)
		const query = mergeUrlQuery(route.query, url)
		await router.push({
			path: `/project/${id}${pathSuffix}`,
			query,
			hash: url.hash || undefined,
		})
	} catch (err) {
		if (err instanceof ModrinthApiError && err.statusCode === 404) {
			openUrl(url.href)
		} else {
			handleError(err)
		}
	} finally {
		loading.end(loadToken)
	}
}

function handleClick(e) {
	let target = e.target
	while (target != null) {
		if (target.matches('a')) {
			if (
				target.href &&
				['http://', 'https://', 'mailto:', 'tel:'].some((v) => target.href.startsWith(v)) &&
				!target.classList.contains('router-link-active') &&
				!target.href.startsWith('http://localhost') &&
				!target.href.startsWith('https://tauri.localhost') &&
				!target.href.startsWith('http://tauri.localhost')
			) {
				const parsed = parseModrinthLink(target.href)
				if (target.target !== '_blank' && parsed) {
					void openModrinthProjectLinkInApp(parsed)
				} else {
					openUrl(target.href)
				}
			}
			e.preventDefault()
			break
		}
		target = target.parentElement
	}
}

function handleAuxClick(e) {
	// disables middle click -> new tab
	if (e.button === 1) {
		e.preventDefault()
		// instead do a left click
		const event = new MouseEvent('click', {
			view: window,
			bubbles: true,
			cancelable: true,
		})
		e.target.dispatchEvent(event)
	}
}

provideAppUpdateDownloadProgress(appUpdateDownload)
</script>

<template>
	<SplashScreen v-if="!stateFailed" ref="splashScreen" data-tauri-drag-region />
	<div id="teleports"></div>
	<div
		v-if="stateInitialized && themeStore.customBackgroundPath && !themeStore.transparentBackground"
		class="launcher-background"
		:style="customBackgroundStyle"
	/>
	<div
		v-if="stateInitialized"
		class="app-grid-layout relative"
		:class="{
			'disable-advanced-rendering': !themeStore.advancedRendering,
			'has-custom-background': themeStore.customBackgroundPath && !themeStore.transparentBackground,
			'has-transparent-background': themeStore.transparentBackground,
			'is-maximized': isMaximized,
		}"
	>
		<Transition name="fade">
			<div
				v-if="restarting"
				data-tauri-drag-region
				class="inset-0 fixed bg-black/80 backdrop-blur z-[200] flex items-center justify-center"
			>
				<span
					data-tauri-drag-region
					class="flex items-center gap-4 text-contrast font-semibold text-xl select-none cursor-default"
				>
					<RefreshCwIcon data-tauri-drag-region class="animate-spin w-6 h-6" />
					{{ formatMessage(messages.restarting) }}
				</span>
			</div>
		</Transition>
		<Suspense>
			<AppSettingsModal ref="settingsModal" />
		</Suspense>
		<Suspense>
			<AuthGrantFlowWaitModal ref="modrinthLoginFlowWaitModal" @flow-cancel="cancelLogin" />
		</Suspense>
		<InstanceIconPickerModal ref="instanceIconPickerModal" />
		<CreationFlowModal
			ref="installationModal"
			type="instance"
			show-snapshot-toggle
			:fetch-existing-instance-names="fetchExistingInstanceNames"
			:search-modpacks="searchModpacks"
			:get-project-versions="getProjectVersions"
			:get-loader-manifest="getLoaderManifest"
			:on-import-file-received="onImportFileReceived"
			@create="handleCreate"
			@browse-modpacks="handleBrowseModpacks"
		/>
		<UnknownPackWarningModal ref="unknownPackWarningModal" />
		<div
			class="app-grid-navbar bg-bg-raised flex flex-col p-[0.5rem] pt-0 gap-[0.5rem] w-[--left-bar-width] overflow-hidden"
		>
			<NavRail>
				<NavButton v-tooltip.right="formatMessage(messages.home)" to="/">
					<HomeIcon />
				</NavButton>
				<NavButton
					v-if="themeStore.featureFlags.worlds_tab"
					v-tooltip.right="formatMessage(messages.worlds)"
					to="/worlds"
				>
					<WorldIcon />
				</NavButton>
				<NavButton
					v-tooltip.right="formatMessage(messages.discoverContent)"
					data-onboarding-id="nav-discover"
					to="/browse/modpack"
					:disabled="offline"
					:is-primary="() => route.path.startsWith('/browse') && !route.query.i"
					:is-subpage="(route) => route.path.startsWith('/project') && !route.query.i"
				>
					<CompassIcon />
				</NavButton>
				<NavButton
					v-tooltip.right="formatMessage(messages.skinSelector)"
					data-onboarding-id="nav-skins"
					to="/skins"
				>
					<ChangeSkinIcon />
				</NavButton>
				<NavButton
					v-tooltip.right="formatMessage(messages.library)"
					data-onboarding-id="nav-library"
					to="/library"
					:is-primary="(r) => r.path === '/library' || r.path === '/library'"
					:is-subpage="
						() =>
							route.path.startsWith('/instance') ||
							((route.path.startsWith('/browse') || route.path.startsWith('/project')) &&
								route.query.i)
					"
				>
					<LibraryIcon />
				</NavButton>
				<NavButton
					v-tooltip.right="formatMessage(messages.downloads)"
					data-onboarding-id="nav-downloads"
					to="/downloads"
					class="relative"
				>
					<DownloadIcon />
					<span
						v-if="downloadManager.activeCount.value > 0"
						class="absolute right-0 top-0 min-w-4 rounded-full bg-brand px-1 text-center text-[10px] font-bold leading-4 text-white"
					>
						{{ Math.min(downloadManager.activeCount.value, 99) }}
					</span>
				</NavButton>
			</NavRail>
			<div class="h-px w-6 mx-auto my-2 bg-surface-5"></div>
			<div class="flex-1 min-h-0 overflow-y-auto">
				<suspense>
					<QuickInstanceSwitcher />
				</suspense>
			</div>
			<NavButton
				v-tooltip.right="formatMessage(messages.createInstance)"
				data-onboarding-id="create-instance"
				to="/create"
				:disabled="offline"
			>
				<PlusIcon />
			</NavButton>
			<NavButton
				v-tooltip.right="formatMessage(commonMessages.settingsLabel)"
				data-onboarding-id="nav-settings"
				:to="() => settingsModal?.show()"
			>
				<SettingsIcon />
			</NavButton>
			<OverflowMenu
				v-if="AxolotlBrandConfig.capabilities.privateModrinthServices && credentials?.user"
				v-tooltip.right="`Modrinth account`"
				data-onboarding-id="account-entry"
				class="w-12 h-12 text-primary rounded-full flex items-center justify-center text-2xl transition-all bg-transparent hover:bg-button-bg hover:text-contrast border-0 cursor-pointer"
				:options="[
					{
						id: 'view-profile',
						action: () => openUrl('https://modrinth.com/user/' + credentials.user.username),
					},
					{
						id: 'sign-out',
						action: () => logOut(),
						color: 'danger',
					},
				]"
				placement="right-end"
			>
				<Avatar :src="credentials?.user?.avatar_url" alt="" size="32px" circle />
				<template #view-profile>
					<UserIcon />
					<span class="inline-flex items-center gap-1">
						{{ formatMessage(messages.signedInAs) }}
						<span class="inline-flex items-center gap-1 text-contrast font-semibold">
							<Avatar :src="credentials?.user?.avatar_url" alt="" size="20px" circle />
							{{ credentials?.user?.username }}
						</span>
					</span>
					<ExternalIcon />
				</template>
				<template #sign-out> <LogOutIcon /> Sign out </template>
			</OverflowMenu>
			<NavButton
				v-else-if="AxolotlBrandConfig.capabilities.privateModrinthServices"
				v-tooltip.right="'Sign in to a Modrinth account'"
				data-onboarding-id="account-entry"
				:to="() => signIn()"
			>
				<LogInIcon class="text-brand" />
			</NavButton>
		</div>
		<div data-tauri-drag-region class="app-grid-statusbar bg-bg-raised h-[--top-bar-height] flex">
			<div data-tauri-drag-region class="flex min-w-0 flex-1 overflow-hidden p-3">
				<AxolotlLogo class="h-full w-auto shrink-0 pointer-events-none" />
				<div data-tauri-drag-region class="flex shrink-0 items-center gap-1 ml-3">
					<button
						class="cursor-pointer p-0 m-0 text-contrast border-none outline-none bg-button-bg rounded-full flex items-center justify-center w-6 h-6 hover:brightness-75 transition-all"
						@click="router.back()"
					>
						<LeftArrowIcon />
					</button>
					<button
						class="cursor-pointer p-0 m-0 text-contrast border-none outline-none bg-button-bg rounded-full flex items-center justify-center w-6 h-6 hover:brightness-75 transition-all"
						@click="router.forward()"
					>
						<RightArrowIcon />
					</button>
				</div>
				<Breadcrumbs class="pt-[2px]" />
			</div>
			<section data-tauri-drag-region class="flex shrink-0 ml-auto items-center">
				<ButtonStyled
					v-if="!forceSidebar && themeStore.toggleSidebar"
					:type="sidebarToggled ? 'standard' : 'transparent'"
					circular
				>
					<button
						class="mr-3 transition-transform"
						:class="{ 'rotate-180': !sidebarToggled }"
						@click="sidebarToggled = !sidebarToggled"
					>
						<RightArrowIcon />
					</button>
				</ButtonStyled>
				<div class="flex mr-3">
					<Suspense>
						<AppActionBar />
					</Suspense>
				</div>
				<WindowControls />
			</section>
		</div>
	</div>
	<div
		v-if="stateInitialized"
		class="app-contents"
		:class="{
			'sidebar-enabled': sidebarVisible,
			'disable-advanced-rendering': !themeStore.advancedRendering,
			'has-custom-background': themeStore.customBackgroundPath && !themeStore.transparentBackground,
			'has-transparent-background': themeStore.transparentBackground,
		}"
	>
		<div class="app-viewport flex-grow router-view">
			<div
				class="loading-indicator-container h-8 fixed z-50 pointer-events-none"
				:style="{
					top: 'calc(var(--top-bar-height))',
					left: 'calc(var(--left-bar-width))',
					width: 'calc(100% - var(--left-bar-width) - var(--right-bar-width))',
				}"
			>
				<LoadingBar position="absolute" />
			</div>
			<div
				v-if="themeStore.featureFlags.page_path"
				class="absolute bottom-0 left-0 m-2 bg-tooltip-bg text-tooltip-text font-semibold rounded-full px-2 py-1 text-xs z-50"
			>
				{{ route.fullPath }}
			</div>
			<div
				id="background-teleport-target"
				class="absolute h-full -z-10 rounded-tl-[--radius-xl] overflow-hidden"
				:style="{
					width: 'calc(100% - var(--right-bar-width))',
				}"
			></div>
			<Admonition
				v-if="authUnreachable"
				type="warning"
				:header="formatMessage(messages.authUnreachableHeader)"
				class="m-6 mb-0"
			>
				{{ formatMessage(messages.authUnreachableBody) }}
			</Admonition>
			<RouterView v-slot="{ Component }">
				<template v-if="Component">
					<Suspense @pending="onSuspensePending" @resolve="onSuspenseResolve">
						<component :is="Component"></component>
					</Suspense>
				</template>
			</RouterView>
		</div>
		<div
			class="app-sidebar mt-px shrink-0 flex flex-col border-0 border-l-[1px] border-[--brand-gradient-border] border-solid"
		>
			<div
				v-overlay-scrollbars="sidebarOverlayScrollbarsOptions"
				class="app-sidebar-scrollable flex-grow shrink relative"
				data-overlayscrollbars-initialize
			>
				<div id="sidebar-teleport-target" class="sidebar-teleport-content"></div>
				<div class="sidebar-default-content" :class="{ 'sidebar-enabled': sidebarVisible }">
					<div class="p-4 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid">
						<h3 class="text-base text-primary font-medium m-0">
							{{ formatMessage(messages.playingAs) }}
						</h3>
						<suspense>
							<AccountsCard ref="accounts" />
						</suspense>
					</div>
					<div id="sidebar-default-teleport-target"></div>
				</div>
			</div>
		</div>
	</div>
	<I18nDebugPanel />
	<NotificationPanel
		:has-sidebar="sidebarVisible"
		:on-error-action="exportNotificationErrorLogs"
		:error-action-label="formatMessage(messages.exportErrorLogs)"
	/>
	<PopupNotificationPanel
		:has-sidebar="sidebarVisible"
		:on-error-action="exportNotificationErrorLogs"
		:error-action-label="formatMessage(messages.exportErrorLogs)"
	/>
	<MinecraftCrashModal ref="minecraftCrashModal" @error="handleError" />
	<CommunityAnnouncementModal ref="communityAnnouncementModal" />
	<UpdateAnnouncementModal ref="updateAnnouncementModal" @closed="handleUpdateAnnouncementClosed" />
	<ErrorModal ref="errorModal" />
	<MinecraftAuthErrorModal ref="minecraftAuthErrorModal" />
	<ContentInstallModal
		ref="modInstallModal"
		:instances="contentInstallInstances"
		:compatible-loaders="contentInstallLoaders"
		:game-versions="contentInstallGameVersions"
		:loading="contentInstallLoading"
		:default-tab="contentInstallDefaultTab"
		:preferred-loader="contentInstallPreferredLoader"
		:preferred-game-version="contentInstallPreferredGameVersion"
		:release-game-versions="contentInstallReleaseGameVersions"
		:project-info="contentInstallProjectInfo"
		:symlink-target="contentInstallSymlinkTarget"
		@install="handleInstallToInstance"
		@create-and-install="handleCreateAndInstall"
		@navigate="handleContentInstallNavigate"
		@cancel="handleContentInstallCancel"
	/>
	<ModpackAlreadyInstalledModal
		ref="modpackAlreadyInstalledModal"
		@create-anyway="handleModpackDuplicateCreateAnyway"
		@go-to-instance="handleModpackDuplicateGoToInstance"
	/>
	<AddServerToInstanceModal
		ref="addServerToInstanceModal"
		:symlink-target="addServerSymlinkTarget"
	/>
	<ContentUpdaterModal
		ref="incompatibilityWarningModal"
		mode="incompatibility-warning"
		:versions="contentInstallIncompatibilityWarningVersions"
		:current-game-version="contentInstallIncompatibilityWarningCurrentGameVersion"
		:current-loader="contentInstallIncompatibilityWarningCurrentLoader"
		current-version-id=""
		:is-app="true"
		:project-type="contentInstallIncompatibilityWarningProjectType"
		:project-icon-url="contentInstallIncompatibilityWarningProjectIconUrl"
		:project-name="contentInstallIncompatibilityWarningProjectName"
		:warning="contentInstallIncompatibilityWarningMessage"
		:action-loading="contentInstallIncompatibilityWarningInstalling"
		@update="handleIncompatibilityWarningUpdate"
		@cancel="handleIncompatibilityWarningCancel"
		@searchCompat="handleDropInstallSearchCompat"
	/>
	<ModpackAlreadyInstalledModal
		ref="contentInstallModpackAlreadyInstalledModal"
		@create-anyway="handleContentInstallModpackDuplicateCreateAnyway"
		@go-to-instance="handleContentInstallModpackDuplicateGoToInstance"
	/>
	<CurseForgeManualDownloadsModal
		ref="contentInstallCurseForgeManualDownloadsModal"
		@view-instance="handleContentInstallModpackDuplicateGoToInstance"
	/>
	<InstallToPlayModal ref="installToPlayModal" />
	<UpdateToPlayModal ref="updateToPlayModal" />

	<!-- Global drop overlay -->
	<div
		v-if="isDragging && !onSkinsPage"
		class="fixed inset-0 z-[9999] bg-black/40 flex items-center justify-center pointer-events-none"
	>
		<div class="rounded-2xl border-2 border-dashed border-brand bg-surface-2/90 p-8 text-center">
			<p class="text-lg text-contrast">{{ formatMessage(messages.dropOverlayTitle) }}</p>
			<p class="text-sm text-secondary mt-2">{{ formatMessage(messages.dropOverlaySubtitle) }}</p>
		</div>
	</div>

	<!-- Processing overlay -->
	<div
		v-if="(isProcessing || scanningInstances) && !isDragging && !onSkinsPage"
		class="fixed inset-0 z-[9999] bg-black/20 flex items-center justify-center"
	>
		<div class="flex flex-col items-center gap-3">
			<SpinnerIcon class="h-10 w-10 animate-spin text-contrast" />
			<span v-if="scanningInstances" class="text-sm text-secondary"
				>{{ formatMessage(messages.dropNoInstances) }}…</span
			>
		</div>
	</div>

	<!-- Drop type confirmation modal -->
	<ConfirmDropTypeModal
		ref="confirmDropModal"
		:classification="dropClassification"
		:file-name="dropFileName"
		@confirm="handleDropConfirm"
		@cancel="handleDropCancel"
		@help="handleDropHelp"
	/>

	<!-- Generic content install modal (instance selection when not in an instance page) -->
	<GenericContentInstallModal
		ref="genericInstallModal"
		@install="handleGenericInstall"
		@cancel="dropClassification = null"
		@navigate-create="handleGenericInstallNavigateCreate"
	/>

	<!-- Launcher import instance selection modal -->
	<LauncherImportModal
		ref="launcherImportModal"
		@confirm="onImportSelected"
		@cancel="launcherImportModal?.hide()"
	/>

	<!-- Symlink method selection modal -->
	<SymlinkMethodCards
		ref="symlinkCardsModal"
		@confirm="onSymlinkMethodConfirmed"
		@cancel="onSymlinkMethodCancelled"
	/>

	<OnboardingOverlay
		:visible="showOnboarding"
		:mode="onboardingMode"
		@complete="finishOnboarding"
		@skip="skipOnboarding"
		@request-close-settings="closeOnboardingSettings"
	/>
</template>

<style lang="scss" scoped>
.app-grid-layout,
.app-contents {
	--top-bar-height: 3rem;
	--left-bar-width: 4rem;
	--right-bar-width: 300px;
}

.app-grid-layout {
	display: grid;
	grid-template: 'status status' 'nav dummy';
	grid-template-columns: auto 1fr;
	grid-template-rows: auto 1fr;
	position: relative;
	//z-index: 0;
	background-color: var(--color-raised-bg);
	height: 100vh;
}

.launcher-background {
	position: fixed;
	inset: -3rem;
	z-index: 0;
	pointer-events: none;
	background-position: center;
	background-size: cover;
	background-repeat: no-repeat;
	transition:
		filter 180ms ease,
		opacity 180ms ease;
}

.app-grid-layout.has-custom-background,
.app-grid-layout.has-transparent-background {
	&:not(.is-maximized) {
		border-radius: 8px;
		clip-path: inset(0 round 8px);
		overflow: hidden;
	}
	background-color: transparent;

	.app-grid-navbar,
	.app-grid-statusbar {
		background-color: color-mix(in srgb, var(--color-raised-bg) 82%, transparent) !important;
		backdrop-filter: blur(18px) saturate(120%);
	}
}

.app-grid-navbar {
	grid-area: nav;
	position: relative;
	z-index: 2;
}

.app-grid-statusbar {
	grid-area: status;
	padding-right: var(--window-controls-width, 0px);
	position: relative;
	z-index: 2;
}

[data-tauri-drag-region-exclude] {
	-webkit-app-region: no-drag;
}

.app-contents {
	position: absolute;
	z-index: 1;
	left: var(--left-bar-width);
	top: var(--top-bar-height);
	right: 0;
	bottom: 0;
	height: calc(100vh - var(--top-bar-height));
	background-color: var(--color-bg);
	border-top-left-radius: var(--radius-xl);
	overflow: hidden;

	display: grid;
	grid-template-columns: 1fr 0px;
	// transition: grid-template-columns 0.4s ease-in-out;

	&.sidebar-enabled {
		grid-template-columns: 1fr 300px;
	}

	&.has-custom-background,
	&.has-transparent-background {
		background-color: color-mix(in srgb, var(--color-bg) 76%, transparent);
		border-top-left-radius: 0;

		&::before {
			border: none;
			box-shadow: none;
		}

		.loading-indicator-container {
			border-top-left-radius: 0;
		}
	}
}

.app-grid-layout.has-transparent-background {
	.app-grid-navbar,
	.app-grid-statusbar {
		background-color: color-mix(
			in srgb,
			var(--surface-3-opaque) var(--window-alpha-chrome),
			transparent
		) !important;
	}

	// Without native decorations or rounded corners the window edge dissolves
	// into the desktop, so it needs drawing. Dark outside, light inside, to stay
	// legible over any wallpaper.
	&::after {
		content: '';
		position: fixed;
		inset: 0;
		border-radius: inherit;
		z-index: 100;
		pointer-events: none;
		box-shadow:
			inset 0 0 0 1px rgba(0, 0, 0, 0.5),
			inset 0 0 0 2px rgba(255, 255, 255, 0.14);
	}
}

.app-contents.has-transparent-background {
	// Sourced from the opaque snapshot: `--color-bg` is itself translucent in
	// this mode, so mixing it again would compound. Sits slightly below the
	// chosen alpha because pages paint their own surface on top of it.
	background-color: color-mix(
		in srgb,
		var(--surface-1-opaque) calc(var(--window-alpha) * 0.82),
		transparent
	);
}

.loading-indicator-container {
	border-top-left-radius: var(--radius-xl);
	overflow: hidden;
}

.app-sidebar {
	overflow: visible;
	width: 300px;
	position: relative;
	height: calc(100vh - var(--top-bar-height));
	background: var(--brand-gradient-bg);

	--color-button-bg: var(--brand-gradient-button);
	--color-button-bg-hover: var(--brand-gradient-border);
	--color-divider: var(--brand-gradient-border);
	--color-divider-dark: var(--brand-gradient-border);
}

.disable-advanced-rendering {
	.app-sidebar::before {
		box-shadow: none;
	}

	&.app-contents::before {
		box-shadow: none;
	}

	*,
	:deep(*) {
		box-shadow: none !important;
		--tw-drop-shadow:;
	}
}

.app-sidebar::before {
	content: '';
	box-shadow: -15px 0 15px -15px rgba(0, 0, 0, 0.1) inset;
	top: 0;
	bottom: 0;
	left: -2rem;
	width: 2rem;
	position: absolute;
	pointer-events: none;
}

.app-viewport {
	flex-grow: 1;
	height: 100%;
	overflow: auto;
	overflow-x: hidden;
	scrollbar-gutter: stable;
}

.app-contents::before {
	z-index: 30;
	content: '';
	position: fixed;
	left: var(--left-bar-width);
	top: var(--top-bar-height);
	right: calc(-1 * var(--left-bar-width));
	bottom: calc(-1 * var(--left-bar-width));
	border-radius: var(--radius-xl);
	box-shadow: 1px 1px 15px rgba(0, 0, 0, 0.1) inset;
	border-color: var(--surface-5);
	border-width: 1px;
	border-style: solid;
	pointer-events: none;
}

.sidebar-teleport-content {
	display: contents;
}

.sidebar-default-content {
	display: none;
}

.sidebar-teleport-content:empty + .sidebar-default-content.sidebar-enabled {
	display: contents;
}

.popup-survey-enter-active {
	transition:
		opacity 0.25s ease,
		transform 0.25s cubic-bezier(0.51, 1.08, 0.35, 1.15);
	transform-origin: top center;
}

.popup-survey-leave-active {
	transition:
		opacity 0.25s ease,
		transform 0.25s cubic-bezier(0.68, -0.17, 0.23, 0.11);
	transform-origin: top center;
}

.popup-survey-enter-from,
.popup-survey-leave-to {
	opacity: 0;
	transform: translateY(10rem) scale(0.8) scaleY(1.6);
}

@media (prefers-reduced-motion: no-preference) {
	.nav-button-animated-enter-active {
		transition: all 0.5s cubic-bezier(0.15, 1.4, 0.64, 0.96);
	}

	.nav-button-animated-leave-active {
		transition: all 0.25s ease;
	}

	.nav-button-animated-enter-active {
		position: relative;
	}

	.nav-button-animated-enter-active::before {
		content: '';
		inset: 0;
		border-radius: 100vw;
		background-color: var(--color-brand-highlight);
		position: absolute;
		animation: pop 0.5s ease-in forwards;
		opacity: 0;
	}

	@keyframes pop {
		0% {
			scale: 0.5;
		}
		50% {
			opacity: 0.5;
		}
		100% {
			scale: 1.5;
		}
	}

	.nav-button-animated-enter-from {
		scale: 0.5;
		translate: -2rem 0;
		opacity: 0;
	}

	.nav-button-animated-leave-to {
		scale: 0.75;
		opacity: 0;
	}

	.fade-enter-active {
		transition: 0.25s ease-in-out;
	}

	.fade-enter-from {
		opacity: 0;
	}
}
</style>
<style>
.os-theme-dark,
.os-theme-light {
	--os-handle-bg: var(--color-scrollbar) !important;
	--os-handle-bg-hover: var(--color-scrollbar) !important;
	--os-handle-bg-active: var(--color-scrollbar) !important;
}

.mac {
	.app-grid-statusbar {
		padding-left: 5rem;
	}
}

.windows {
	.fake-appbar {
		height: 2.5rem !important;
	}

	.info-card {
		right: 22rem;
	}

	.profile-card {
		right: 8rem;
	}
}
</style>
