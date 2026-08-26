<script setup lang="ts">
import {
	Admonition,
	AutoLink,
	commonSettingsMessages,
	IntlFormatted,
	LanguageSelector,
	languageSelectorMessages,
	LOCALES,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { get, set } from '@/helpers/settings.ts'
import i18n from '@/i18n.config'

import SettingsSaveStatus from './SettingsSaveStatus.vue'

const { formatMessage } = useVIntl()

const platform = computed(() => formatMessage(languageSelectorMessages.platformApp))

const settings = ref(await get())
const $isChanging = ref(false)
const saveStatus = ref<'idle' | 'saving' | 'saved' | 'error'>('idle')
const retryLocale = ref<string | null>(null)

async function onLocaleChange(newLocale: string) {
	if (settings.value.locale === newLocale) return

	const previousLocale = settings.value.locale
	$isChanging.value = true
	saveStatus.value = 'saving'
	retryLocale.value = null
	try {
		i18n.global.locale.value = newLocale
		settings.value.locale = newLocale
		await set(settings.value)
		saveStatus.value = 'saved'
	} catch {
		i18n.global.locale.value = previousLocale
		settings.value.locale = previousLocale
		retryLocale.value = newLocale
		saveStatus.value = 'error'
	} finally {
		$isChanging.value = false
	}
}

function retrySave() {
	if (retryLocale.value) void onLocaleChange(retryLocale.value)
}
</script>

<template>
	<div class="flex flex-col gap-3">
		<header class="settings-page-header">
			<h2 id="settings-target-language" tabindex="-1" class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(commonSettingsMessages.language) }}
			</h2>
			<SettingsSaveStatus :status="saveStatus" :retry="retrySave" />
		</header>
		<div class="settings-page-card">
			<Admonition type="warning">
				{{ formatMessage(languageSelectorMessages.languageWarning, { platform }) }}
			</Admonition>
			<p class="settings-page-description">
				<IntlFormatted
					:message-id="languageSelectorMessages.languagesDescription"
					:values="{ platform }"
				>
					<template #~crowdin-link="{ children }">
						<AutoLink to="https://translate.modrinth.com">
							<component :is="() => children" />
						</AutoLink>
					</template>
				</IntlFormatted>
			</p>
			<LanguageSelector
				:current-locale="settings.locale"
				:locales="LOCALES"
				:on-locale-change="onLocaleChange"
				:is-changing="$isChanging"
			/>
		</div>
	</div>
</template>

<style scoped>
.settings-page-header {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: var(--gap-md);
}

.settings-page-card {
	display: flex;
	flex-direction: column;
	gap: var(--gap-lg);
	padding: var(--gap-lg);
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
}

.settings-page-description {
	margin: 0;
	color: var(--color-secondary);
	font-size: 0.875rem;
	line-height: 1.5;
}
</style>
