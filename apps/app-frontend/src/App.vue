<script setup lang="ts">
import { AuthFeature, TauriModrinthClient, VerboseLoggingFeature } from '@modrinth/api-client'
import {
	ChangeSkinIcon,
	CompassIcon,
	DownloadIcon,
	ExternalIcon,
	FlaskConicalIcon,
	FolderOpenIcon,
	HomeIcon,
	LeftArrowIcon,
	LibraryIcon,
	LogInIcon,
	LogOutIcon,
	PlusIcon,
	RefreshCwIcon,
	RightArrowIcon,
	RotateCounterClockwiseIcon,
	SettingsIcon,
	SpinnerIcon,
	UserIcon,
	UsersIcon,
	WorldIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Avatar,
	BigOptionButton,
	ButtonStyled,
	clientInstallableLoaders,
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
	useVIntl,
} from '@modrinth/ui'
import BatchScanOverlay from '@modrinth/ui/src/components/flows/drop/BatchScanOverlay.vue'
import ConfirmDropTypeModal from '@modrinth/ui/src/components/flows/drop/ConfirmDropTypeModal.vue'
import GenericContentInstallModal from '@modrinth/ui/src/components/flows/drop/GenericContentInstallModal.vue'
import LauncherImportModal from '@modrinth/ui/src/components/flows/drop/LauncherImportModal.vue'
import SymlinkMethodCards from '@modrinth/ui/src/components/flows/drop/SymlinkMethodCards.vue'
import { useQuery } from '@tanstack/vue-query'
import { getVersion } from '@tauri-apps/api/app'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { Effect, getCurrentWindow } from '@tauri-apps/api/window'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import { openUrl } from '@tauri-apps/plugin-opener'
import { type as getOsType } from '@tauri-apps/plugin-os'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
import { computed, nextTick, onMounted, onUnmounted, provide, ref, watch } from 'vue'
import { type RouteLocationNormalizedLoaded, RouterView, useRoute, useRouter } from 'vue-router'

import { getAnnouncementByVersion } from '@/announcements/catalog'
import InstanceExportModal from '@/components/lab/recipe-generator/InstanceExportModal.vue'
import AccountsCard from '@/components/ui/AccountsCard.vue'
import UpdateAnnouncementModal from '@/components/ui/announcement/UpdateAnnouncementModal.vue'
import AppActionBar from '@/components/ui/AppActionBar.vue'
import AxolotlLogo from '@/components/ui/AxolotlLogo.vue'
import Breadcrumbs from '@/components/ui/Breadcrumbs.vue'
import ContentInstallPreviewModal from '@/components/ui/ContentInstallPreviewModal.vue'
import ErrorModal from '@/components/ui/ErrorModal.vue'
import AddServerToInstanceModal from '@/components/ui/install_flow/AddServerToInstanceModal.vue'
import UnknownPackWarningModal from '@/components/ui/install_flow/UnknownPackWarningModal.vue'
import MinecraftAuthErrorModal from '@/components/ui/minecraft-auth-error-modal/MinecraftAuthErrorModal.vue'
import MinecraftCrashModal from '@/components/ui/MinecraftCrashModal.vue'
import AuthGrantFlowWaitModal from '@/components/ui/modal/AuthGrantFlowWaitModal.vue'
import CommunityAnnouncementModal from '@/components/ui/modal/CommunityAnnouncementModal.vue'
import CurseForgeManualDownloadsModal from '@/components/ui/modal/CurseForgeManualDownloadsModal.vue'
import InstallToPlayModal from '@/components/ui/modal/InstallToPlayModal.vue'
import InstanceIconPickerModal from '@/components/ui/modal/InstanceIconPickerModal.vue'
import JavaDownloadConfirmationModal from '@/components/ui/modal/JavaDownloadConfirmationModal.vue'
import ModpackAlreadyInstalledModal from '@/components/ui/modal/ModpackAlreadyInstalledModal.vue'
import ModpackInstallModal from '@/components/ui/modal/ModpackInstallModal.vue'
import PrivacyConsentModal from '@/components/ui/modal/PrivacyConsentModal.vue'
import SurveyAnnouncementModal from '@/components/ui/modal/SurveyAnnouncementModal.vue'
import UpdateToPlayModal from '@/components/ui/modal/UpdateToPlayModal.vue'
import NavButton from '@/components/ui/NavButton.vue'
import NavRail from '@/components/ui/NavRail.vue'
import OnboardingOverlay from '@/components/ui/onboarding/OnboardingOverlay.vue'
import QuickInstanceSwitcher from '@/components/ui/QuickInstanceSwitcher.vue'
import SplashScreen from '@/components/ui/SplashScreen.vue'
import WindowControls from '@/components/ui/WindowControls.vue'
import { useCheckDisableMouseover } from '@/composables/macCssFix.js'
import { useDropImport } from '@/composables/useDropImport'
import { minecraftLaunchErrorKey } from '@/composables/useMinecraftLaunchError'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { AxolotlBrandConfig, config, getOfficialLabrinthBaseUrl } from '@/config'
import { trackEvent } from '@/helpers/analytics'
import { check_reachable } from '@/helpers/auth.js'
import { get_user, get_version } from '@/helpers/cache.js'
import { configureCurseForgeManualDownloadWatcher } from '@/helpers/curseforge'
import { getMissingContentScannerSettings } from '@/helpers/downloads-scanner'
import { classifyDroppedItem } from '@/helpers/drop'
import {
	command_listener,
	drop_classify_progress_listener,
	java_download_confirmation_listener,
	warning_listener,
} from '@/helpers/events.js'
import { install_create_modpack_instance, install_get_modpack_preview } from '@/helpers/install'
import { get as getInstance, run } from '@/helpers/instance'
import { reconcileMojangAuthSourceAtStartup } from '@/helpers/mojang-auth'
import { cancelLogin, get as getCreds, login, logout } from '@/helpers/mr_auth.ts'
import { mergeUrlQuery, parseModrinthLink } from '@/helpers/project-links.ts'
import {
	get as getSettings,
	getLastBrowseContentProjectType,
	getPrivacySettings,
	getUpdateSource,
	isBrowseContentProjectType,
	type PrivacySettings,
	savePrivacySettings,
	set as setSettings,
} from '@/helpers/settings.ts'
import { getSidebarExpanded, setSidebarExpanded } from '@/helpers/sidebar-state.ts'
import { get_opening_command, initialize_state, set_discord_activity } from '@/helpers/state'
import {
	areUpdatesEnabled,
	checkAppUpdate,
	enqueueUpdateForInstallation,
	exportErrorLogs,
	getOS,
	getUpdateSize,
	isDev,
	isElevated,
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
import { createContentSelection, provideContentSelection } from '@/providers/content-selection'
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

import { get_available_capes, get_available_skins } from './helpers/skins'
import { AppNotificationManager } from './providers/app-notifications'
import { AppPopupNotificationManager } from './providers/app-popup-notifications'

const themeStore = useTheming()
const router = useRouter()
const route = useRoute()
const onSkinsPage = computed(() => route.path === '/skins')
const onSchematicWorkshopPage = computed(() => route.path === '/lab/schematic-preview')
const isSchematicFile = (path: string) => /\.(litematic|schematic|schem)$/i.test(path)
const APP_LEFT_NAV_WIDTH = '4rem'

const discoverContentPath = computed(() => {
	const projectType = route.params.projectType
	if (
		!route.query.i &&
		!route.query.sid &&
		!route.query.wid &&
		typeof projectType === 'string' &&
		isBrowseContentProjectType(projectType)
	) {
		return `/browse/${projectType}`
	}

	return `/browse/${getLastBrowseContentProjectType()}`
})

function getPageTransitionKey(route: RouteLocationNormalizedLoaded) {
	const transitionGroup = route.meta.pageTransitionGroup
	if (typeof transitionGroup !== 'string') return route.fullPath

	const routeId = route.params.id
	return `${transitionGroup}:${Array.isArray(routeId) ? routeId.join('/') : (routeId ?? '')}`
}
const APP_SIDEBAR_WIDTH = 300
const credentials = ref()
const sidebarToggled = ref(getSidebarExpanded())

function toggleSidebar() {
	sidebarToggled.value = !sidebarToggled.value
	setSidebarExpanded(sidebarToggled.value)
}

const forceSidebar = computed(
	() => route.path.startsWith('/browse') || route.path.startsWith('/project'),
)
const forceSidebarHidden = computed(() => route.path === '/settings')
const sidebarVisible = computed(
	() => !forceSidebarHidden.value && (sidebarToggled.value || forceSidebar.value),
)
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
const contentSelection = createContentSelection({
	addNotification,
	handleError,
	downloadManager,
})
provideContentSelection(contentSelection)

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

const stateInitialization = initialize_state()
const {
	instanceIconPickerModal,
	installationModal,
	unknownPackWarningModal,
	fetchExistingInstanceNames,
	handleCreate,
	handleBrowseModpacks,
	searchModpacks,
	getProjectVersions,
	hasCompatibleOptiFabric,
	getLoaderManifest,
	installModpackFromPath,
	setModpackAlreadyInstalledModal,
	handleModpackDuplicateCreateAnyway,
	handleModpackDuplicateGoToInstance,
	fileDrop,
} = setupProviders(notificationManager, popupNotificationManager, stateInitialization)

const { browserOffline, offline, setNetworkReachable } = useNetworkStatus()

const showOnboarding = ref(false)
const onboardingMode = ref('main')
const onboardingSettings = ref(null)
const onboardingReplay = ref(false)
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
const privacyConsentModal = ref<InstanceType<typeof PrivacyConsentModal>>()
const privacyConsentPending = ref(false)
const communityAnnouncementModal = ref()
const surveyModal = ref()
const updateAnnouncementModal = ref()
const minecraftCrashModal = ref()
const javaDownloadConfirmationModal = ref()
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

onMounted(async () => {
	await useCheckDisableMouseover()

	document.querySelector('body').addEventListener('click', handleClick)
	document.querySelector('body').addEventListener('auxclick', handleAuxClick)

	checkUpdates()
	void warnIfRunningElevated()
})

onUnmounted(async () => {
	document.querySelector('body').removeEventListener('click', handleClick)
	document.querySelector('body').removeEventListener('auxclick', handleAuxClick)
	clearDelayedUpdatePopup()
	await unlistenUpdateDownload?.()
	downloadManager.dispose()
})

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()

async function warnIfRunningElevated() {
	if (await isElevated().catch(() => false)) {
		addNotification({
			title: formatMessage(messages.runningAsAdmin),
			type: 'warning',
			autoCloseMs: null,
		})
	}
}

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
		dropFilePath.value = classification.file_path ?? classification.base_path ?? filePath
		dropFileName.value = fileName

		// ── Unknown + nested archives → confirm unpacking first ──
		if (
			classification.item_type === 'unknown' &&
			classification.reason?.toLowerCase().includes('nested')
		) {
			showNestedUnpackPrompt(classification)
			return
		}

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
				text: unknownReasonMessage(classification.reason),
				type: 'error',
			})
			return
		}

		// ── Known types → show the same confirm modal as drag-drop ──
		confirmDropModal.value?.show()
	} catch (e) {
		notificationManager.removeNotification(processingNotify?.id)
		addNotification({
			title: formatMessage(messages.dropProcessFailedTitle),
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
	runningAsAdmin: {
		id: 'app.warning.running-as-admin',
		defaultMessage:
			'Axolotl is running as administrator. Drag-and-drop file import is disabled in this mode; please restart the launcher without administrator privileges.',
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
	multiplayer: {
		id: 'app.navigation.multiplayer',
		defaultMessage: 'Multiplayer',
	},
	downloads: {
		id: 'app.navigation.downloads',
		defaultMessage: 'Downloads',
	},
	lab: {
		id: 'app.navigation.lab',
		defaultMessage: 'Lab',
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
	collapseSidebar: {
		id: 'app.sidebar.collapse',
		defaultMessage: 'Collapse sidebar',
	},
	expandSidebar: {
		id: 'app.sidebar.expand',
		defaultMessage: 'Expand sidebar',
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
	dropUnknownDepthText: {
		id: 'app.drop.error.unknown-depth-text',
		defaultMessage:
			'The archive is nested too deeply to analyze. Unpack it to a folder and try again.',
	},
	dropUnknownEncryptedText: {
		id: 'app.drop.error.unknown-encrypted-text',
		defaultMessage: 'The archive contains encrypted files and cannot be analyzed.',
	},
	dropNestedUnpackTitle: {
		id: 'app.drop.nested-unpack-title',
		defaultMessage: 'Nested archives detected',
	},
	dropNestedUnpackText: {
		id: 'app.drop.nested-unpack-text',
		defaultMessage:
			'This archive contains nested archives ({size}) that must be unpacked to analyze. This may take some time. Continue?',
	},
	dropNestedUnpackButton: {
		id: 'app.drop.nested-unpack-button',
		defaultMessage: 'Continue analysis',
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
	dropScanning: {
		id: 'app.drop.scanning',
		defaultMessage: 'Scanning for instances',
	},
	dropScanFailed: {
		id: 'app.drop.scan-failed',
		defaultMessage: 'Failed to scan for instances',
	},
	dropExtractFailed: {
		id: 'app.drop.extract-failed',
		defaultMessage: 'Failed to extract archive',
	},
	dropProcessFailedTitle: {
		id: 'app.drop.process-failed-title',
		defaultMessage: 'Failed to process file',
	},
	dropTemporaryFileTitle: {
		id: 'app.drop.temporary-file-title',
		defaultMessage: 'Temporary file detected',
	},
	dropTemporaryFileText: {
		id: 'app.drop.temporary-file-text',
		defaultMessage:
			'The file "{file}" appears to be a temporary copy. Try dragging the file from its original folder instead of from a browser, archive, or cloud storage.',
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
	dropImportCancelledTitle: {
		id: 'app.drop.batch.import-cancelled-title',
		defaultMessage: 'Import cancelled',
	},
	dropImportCancelledText: {
		id: 'app.drop.batch.import-cancelled-text',
		defaultMessage: 'Nothing was imported.',
	},
	dropBatchNothingImportableTitle: {
		id: 'app.drop.batch.nothing-importable-title',
		defaultMessage: 'Nothing to import',
	},
	dropBatchNothingImportableText: {
		id: 'app.drop.batch.nothing-importable-text',
		defaultMessage: '{count, plural, one {# file} other {# files}} could not be recognized.',
	},
	dropBatchCompletedTitle: {
		id: 'app.drop.batch.completed-title',
		defaultMessage: 'Import finished',
	},
	dropBatchCompletedText: {
		id: 'app.drop.batch.completed-text',
		defaultMessage: 'Imported {succeeded} of {total} ({failed} failed, {skipped} skipped).',
	},
	dropBatchTargetLabel: {
		id: 'app.drop.batch.target-label',
		defaultMessage: 'Select target instance for this batch',
	},
	dropBatchGroupFileLabel: {
		id: 'app.drop.batch.group-file-label',
		defaultMessage: '{count, plural, one {# file} other {# files}}: {names}',
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
			'{file} targets {modVersion} ({modLoader}), but the instance is {instVersion} ({instLoader}).',
	},

	// Compatible mode (pre-version-isolation) import
	dropCompatibleModeTitle: {
		id: 'app.drop.compatible-mode-title',
		defaultMessage: 'This appears to be a pre-version-isolation instance',
	},
	dropCompatibleModeDesc: {
		id: 'app.drop.compatible-mode-desc',
		defaultMessage: 'Would you like to import this as a compatible mode instance?',
	},
	dropCompatibleModeImport: {
		id: 'app.drop.compatible-mode-import',
		defaultMessage: 'Compatible Import',
	},
	dropCompatibleModeOldWay: {
		id: 'app.drop.compatible-mode-old-way',
		defaultMessage: 'Import as Old Version',
	},
	dropCompatibleModeCancel: {
		id: 'app.drop.compatible-mode-cancel',
		defaultMessage: 'Cancel',
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
		telemetry_consent_version,
		discord_rpc,
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
		auto_hide_downloads_button,
		home_layout,
		minimal_home_instance_id,
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
		DiscoverContent: `/browse/${getLastBrowseContentProjectType()}`,
		Library: '/library',
	}
	const defaultPageRoute = offline.value ? '/library' : defaultPageRoutes[default_page]
	if (defaultPageRoute && defaultPageRoute !== '/') await router.push(defaultPageRoute)

	os.value = await getOS()
	const dev = await isDev()
	isDevEnvironment.value = dev
	pendingUpdateAnnouncementVersion.value = pending_update_toast_for_version
	if (!onboarded && route.path !== '/') await router.replace('/')
	privacyConsentPending.value = telemetry_consent_version < 1
	showOnboarding.value = false
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
	themeStore.autoHideDownloadsButton = auto_hide_downloads_button
	themeStore.homeLayout = home_layout
	themeStore.minimalHomeInstanceId = minimal_home_instance_id
	themeStore.devMode = developer_mode
	themeStore.featureFlags = feature_flags
	stateInitialized.value = true
	if (privacyConsentPending.value) {
		await nextTick()
		privacyConsentModal.value?.show({
			telemetry,
			discord_rpc,
			consent_version: telemetry_consent_version,
		})
	} else {
		showOnboarding.value = !onboarded
	}
	void reconcileMojangAuthSourceAtStartup().catch(handleError)

	isMaximized.value = await getCurrentWindow().isMaximized()

	await getCurrentWindow().onResized(async () => {
		isMaximized.value = await getCurrentWindow().isMaximized()
	})

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
	await java_download_confirmation_listener((request) => {
		javaDownloadConfirmationModal.value?.show(request)
	})
	await drop_classify_progress_listener((payload) => {
		if (dropProcessingNotificationId.value === null) return
		const name = payload.currentItem?.split(/[/\\]/).pop() || 'file'
		notificationManager.removeNotification(dropProcessingNotificationId.value)
		dropProcessingNotificationId.value = addNotification({
			title: formatMessage(messages.dropProcessing, { name }),
			type: 'info',
			autoCloseMs: null,
		}).id
	})

	get_opening_command().then(handleCommand)
	fetchCredentials()

	try {
		const skins = (await get_available_skins()) ?? []
		const capes = (await get_available_capes()) ?? []
		const { generateSkinPreviews } = await import('./helpers/rendering/skin-preview-renderer')
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
	if (mode === 'main') await router.replace('/')
	showOnboarding.value = true
}

async function finishOnboarding() {
	const wasReplay = onboardingReplay.value
	const settings = onboardingSettings.value ?? (await getSettings())
	if (!onboardingReplay.value) {
		if (onboardingMode.value === 'instance') {
			settings.onboarding_instance_tour_completed = true
		} else if (onboardingMode.value === 'main') {
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

async function closeOnboardingSettings() {
	if (route.path === '/settings') await router.replace('/')
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
	if (
		!stateInitialized.value ||
		privacyConsentPending.value ||
		showOnboarding.value ||
		updateAnnouncementShowing.value
	)
		return

	if (pendingUpdateAnnouncementVersion.value && updateAnnouncementModal.value) {
		updateAnnouncementShowing.value = true
		await nextTick()
		updateAnnouncementModal.value.show(pendingUpdateAnnouncementVersion.value)
		return
	}

	communityAnnouncementModal.value?.showIfNeeded()
	surveyModal.value?.showIfNeeded()
}

async function handlePrivacyConsentSaved(privacy: PrivacySettings) {
	privacyConsentPending.value = false
	if (onboardingSettings.value) {
		onboardingSettings.value.telemetry = privacy.telemetry
		onboardingSettings.value.discord_rpc = privacy.discord_rpc
		onboardingSettings.value.telemetry_consent_version = privacy.consent_version
	}
	if (!onboardingSettings.value?.onboarded) {
		startOnboarding('main')
	} else {
		await scheduleStartupDialogs()
	}
}

async function previewPrivacyConsentModal() {
	try {
		const current = await getPrivacySettings()
		const privacy = await savePrivacySettings({
			telemetry: false,
			discord_rpc: current.discord_rpc,
			consent_version: 0,
		})
		privacyConsentPending.value = true
		if (onboardingSettings.value) {
			onboardingSettings.value.telemetry = privacy.telemetry
			onboardingSettings.value.discord_rpc = privacy.discord_rpc
			onboardingSettings.value.telemetry_consent_version = privacy.consent_version
		}
		await nextTick()
		privacyConsentModal.value?.show(privacy)
	} catch (error) {
		handleError(error)
	}
}

provide('replayOnboarding', replayOnboarding)
provide(
	minecraftLaunchErrorKey,
	async (launchError, payload) =>
		(await minecraftCrashModal.value?.handleLaunchError(launchError, payload)) ?? false,
)
provide('previewMinecraftCrashModal', () => minecraftCrashModal.value?.showPreview())
provide('previewPrivacyConsentModal', previewPrivacyConsentModal)
provide('previewUpdateAnnouncement', (version = null) => {
	const previewVersion = version ?? pendingUpdateAnnouncementVersion.value
	if (previewVersion) updateAnnouncementModal.value?.show(previewVersion)
})

const stateFailed = ref(false)
stateInitialization
	.then(() => {
		const scannerSettings = getMissingContentScannerSettings()
		void configureCurseForgeManualDownloadWatcher(
			scannerSettings.enabled,
			scannerSettings.directory,
		).catch((error) => {
			console.warn('Failed to configure manual-download watcher', error)
		})
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
let lastDiscordActivity = null
let discordActivityUpdate = Promise.resolve()

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

function syncDiscordActivity(to: RouteLocationNormalizedLoaded) {
	const activity =
		typeof to.meta.discordActivity === 'string' ? to.meta.discordActivity : 'Idling...'
	if (activity === lastDiscordActivity) return

	lastDiscordActivity = activity
	discordActivityUpdate = discordActivityUpdate
		.then(() => set_discord_activity(activity))
		.catch((error) => {
			if (lastDiscordActivity === activity) lastDiscordActivity = null
			console.error('Failed to update Discord activity', error)
		})
}

router.afterEach((to, from, failure) => {
	if (!failure) void invoke('lightweight_mode_set_route', { route: to.fullPath })
	trackEvent('PageView', {
		path: to.path,
		fromPath: from.path,
		failed: failure,
	})
	if (!failure && stateInitialized.value) syncDiscordActivity(to)
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
			syncDiscordActivity(router.currentRoute.value)
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
	setContentInstallPreviewModal,
	setModpackInstallModal: setContentInstallModpackInstallModal,
	handleModpackInstall: handleContentInstallModpackInstall,
	handleModpackInstallCancel: handleContentInstallModpackInstallCancel,
	setCurseForgeManualDownloadsModal: setContentInstallCurseForgeManualDownloadsModal,
	handleCurseForgeManualDownloadsImported: handleContentInstallCurseForgeManualDownloadsImported,
	setIncompatibilityWarningModal: setContentIncompatibilityWarningModal,
	incompatibilityWarningVersions: contentInstallIncompatibilityWarningVersions,
	incompatibilityWarningCurrentGameVersion: contentInstallIncompatibilityWarningCurrentGameVersion,
	incompatibilityWarningCurrentLoader: contentInstallIncompatibilityWarningCurrentLoader,
	incompatibilityWarningProjectType: contentInstallIncompatibilityWarningProjectType,
	incompatibilityWarningProjectIconUrl: contentInstallIncompatibilityWarningProjectIconUrl,
	incompatibilityWarningProjectName: contentInstallIncompatibilityWarningProjectName,
	incompatibilityWarningMessage: contentInstallIncompatibilityWarningMessage,
	incompatibilityWarningInstalling: contentInstallIncompatibilityWarningInstalling,
} = contentInstall

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
const contentInstallPreviewModal = ref<InstanceType<typeof ContentInstallPreviewModal> | null>(null)
const modpackAlreadyInstalledModal = ref()
const contentInstallModpackInstallModal = ref<InstanceType<typeof ModpackInstallModal> | null>(null)
const handleContentInstallModpackDuplicateGoToInstance = (instanceId: string) =>
	router.push(`/instance/${encodeURIComponent(instanceId)}`)
const contentInstallCurseForgeManualDownloadsModal = ref()
const addServerToInstanceModal = ref()
const incompatibilityWarningModal = ref()
const installToPlayModal = ref()
const updateToPlayModal = ref()

const modrinthLoginFlowWaitModal = ref()

// ── Drop import system ──────────────────────────────────────────────────
const dropImport = useDropImport({
	notificationManager,
	popupNotificationManager,
	installModpackFromPath,
	contentInstall,
	fileDrop,
	onSkinsPage,
	onSchematicWorkshopPage,
	isSchematicFile,
	trackEvent,
	router,
})

// Destructure what we need from dropImport
const {
	// State
	isDragging,
	isProcessing,
	batchActive,
	batchPhase,
	batchItems,
	batchOriginalCount,
	batchScanDone,
	dropClassification,
	dropFileName,
	dropFilePath,
	dropProcessingNotificationId,
	scanningInstances,
	batchGroupKey,
	incompatWarningKey,

	// Modal refs
	confirmDropModal,
	genericInstallModal,
	launcherImportModal,
	symlinkCardsModal,
	dataPackWorldModal,
	compatibleModeConfirmModal,
	incompatibilityWarningModal: dropIncompatibilityWarningModal,

	// Handlers
	handleConfirmDropCancel,
	handleConfirmDropConfirm,
	handleConfirmDropHelp,
	handleCompatibleModeConfirm,
	handleGenericInstall,
	handleGenericInstallCancel,
	handleGenericInstallNavigateCreate,
	handleBatchOrDatapackWorldSelect,
	handleBatchWorldAfterHide,
	onLauncherImportCancelled,
	onImportSelected,
	onSymlinkMethodCancelled,
	onSymlinkMethodConfirmed,
	chooseImportMethod,
	cancelBatchScan,
	handleIncompatibilityWarningUpdate,
	handleIncompatibilityWarningCancel,
	handleDropInstallSearchCompat,

	// Utility
	clearDropProcessingNotification,
	showNestedUnpackPrompt,
	showForceAnalysisPrompt,
	unknownReasonMessage,
} = dropImport

provide('chooseImportMethod', chooseImportMethod)

watch(
	dropIncompatibilityWarningModal,
	(modal) => {
		if (modal) {
			setContentIncompatibilityWarningModal(modal)
		}
	},
	{ flush: 'post' },
)

watch(
	incompatibilityWarningModal,
	(modal) => {
		dropIncompatibilityWarningModal.value = modal
	},
	{ flush: 'post' },
)

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
	dropIncompatibilityWarningModal.value = incompatibilityWarningModal.value
	setContentInstallModal(modInstallModal.value)
	setContentInstallPreviewModal(contentInstallPreviewModal.value)
	contentSelection.setPreviewModal(contentInstallPreviewModal.value)
	setContentInstallModpackInstallModal(contentInstallModpackInstallModal.value!)
	setContentInstallCurseForgeManualDownloadsModal(
		contentInstallCurseForgeManualDownloadsModal.value,
	)
	setModpackAlreadyInstalledModal(modpackAlreadyInstalledModal.value)
	setServerAddServerToInstanceModal(addServerToInstanceModal.value)
	setServerInstallToPlayModal(installToPlayModal.value)
	setServerUpdateToPlayModal(updateToPlayModal.value)
	void (async () => {
		try {
			const ready = await invoke<{
				pending_crashes: { instance_id: string; uuid: string }[]
				pending_commands: Parameters<typeof handleCommand>[0][]
			}>('lightweight_mode_frontend_ready', { route: route.fullPath })
			for (const pendingCrash of ready.pending_crashes) {
				const instance = await getInstance(pendingCrash.instance_id).catch(() => null)
				await minecraftCrashModal.value?.handleWarning({
					message: `Instance ${instance?.name || 'Minecraft'} has crashed`,
					kind: 'minecraft_crash',
					instance_id: pendingCrash.instance_id,
					instance_name: instance?.name || 'Minecraft',
				})
			}
			for (const command of ready.pending_commands) await handleCommand(command)
		} catch (error) {
			handleError(error)
		}
	})()
})

const accounts = ref(null)
provide('accountsCard', accounts)

command_listener(handleCommand)

async function handleCommand(e) {
	if (!e) return
	if (e.event === 'OpenSeedMap') {
		const query = Object.fromEntries(new URLSearchParams(e.query ?? ''))
		await router.push({ path: '/lab/seed-map', query })
		return
	}
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
			'Axolotl Launcher v{version} is available. Use your package manager to update for the latest features and fixes!',
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

	if (metered.value && !finishedDownloading.value) {
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
	if (!metered.value) {
		console.log('Starting download of update')
		downloadUpdate(update)
	} else {
		console.log(`Metered connection detected, not auto-downloading update.`)
		markAppUpdateActionable(update.version)
		scheduleDelayedUpdatePopup()
	}

	getUpdateSize(update.rid)
		.then((size) => (updateSize.value = size))
		.catch((error) => console.warn('Failed to fetch update size', error))
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

async function downloadAvailableUpdate() {
	return downloadUpdate(availableUpdate.value)
}

const UPDATE_SOURCE_ORDER = ['miawa', 'cnb', 'github']

async function downloadUpdate(versionToDownload, source = getUpdateSource()) {
	if (!versionToDownload) {
		handleError(`Failed to download update: no version available`)
		return
	}

	if (downloading.value || appUpdateDownload.progress.value !== 0) {
		console.error(`Update ${versionToDownload.version} already downloading`)
		return
	}

	console.log(`Downloading update ${versionToDownload.version} from ${source}`)
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
			.catch((error) => {
				downloading.value = false
				appUpdateDownload.progress.value = 0
				unlistenUpdateDownload?.().then(() => {
					unlistenUpdateDownload = null
				})
				retryUpdateFromNextSource(source, error)
			})
		unlistenUpdateDownload = await subscribeToDownloadProgress(
			appUpdateDownload,
			versionToDownload.version,
		)
	} catch (error) {
		downloading.value = false
		appUpdateDownload.progress.value = 0
		retryUpdateFromNextSource(source, error)
	}
}

// Any download failure falls back to the next update source in line
// (miawa → cnb → github): re-check there and download its installer, so a
// broken mirror never strands users on an outdated build.
async function retryUpdateFromNextSource(failedSource, originalError) {
	const startIndex = UPDATE_SOURCE_ORDER.indexOf(failedSource)
	const remaining = startIndex >= 0 ? UPDATE_SOURCE_ORDER.slice(startIndex + 1) : []

	for (const next of remaining) {
		console.warn(`Update download failed via ${failedSource}; retrying via ${next}`, originalError)
		try {
			const fallbackUpdate = await checkAppUpdate(next)
			if (!fallbackUpdate) {
				console.warn(`No update available via ${next}`)
				continue
			}
			availableUpdate.value = fallbackUpdate
			updateSize.value = null
			getUpdateSize(fallbackUpdate.rid)
				.then((size) => (updateSize.value = size))
				.catch((error) => console.warn('Failed to fetch update size', error))
			await downloadUpdate(fallbackUpdate, next)
			return
		} catch (error) {
			console.warn(`Update check via ${next} failed`, error)
		}
	}

	handleError(originalError)
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
			<AuthGrantFlowWaitModal ref="modrinthLoginFlowWaitModal" @flow-cancel="cancelLogin" />
		</Suspense>
		<InstanceIconPickerModal ref="instanceIconPickerModal" />
		<CreationFlowModal
			ref="installationModal"
			type="instance"
			:available-loaders="[...clientInstallableLoaders]"
			show-snapshot-toggle
			:fetch-existing-instance-names="fetchExistingInstanceNames"
			:search-modpacks="searchModpacks"
			:get-project-versions="getProjectVersions"
			:has-compatible-opti-fabric="hasCompatibleOptiFabric"
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
					:to="discoverContentPath"
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
					v-tooltip.right="formatMessage(messages.multiplayer)"
					to="/multiplayer"
					:is-primary="(r) => r.path.startsWith('/multiplayer')"
				>
					<UsersIcon />
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
					v-tooltip.right="formatMessage(messages.lab)"
					data-onboarding-id="nav-lab"
					to="/lab"
					:is-primary="(r) => r.path.startsWith('/lab')"
				>
					<FlaskConicalIcon />
				</NavButton>
				<NavButton
					v-if="!themeStore.autoHideDownloadsButton || downloadManager.activeCount.value > 0"
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
			<div class="quick-instance-scroll flex-1 min-h-0 overflow-x-hidden overflow-y-auto">
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
				to="/settings"
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
			'studio-mode': route.name === 'FileStudio' || route.name === 'MultiplayerServerFileStudio',
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
			<div class="page-transition-grid grid min-h-full">
				<RouterView v-slot="{ Component, route }">
					<Transition name="page-slide" :css="themeStore.getFeatureFlag('page_transitions')">
						<div v-if="Component" :key="getPageTransitionKey(route)" class="page-transition-layer">
							<Suspense @pending="onSuspensePending" @resolve="onSuspenseResolve">
								<component :is="Component"></component>
							</Suspense>
						</div>
					</Transition>
				</RouterView>
			</div>
		</div>
		<div
			class="app-sidebar mt-px shrink-0 flex flex-col border-0 border-l-[1px] border-[--brand-gradient-border] border-solid"
		>
			<button
				v-if="!forceSidebar && !forceSidebarHidden"
				v-tooltip.left="
					sidebarToggled
						? formatMessage(messages.collapseSidebar)
						: formatMessage(messages.expandSidebar)
				"
				class="sidebar-toggle-handle"
				:aria-label="
					sidebarToggled
						? formatMessage(messages.collapseSidebar)
						: formatMessage(messages.expandSidebar)
				"
				type="button"
				@click="toggleSidebar"
			>
				<RightArrowIcon
					class="w-2.5 h-2.5 -translate-x-[1px] transition-transform duration-300"
					:class="{ 'rotate-180': !sidebarToggled }"
				/>
			</button>
			<div
				v-overlay-scrollbars="sidebarOverlayScrollbarsOptions"
				class="app-sidebar-scrollable relative min-h-0 flex-1"
				data-overlayscrollbars-initialize
			>
				<div id="sidebar-teleport-target" class="sidebar-teleport-content contents"></div>
				<div class="sidebar-default-content hidden" :class="{ 'sidebar-enabled': sidebarVisible }">
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
	<JavaDownloadConfirmationModal ref="javaDownloadConfirmationModal" />
	<PrivacyConsentModal ref="privacyConsentModal" @saved="handlePrivacyConsentSaved" />
	<CommunityAnnouncementModal ref="communityAnnouncementModal" />
	<SurveyAnnouncementModal ref="surveyModal" />
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
	<ContentInstallPreviewModal ref="contentInstallPreviewModal" />
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
		:key="incompatWarningKey"
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
		@search-compat="handleDropInstallSearchCompat"
	/>
	<ModpackInstallModal
		ref="contentInstallModpackInstallModal"
		@install="handleContentInstallModpackInstall"
		@cancel="handleContentInstallModpackInstallCancel"
	/>
	<CurseForgeManualDownloadsModal
		ref="contentInstallCurseForgeManualDownloadsModal"
		@view-instance="handleContentInstallModpackDuplicateGoToInstance"
		@imported="handleContentInstallCurseForgeManualDownloadsImported"
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
		v-if="(isProcessing || scanningInstances) && !isDragging && !onSkinsPage && !batchActive"
		class="fixed inset-0 z-[9999] bg-black/20 flex items-center justify-center"
	>
		<div class="flex flex-col items-center gap-3">
			<SpinnerIcon class="h-10 w-10 animate-spin text-contrast" />
			<span v-if="scanningInstances" class="text-sm text-secondary"
				>{{ formatMessage(messages.dropScanning) }}…</span
			>
		</div>
	</div>

	<!-- Drop type confirmation modal -->
	<ConfirmDropTypeModal
		ref="confirmDropModal"
		:key="batchGroupKey"
		:classification="dropClassification"
		:file-name="dropFileName"
		@confirm="handleConfirmDropConfirm"
		@cancel="handleConfirmDropCancel"
		@help="handleConfirmDropHelp"
	/>

	<!-- Generic content install modal (instance selection when not in an instance page) -->
	<GenericContentInstallModal
		ref="genericInstallModal"
		@install="handleGenericInstall"
		@cancel="handleGenericInstallCancel"
		@navigate-create="handleGenericInstallNavigateCreate"
	/>

	<!-- Data pack world selection modal -->
	<InstanceExportModal
		ref="dataPackWorldModal"
		:show-save-as="false"
		@select="handleBatchOrDatapackWorldSelect"
		@after-hide="handleBatchWorldAfterHide"
	/>

	<!-- Launcher import instance selection modal -->
	<LauncherImportModal
		ref="launcherImportModal"
		@confirm="onImportSelected"
		@cancel="onLauncherImportCancelled"
	/>

	<!-- Symlink method selection modal -->
	<SymlinkMethodCards
		ref="symlinkCardsModal"
		@confirm="onSymlinkMethodConfirmed"
		@cancel="onSymlinkMethodCancelled"
	/>

	<NewModal ref="compatibleModeConfirmModal" max-width="560px" :closable="true">
		<template #title>
			<span class="text-contrast">{{ formatMessage(messages.dropCompatibleModeTitle) }}</span>
		</template>
		<div class="flex flex-col gap-4">
			<span class="text-secondary text-sm">{{
				formatMessage(messages.dropCompatibleModeDesc)
			}}</span>
			<div class="grid grid-cols-2 gap-3">
				<BigOptionButton
					:icon="FolderOpenIcon"
					:title="formatMessage(messages.dropCompatibleModeImport)"
					:description="formatMessage(messages.dropCompatibleModeImport)"
					no-icon-border
					@click="handleCompatibleModeConfirm('compatible')"
				/>
				<BigOptionButton
					:icon="RotateCounterClockwiseIcon"
					:title="formatMessage(messages.dropCompatibleModeOldWay)"
					:description="formatMessage(messages.dropCompatibleModeOldWay)"
					no-icon-border
					@click="handleCompatibleModeConfirm('old-way')"
				/>
			</div>
		</div>
		<template #actions>
			<div class="flex w-full items-center justify-end">
				<ButtonStyled>
					<button class="flex items-center gap-2" @click="handleCompatibleModeConfirm('cancel')">
						{{ formatMessage(messages.dropCompatibleModeCancel) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>

	<!-- Batch drag-and-drop import UI -->
	<BatchScanOverlay
		v-if="batchPhase === 'scanning'"
		:items="batchItems"
		:done="batchScanDone"
		:total="batchOriginalCount"
		@cancel="cancelBatchScan"
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
@property --right-bar-width {
	syntax: '<length>';
	inherits: true;
	initial-value: 0px;
}

.app-grid-layout,
.app-contents {
	--top-bar-height: 3rem;
	--left-bar-width: 4rem;
	--right-bar-width: 300px;
}

.app-contents.studio-mode {
	grid-template-columns: 1fr 0;

	.app-sidebar {
		display: none;
	}
}

.app-contents.studio-mode {
	top: var(--top-bar-height);
	height: calc(100vh - var(--top-bar-height));
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

.quick-instance-scroll {
	-ms-overflow-style: none;
	scrollbar-width: none;

	&::-webkit-scrollbar {
		display: none;
	}
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
}

.app-grid-layout.has-custom-background {
	.app-grid-navbar,
	.app-grid-statusbar {
		background-color: color-mix(in srgb, var(--color-raised-bg) 82%, transparent) !important;

		backdrop-filter: none;
		-webkit-backdrop-filter: none;
	}
}

.app-grid-navbar {
	grid-area: nav;
	position: relative;
	z-index: 2;
	user-select: none;
}

.app-grid-statusbar {
	grid-area: status;
	padding-right: var(--window-controls-width, 0px);
	position: relative;
	z-index: 200;
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
	--right-bar-width: 0px;

	display: grid;
	grid-template-columns: 1fr var(--right-bar-width);
	// 显式行高：让 .app-viewport 的 height: 100% 有确定参照（隐式 auto 行会使百分比高度失效）
	grid-template-rows: 1fr;

	&.sidebar-enabled {
		--right-bar-width: 300px;
	}

	&.has-custom-background,
	&.has-transparent-background {
		background-color: color-mix(in srgb, var(--color-bg) 76%, transparent);
		border-top-left-radius: 0;

		&::before {
			border: none;
			box-shadow: none;
		}
	}

	&.has-custom-background {
		.loading-indicator-container {
			border-top-left-radius: 0;
		}
	}
}

@media (prefers-reduced-motion: no-preference) {
	.app-contents {
		transition: --right-bar-width 320ms cubic-bezier(0.22, 1, 0.36, 1);
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

		backdrop-filter: none;
		-webkit-backdrop-filter: none;
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
	background-color: color-mix(
		in srgb,
		var(--surface-3-opaque) var(--window-alpha-chrome),
		transparent
	);

	&::before {
		position: absolute;
		inset: 0;
		z-index: -10;
		border: none;
		box-shadow: none;
		border-top-left-radius: var(--radius-xl);
		background-color: color-mix(
			in srgb,
			var(--surface-1-opaque) calc(var(--window-alpha) * 0.82),
			transparent
		);
	}

	:deep(.browse-install-header) {
		background-color: color-mix(in srgb, var(--surface-1-opaque) 68%, transparent) !important;

		backdrop-filter: blur(20px) saturate(115%);
		-webkit-backdrop-filter: blur(20px) saturate(115%);
	}
}

.loading-indicator-container {
	border-top-left-radius: var(--radius-xl);
	overflow: hidden;
}

.app-sidebar {
	overflow: visible;
	width: 300px;
	position: relative;
	z-index: 11;
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
		--tw-drop-shadow: initial;
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

.sidebar-toggle-handle {
	--handle-bg: color-mix(in srgb, var(--color-brand) 12%, var(--color-bg));
	--handle-bg-hover: color-mix(in srgb, var(--color-brand) 20%, var(--color-bg));
	--handle-border: var(--brand-gradient-border);
	--handle-border-hover: color-mix(in srgb, var(--color-brand) 45%, transparent);

	position: absolute;
	top: 50%;
	left: -15px;
	transform: translateY(-50%);
	z-index: 12;

	display: flex;
	align-items: center;
	justify-content: center;
	width: 15px;
	height: 40px;
	padding: 0;

	// 只让外侧(左边)圆润,右边与 Sidebar 完全贴合,单一元素完成形状
	border: 1px solid var(--handle-border);
	border-right: none;
	border-radius: 12px 0 0 12px;

	background-color: var(--handle-bg);
	color: var(--color-contrast);
	cursor: pointer;

	box-shadow: -4px 0 10px rgba(0, 0, 0, 0.08);

	transition:
		background-color 180ms ease,
		border-color 180ms ease,
		color 180ms ease;

	&:hover {
		color: var(--color-button-text-selected);
		background-color: var(--handle-bg-hover);
		border-color: var(--handle-border-hover);
	}

	&:hover svg {
		transform: scale(1.12);
	}

	&:active svg {
		transform: scale(0.9);
	}

	&:focus-visible {
		outline: 2px solid var(--color-button-text-selected);
		outline-offset: 1px;
	}

	svg {
		transform-origin: center;
		transition: transform 180ms ease;
	}
}

.app-viewport {
	flex-grow: 1;
	height: 100%;
	overflow: auto;
	overflow-x: hidden;
	scrollbar-gutter: stable;
	padding-bottom: var(--floating-action-bar-clearance, 0px);
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
