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
let useModrinthMirror = false

export function setModrinthMirrorEnabled(enabled: boolean) {
	useModrinthMirror = enabled
}

export function getOfficialLabrinthBaseUrl() {
	return officialLabrinthBaseUrl
}

export function getLabrinthBaseUrl() {
	return useModrinthMirror ? MODRINTH_MIRROR_BASE_URL : officialLabrinthBaseUrl
}

export const config = {
	siteUrl,
	stripePublishableKey:
		import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY ||
		'pk_test_51JbFxJJygY5LJFfKV50mnXzz3YLvBVe2Gd1jn7ljWAkaBlRz3VQdxN9mXcPSrFbSqxwAb0svte9yhnsmm7qHfcWn00R611Ce7b',
	labrinthBaseUrl: getLabrinthBaseUrl,
}
