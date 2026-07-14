<script setup lang="ts">
import { CheckIcon, KeyIcon, PlugIcon, TrashIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	defineMessages,
	injectNotificationManager,
	LOCALES,
	StyledInput,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, onUnmounted, ref, watch } from 'vue'

import {
	clearTranslationCache,
	getTranslationSettings,
	setTranslationSecret,
	testTranslationProvider,
	type TranslationProvider,
	type TranslationStyle,
	updateTranslationSettings,
} from '@/helpers/translation'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const settings = ref(await getTranslationSettings())
const openaiSecret = ref('')
const status = ref('')
const testing = ref(false)
let saveTimer: ReturnType<typeof setTimeout> | undefined

const messages = defineMessages({
	title: { id: 'app.translation-settings.title', defaultMessage: 'Translation' },
	description: {
		id: 'app.translation-settings.description',
		defaultMessage:
			'Translate Modrinth project titles, summaries, and descriptions while browsing content.',
	},
	provider: { id: 'app.translation-settings.provider', defaultMessage: 'Translation service' },
	microsoft: {
		id: 'app.translation-settings.provider.microsoft',
		defaultMessage: 'Microsoft Translate (free)',
	},
	google: {
		id: 'app.translation-settings.provider.google',
		defaultMessage: 'Google Translate (free)',
	},
	openai: {
		id: 'app.translation-settings.provider.openai-compatible',
		defaultMessage: 'OpenAI compatible',
	},
	targetLanguage: {
		id: 'app.translation-settings.target-language',
		defaultMessage: 'Target language',
	},
	followApp: {
		id: 'app.translation-settings.target-language.follow-app',
		defaultMessage: 'Follow launcher language',
	},
	displayMode: {
		id: 'app.translation-settings.display-mode',
		defaultMessage: 'Display mode',
	},
	bilingual: {
		id: 'app.translation-settings.display-mode.bilingual',
		defaultMessage: 'Original and translation',
	},
	translationOnly: {
		id: 'app.translation-settings.display-mode.translation-only',
		defaultMessage: 'Translation only',
	},
	autoTranslate: {
		id: 'app.translation-settings.auto-translate',
		defaultMessage: 'Translate project pages automatically',
	},
	autoTranslateDescription: {
		id: 'app.translation-settings.auto-translate-description',
		defaultMessage: 'Start translating as soon as a Modrinth project page is opened.',
	},
	style: { id: 'app.translation-settings.style', defaultMessage: 'Translation style' },
	styleDefault: { id: 'app.translation-settings.style.default', defaultMessage: 'Default' },
	styleWeakened: { id: 'app.translation-settings.style.weakened', defaultMessage: 'Muted' },
	styleBrand: { id: 'app.translation-settings.style.brand', defaultMessage: 'Accent color' },
	styleBorder: { id: 'app.translation-settings.style.border', defaultMessage: 'Left border' },
	styleBackground: {
		id: 'app.translation-settings.style.background',
		defaultMessage: 'Background',
	},
	stylePreview: {
		id: 'app.translation-settings.style.preview',
		defaultMessage: 'Preview',
	},
	stylePreviewOriginalText: {
		id: 'app.translation-settings.style.preview-original-text',
		defaultMessage: 'Explore high-quality Minecraft content on Modrinth.',
	},
	stylePreviewText: {
		id: 'app.translation-settings.style.preview-text',
		defaultMessage: 'Discover high-quality Minecraft content on Modrinth.',
	},
	openaiConfiguration: {
		id: 'app.translation-settings.openai.configuration',
		defaultMessage: 'OpenAI-compatible configuration',
	},
	baseUrl: { id: 'app.translation-settings.base-url', defaultMessage: 'Base URL' },
	model: { id: 'app.translation-settings.model', defaultMessage: 'Model' },
	apiKey: { id: 'app.translation-settings.api-key', defaultMessage: 'API key' },
	apiKeyConfigured: {
		id: 'app.translation-settings.api-key-configured',
		defaultMessage: 'An API key is already configured. Enter a new value to replace it.',
	},
	apiKeyOptional: {
		id: 'app.translation-settings.api-key-optional',
		defaultMessage: 'Optional for local or unauthenticated endpoints.',
	},
	saveKey: { id: 'app.translation-settings.save-key', defaultMessage: 'Save API key' },
	clearKey: { id: 'app.translation-settings.clear-key', defaultMessage: 'Clear API key' },
	test: { id: 'app.translation-settings.test', defaultMessage: 'Test service' },
	testing: { id: 'app.translation-settings.testing', defaultMessage: 'Testing…' },
	testSuccess: {
		id: 'app.translation-settings.test-success',
		defaultMessage: 'Connection succeeded: {translation}',
	},
	cache: { id: 'app.translation-settings.cache', defaultMessage: 'Translation cache' },
	cacheDescription: {
		id: 'app.translation-settings.cache-description',
		defaultMessage: 'Successful translations are cached for seven days to reduce requests.',
	},
	clearCache: {
		id: 'app.translation-settings.clear-cache',
		defaultMessage: 'Clear translation cache',
	},
	cacheCleared: {
		id: 'app.translation-settings.cache-cleared',
		defaultMessage: 'Translation cache cleared.',
	},
	operationFailed: {
		id: 'app.translation-settings.operation-failed',
		defaultMessage: 'The translation operation failed. Check the configuration and try again.',
	},
})

const providers: TranslationProvider[] = ['microsoft', 'google', 'openai-compatible']
const modes = ['bilingual', 'translation-only'] as const
const styles: TranslationStyle[] = ['default', 'weakened', 'brand', 'border', 'background']
const languages = ['follow-app', ...LOCALES.map((locale) => locale.code)]

const targetLanguage = computed({
	get: () => settings.value.target_language || 'follow-app',
	set: (value: string) => {
		settings.value.target_language = value === 'follow-app' ? '' : value
	},
})

function providerName(provider: TranslationProvider) {
	return formatMessage(
		{
			microsoft: messages.microsoft,
			google: messages.google,
			'openai-compatible': messages.openai,
		}[provider],
	)
}

function languageName(code: string) {
	if (code === 'follow-app') return formatMessage(messages.followApp)
	const locale = LOCALES.find((locale) => locale.code === code)
	return locale ? `${locale.name} — ${formatMessage(locale.translatedName)}` : code
}

function modeName(mode: string) {
	return formatMessage(mode === 'bilingual' ? messages.bilingual : messages.translationOnly)
}

function styleName(style: TranslationStyle) {
	return formatMessage(
		{
			default: messages.styleDefault,
			weakened: messages.styleWeakened,
			brand: messages.styleBrand,
			border: messages.styleBorder,
			background: messages.styleBackground,
		}[style],
	)
}

const providerOptions = computed(() =>
	providers.map((provider) => ({ value: provider, label: providerName(provider) })),
)
const languageOptions = computed(() =>
	languages.map((language) => ({ value: language, label: languageName(language) })),
)
const modeOptions = computed(() => modes.map((mode) => ({ value: mode, label: modeName(mode) })))
const styleOptions = computed(() =>
	styles.map((style) => ({ value: style, label: styleName(style) })),
)
const stylePreviewClass = computed(() => `translation-style-preview-${settings.value.style}`)

function reportOperationError() {
	handleError(formatMessage(messages.operationFailed))
}

watch(
	settings,
	() => {
		clearTimeout(saveTimer)
		saveTimer = setTimeout(
			() => void updateTranslationSettings(settings.value).catch(reportOperationError),
			250,
		)
	},
	{ deep: true },
)

onUnmounted(() => clearTimeout(saveTimer))

async function saveSecret() {
	try {
		await setTranslationSecret('openai-compatible', openaiSecret.value)
		settings.value.openai_has_api_key = !!openaiSecret.value.trim()
		openaiSecret.value = ''
		return true
	} catch {
		reportOperationError()
		return false
	}
}

async function clearSecret() {
	try {
		await setTranslationSecret('openai-compatible', null)
		settings.value.openai_has_api_key = false
	} catch {
		reportOperationError()
	}
}

async function testProvider() {
	testing.value = true
	status.value = ''
	try {
		await updateTranslationSettings(settings.value)
		if (settings.value.provider === 'openai-compatible' && openaiSecret.value) {
			if (!(await saveSecret())) return
		}
		const result = await testTranslationProvider(settings.value.provider)
		status.value = formatMessage(messages.testSuccess, { translation: result })
	} catch {
		reportOperationError()
	} finally {
		testing.value = false
	}
}

async function clearCache() {
	try {
		await clearTranslationCache()
		status.value = formatMessage(messages.cacheCleared)
	} catch {
		reportOperationError()
	}
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
			<p class="m-0 mt-2 text-secondary">{{ formatMessage(messages.description) }}</p>
		</div>

		<div class="grid grid-cols-1 gap-5 md:grid-cols-2">
			<div class="flex flex-col gap-2 font-semibold text-contrast">
				<span>{{ formatMessage(messages.provider) }}</span>
				<Combobox
					v-model="settings.provider"
					:options="providerOptions"
				/>
			</div>
			<div class="flex flex-col gap-2 font-semibold text-contrast">
				<span>{{ formatMessage(messages.targetLanguage) }}</span>
				<Combobox
					v-model="targetLanguage"
					:options="languageOptions"
					searchable
				/>
			</div>
			<div class="flex flex-col gap-2 font-semibold text-contrast">
				<span>{{ formatMessage(messages.displayMode) }}</span>
				<Combobox
					v-model="settings.mode"
					:options="modeOptions"
				/>
			</div>
			<div class="flex flex-col gap-2 font-semibold text-contrast">
				<span>{{ formatMessage(messages.style) }}</span>
				<Combobox
					v-model="settings.style"
					:options="styleOptions"
				/>
			</div>
		</div>

		<div class="flex w-full flex-col gap-2 font-semibold text-contrast">
			<span>{{ formatMessage(messages.stylePreview) }}</span>
			<div class="translation-style-preview-container">
				<p
					v-if="settings.mode === 'bilingual'"
					class="translation-style-preview-original m-0"
				>
					{{ formatMessage(messages.stylePreviewOriginalText) }}
				</p>
				<p class="translation-style-preview m-0" :class="stylePreviewClass">
					{{ formatMessage(messages.stylePreviewText) }}
				</p>
			</div>
		</div>

		<div class="flex items-center justify-between gap-4">
			<div>
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.autoTranslate) }}
				</h3>
				<p class="m-0 mt-1 text-sm text-secondary">
					{{ formatMessage(messages.autoTranslateDescription) }}
				</p>
			</div>
			<Toggle id="translation-auto" v-model="settings.auto_translate" />
		</div>

		<div v-if="settings.provider === 'openai-compatible'" class="flex flex-col gap-3">
			<h3 class="m-0 text-base font-semibold text-contrast">
				{{ formatMessage(messages.openaiConfiguration) }}
			</h3>
			<label class="flex flex-col gap-1.5 text-sm font-semibold">
				{{ formatMessage(messages.baseUrl) }}
				<StyledInput v-model="settings.openai_base_url" type="url" wrapper-class="w-full" />
			</label>
			<label class="flex flex-col gap-1.5 text-sm font-semibold">
				{{ formatMessage(messages.model) }}
				<StyledInput v-model="settings.openai_model" wrapper-class="w-full" />
			</label>
			<label class="flex flex-col gap-1.5 text-sm font-semibold">
				{{ formatMessage(messages.apiKey) }}
				<StyledInput
					v-model="openaiSecret"
					:icon="KeyIcon"
					type="password"
					autocomplete="off"
					wrapper-class="w-full"
				/>
			</label>
			<p class="m-0 text-sm text-secondary">
				{{
					formatMessage(
						settings.openai_has_api_key ? messages.apiKeyConfigured : messages.apiKeyOptional,
					)
				}}
			</p>
			<div class="flex flex-wrap gap-2">
				<ButtonStyled>
					<button @click="saveSecret">
						<CheckIcon />{{ formatMessage(messages.saveKey) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="settings.openai_has_api_key" color="red">
					<button @click="clearSecret">
						<TrashIcon />{{ formatMessage(messages.clearKey) }}
					</button>
				</ButtonStyled>
			</div>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<ButtonStyled color="brand">
				<button :disabled="testing" @click="testProvider">
					<PlugIcon />{{ formatMessage(testing ? messages.testing : messages.test) }}
				</button>
			</ButtonStyled>
			<span v-if="status" class="text-sm text-secondary">{{ status }}</span>
		</div>

		<div class="flex flex-col gap-2">
			<h3 class="m-0 text-base font-semibold text-contrast">{{ formatMessage(messages.cache) }}</h3>
			<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.cacheDescription) }}</p>
			<ButtonStyled>
				<button @click="clearCache"><TrashIcon />{{ formatMessage(messages.clearCache) }}</button>
			</ButtonStyled>
		</div>
	</div>
</template>

<style scoped>
.translation-style-preview-container {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	width: 100%;
	min-height: 6.5rem;
	box-sizing: border-box;
	padding: 1rem;
	border: 1px solid var(--color-surface-5);
	border-radius: var(--radius-lg);
}

.translation-style-preview-original,
.translation-style-preview {
	font-weight: 400;
}

.translation-style-preview-original {
	color: var(--color-text-primary);
}

.translation-style-preview-default {
	color: var(--color-text-primary);
}

.translation-style-preview-weakened {
	color: var(--color-secondary);
}

.translation-style-preview-brand {
	color: var(--color-brand);
}

.translation-style-preview-border {
	padding-left: 0.875rem;
	border-left: 3px solid var(--color-brand);
}

.translation-style-preview-background {
	padding: 0.75rem 1rem;
	border-radius: var(--radius-lg);
	background: var(--color-button-bg);
}
</style>
