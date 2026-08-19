<script setup lang="ts">
import { PlugIcon, SpinnerIcon, TrashIcon } from '@modrinth/assets'
import {
	Combobox,
	defineMessages,
	injectNotificationManager,
	LOCALES,
	NewButton as Button,
	StyledInput,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import { type AIProviderDefinition, getAICatalog, getAIState, sharedAIState } from '@/helpers/ai'
import {
	clearTranslationCache,
	getGoogleIpPoolSize,
	getTranslationErrorKind,
	getTranslationSettings,
	testTranslationProvider,
	type TranslationProvider,
	type TranslationSettings as TranslationSettingsState,
	type TranslationStyle,
	updateTranslationSettings,
} from '@/helpers/translation'

import AIIcon from './AIIcon.vue'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const settings = ref<TranslationSettingsState>({
	provider: 'google',
	target_language: '',
	mode: 'bilingual',
	auto_translate: false,
	style: 'weakened',
	ai_provider_id: '',
	ai_model_id: '',
	ai_system_prompt: '',
})
const aiCatalog = ref<AIProviderDefinition[]>([])
const loading = ref(true)
const status = ref('')
const testing = ref(false)
const googleIpPoolSize = ref(0)
let saveTimer: ReturnType<typeof setTimeout> | undefined
let poolTimer: ReturnType<typeof setInterval> | undefined

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
		defaultMessage: 'Microsoft Translate (unavailable)',
	},
	google: {
		id: 'app.translation-settings.provider.google',
		defaultMessage: 'Google Translate (free)',
	},
	googleIpPool: {
		id: 'app.translation-settings.google-ip-pool',
		defaultMessage: 'IP pool {count}',
	},
	ai: { id: 'app.translation-settings.provider.ai', defaultMessage: 'AI model' },
	aiProvider: { id: 'app.translation-settings.ai-provider', defaultMessage: 'AI provider' },
	aiModel: { id: 'app.translation-settings.ai-model', defaultMessage: 'Text model' },
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
	styleBlur: { id: 'app.translation-settings.style.blur', defaultMessage: 'Blur' },
	styleBlockquote: {
		id: 'app.translation-settings.style.blockquote',
		defaultMessage: 'Block quote',
	},
	styleWeakened: { id: 'app.translation-settings.style.weakened', defaultMessage: 'Muted' },
	styleDashedLine: {
		id: 'app.translation-settings.style.dashed-line',
		defaultMessage: 'Dashed underline',
	},
	styleBorder: { id: 'app.translation-settings.style.border', defaultMessage: 'Border' },
	styleTextColor: {
		id: 'app.translation-settings.style.text-color',
		defaultMessage: 'Text color',
	},
	styleBackground: {
		id: 'app.translation-settings.style.background',
		defaultMessage: 'Background',
	},
	stylePreview: { id: 'app.translation-settings.style.preview', defaultMessage: 'Preview' },
	stylePreviewOriginalText: {
		id: 'app.translation-settings.style.preview-original-text',
		defaultMessage: 'Explore high-quality Minecraft content on Modrinth.',
	},
	stylePreviewText: {
		id: 'app.translation-settings.style.preview-text',
		defaultMessage: 'Discover high-quality Minecraft content on Modrinth.',
	},
	systemPrompt: {
		id: 'app.translation-settings.system-prompt',
		defaultMessage: 'Translation instructions',
	},
	systemPromptDescription: {
		id: 'app.translation-settings.system-prompt-description',
		defaultMessage:
			'Optional feature-specific instructions. The launcher always appends its structured translation contract.',
	},
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
	rateLimited: {
		id: 'app.translation.error.rate-limited',
		defaultMessage: 'The translation service is temporarily rate limited. Please try again later.',
	},
	authenticationFailed: {
		id: 'app.translation.error.authentication',
		defaultMessage: 'The translation service could not authenticate. Please try again later.',
	},
	contentTooLong: {
		id: 'app.translation.error.content-too-long',
		defaultMessage: 'This content is too long for the selected translation service.',
	},
	networkFailed: {
		id: 'app.translation.error.network',
		defaultMessage: 'The translation service could not be reached. Check your network or proxy.',
	},
})

const configuredAIProviders = computed(() =>
	(sharedAIState.value?.providers ?? []).filter(
		(provider) => provider.enabled && provider.models.some((model) => model.enabled),
	),
)
const aiAvailable = computed(
	() => !!sharedAIState.value?.settings.enabled && configuredAIProviders.value.length > 0,
)

const modes = ['bilingual', 'translation-only'] as const
const styles: TranslationStyle[] = [
	'default',
	'blur',
	'blockquote',
	'weakened',
	'dashed-line',
	'border',
	'text-color',
	'background',
]
const languages = ['follow-app', ...LOCALES.map((locale) => locale.code)]

const targetLanguage = computed({
	get: () => settings.value.target_language || 'follow-app',
	set: (value: string) => {
		settings.value.target_language = value === 'follow-app' ? '' : value
	},
})

function providerName(provider: TranslationProvider) {
	return formatMessage(
		{ microsoft: messages.microsoft, google: messages.google, ai: messages.ai }[provider],
	)
}

function languageName(code: string) {
	if (code === 'follow-app') return formatMessage(messages.followApp)
	const locale = LOCALES.find((item) => item.code === code)
	return locale ? `${locale.name} — ${formatMessage(locale.translatedName)}` : code
}

function styleName(style: TranslationStyle) {
	return formatMessage(
		{
			default: messages.styleDefault,
			blur: messages.styleBlur,
			blockquote: messages.styleBlockquote,
			weakened: messages.styleWeakened,
			'dashed-line': messages.styleDashedLine,
			border: messages.styleBorder,
			'text-color': messages.styleTextColor,
			background: messages.styleBackground,
		}[style],
	)
}

const translationProviders = computed<TranslationProvider[]>(() => [
	'google',
	...(aiAvailable.value ? (['ai'] as const) : []),
	'microsoft',
])
const providerOptions = computed(() =>
	translationProviders.value.map((provider) => ({
		value: provider,
		label: providerName(provider),
	})),
)
const languageOptions = computed(() =>
	languages.map((language) => ({ value: language, label: languageName(language) })),
)
const modeOptions = computed(() =>
	modes.map((mode) => ({
		value: mode,
		label: formatMessage(mode === 'bilingual' ? messages.bilingual : messages.translationOnly),
	})),
)
const styleOptions = computed(() =>
	styles.map((style) => ({ value: style, label: styleName(style) })),
)
const aiProviderOptions = computed(() =>
	configuredAIProviders.value.map((provider) => ({
		value: provider.provider_id,
		label:
			provider.custom_name ||
			aiCatalog.value.find((definition) => definition.id === provider.provider_id)?.name ||
			provider.provider_id,
	})),
)
const selectedAIProvider = computed({
	get: () => settings.value.ai_provider_id,
	set: (providerId: string) => {
		settings.value.ai_provider_id = providerId
		settings.value.ai_model_id =
			configuredAIProviders.value
				.find((provider) => provider.provider_id === providerId)
				?.models.find((model) => model.enabled)?.id ?? ''
	},
})
const aiModelOptions = computed(() =>
	(
		configuredAIProviders.value.find(
			(provider) => provider.provider_id === settings.value.ai_provider_id,
		)?.models ?? []
	)
		.filter((model) => model.enabled)
		.map((model) => ({ value: model.id, label: model.name || model.id })),
)
const stylePreviewClass = computed(() => `translation-style-preview-${settings.value.style}`)

watch(
	[aiAvailable, configuredAIProviders],
	() => {
		if (!aiAvailable.value) {
			if (settings.value.provider === 'ai') settings.value.provider = 'google'
			return
		}
		if (
			!configuredAIProviders.value.some(
				(provider) => provider.provider_id === settings.value.ai_provider_id,
			)
		) {
			selectedAIProvider.value = configuredAIProviders.value[0]?.provider_id ?? ''
		}
		if (!aiModelOptions.value.some((model) => model.value === settings.value.ai_model_id)) {
			settings.value.ai_model_id = aiModelOptions.value[0]?.value ?? ''
		}
	},
	{ immediate: true, deep: true },
)

function reportOperationError(error?: unknown) {
	const message = error
		? {
				'rate-limited': messages.rateLimited,
				authentication: messages.authenticationFailed,
				'content-too-long': messages.contentTooLong,
				network: messages.networkFailed,
				provider: messages.operationFailed,
			}[getTranslationErrorKind(error)]
		: messages.operationFailed
	handleError(new Error(formatMessage(message)))
}

watch(
	settings,
	() => {
		if (loading.value) return
		clearTimeout(saveTimer)
		saveTimer = setTimeout(
			() => void updateTranslationSettings(settings.value).catch(reportOperationError),
			250,
		)
	},
	{ deep: true },
)

async function refreshGoogleIpPool() {
	try {
		googleIpPoolSize.value = await getGoogleIpPoolSize()
	} catch (error) {
		reportOperationError(error)
	}
}

watch(
	() => settings.value.provider,
	(provider) => {
		clearInterval(poolTimer)
		if (provider !== 'google') return
		void refreshGoogleIpPool()
		poolTimer = setInterval(() => void refreshGoogleIpPool(), 5000)
	},
	{ immediate: true },
)

onUnmounted(() => {
	clearInterval(poolTimer)
	if (loading.value || !saveTimer) return
	clearTimeout(saveTimer)
	void updateTranslationSettings(settings.value).catch(reportOperationError)
})

onMounted(async () => {
	try {
		const [loadedSettings, , loadedCatalog] = await Promise.all([
			getTranslationSettings(),
			getAIState(),
			getAICatalog(),
		])
		settings.value = loadedSettings
		aiCatalog.value = loadedCatalog
	} catch (error) {
		reportOperationError(error)
	} finally {
		loading.value = false
	}
})

async function testProvider() {
	testing.value = true
	status.value = ''
	try {
		await updateTranslationSettings(settings.value)
		const result = await testTranslationProvider(settings.value.provider)
		status.value = formatMessage(messages.testSuccess, { translation: result })
	} catch (error) {
		reportOperationError(error)
	} finally {
		testing.value = false
	}
}

async function clearCache() {
	try {
		await clearTranslationCache()
		status.value = formatMessage(messages.cacheCleared)
	} catch (error) {
		reportOperationError(error)
	}
}
</script>

<template>
	<div v-if="loading" class="flex min-h-48 items-center justify-center">
		<SpinnerIcon class="size-6 animate-spin text-secondary" />
	</div>
	<div v-else class="flex flex-col gap-6">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
			<p class="m-0 mt-2 text-secondary">{{ formatMessage(messages.description) }}</p>
		</div>

		<div class="grid grid-cols-1 gap-5 md:grid-cols-2">
			<label class="flex flex-col gap-2 font-semibold text-contrast">
				{{ formatMessage(messages.provider) }}
				<Combobox v-model="settings.provider" :options="providerOptions" />
				<span v-if="settings.provider === 'google'" class="text-xs font-normal text-secondary">
					{{ formatMessage(messages.googleIpPool, { count: googleIpPoolSize }) }}
				</span>
			</label>
			<label class="flex flex-col gap-2 font-semibold text-contrast">
				{{ formatMessage(messages.targetLanguage) }}
				<Combobox v-model="targetLanguage" :options="languageOptions" searchable />
			</label>
			<label class="flex flex-col gap-2 font-semibold text-contrast">
				{{ formatMessage(messages.displayMode) }}
				<Combobox v-model="settings.mode" :options="modeOptions" />
			</label>
			<label class="flex flex-col gap-2 font-semibold text-contrast">
				{{ formatMessage(messages.style) }}
				<Combobox v-model="settings.style" :options="styleOptions" />
			</label>
		</div>

		<div
			v-if="settings.provider === 'ai' && aiAvailable"
			class="grid grid-cols-1 gap-4 md:grid-cols-2"
		>
			<label class="flex flex-col gap-2 font-semibold text-contrast">
				{{ formatMessage(messages.aiProvider) }}
				<Combobox v-model="selectedAIProvider" :options="aiProviderOptions">
					<template #selected="{ label }">
						<span class="inline-flex min-w-0 items-center gap-2">
							<AIIcon kind="provider-avatar" :value="selectedAIProvider" :size="20" />
							<span class="truncate">{{ label }}</span>
						</span>
					</template>
					<template #option="{ item, isSelected }">
						<div class="flex min-w-0 items-center gap-2.5">
							<AIIcon kind="provider-avatar" :value="String(item.value)" :size="22" />
							<span
								class="truncate font-semibold leading-tight"
								:class="isSelected ? 'text-brand' : 'text-primary'"
							>
								{{ item.label }}
							</span>
						</div>
					</template>
				</Combobox>
			</label>
			<label class="flex flex-col gap-2 font-semibold text-contrast">
				{{ formatMessage(messages.aiModel) }}
				<div class="translation-model-combobox" :class="{ 'has-model-icon': settings.ai_model_id }">
					<AIIcon
						v-if="settings.ai_model_id"
						class="pointer-events-none absolute left-3 top-1/2 z-[2] -translate-y-1/2"
						kind="model"
						:value="settings.ai_model_id"
						:size="20"
					/>
					<Combobox v-model="settings.ai_model_id" :options="aiModelOptions" searchable>
						<template #option="{ item, isSelected }">
							<div class="flex min-w-0 items-center gap-2.5">
								<AIIcon kind="model" :value="String(item.value)" :size="22" />
								<span
									class="truncate font-semibold leading-tight"
									:class="isSelected ? 'text-brand' : 'text-primary'"
								>
									{{ item.label }}
								</span>
							</div>
						</template>
					</Combobox>
				</div>
			</label>
			<label class="flex flex-col gap-1.5 text-sm font-semibold text-contrast md:col-span-2">
				{{ formatMessage(messages.systemPrompt) }}
				<StyledInput
					v-model="settings.ai_system_prompt"
					multiline
					:rows="3"
					resize="vertical"
					wrapper-class="w-full"
				/>
				<span class="font-normal text-secondary">
					{{ formatMessage(messages.systemPromptDescription) }}
				</span>
			</label>
		</div>

		<div class="flex w-full flex-col gap-2 font-semibold text-contrast">
			<span>{{ formatMessage(messages.stylePreview) }}</span>
			<div class="translation-style-preview-container">
				<p v-if="settings.mode === 'bilingual'" class="translation-style-preview-original m-0">
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

		<div class="flex flex-wrap items-center gap-2">
			<Button type="colored" color="brand" :disabled="testing" @click="testProvider">
				<PlugIcon />{{ formatMessage(testing ? messages.testing : messages.test) }}
			</Button>
			<span v-if="status" class="text-sm text-secondary">{{ status }}</span>
		</div>

		<div class="flex flex-col gap-2">
			<h3 class="m-0 text-base font-semibold text-contrast">{{ formatMessage(messages.cache) }}</h3>
			<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.cacheDescription) }}</p>
			<Button type="base" @click="clearCache"
				><TrashIcon />{{ formatMessage(messages.clearCache) }}</Button
			>
		</div>
	</div>
</template>

<style scoped>
.translation-model-combobox {
	position: relative;
}

.translation-model-combobox.has-model-icon :deep(input) {
	padding-left: 2.75rem !important;
}

.translation-style-preview-container {
	display: flex;
	width: 100%;
	min-height: 6.5rem;
	flex-direction: column;
	box-sizing: border-box;
	gap: 0.75rem;
	padding: 1rem;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-lg);
}

.translation-style-preview-original,
.translation-style-preview {
	font-weight: 400;
}

.translation-style-preview-original,
.translation-style-preview-default {
	color: var(--color-text-primary);
}

.translation-style-preview-weakened {
	color: var(--color-secondary) !important;
}

.translation-style-preview-blur {
	filter: blur(4px);
	opacity: 0.75;
	transition:
		filter 0.1s ease-in-out,
		opacity 0.1s ease-in-out;
}

.translation-style-preview-blur:hover {
	filter: blur(0);
	opacity: 1;
}

.translation-style-preview-blockquote {
	padding: 4px 0 4px 8px;
	border-left: 4px solid var(--color-brand);
}

.translation-style-preview-dashed-line {
	text-decoration: underline dashed var(--color-brand) !important;
	text-underline-offset: 5px;
}

.translation-style-preview-border {
	padding: 2px 4px;
	border: 1px solid var(--color-brand);
	border-radius: 4px;
}

.translation-style-preview-text-color {
	color: oklch(0.693 0.17 162.48) !important;
}

.translation-style-preview-background {
	padding: 2px 4px;
	border-radius: 4px;
	background-color: color-mix(in srgb, var(--color-brand) 15%, transparent);
}
</style>
