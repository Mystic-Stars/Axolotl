<script setup lang="ts">
import AppleIcon from '@modrinth/assets/external/apple.svg?component'
import LinuxIcon from '@modrinth/assets/external/linux.svg?component'
import WindowsIcon from '@modrinth/assets/external/windows.svg?component'
import ArrowDownIcon from '@modrinth/assets/icons/arrow-down.svg?component'
import BoxesIcon from '@modrinth/assets/icons/boxes.svg?component'
import DownloadIcon from '@modrinth/assets/icons/download.svg?component'
import GaugeIcon from '@modrinth/assets/icons/gauge.svg?component'
import GitGraphIcon from '@modrinth/assets/icons/git-graph.svg?component'
import IssuesIcon from '@modrinth/assets/icons/issues.svg?component'
import SearchIcon from '@modrinth/assets/icons/search.svg?component'
import TrashIcon from '@modrinth/assets/icons/trash.svg?component'
import Accordion from '@modrinth/ui/src/components/base/Accordion.vue'
import Avatar from '@modrinth/ui/src/components/base/Avatar.vue'
import ButtonStyled from '@modrinth/ui/src/components/base/ButtonStyled.vue'
import Checkbox from '@modrinth/ui/src/components/base/Checkbox.vue'
import IntlFormatted from '@modrinth/ui/src/components/base/IntlFormatted.vue'
import { defineMessages, useVIntl } from '@modrinth/ui/src/composables/i18n.ts'

import AppleLogo from '~/components/landing/AppleLogo.vue'
import CommunitySection from '~/components/landing/CommunitySection.vue'
import ImportGradientIcon from '~/components/landing/ImportGradientIcon.vue'
import LinuxLogo from '~/components/landing/LinuxLogo.vue'
import MultiplayerIcon from '~/components/landing/MultiplayerIcon.vue'
import OfflineModeIcon from '~/components/landing/OfflineModeIcon.vue'
import ProjectsShowcase from '~/components/landing/ProjectsShowcase.vue'
import WindowsLogo from '~/components/landing/WindowsLogo.vue'

interface UpdateServerDownloadMetadata {
	version: string
	downloads: Array<{ filename: string; url: string }>
}

type OSType = 'Mac' | 'Windows' | 'Linux' | null

const downloadWindows = ref<HTMLAnchorElement | null>(null)
const downloadMac = ref<HTMLAnchorElement | null>(null)
const downloadSection = ref<HTMLElement | null>(null)
const hero = ref<HTMLElement | null>(null)

const updateHeroGlow = (event: PointerEvent) => {
	if (!hero.value) return

	const bounds = hero.value.getBoundingClientRect()
	const x = Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width))
	const y = Math.max(0, Math.min(1, (event.clientY - bounds.top) / bounds.height))

	hero.value.style.setProperty('--pointer-x', `${x * 100}%`)
	hero.value.style.setProperty('--pointer-y', `${y * 100}%`)
}

const resetHeroGlow = () => {
	if (!hero.value) return

	hero.value.style.removeProperty('--pointer-x')
	hero.value.style.removeProperty('--pointer-y')
}

const { resolvedSource } = useDownloadSource()
const UPDATE_SERVER_BASE_URL = 'https://update.axlmc.org'
const GITHUB_RELEASE_BASE_URL = 'https://github.com/Mystic-Stars/Axolotl/releases/download'
const releaseApi = `${UPDATE_SERVER_BASE_URL}/api/downloads/latest?channel=release`

const windowsLink = ref<string | null>(null)

const linuxLinks = reactive({
	appImage: null as string | null,
	deb: null as string | null,
	rpm: null as string | null,
})

const macLinks = reactive({
	universal: null as string | null,
})

// 使用 Update Server 的下载目录作为默认发布数据源；GitHub 保留为手动备用源。
const { data: launcherRelease, status: launcherReleaseStatus } =
	await useFetch<UpdateServerDownloadMetadata>(releaseApi, {
		// Fetch at generate time so the static site does not depend on browser CORS.
		transform(data: UpdateServerDownloadMetadata) {
			return {
				tag_name: `v${data.version}`,
				assets: data.downloads.map((download) => download.filename),
			}
		},
	})

// 下载链接的三种状态：获取中 / 就绪 / 失败。失败时主 CTA 降级为跳转下载区，
// 下载区显示错误提示与 GitHub Releases 手动下载入口，避免点击死链接。
const downloadState = computed<'loading' | 'ready' | 'error'>(() => {
	if (launcherReleaseStatus.value === 'success') return 'ready'
	if (launcherReleaseStatus.value === 'error') return 'error'
	return 'loading'
})

const linkUnavailableLabel = computed(() =>
	downloadState.value === 'error'
		? formatMessage(messages.downloadLinksFailed)
		: formatMessage(messages.fetchingDownloadLinks),
)

const platform = computed<string>(() => {
	if (import.meta.server) {
		const headers = useRequestHeaders()
		return headers['user-agent'] || ''
	} else {
		return navigator.userAgent || ''
	}
})
const os = computed<OSType>(() => {
	if (/(iPhone|iPad|Android|Mobile)/.test(platform.value)) {
		return null
	} else if (platform.value.includes('Mac')) {
		return 'Mac'
	} else if (platform.value.includes('Win')) {
		return 'Windows'
	} else if (platform.value.includes('Linux')) {
		return 'Linux'
	} else {
		return null
	}
})

const modManagementData = [
	{
		id: 'P7dR8mSH', // Todo: fetch name + author + icon from api
		name: 'Fabric API',
		author: 'modmuss50',
		version: '0.86.1+1.20.1',
		iconUrl: 'https://cdn.modrinth.com/data/P7dR8mSH/icon.png',
	},
	{
		id: 'AANobbMI',
		name: 'Sodium',
		author: 'jellysquid3',
		version: 'mc1.20.1-0.5.0',
		iconUrl: 'https://cdn.modrinth.com/data/AANobbMI/icon.png',
	},
	{
		id: 'YL57xq9U',
		name: 'Iris Shaders',
		author: 'coderbot',
		version: '1.6.5+1.20.1',
		iconUrl: 'https://cdn.modrinth.com/data/YL57xq9U/dc558eece920db435f9823ce86de0c4cde89800b.png',
	},
	{
		id: 'gvQqBUqZ',
		name: 'Lithium',
		author: 'jellysquid3',
		version: 'mc1.20.1-0.11.2',
		iconUrl: 'https://cdn.modrinth.com/data/gvQqBUqZ/icon.png',
	},
	{
		id: 'mOgUt4GM',
		name: 'Mod Menu',
		author: 'Prospector',
		version: '7.2.1',
		iconUrl: 'https://cdn.modrinth.com/data/mOgUt4GM/5a20ed1450a0e1e79a1fe04e61bb4e5878bf1d20.png',
	},
	{
		id: '9s6osm5g',
		name: 'Cloth Config API',
		author: 'shedaniel',
		version: '11.1.106+fabric',
		iconUrl: 'https://cdn.modrinth.com/data/9s6osm5g/icon.png',
	},
	{
		id: 'lhGA9TYQ',
		name: 'Architectury API',
		author: 'shedaniel',
		version: '9.1.12+fabric',
		iconUrl: 'https://cdn.modrinth.com/data/lhGA9TYQ/icon.png',
	},
	{
		id: 'nrJ2NpD0',
		name: 'Craftify',
		author: 'ThatGravyBoat',
		version: '8.5.2023',
		iconUrl: 'https://cdn.modrinth.com/data/nrJ2NpD0/4f21214db060ed4542b1f3983c4113d293480a1b.webp',
	},
]

// 演示表格的交互状态：checkbox 真实可切换，删除按钮移除对应行（刷新恢复）
const checkedMods = ref(modManagementData.map(() => true))
const removeMod = (index: number) => {
	modManagementData.splice(index, 1)
	checkedMods.value.splice(index, 1)
}

const downloadLauncher = computed(() => {
	// 获取中：按钮禁用，不做任何事
	if (downloadState.value === 'loading') return () => {}
	// 获取失败：降级为跳转下载区（那里有错误提示和手动下载入口）
	if (downloadState.value === 'error') return scrollToSection
	if (os.value === 'Windows') {
		return () => {
			downloadWindows.value?.click()
		}
	} else if (os.value === 'Mac') {
		return () => {
			downloadMac.value?.click()
		}
	} else {
		return () => {
			scrollToSection()
		}
	}
})

const handleDownload = () => {
	downloadLauncher.value()
}

watch(
	[launcherRelease, resolvedSource],
	([release]) => {
		const findAsset = (patterns: RegExp[]) => {
			const assetName = release?.assets.find((name) =>
				patterns.some((pattern) => pattern.test(name)),
			)
			if (!assetName) return null

			if (resolvedSource.value === 'update-server') {
				const version = release.tag_name.replace(/^v/, '')
				return `${UPDATE_SERVER_BASE_URL}/dist/${encodeURIComponent(version)}/${encodeURIComponent(assetName)}`
			}

			return `${GITHUB_RELEASE_BASE_URL}/${encodeURIComponent(release.tag_name)}/${encodeURIComponent(assetName)}`
		}

		windowsLink.value = findAsset([/x64-setup\.exe$/i, /\.exe$/i])
		macLinks.universal = findAsset([/universal\.dmg$/i, /\.dmg$/i])
		linuxLinks.appImage = findAsset([/(amd64|x86_64)\.AppImage$/i])
		linuxLinks.deb = findAsset([/_amd64\.deb$/i, /(amd64|x86_64).*\.deb$/i])
		linuxLinks.rpm = findAsset([/(x86_64|amd64).*\.rpm$/i])
	},
	{ immediate: true },
)

const scrollToSection = () => {
	nextTick(() => {
		if (downloadSection.value) {
			window.scrollTo({
				top: downloadSection.value.offsetTop,
				behavior: 'smooth',
			})
		}
	})
}

const { formatMessage, locale } = useVIntl()

const messages = defineMessages({
	openSourceBadge: {
		id: 'axolotl-marketing.hero.open-source',
		defaultMessage: 'Tauri v2 - Rust - Vue 3',
	},
	oneLauncher: {
		id: 'axolotl-marketing.demo.one-launcher',
		defaultMessage: 'One launcher. Every world.',
	},
	everythingTogether: {
		id: 'axolotl-marketing.demo.everything-together',
		defaultMessage: 'Profiles, mods, saves, and settings stay together.',
	},
	includedMods: {
		id: 'axolotl-marketing.demo.included-mods',
		defaultMessage: 'Included mods',
	},
	downloadAxolotl: {
		id: 'axolotl-marketing.hero.download',
		defaultMessage: 'Axolotl Launcher',
	},
	downloadAxolotlForOs: {
		id: 'axolotl-marketing.hero.download-for-os',
		defaultMessage: 'Axolotl Launcher for {os}',
	},
	description: {
		id: 'app-marketing.hero.description',
		defaultMessage:
			'Axolotl Launcher is a free, open-source, ad-free, cross-platform Minecraft Java Edition launcher for searching, installing, and updating mods, modpacks, resource packs, and shaders from Modrinth and CurseForge, with Axolotl Labs built in.',
	},
	heroScreenshotAlt: {
		id: 'axolotl-marketing.hero.screenshot-alt',
		defaultMessage: 'Axolotl Launcher home screen.',
	},
	builtOnModrinth: {
		id: 'axolotl-marketing.highlights.eyebrow',
		defaultMessage: 'One launcher, two sources',
	},
	highlightsTitle: {
		id: 'axolotl-marketing.highlights.title',
		defaultMessage: 'Manage Minecraft content',
	},
	highlightsTitleSecond: {
		id: 'axolotl-marketing.highlights.title-second',
		defaultMessage: 'without detours',
	},
	highlightsDescription: {
		id: 'axolotl-marketing.highlights.description',
		defaultMessage:
			'Search Modrinth and CurseForge, then inspect projects, choose versions, install content, resolve dependencies, and keep it updated from the launcher.',
	},
	adFree: {
		id: 'axolotl-marketing.highlights.ad-free.title',
		defaultMessage: 'Free, open, and independent',
	},
	adFreeDescription: {
		id: 'axolotl-marketing.highlights.ad-free.description',
		defaultMessage:
			'GPL-3.0, free to use, and ad-free. Axolotl is not an official Modrinth client.',
	},
	localized: {
		id: 'axolotl-marketing.highlights.localized.title',
		defaultMessage: 'Content management that stays organized',
	},
	localizedDescription: {
		id: 'axolotl-marketing.highlights.localized.description',
		defaultMessage:
			'Install and manage modpacks alongside individual projects. Some CurseForge files have distribution limits and may require a manual download.',
	},
	offlineAccounts: {
		id: 'axolotl-marketing.showcase.offline.title',
		defaultMessage: 'Accounts on your terms',
	},
	offlineAccountsDescription: {
		id: 'axolotl-marketing.showcase.offline.description',
		defaultMessage:
			'Sign in with Microsoft, create a local offline identity, or use Yggdrasil authentication with LittleSkin presets and custom servers.',
	},
	offlineLabel: { id: 'axolotl-marketing.showcase.offline.label', defaultMessage: 'Accounts' },
	themes: {
		id: 'axolotl-marketing.showcase.themes.title',
		defaultMessage: 'A color theme for every setup',
	},
	themesDescription: {
		id: 'axolotl-marketing.showcase.themes.description',
		defaultMessage:
			'Switch between light, dark, OLED, and system modes, then set your accent color, background, and transparency to match your setup.',
	},
	personalizeLabel: {
		id: 'axolotl-marketing.showcase.themes.label',
		defaultMessage: 'Personalize',
	},
	translation: {
		id: 'axolotl-marketing.showcase.translation.title',
		defaultMessage: 'Axolotl Lab, inside the launcher',
	},
	translationDescription: {
		id: 'axolotl-marketing.showcase.translation.description',
		defaultMessage:
			'Use the gradient text generator, Java Edition seed map, and 3D schematic workshop directly in Axolotl, not through external web pages.',
	},
	translateLabel: { id: 'axolotl-marketing.showcase.translation.label', defaultMessage: 'Lab' },
	offlineScreenshotAlt: {
		id: 'axolotl-marketing.showcase.offline.alt',
		defaultMessage: 'Axolotl Launcher offline account dialog.',
	},
	themesScreenshotAlt: {
		id: 'axolotl-marketing.showcase.themes.alt',
		defaultMessage: 'Axolotl Launcher theme customization settings.',
	},
	translationScreenshotAlt: {
		id: 'axolotl-marketing.showcase.translation.alt',
		defaultMessage: 'Axolotl Launcher Lab.',
	},
	downloadAxolotlButton: {
		id: 'axolotl-marketing.hero.download-button',
		defaultMessage: 'Download Axolotl',
	},
	fetchingDownloadLinks: {
		id: 'axolotl-marketing.download.fetching-links',
		defaultMessage: 'Fetching download links…',
	},
	downloadLinksFailed: {
		id: 'axolotl-marketing.download.links-failed',
		defaultMessage: 'Could not fetch the latest download links.',
	},
	manualDownloadFallback: {
		id: 'axolotl-marketing.download.manual-fallback',
		defaultMessage: 'Download manually from GitHub Releases',
	},
	moreDownloadOptions: {
		id: 'app-marketing.hero.more-download-options',
		defaultMessage: 'More Download Options',
	},
	installedMods: {
		id: 'app-marketing.features.mod-management.installed-mods',
		defaultMessage: 'Installed mods',
	},
	searchMods: {
		id: 'app-marketing.features.mod-management.search-mods',
		defaultMessage: 'Search mods',
	},
	name: {
		id: 'app-marketing.features.mod-management.name',
		defaultMessage: 'Name',
	},
	version: {
		id: 'app-marketing.features.mod-management.version',
		defaultMessage: 'Version',
	},
	actions: {
		id: 'app-marketing.features.mod-management.actions',
		defaultMessage: 'Actions',
	},
	byAuthor: {
		id: 'app-marketing.features.mod-management.byAuthor',
		defaultMessage: 'by {author}',
	},
	modManagement: {
		id: 'app-marketing.features.mod-management.title',
		defaultMessage: 'Efficient instance management',
	},
	modManagementDescription: {
		id: 'app-marketing.features.mod-management.description',
		defaultMessage:
			'Create, import, and manage instances in bulk. Keep mods, resource packs, shaders, files, worlds, screenshots, and logs together with updates, launch settings, and modpack export.',
	},
	performant: {
		id: 'app-marketing.features.performance.title',
		defaultMessage: 'Performant',
	},
	performantDescription: {
		id: 'app-marketing.features.performance.description',
		defaultMessage:
			'Axolotl stays out of your way with a responsive interface and a lightweight desktop core.',
	},
	profileImporting: {
		id: 'app-marketing.features.importing.title',
		defaultMessage: 'Profile importing',
	},
	profileImportingDescription: {
		id: 'app-marketing.features.importing.description',
		defaultMessage:
			'Import your existing profiles from PCL2, HMCL, or any launcher you like with one click, and keep playing without rebuilding everything by hand.',
	},
	offlineMode: {
		id: 'app-marketing.features.offline.title',
		defaultMessage: 'Useful around every world',
	},
	offlineModeDescription: {
		id: 'app-marketing.features.offline.description',
		defaultMessage:
			'Chinese search and project translation, drag-and-drop import, Java management, offline mode, and skin management are ready when you need them.',
	},
	followProjects: {
		id: 'app-marketing.features.follow.title',
		defaultMessage: 'Multiplayer support',
	},
	followProjectsDescription: {
		id: 'app-marketing.features.follow.description',
		defaultMessage: 'Terracotta-powered multiplayer networking, jump in with a single click.',
	},
	downloadOptions: {
		id: 'app-marketing.download.options-title',
		defaultMessage: 'Download options',
	},
	downloadAxolotlTitle: {
		id: 'axolotl-marketing.download.title',
		defaultMessage: 'Download Axolotl Launcher',
	},
	downloadDescription: {
		id: 'app-marketing.download.description',
		defaultMessage:
			'Our desktop app is available across all platforms, choose your desired version.',
	},
	windows: {
		id: 'app-marketing.download.windows',
		defaultMessage: 'Windows',
	},
	mac: {
		id: 'app-marketing.download.mac',
		defaultMessage: 'Mac',
	},
	linux: {
		id: 'app-marketing.download.linux',
		defaultMessage: 'Linux',
	},
	downloadInstaller: {
		id: 'axolotl-marketing.download.installer',
		defaultMessage: 'Download installer',
	},
	downloadAppImage: {
		id: 'axolotl-marketing.download.appimage',
		defaultMessage: 'Download the AppImage',
	},
	showOtherPackages: {
		id: 'app-marketing.show-other-packages',
		defaultMessage: 'Show other packages',
	},
	hideOtherPackages: {
		id: 'app-marketing.hide-other-packages',
		defaultMessage: 'Hide other packages',
	},
	notRecommended: {
		id: 'app-marketing.not-recommended',
		defaultMessage: 'Choose the package format that matches your Linux distribution.',
	},
	downloadTheDEB: {
		id: 'app-marketing.download.download-deb',
		defaultMessage: 'Download the DEB',
	},
	downloadTheRPM: {
		id: 'app-marketing.download.download-rpm',
		defaultMessage: 'Download the RPM',
	},
	downloadTerms: {
		id: 'app-marketing.download.terms',
		defaultMessage:
			'Axolotl is free software released under <terms-link>GPL-3.0</terms-link>. Read the <privacy-link>Privacy Policy</privacy-link> before installing.',
	},
	linuxDisclaimer: {
		id: 'app-marketing.download.linux-disclaimer',
		defaultMessage:
			'Linux packages are published with every release. Check the <issues-link>release page</issues-link> for architecture details or <prism-link>report an issue</prism-link> if your distribution needs extra setup.',
	},
	seoTitle: {
		id: 'axolotl-site.seo.title',
		defaultMessage:
			'Axolotl Launcher - Free Open-Source Modrinth plus Curseforge Minecraft Launcher',
	},
	seoDescription: {
		id: 'axolotl-site.seo.description',
		defaultMessage:
			'Download Axolotl Launcher, a free, open-source Tauri v2 Minecraft launcher for Windows, macOS, and Linux with Modrinth and CurseForge content management, themes, accounts, and more.',
	},
	socialImageAlt: {
		id: 'axolotl-site.seo.social-image-alt',
		defaultMessage: 'Axolotl Launcher showing a Minecraft instance and its installed content.',
	},
	faqEyebrow: {
		id: 'axolotl-site.faq.eyebrow',
		defaultMessage: 'Frequently asked questions',
	},
	faqTitle: {
		id: 'axolotl-site.faq.title',
		defaultMessage: 'Everything you need to know about Axolotl',
	},
	faqDescription: {
		id: 'axolotl-site.faq.description',
		defaultMessage: 'Learn about supported platforms, accounts, content, and downloads.',
	},
	faqPlatformsQuestion: {
		id: 'axolotl-site.faq.platforms.question',
		defaultMessage: 'Which operating systems does Axolotl Launcher support?',
	},
	faqPlatformsAnswer: {
		id: 'axolotl-site.faq.platforms.answer',
		defaultMessage:
			'Axolotl Launcher supports Windows 10 and 11 on x64, macOS on Intel and Apple Silicon, and Linux x64 through AppImage, DEB, and RPM packages.',
	},
	faqFreeQuestion: {
		id: 'axolotl-site.faq.free.question',
		defaultMessage: 'Is Axolotl Launcher free and open source?',
	},
	faqFreeAnswer: {
		id: 'axolotl-site.faq.free.answer',
		defaultMessage:
			'Yes. Axolotl Launcher is free software released under GPL-3.0. Its source code and release history are publicly available on GitHub.',
	},
	faqAccountsQuestion: {
		id: 'axolotl-site.faq.accounts.question',
		defaultMessage: 'Can I use Microsoft and offline Minecraft accounts?',
	},
	faqAccountsAnswer: {
		id: 'axolotl-site.faq.accounts.answer',
		defaultMessage:
			'Yes. Axolotl supports Microsoft Minecraft accounts, local offline accounts, and third-party Yggdrasil authentication, including LittleSkin presets and custom servers.',
	},
	faqContentQuestion: {
		id: 'axolotl-site.faq.content.question',
		defaultMessage: 'Where does Axolotl get mods and other Minecraft content?',
	},
	faqContentAnswer: {
		id: 'axolotl-site.faq.content.answer',
		defaultMessage:
			'Axolotl helps you search, inspect, choose versions for, install, update, and manage content from Modrinth and CurseForge. Files with CurseForge distribution restrictions may require a manual download.',
	},
	faqDownloadQuestion: {
		id: 'axolotl-site.faq.download.question',
		defaultMessage: 'Where should I download Axolotl Launcher?',
	},
	faqDownloadAnswer: {
		id: 'axolotl-site.faq.download.answer',
		defaultMessage:
			'Use the download section on this official website. Automatic mode uses the Update Server, with GitHub available as a backup source.',
	},
	appScreenshotAlt: {
		id: 'app-marketing.hero.app-screenshot-alt',
		defaultMessage: `Axolotl Launcher instance content preview.`,
	},
	structuredFeatureContentSources: {
		id: 'axolotl-site.structured-data.feature.content-sources',
		defaultMessage:
			'Search, install, and update mods, modpacks, resource packs, and shaders from Modrinth and CurseForge',
	},
	structuredFeatureLab: {
		id: 'axolotl-site.structured-data.feature.lab',
		defaultMessage:
			'Axolotl Labs with gradient text generator, Java Edition seed map, and 3D schematic workshop',
	},
	structuredFeatureInstances: {
		id: 'axolotl-site.structured-data.feature.instances',
		defaultMessage: 'Instance, world, screenshot, log, Java, and modpack management',
	},
	structuredFeatureAccounts: {
		id: 'axolotl-site.structured-data.feature.accounts',
		defaultMessage: 'Microsoft, offline, LittleSkin, and custom Yggdrasil account support',
	},
	faqLabQuestion: {
		id: 'axolotl-site.faq.lab.question',
		defaultMessage: 'What is Axolotl Labs?',
	},
	faqLabAnswer: {
		id: 'axolotl-site.faq.lab.answer',
		defaultMessage:
			'Axolotl Labs is a collection of built-in launcher tools, including a gradient text generator, Java Edition seed map, and 3D schematic workshop.',
	},
	faqProjectDisclaimerQuestion: {
		id: 'axolotl-site.faq.project-disclaimer.question',
		defaultMessage: 'Is Axolotl Launcher and Axolotl Client the same project?',
	},
	faqProjectDisclaimerAnswer: {
		id: 'axolotl-site.faq.project-disclaimer.answer',
		defaultMessage:
			'No. Axolotl Launcher and Axolotl Client are separate projects. Axolotl Launcher is an independent, unofficial downstream launcher based on the Modrinth monorepo. It is not affiliated with other Minecraft projects named Axolotl Client.',
	},
	seoKeywords: {
		id: 'axolotl-site.seo.keywords',
		defaultMessage:
			'Axolotl Launcher, Minecraft Launcher, Modrinth, CurseForge, Minecraft Java Edition, Axolotl Labs',
	},
})

const config = useRuntimeConfig()
const siteUrl = config.public.siteUrl
const canonicalUrl = `${siteUrl}/`
const socialImageUrl = `${siteUrl}/showcase/launcher-home.webp`
const githubUrl = 'https://github.com/Mystic-Stars/Axolotl'
const licenseUrl = `${githubUrl}/blob/main/LICENSE`

const title = computed(() => formatMessage(messages.seoTitle))
const description = computed(() => formatMessage(messages.seoDescription))
const socialImageAlt = computed(() => formatMessage(messages.socialImageAlt))
const faqItems = computed(() => [
	{
		question: formatMessage(messages.faqPlatformsQuestion),
		answer: formatMessage(messages.faqPlatformsAnswer),
	},
	{
		question: formatMessage(messages.faqFreeQuestion),
		answer: formatMessage(messages.faqFreeAnswer),
	},
	{
		question: formatMessage(messages.faqAccountsQuestion),
		answer: formatMessage(messages.faqAccountsAnswer),
	},
	{
		question: formatMessage(messages.faqContentQuestion),
		answer: formatMessage(messages.faqContentAnswer),
	},
	{
		question: formatMessage(messages.faqDownloadQuestion),
		answer: formatMessage(messages.faqDownloadAnswer),
	},
	{
		question: formatMessage(messages.faqLabQuestion),
		answer: formatMessage(messages.faqLabAnswer),
	},
	{
		question: formatMessage(messages.faqProjectDisclaimerQuestion),
		answer: formatMessage(messages.faqProjectDisclaimerAnswer),
	},
])
const keywords = computed(() => formatMessage(messages.seoKeywords))

const structuredData = computed(() => ({
	'@context': 'https://schema.org',
	'@graph': [
		{
			'@type': 'WebSite',
			'@id': `${canonicalUrl}#website`,
			url: canonicalUrl,
			name: 'Axolotl Launcher',
			description: description.value,
			inLanguage: locale.value,
			publisher: { '@id': `${canonicalUrl}#organization` },
		},
		{
			'@type': 'Organization',
			'@id': `${canonicalUrl}#organization`,
			name: 'Axolotl Launcher Team',
			url: canonicalUrl,
			logo: {
				'@type': 'ImageObject',
				url: `${siteUrl}/axolotl.png`,
				width: 256,
				height: 256,
			},
			sameAs: [githubUrl],
		},
		{
			'@type': 'SoftwareApplication',
			'@id': `${canonicalUrl}#software`,
			name: 'Axolotl Launcher',
			alternateName: ['美西螈启动器', 'AXL Launcher'],
			sameAs: [githubUrl],
			description: description.value,
			url: canonicalUrl,
			downloadUrl: `${canonicalUrl}#download`,
			image: socialImageUrl,
			applicationCategory: 'GameApplication',
			applicationSubCategory: 'Minecraft Launcher',
			operatingSystem: 'Windows 10/11, macOS, Linux',
			isAccessibleForFree: true,
			license: licenseUrl,
			softwareHelp: `${githubUrl}#readme`,
			author: { '@id': `${canonicalUrl}#organization` },
			inLanguage: ['zh-CN', 'en-US'],
			featureList: [
				formatMessage(messages.structuredFeatureContentSources),
				formatMessage(messages.structuredFeatureLab),
				formatMessage(messages.structuredFeatureInstances),
				formatMessage(messages.structuredFeatureAccounts),
			],
		},
		{
			'@type': 'FAQPage',
			'@id': `${canonicalUrl}#faq`,
			inLanguage: locale.value,
			mainEntity: faqItems.value.map((item) => ({
				'@type': 'Question',
				name: item.question,
				acceptedAnswer: {
					'@type': 'Answer',
					text: item.answer,
				},
			})),
		},
	],
}))

useSeoMeta({
	title: () => title.value,
	description: () => description.value,
	robots: 'index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1',
	author: 'Axolotl Launcher Team',
	applicationName: 'Axolotl Launcher',
	themeColor: '#ff82b2',
	colorScheme: 'dark light',
	ogTitle: () => title.value,
	ogDescription: () => description.value,
	ogType: 'website',
	ogUrl: canonicalUrl,
	ogSiteName: 'Axolotl Launcher',
	ogLocale: () => locale.value.replace('-', '_'),
	ogLocaleAlternate: () => (locale.value === 'zh-CN' ? 'en_US' : 'zh_CN'),
	ogImage: socialImageUrl,
	ogImageAlt: () => socialImageAlt.value,
	ogImageWidth: 2560,
	ogImageHeight: 1489,
	twitterCard: 'summary_large_image',
	twitterTitle: () => title.value,
	twitterDescription: () => description.value,
	twitterImage: socialImageUrl,
	twitterImageAlt: () => socialImageAlt.value,
})

useHead(() => ({
	link: [{ rel: 'canonical', href: canonicalUrl }],
	meta: [
		{
			name: 'keywords',
			content: keywords.value,
		},
	],
	script: [
		{
			key: 'axolotl-structured-data',
			type: 'application/ld+json',
			innerHTML: JSON.stringify(structuredData.value).replace(/</g, '\\u003c'),
		},
	],
}))
</script>

<template>
	<div>
		<div
			ref="hero"
			class="landing-hero"
			@pointerleave="resetHeroGlow"
			@pointermove="updateHeroGlow"
		>
			<div class="hero-grid" aria-hidden="true" />
			<div class="hero-content">
				<div class="flex items-center gap-3">
					<div class="hero-kicker">
						{{ formatMessage(messages.openSourceBadge) }}
					</div>
				</div>
				<h1
					class="main-header mb-8 mt-4 max-w-[52rem] text-balance text-[5.25rem] font-semibold leading-none text-[var(--color-contrast)]"
				>
					{{ formatMessage(messages.downloadAxolotl) }}
				</h1>
				<p class="main-subheader">
					{{ formatMessage(messages.description) }}
				</p>
				<div class="button-group mt-3 flex flex-wrap justify-end gap-2">
					<ButtonStyled v-if="os" color="brand" size="large">
						<button
							class="hero-download-button"
							:disabled="downloadState === 'loading'"
							@click="handleDownload"
						>
							<LinuxIcon v-if="os === 'Linux'" />
							<WindowsIcon v-else-if="os === 'Windows'" />
							<AppleIcon v-else-if="os === 'Mac'" />
							{{
								downloadState === 'loading'
									? formatMessage(messages.fetchingDownloadLinks)
									: formatMessage(messages.downloadAxolotlButton)
							}}
						</button>
					</ButtonStyled>
					<ButtonStyled type="outlined" size="large">
						<button @click="scrollToSection">
							<ArrowDownIcon />
							{{ formatMessage(messages.moreDownloadOptions) }}
						</button>
					</ButtonStyled>
				</div>
			</div>
			<div class="hero-product">
				<img
					class="block h-auto w-full rounded"
					src="/showcase/launcher-home.webp"
					:alt="formatMessage(messages.heroScreenshotAlt)"
					width="2560"
					height="1489"
					decoding="async"
					fetchpriority="high"
				/>
			</div>
			<div class="hero-scroll-mark" aria-hidden="true"><span /></div>
			<div class="bottom-transition" />
		</div>
		<section id="features" class="axolotl-highlights" aria-labelledby="axolotl-highlights-title">
			<div class="highlights-intro">
				<span class="text-xs font-extrabold uppercase tracking-[0.1em] text-brand">{{
					formatMessage(messages.builtOnModrinth)
				}}</span>
				<h2 id="axolotl-highlights-title">
					{{ formatMessage(messages.highlightsTitle) }}<br />
					{{ formatMessage(messages.highlightsTitleSecond) }}
				</h2>
				<p>{{ formatMessage(messages.highlightsDescription) }}</p>
			</div>

			<div class="modrinth-feature-grid">
				<article
					class="feature gradient-border promise-card col-span-2 min-h-[12.5rem] p-6"
					data-number="01"
				>
					<div class="promise-meta"><GitGraphIcon /><span>01</span></div>
					<h3>{{ formatMessage(messages.adFree) }}</h3>
					<p>{{ formatMessage(messages.adFreeDescription) }}</p>
				</article>
				<article
					class="feature gradient-border promise-card col-span-2 min-h-[12.5rem] p-6"
					data-number="02"
				>
					<div class="promise-meta"><BoxesIcon /><span>02</span></div>
					<h3>{{ formatMessage(messages.localized) }}</h3>
					<p>{{ formatMessage(messages.localizedDescription) }}</p>
				</article>
				<article
					class="feature gradient-border promise-card col-span-2 min-h-[12.5rem] p-6"
					data-number="03"
				>
					<div class="promise-meta"><GaugeIcon /><span>03</span></div>
					<h3>{{ formatMessage(messages.performant) }}</h3>
					<p>{{ formatMessage(messages.performantDescription) }}</p>
				</article>
				<article
					class="feature gradient-border showcase-card showcase-card-wide col-span-3 flex min-w-0 flex-col overflow-hidden p-0"
				>
					<div class="showcase-copy px-7 pb-6 pt-7">
						<span>{{ formatMessage(messages.offlineLabel) }}</span>
						<h3>{{ formatMessage(messages.offlineAccounts) }}</h3>
						<p>{{ formatMessage(messages.offlineAccountsDescription) }}</p>
					</div>
					<img
						class="block h-auto w-full"
						src="/showcase/account-login.png"
						:alt="formatMessage(messages.offlineScreenshotAlt)"
						width="3104"
						height="1814"
						decoding="async"
						loading="lazy"
					/>
				</article>

				<article
					class="feature gradient-border showcase-card col-span-3 flex min-w-0 flex-col overflow-hidden p-0"
				>
					<div class="showcase-copy px-7 pb-6 pt-7">
						<span>{{ formatMessage(messages.personalizeLabel) }}</span>
						<h3>{{ formatMessage(messages.themes) }}</h3>
						<p>{{ formatMessage(messages.themesDescription) }}</p>
					</div>
					<img
						class="block h-auto w-full"
						src="/showcase/theme-accent.png"
						:alt="formatMessage(messages.themesScreenshotAlt)"
						width="3104"
						height="1814"
						decoding="async"
						loading="lazy"
					/>
				</article>

				<article
					class="feature gradient-border showcase-card col-span-3 flex min-w-0 flex-col overflow-hidden p-0"
				>
					<div class="showcase-copy px-7 pb-6 pt-7">
						<span>{{ formatMessage(messages.translateLabel) }}</span>
						<h3>{{ formatMessage(messages.translation) }}</h3>
						<p>{{ formatMessage(messages.translationDescription) }}</p>
					</div>
					<img
						class="block h-auto w-full"
						src="/showcase/axolotl-lab.png"
						:alt="formatMessage(messages.translationScreenshotAlt)"
						width="3104"
						height="1814"
						decoding="async"
						loading="lazy"
					/>
				</article>

				<div class="feature gradient-border mods">
					<div class="search-bar">
						<h4>{{ formatMessage(messages.installedMods) }}</h4>
						<div class="mini-input">
							<SearchIcon aria-hidden="true" />
							<div class="search">{{ formatMessage(messages.searchMods) }}</div>
						</div>
					</div>
					<div class="row select-none hover:cursor-default">
						<div />
						<div class="cell">{{ formatMessage(messages.name) }}</div>
						<div class="cell">{{ formatMessage(messages.version) }}</div>
						<div class="cell">{{ formatMessage(messages.actions) }}</div>
					</div>
					<TransitionGroup name="mod-row" tag="div" class="table">
						<div
							v-for="(mod, index) in modManagementData"
							:key="mod.id"
							:class="['row', { first: index === 0 }]"
						>
							<div class="cell">
								<Avatar size="sm" :src="mod.iconUrl" />
							</div>
							<div class="cell">
								<div class="name">{{ mod.name }}</div>
								<div class="description">
									{{ formatMessage(messages.byAuthor, { author: mod.author }) }}
								</div>
							</div>
							<div class="cell">{{ mod.version }}</div>
							<div class="cell check">
								<Checkbox
									v-model="checkedMods[index]"
									:aria-label="`${mod.name} ${formatMessage(messages.installedMods)}`"
								/>
								<ButtonStyled circular type="transparent">
									<button
										:aria-label="`${formatMessage(messages.actions)}: ${mod.name}`"
										@click="removeMod(index)"
									>
										<TrashIcon />
									</button>
								</ButtonStyled>
							</div>
						</div>
					</TransitionGroup>
					<h3>{{ formatMessage(messages.modManagement) }}</h3>
					<p>
						{{ formatMessage(messages.modManagementDescription) }}
					</p>
				</div>
				<div class="feature gradient-border website">
					<ProjectsShowcase />
				</div>
			</div>
			<div class="feature-row">
				<div class="point">
					<div class="title">
						<ImportGradientIcon />
						<h3>{{ formatMessage(messages.profileImporting) }}</h3>
					</div>
					<div class="description">
						{{ formatMessage(messages.profileImportingDescription) }}
					</div>
				</div>
				<div class="point">
					<div class="title">
						<OfflineModeIcon />
						<h3>{{ formatMessage(messages.offlineMode) }}</h3>
					</div>
					<div class="description">
						{{ formatMessage(messages.offlineModeDescription) }}
					</div>
				</div>
				<div class="point">
					<div class="title">
						<MultiplayerIcon />
						<h3>{{ formatMessage(messages.followProjects) }}</h3>
					</div>
					<div class="description">{{ formatMessage(messages.followProjectsDescription) }}</div>
				</div>
			</div>
		</section>
		<section id="faq" class="faq-section" aria-labelledby="faq-title">
			<div class="faq-intro">
				<span class="text-xs font-extrabold uppercase tracking-[0.1em] text-brand">{{
					formatMessage(messages.faqEyebrow)
				}}</span>
				<h2 id="faq-title">{{ formatMessage(messages.faqTitle) }}</h2>
				<p>{{ formatMessage(messages.faqDescription) }}</p>
			</div>
			<div class="faq-list flex flex-col gap-3">
				<details
					v-for="item in faqItems"
					:key="item.question"
					class="faq-item rounded-2xl border border-divider bg-surface-2"
				>
					<summary>{{ item.question }}</summary>
					<p>{{ item.answer }}</p>
				</details>
			</div>
		</section>
		<CommunitySection />
		<div
			id="download"
			ref="downloadSection"
			class="footer relative flex flex-col items-center justify-center gap-6 overflow-hidden bg-[var(--color-accent-contrast)] px-6 py-[clamp(4rem,8vw,7rem)] text-center text-[var(--color-contrast)]"
		>
			<div class="section-badge">{{ formatMessage(messages.downloadOptions) }}</div>
			<div class="section-subheader">
				<div class="section-subheader-title">
					{{ formatMessage(messages.downloadAxolotlTitle) }}
				</div>
				<div class="section-subheader-description">
					{{ formatMessage(messages.downloadDescription) }}
				</div>
			</div>
			<div class="download-section">
				<div class="download-card">
					<div class="title">
						<WindowsLogo />
						{{ formatMessage(messages.windows) }}
					</div>
					<div class="description">
						<a v-if="windowsLink" ref="downloadWindows" :href="windowsLink" download="">
							<DownloadIcon />
							<span>{{ formatMessage(messages.downloadInstaller) }}</span>
						</a>
						<span v-else class="download-unavailable">
							{{ linkUnavailableLabel }}
						</span>
					</div>
				</div>
				<div class="divider" />
				<div class="download-card">
					<div class="title">
						<AppleLogo />
						{{ formatMessage(messages.mac) }}
					</div>
					<div class="description apple">
						<a v-if="macLinks.universal" ref="downloadMac" :href="macLinks.universal" download="">
							<DownloadIcon />
							<span>{{ formatMessage(messages.downloadInstaller) }}</span>
						</a>
						<span v-else class="download-unavailable">
							{{ linkUnavailableLabel }}
						</span>
					</div>
				</div>
				<div class="divider" />
				<div class="download-card">
					<div class="title">
						<LinuxLogo />
						<div class="flex">
							{{ formatMessage(messages.linux) }}<span class="text-sm text-secondary">*</span>
						</div>
					</div>
					<div class="description apple">
						<a v-if="linuxLinks.appImage" :href="linuxLinks.appImage" download="">
							<DownloadIcon />
							<span>{{ formatMessage(messages.downloadAppImage) }}</span>
						</a>
						<span v-else class="download-unavailable">
							{{ linkUnavailableLabel }}
						</span>
						<Accordion
							class="mt-2 flex flex-col items-center"
							content-class="flex flex-col items-start gap-2 mt-2 text-sm"
							button-class="text-sm text-secondary bg-transparent p-0 w-fit text-left m-0 active:scale-[0.98] transition-transform"
						>
							<template #title="{ open }">
								{{ formatMessage(open ? messages.hideOtherPackages : messages.showOtherPackages) }}
							</template>
							<span class="grid grid-cols-[auto_1fr] gap-2 text-left text-orange"
								><IssuesIcon class="mt-1" /> {{ formatMessage(messages.notRecommended) }}</span
							>
							<a v-if="linuxLinks.deb" :href="linuxLinks.deb" download="" class="text-primary">
								<DownloadIcon />
								<span>{{ formatMessage(messages.downloadTheDEB) }}</span>
							</a>
							<span v-else class="download-unavailable text-primary">
								{{ linkUnavailableLabel }}
							</span>
							<a v-if="linuxLinks.rpm" :href="linuxLinks.rpm" download="" class="text-primary">
								<DownloadIcon />
								<span>{{ formatMessage(messages.downloadTheRPM) }}</span>
							</a>
							<span v-else class="download-unavailable text-primary">
								{{ linkUnavailableLabel }}
							</span>
						</Accordion>
					</div>
				</div>
			</div>
			<div v-if="downloadState === 'error'" class="download-error-banner" role="alert">
				<span>{{ formatMessage(messages.downloadLinksFailed) }}</span>
				<span class="download-error-links">
					<a
						href="https://github.com/Mystic-Stars/Axolotl/releases/latest"
						target="_blank"
						rel="noopener"
					>
						{{ formatMessage(messages.manualDownloadFallback) }}
					</a>
				</span>
			</div>
			<p class="terms">
				<IntlFormatted :message-id="messages.downloadTerms">
					<template #terms-link="{ children }">
						<a href="https://www.gnu.org/licenses/gpl-3.0.html" target="_blank" rel="noopener">
							<component :is="() => children" />
						</a>
					</template>
					<template #privacy-link="{ children }">
						<NuxtLink to="/privacy">
							<component :is="() => children" />
						</NuxtLink>
					</template>
				</IntlFormatted>
			</p>
			<p class="max-w-[50rem] text-xs text-secondary">
				*<IntlFormatted :message-id="messages.linuxDisclaimer">
					<template #issues-link="{ children }">
						<a
							class="underline hover:brightness-[--hover-brightness]"
							href="https://github.com/Mystic-Stars/Axolotl/releases/latest"
							target="_blank"
							rel="noopener"
						>
							<component :is="() => children" />
						</a>
					</template>
					<template #prism-link="{ children }">
						<a
							class="underline hover:brightness-[--hover-brightness]"
							href="https://github.com/Mystic-Stars/Axolotl/issues"
							target="_blank"
							rel="noopener"
						>
							<component :is="() => children" />
						</a>
					</template>
				</IntlFormatted>
			</p>
		</div>
	</div>
</template>

<style scoped lang="scss">
.faq-section {
	display: grid;
	grid-template-columns: minmax(0, 0.8fr) minmax(0, 1.2fr);
	gap: 4rem;
	width: min(76rem, calc(100% - 3rem));
	margin: 0 auto;
	padding: 7rem 0;
}

.faq-intro {
	h2 {
		margin: 0.75rem 0 1rem;
		color: var(--color-contrast);
		font-size: clamp(2rem, 4vw, 3.25rem);
		line-height: 1.08;
	}

	p {
		max-width: 32rem;
		margin: 0;
		color: var(--color-secondary);
		font-size: 1.05rem;
		line-height: 1.7;
	}
}

.faq-item {
	summary {
		padding: 1.15rem 1.25rem;
		color: var(--color-contrast);
		font-weight: 700;
		line-height: 1.4;
		cursor: pointer;
	}

	p {
		margin: 0;
		padding: 0 1.25rem 1.25rem;
		color: var(--color-secondary);
		line-height: 1.7;
	}
}

@media (max-width: 800px) {
	.faq-section {
		grid-template-columns: 1fr;
		gap: 2rem;
		width: calc(100% - 2rem);
		padding: 5rem 0;
	}
}

.landing-hero {
	--pointer-x: 50%;
	--pointer-y: 40%;
	position: relative;
	display: flex;
	min-height: min(63rem, calc(100svh + 8rem));
	align-items: center;
	flex-direction: column;
	overflow: hidden;
	padding: clamp(10rem, 12vw, 11.5rem) 1.5rem 0;
	margin-top: -5.25rem;
	background:
		radial-gradient(
			circle at var(--pointer-x) var(--pointer-y),
			rgb(255 155 197 / 22%),
			transparent 20rem
		),
		radial-gradient(circle at 12% 46%, rgb(70 190 176 / 11%), transparent 26rem),
		linear-gradient(155deg, #161018 0%, #11121a 52%, #121520 100%);
	isolation: isolate;

	&::before,
	&::after {
		position: absolute;
		z-index: -1;
		content: '';
		pointer-events: none;
	}

	&::before {
		inset: 0;
		background: linear-gradient(90deg, rgb(255 255 255 / 3%) 1px, transparent 1px);
		background-size: min(9vw, 9rem) 100%;
		mask-image: linear-gradient(180deg, black, transparent 72%);
	}

	&::after {
		inset: 9.25rem 7% auto;
		height: 1px;
		background: linear-gradient(90deg, transparent, rgb(255 170 206 / 38%), transparent);
	}
}

@media (max-width: 1023px) {
	.landing-hero::after {
		inset: 8.5rem 7% auto;
	}
}

.hero-grid {
	position: absolute;
	inset: 0;
	z-index: -1;
	background-image: linear-gradient(rgb(255 255 255 / 3%) 1px, transparent 1px);
	background-size: 100% min(9vw, 9rem);
	mask-image: linear-gradient(180deg, black, transparent 70%);
	pointer-events: none;
}

.hero-content {
	display: flex;
	align-items: center;
	flex-direction: column;
	width: min(100%, 59rem);
	text-align: center;
}

.hero-kicker {
	display: inline-flex;
	align-items: center;
	width: fit-content;
	height: fit-content;
	padding: 0.25rem 0.75rem;
	border-radius: 9999px;
	background: color-mix(in srgb, var(--color-brand) 12%, transparent);
	color: var(--color-brand);
	font-size: 0.875rem;
	font-weight: 700;
	line-height: 1;
	backdrop-filter: blur(12px);
	-webkit-backdrop-filter: blur(12px);
	box-shadow: 0 0.75rem 2.5rem color-mix(in srgb, var(--color-brand) 12%, transparent);
}

.landing-hero .main-subheader {
	max-width: 46rem;
	margin: 0;
	color: var(--landing-color-subheading);
	font-size: clamp(1rem, 1.6vw, 1.25rem);
	font-weight: 450;
	line-height: 1.65;
	text-wrap: balance;
}

.landing-hero .button-group {
	display: flex;
	flex-wrap: wrap;
	justify-content: center;
	gap: 0.75rem;
	margin: 2rem 0 0;
	mask-image: none;

	.hero-download-button {
		&:disabled {
			opacity: 0.65;
			cursor: wait;
		}
	}
}

.hero-product {
	position: relative;
	width: min(79rem, 112%);
	margin-top: clamp(3.25rem, 7vw, 5.5rem);
}

.hero-scroll-mark {
	position: absolute;
	bottom: 2.25rem;
	left: 50%;
	display: flex;
	width: 1px;
	height: 3rem;
	justify-content: flex-start;
	overflow: hidden;
	background: rgb(255 255 255 / 14%);
	transform: translateX(-50%);

	span {
		width: 100%;
		height: 45%;
		background: var(--color-brand);
		animation: scroll-mark 2.3s ease-in-out infinite;
	}
}

@keyframes scroll-mark {
	0%,
	100% {
		transform: translateY(-110%);
	}
	55% {
		transform: translateY(220%);
	}
}

.axolotl-highlights {
	position: relative;
	padding: clamp(5rem, 10vw, 9rem) 1.5rem 3rem;
	background: var(--landing-transition-gradient-end);

	&::before {
		position: absolute;
		top: 0;
		left: 50%;
		width: min(76rem, calc(100% - 3rem));
		height: 1px;
		background: linear-gradient(90deg, transparent, var(--landing-border-color), transparent);
		content: '';
		transform: translateX(-50%);
	}
}

.highlights-intro {
	max-width: 52rem;
	margin: 0 auto clamp(2.75rem, 6vw, 5rem);
	text-align: center;

	h2 {
		margin: 0.7rem 0 1rem;
		color: var(--color-contrast);
		font-size: clamp(2.5rem, 5.5vw, 4.75rem);
		font-weight: 700;
		letter-spacing: 0;
		line-height: 1.04;
	}

	p {
		margin: 0;
		color: var(--color-secondary);
		font-size: 1.05rem;
		line-height: 1.65;
	}
}

.modrinth-feature-grid {
	width: min(100%, 68.5rem);
	margin: 0 auto;
	display: grid;
	grid-template-columns: repeat(6, minmax(0, 1fr));
	gap: 1rem;
}

.modrinth-feature-grid .feature {
	padding: var(--gap-xl);
	z-index: 1;
	background: radial-gradient(
		50% 50% at 50% 50%,
		rgba(44, 48, 79, 0.35) 0%,
		rgba(32, 35, 50, 0.27) 100%
	);
	box-shadow:
		0 1.25rem 3rem rgb(0 0 0 / 12%),
		0 0 4rem rgb(57 61 94 / 20%) inset;
	backdrop-filter: blur(6px);
	-webkit-backdrop-filter: blur(6px);
	overflow: hidden;
}

.promise-card {
	.promise-meta {
		display: flex;
		align-items: center;
		justify-content: space-between;
		color: var(--color-brand);

		svg {
			width: 1.35rem;
			height: 1.35rem;
		}

		span {
			color: var(--color-brand);
			font-size: 0.75rem;
			font-weight: 800;
			letter-spacing: 0.09em;
		}
	}

	h3 {
		margin: 2.2rem 0 0.65rem;
		color: var(--color-contrast);
		font-size: 1.2rem;
		letter-spacing: -0.025em;
	}

	p {
		margin: 0;
		color: var(--color-secondary);
		font-size: 0.9rem;
		line-height: 1.6;
	}

	&::after {
		position: absolute;
		right: -1.75rem;
		bottom: -2.25rem;
		z-index: -1;
		color: rgb(255 255 255 / 4%);
		content: attr(data-number);
		font-size: 8rem;
		font-weight: 800;
		line-height: 1;
	}
}

.showcase-card-wide {
	grid-column: 1 / -1;
	display: grid;
	grid-template-columns: minmax(17rem, 0.78fr) minmax(0, 1.22fr);
	align-items: center;
}

.showcase-copy {
	span {
		color: var(--color-brand);
		font-size: 0.72rem;
		font-weight: 800;
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}

	h3 {
		margin: 0.55rem 0 0.65rem;
		color: var(--color-contrast);
		font-size: clamp(1.35rem, 2.4vw, 1.8rem);
		letter-spacing: -0.035em;
		line-height: 1.1;
	}

	p {
		margin: 0;
		color: var(--color-secondary);
		font-size: 0.9rem;
		line-height: 1.6;
	}
}

:global(html.light-mode) .axolotl-highlights {
	background: #f8f7f8;
}

:global(html.light-mode) .promise-card::after {
	color: rgb(0 0 0 / 5%);
}

:global(html.light-mode) .mods .row:not(.header):hover {
	background: rgb(0 0 0 / 4%);
}

.mods,
.website {
	grid-column: span 3 / span 3;

	h3,
	p {
		margin: 0;
	}

	h3 {
		font-weight: 500;
		font-size: var(--font-size-xl);
		color: var(--landing-color-heading);
		margin-bottom: 0.375rem;
	}

	p {
		font-size: var(--font-size-md);
		color: var(--landing-color-subheading);
	}
}

.mods {
	.table {
		margin-bottom: 1rem;
		overflow: hidden;
		max-height: 32rem;
	}

	h3,
	p {
		text-align: center;
	}

	h4 {
		margin: 0;
		color: var(--color-contrast);
	}

	.search-bar {
		width: 100%;
		padding: var(--gap-sm);
		display: flex;
		flex-direction: row;
		justify-content: space-between;
		align-items: center;
		border-radius: var(--radius-md);
		border: 1px solid var(--landing-border-color);
		background: linear-gradient(0deg, #3b3f55 0%, #3b3f55 100%), rgba(59, 63, 85, 0.15);
		margin-bottom: 0.5rem;
		white-space: nowrap;
		font-size: var(--font-size-sm);

		.mini-input {
			display: flex;
			flex-direction: row;
			align-items: center;
			gap: 0.5rem;
			padding: var(--gap-sm) var(--gap-md);
			border-radius: var(--radius-sm);
			background-color: #1e202f;
			flex-grow: 1;
			max-width: 12rem;
		}

		h4 {
			font-weight: normal;
			margin-left: 0.5rem;
		}
	}

	.row {
		display: grid;
		grid-template-columns: 3rem 2fr 1fr 3.75rem;
		padding: 0 var(--gap-sm);
		gap: 1rem;

		&:not(.header):hover {
			background: rgb(255 255 255 / 5%);
		}

		.cell {
			display: flex;
			flex-direction: column;
			justify-content: center;
			padding: var(--gap-sm) 0;
			font-size: var(--font-size-sm);

			.name {
				color: var(--color-contrast);
			}

			.description {
				font-size: var(--font-size-xs);
			}

			&.last {
				align-items: flex-end;
			}

			&.check {
				align-items: center;
				flex-direction: row;
			}
		}
	}

	.header {
		.cell {
			color: var(--color-base);
		}
	}
}

.website {
	text-align: center;
	padding: 0 !important;

	position: relative;
}

.feature-row {
	display: grid;
	grid-template-columns: repeat(3, 1fr);
	gap: var(--gap-lg);
	max-width: 1096px;
	margin: 0 auto;
	padding: calc(var(--gap-xl) * 2) 1rem;

	@media (max-width: 1024px) {
		grid-template-columns: repeat(1, 1fr);

		.point {
			text-align: center;

			.title {
				justify-content: center;
			}
		}
	}

	.point {
		display: flex;
		flex-direction: column;
		gap: var(--gap-md);
		padding: 1rem 0;

		.title {
			display: flex;
			align-items: center;
			gap: 0.5rem;
		}

		h3 {
			font-size: var(--font-size-lg);
			font-weight: normal;
			color: var(--landing-color-heading);
			margin: 0;
		}

		p {
			color: var(--landing-color-subheading);
			margin: 0;
		}

		a {
			text-decoration: underline;
		}
	}
}

.table {
	display: grid;
	border: 1px solid rgba(#a8b1ddbf, 0.25);
	gap: 0.25rem;
	overflow: hidden;
	font-size: var(--font-size-sm);
	background: rgba(59, 63, 85, 0.15);
	box-shadow: 2px 2px 12px 0px rgba(0, 0, 0, 0.16);

	.first {
		border-top: none !important;
	}

	.row {
		&:not(.header) {
			border-top: 1px solid rgba(#a8b1ddbf, 0.25);
		}
	}
}

.mod-row-leave-active {
	transition:
		opacity 0.2s ease,
		transform 0.2s ease;
}

.mod-row-leave-to {
	opacity: 0;
	transform: translateX(-0.5rem);
}

// 表格是真实文本（模组名、版本），允许选中复制；projects 滚动区保持禁选
.table .row {
	user-select: text;
}

.footer {
	&::before {
		position: absolute;
		top: 0;
		left: 50%;
		width: min(50rem, 90%);
		height: 1px;
		background: linear-gradient(90deg, transparent, var(--color-brand), transparent);
		content: '';
		transform: translateX(-50%);
	}

	.section-badge {
		border: 1px solid color-mix(in srgb, var(--color-brand) 40%, transparent);
		background-color: var(--color-brand-highlight);
		color: var(--color-brand);
		border-radius: 0;
		width: min-content;
		padding: var(--gap-lg) var(--gap-xl);
		white-space: nowrap;
	}

	.section-subheader {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--gap-sm);
		font-size: clamp(1.05rem, 1.5vw, 1.25rem);
		margin: 2rem 0;

		.section-subheader-title {
			font-size: clamp(2rem, 4vw, 3.75rem);
			font-weight: 700;
			letter-spacing: 0;
			line-height: 1;
			margin: 0;
		}

		.section-subheader-description {
			color: var(--color-base);
			margin: 0;
		}
	}

	.download-section {
		display: grid;
		grid-template-columns: 1fr 1px 1fr 1px 1fr;
		height: 100%;
		gap: var(--gap-lg);
		max-width: 1096px;
		margin: 0 auto;

		@media (max-width: 1024px) {
			grid-template-columns: repeat(1, 1fr);
			max-width: 340px;

			.divider {
				display: none;
			}
		}

		.divider {
			height: 13rem;
			width: 1px;
			background: var(--landing-border-color);
			margin: 0;
		}

		.download-card {
			display: flex;
			flex-direction: column;
			gap: calc(var(--gap-lg) * 2);
			padding: calc(var(--gap-lg) * 2);
			height: min-content;

			.title {
				display: flex;
				flex-direction: column;
				align-items: center;
				justify-content: center;
				font-size: var(--font-size-2xl);
				gap: var(--gap-sm);
				border-radius: var(--radius-md) var(--radius-md) 0 0;
				color: var(--color-contrast);
			}

			.description {
				display: flex;
				flex-direction: column;
				align-items: center;
				border-top: none;
				font-size: var(--font-size-md);
				color: var(--color-brand);
				gap: var(--gap-sm);

				a {
					display: flex;
					align-items: center;
					gap: var(--gap-sm);
					justify-content: center;

					&:hover {
						cursor: pointer;
					}

					span {
						text-align: left;
					}
				}

				.download-unavailable {
					display: inline-flex;
					align-items: center;
					justify-content: center;
					gap: var(--gap-sm);
					min-height: 2.25rem;
					color: var(--color-secondary);
					font-size: var(--font-size-sm);
					text-align: center;
					cursor: default;
				}
			}

			:deep(.animated-dropdown) {
				color: var(--color-brand);
				width: 16rem;
				white-space: nowrap;

				.selected {
					border: 1px solid var(--color-brand);
					background-color: var(--color-accent-contrast);
				}

				.options {
					border: 1px solid var(--color-brand);
					border-radius: 0 0 var(--radius-md) var(--radius-md);
				}

				.option {
					background-color: var(--color-accent-contrast);
				}

				.selected-option {
					background-color: var(--color-brand);
				}
			}
		}
	}

	.download-error-banner {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-wrap: wrap;
		gap: 0.5rem 1rem;
		width: min(100%, 68.5rem);
		margin: 0 auto;
		padding: 0.75rem 1.25rem;
		border: 1px solid color-mix(in srgb, var(--color-brand) 40%, transparent);
		border-radius: var(--radius-md);
		background: var(--color-brand-highlight);
		color: var(--color-contrast);
		font-size: var(--font-size-sm);
		text-align: center;

		.download-error-links {
			display: inline-flex;
			align-items: center;
			flex-wrap: wrap;
			justify-content: center;
			gap: 0.5rem 1rem;

			a {
				color: var(--color-brand);
				font-weight: 700;
				text-decoration: underline;
				text-underline-offset: 0.15rem;
			}

			a + a::before {
				content: '·';
				margin-right: 1rem;
				color: var(--color-secondary);
				font-weight: 400;
			}
		}
	}

	.terms {
		margin: var(--gap-xl);
		font-size: var(--font-size-lg);
		color: var(--landing-color-subheading);
		text-align: center;
		line-height: 1.5;

		a {
			text-decoration: underline;
		}
	}
}

.gradient-border {
	position: relative;
	border-radius: var(--radius-lg);

	&:before {
		content: '';
		position: absolute;
		inset: 0;
		padding: 1px;
		z-index: -1;
		border-radius: 1rem;
		background: var(--landing-border-gradient);

		-webkit-mask:
			linear-gradient(#fff 0 0) content-box,
			linear-gradient(#fff 0 0);
		mask:
			linear-gradient(#fff 0 0) content-box,
			linear-gradient(#fff 0 0);
		-webkit-mask-composite: xor;
		mask-composite: exclude;
	}
}

.bottom-transition {
	position: absolute;
	bottom: 0;
	width: 100%;
	height: 30rem;
	background: linear-gradient(
		0deg,
		var(--landing-transition-gradient-end) 0%,
		var(--landing-transition-gradient-start) 100%
	);
}

@media screen and (max-width: 1024px) {
	.mods,
	.website {
		grid-column: 1 / -1 !important;
	}

	.main-header {
		font-size: 4rem !important;
	}

	.main-subheader {
		font-size: 1.25rem !important;
	}
}

@media screen and (max-width: 746px) {
	.axolotl-highlights {
		padding: 0 1rem 1rem;
	}

	.highlights-intro {
		margin-bottom: 2rem;

		p {
			font-size: 0.95rem;
		}
	}

	.modrinth-feature-grid {
		grid-template-columns: 1fr;
	}

	.promise-card,
	.showcase-card,
	.mods,
	.website {
		grid-column: auto;
	}

	.promise-card {
		min-height: auto;
		padding: 1.25rem;

		h3 {
			margin-top: 1.5rem;
		}
	}

	.showcase-card-wide {
		grid-column: auto;
		grid-template-columns: 1fr;
	}

	.showcase-copy {
		padding: 1.35rem 1.25rem 1.15rem;
	}

	.main-header {
		font-size: 3rem !important;
	}

	.main-subheader {
		font-size: 1.1rem !important;
	}
}

.light-mode {
	.footer {
		background: #f8f7f8;
	}

	.bottom-transition {
		background: linear-gradient(rgba(#f8f7f8, 0) 0%, #f8f7f8 100%);
	}

	.feature {
		background: radial-gradient(
			50% 50% at 50% 50%,
			rgba(255, 255, 255, 0.35) 0%,
			rgba(255, 255, 255, 0.27) 100%
		) !important;
		box-shadow:
			2px 2px 64px 0px rgba(255, 255, 255, 0.45) inset,
			2px 2px 12px 0px rgba(0, 0, 0, 0.16) !important;
		border: none !important;
	}

	.gradient-border {
		&:before {
			background: var(--landing-border-gradient-light);
		}
	}

	.search-bar {
		background: var(--color-raised-bg) !important;
		border: 2px solid var(--color-brand) !important;

		.mini-input {
			background: var(--color-raised-bg) !important;
			border: 2px solid var(--color-bg);
		}
	}

	.landing-hero {
		background:
			radial-gradient(circle at 18% 20%, rgb(239 126 170 / 20%), transparent 28rem),
			radial-gradient(circle at 82% 36%, rgb(142 119 230 / 11%), transparent 32rem),
			linear-gradient(180deg, #fff9fc 0%, #faf6fa 58%, #f8f4f7 100%);

		.hero-grid {
			background-image: linear-gradient(rgb(105 73 88 / 5%) 1px, transparent 1px);
		}

		&::after {
			background: linear-gradient(90deg, transparent, rgb(199 47 108 / 22%), transparent);
		}
	}

	.table {
		background: white;
	}
}
</style>
