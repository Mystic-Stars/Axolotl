<script setup lang="ts">
import { Combobox, defineMessages, injectNotificationManager, Toggle, useVIntl } from '@modrinth/ui'
import { computed, onMounted, ref } from 'vue'

import { type AIState, getAIState, sharedAIState } from '@/helpers/ai'
import { get_crash_analysis_ai_settings, update_crash_analysis_ai_settings } from '@/helpers/logs'

import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

type CrashAnalysisAISettings = {
	enabled: boolean
	provider_id: string
	model_id: string
}

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const settings = ref<CrashAnalysisAISettings>({ enabled: false, provider_id: '', model_id: '' })
const loading = ref(true)
const saving = ref(false)

const messages = defineMessages({
	title: { id: 'app.crash-analysis.ai.settings.title', defaultMessage: 'Crash AI explanation' },
	description: {
		id: 'app.crash-analysis.ai.settings.description',
		defaultMessage:
			'Optionally send a sanitized and shortened crash context to a model configured in AI Providers.',
	},
	enabled: {
		id: 'app.crash-analysis.ai.settings.enabled',
		defaultMessage: 'Enable AI explanations',
	},
	enabledDescription: {
		id: 'app.crash-analysis.ai.settings.enabled-description',
		defaultMessage: 'Local rule-based crash analysis remains available without AI.',
	},
	provider: { id: 'app.crash-analysis.ai.settings.provider', defaultMessage: 'AI provider' },
	model: { id: 'app.crash-analysis.ai.settings.model', defaultMessage: 'Text model' },
	noProviders: {
		id: 'app.crash-analysis.ai.settings.no-providers',
		defaultMessage: 'Enable a provider and a text model above before using AI crash explanations.',
	},
})

const emptyState: AIState = { settings: { enabled: false }, catalog_source: '', providers: [] }
const aiState = computed(() => sharedAIState.value ?? emptyState)
const providers = computed(() =>
	aiState.value.providers.filter(
		(provider) => provider.enabled && provider.models.some((model) => model.enabled),
	),
)
const providerOptions = computed(() =>
	providers.value.map((provider) => ({
		value: provider.provider_id,
		label: provider.custom_name || provider.provider_id,
	})),
)
const modelOptions = computed(
	() =>
		providers.value
			.find((provider) => provider.provider_id === settings.value.provider_id)
			?.models.filter((model) => model.enabled)
			.map((model) => ({ value: model.id, label: model.name || model.id })) ?? [],
)

async function save(next: CrashAnalysisAISettings): Promise<void> {
	saving.value = true
	try {
		await update_crash_analysis_ai_settings(next)
		settings.value = next
	} catch (error) {
		handleError(error)
	} finally {
		saving.value = false
	}
}

function updateEnabled(enabled: boolean): void {
	void save({ ...settings.value, enabled })
}

function updateProvider(providerId: string): void {
	const provider = providers.value.find((item) => item.provider_id === providerId)
	const modelId = provider?.models.find((model) => model.enabled)?.id ?? ''
	void save({ ...settings.value, provider_id: providerId, model_id: modelId })
}

function updateModel(modelId: string): void {
	void save({ ...settings.value, model_id: modelId })
}

onMounted(async () => {
	try {
		const [nextSettings] = await Promise.all([get_crash_analysis_ai_settings(), getAIState()])
		settings.value = nextSettings
	} catch (error) {
		handleError(error)
	} finally {
		loading.value = false
	}
})
</script>

<template>
	<SettingsSection
		v-if="!loading"
		id="settings-target-crash-analysis-ai"
		:title="formatMessage(messages.title)"
		:description="formatMessage(messages.description)"
	>
		<SettingsRow>
			<template #label>{{ formatMessage(messages.enabled) }}</template>
			<template #description>{{ formatMessage(messages.enabledDescription) }}</template>
			<template #control>
				<Toggle
					id="crash-analysis-ai-enabled"
					:model-value="settings.enabled"
					:disabled="saving || !aiState.settings.enabled || !providers.length"
					@update:model-value="updateEnabled(!!$event)"
				/>
			</template>
		</SettingsRow>
		<template v-if="providers.length">
			<SettingsRow>
				<template #label>{{ formatMessage(messages.provider) }}</template>
				<template #control>
					<Combobox
						id="crash-analysis-ai-provider"
						:model-value="settings.provider_id"
						:options="providerOptions"
						:disabled="saving"
						@update:model-value="updateProvider(String($event))"
					/>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>{{ formatMessage(messages.model) }}</template>
				<template #control>
					<Combobox
						id="crash-analysis-ai-model"
						:model-value="settings.model_id"
						:options="modelOptions"
						:disabled="saving || !settings.provider_id"
						@update:model-value="updateModel(String($event))"
					/>
				</template>
			</SettingsRow>
		</template>
		<p v-else class="m-0 p-4 text-sm text-secondary">{{ formatMessage(messages.noProviders) }}</p>
	</SettingsSection>
</template>
