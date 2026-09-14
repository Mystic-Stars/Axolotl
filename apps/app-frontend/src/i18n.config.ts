import { buildLocaleMessages, createMessageCompiler, type CrowdinMessages } from '@modrinth/ui'
import { uiLocaleModulesEager } from '@modrinth/ui/src/locales.eager.ts'
import { createI18n } from 'vue-i18n'

const localeModules = import.meta.glob<{ default: CrowdinMessages }>('./locales/*/index.json', {
	eager: true,
})

const i18n = createI18n({
	legacy: false,
	locale: 'en-US',
	fallbackLocale: 'en-US',
	messageCompiler: createMessageCompiler(),
	missingWarn: false,
	fallbackWarn: false,
	messages: buildLocaleMessages(localeModules, uiLocaleModulesEager),
})

export function resolveInitialLocale(preferredLocales: readonly string[]): string {
	const availableLocales = new Set(i18n.global.availableLocales)

	for (const preferredLocale of preferredLocales) {
		const normalizedLocale = preferredLocale.replace('_', '-')
		if (availableLocales.has(normalizedLocale)) return normalizedLocale

		const language = normalizedLocale.split('-')[0].toLowerCase()
		if (language === 'zh') {
			const traditionalChinese = /-(tw|hk|mo)|-hant/i.test(normalizedLocale)
			return traditionalChinese ? 'zh-TW' : 'zh-CN'
		}

		const languageMatch = i18n.global.availableLocales.find((locale) =>
			locale.toLowerCase().startsWith(`${language}-`),
		)
		if (languageMatch) return languageMatch
	}

	return 'en-US'
}

export const SYSTEM_LOCALE_VALUE = 'system'
const FOLLOW_SYSTEM_LOCALE_KEY = 'axolotl.follow-system-locale'

function preferredNavigatorLocales(): readonly string[] {
	return navigator.languages?.length ? navigator.languages : [navigator.language ?? 'en-US']
}

export function getSystemResolvedLocale(): string {
	return resolveInitialLocale(preferredNavigatorLocales())
}

export function isFollowingSystemLocale(storedLocale: string | null | undefined): boolean {
	if (!storedLocale || storedLocale === SYSTEM_LOCALE_VALUE) return true
	try {
		return localStorage.getItem(FOLLOW_SYSTEM_LOCALE_KEY) === '1'
	} catch {
		return false
	}
}

export function setFollowSystemLocale(follow: boolean) {
	try {
		if (follow) localStorage.setItem(FOLLOW_SYSTEM_LOCALE_KEY, '1')
		else localStorage.removeItem(FOLLOW_SYSTEM_LOCALE_KEY)
	} catch {
		// localStorage may be unavailable; system-follow then relies on empty/'system' locale only.
	}
}

/** Applies the stored locale preference and returns the concrete locale used. */
export function applyLocalePreference(storedLocale: string | null | undefined): string {
	if (isFollowingSystemLocale(storedLocale)) {
		const resolved = getSystemResolvedLocale()
		i18n.global.locale.value = resolved
		return resolved
	}

	const locale = storedLocale && storedLocale !== SYSTEM_LOCALE_VALUE ? storedLocale : 'en-US'
	i18n.global.locale.value = locale
	return locale
}

// The locale the user picked lives in the app database, which is exactly what is
// unreadable when startup fails - and a startup failure is when the remaining
// dialogs matter most. Starting from the system language keeps them in a
// language the user reads; `setupApp` still applies the saved locale once the
// database is available.
i18n.global.locale.value = getSystemResolvedLocale()

export default i18n
