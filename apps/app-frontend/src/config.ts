const trimTrailingSlash = (url: string) => url.replace(/\/$/, '')

export const AxolotlBrandConfig = Object.freeze({
	productName: 'Axolotl Launcher',
	shortProductName: 'Axolotl',
	organizationName: 'Garbage Human Studio',
	shortOrganizationName: 'GHS',
	developerName: 'Mystic Stars',
	website: 'https://www.axlmc.org/',
	sourceUrl: 'https://www.ghs.red',
	supportUrl: 'https://github.com/Mystic-Stars/Axolotl/issues',
	qqGroupNumber: '955605306',
	sponsorUrl: 'https://afdian.com/a/Mystic-Stars',
	bundleIdentifier: 'red.ghs.axolotl',
	deepLinkScheme: 'axolotl',
	userAgent: (version: string, os: string) => `garbage-human-studio/axolotl/${version} (${os})`,
	capabilities: Object.freeze({
		publicModrinthApi: true,
		privateModrinthServices: false,
		ghsTelemetry: false,
	}),
})

const siteUrl = trimTrailingSlash(import.meta.env.MODRINTH_URL || 'https://modrinth.com')
const officialLabrinthBaseUrl = trimTrailingSlash(
	import.meta.env.MODRINTH_API_BASE_URL || 'https://api.modrinth.com',
)
export const MODRINTH_MIRROR_BASE_URL = 'https://mod.mcimirror.top/modrinth'
type DownloadSourceMode = 'auto' | 'official_only' | 'mirror_preferred'

let modrinthSourceMode: DownloadSourceMode = 'auto'

function autoPrefersMirror() {
	if (typeof navigator === 'undefined') return false

	const languages = [...(navigator.languages ?? []), navigator.language]
	const usesMainlandChinese = languages.some((language) => {
		const normalized = language.toLowerCase().replace('_', '-')
		return normalized.startsWith('zh-cn') || normalized.startsWith('zh-hans')
	})
	const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone?.toLowerCase()
	const usesMainlandTimeZone = [
		'asia/shanghai',
		'asia/chongqing',
		'asia/harbin',
		'asia/urumqi',
	].includes(timeZone ?? '')

	return usesMainlandTimeZone || (!timeZone && usesMainlandChinese)
}

export function setModrinthSourceMode(sourceMode: DownloadSourceMode) {
	modrinthSourceMode = sourceMode
}

export function setModrinthMirrorEnabled(enabled: boolean) {
	setModrinthSourceMode(enabled ? 'mirror_preferred' : 'official_only')
}

export function getOfficialLabrinthBaseUrl() {
	return officialLabrinthBaseUrl
}

export function getLabrinthBaseUrl() {
	const useMirror =
		modrinthSourceMode === 'mirror_preferred' ||
		(modrinthSourceMode === 'auto' && autoPrefersMirror())
	return useMirror ? MODRINTH_MIRROR_BASE_URL : officialLabrinthBaseUrl
}

export const config = {
	siteUrl,
	stripePublishableKey:
		import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY ||
		'pk_test_51JbFxJJygY5LJFfKV50mnXzz3YLvBVe2Gd1jn7ljWAkaBlRz3VQdxN9mXcPSrFbSqxwAb0svte9yhnsmm7qHfcWn00R611Ce7b',
	labrinthBaseUrl: getLabrinthBaseUrl,
}
