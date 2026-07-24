<script setup lang="ts">
import { configuredXss, renderHighlightedString } from '@modrinth/utils'
import { computed } from 'vue'

import {
	prepareDescription,
	renderTranslatedDescription,
	type TranslationMode,
	type TranslationStyle,
} from '@/helpers/translation'

const props = defineProps<{
	description: string
	active: boolean
	translations: Record<string, string>
	mode: TranslationMode
	style: TranslationStyle
	format?: 'markdown' | 'html'
}>()

const renderedDescription = computed(() => {
	if (!props.active) {
		return props.format === 'html'
			? configuredXss.process(props.description ?? '')
			: renderHighlightedString(props.description ?? '')
	}
	return renderTranslatedDescription(
		prepareDescription(props.description, props.format),
		props.translations,
		props.mode,
		props.style,
	)
})

const translationOnlyClass = computed(() =>
	props.active && props.mode === 'translation-only'
		? ['ax-translation-only', `ax-translation-style-${props.style}`]
		: [],
)
</script>

<template>
	<!-- eslint-disable-next-line vue/no-v-html -->
	<div class="markdown-body" :class="translationOnlyClass" v-html="renderedDescription" />
</template>

<style scoped>
:deep(.ax-translation-block) {
	margin-block: 0.5rem 1rem;
	animation: translation-float-in 0.5s ease-out both;
}

:deep(.ax-translation-block > :first-child) {
	margin-top: 0;
}

:deep(.ax-translation-block > :last-child) {
	margin-bottom: 0;
}

:deep(.ax-translation-style-weakened) {
	color: var(--color-secondary);
}

:deep(.ax-translation-style-brand) {
	color: var(--color-brand);
}

:deep(.ax-translation-style-border) {
	padding-left: 0.875rem;
	border-left: 3px solid var(--color-brand);
}

:deep(.ax-translation-style-background) {
	padding: 0.75rem 1rem;
	border-radius: var(--radius-lg);
	background: var(--color-button-bg);
}

.ax-translation-only.ax-translation-style-weakened {
	color: var(--color-secondary);
}

.ax-translation-only.ax-translation-style-brand {
	color: var(--color-brand);
}

.ax-translation-only.ax-translation-style-border {
	padding-left: 0.875rem;
	border-left: 3px solid var(--color-brand);
}

.ax-translation-only.ax-translation-style-background {
	padding: 0.75rem 1rem;
	border-radius: var(--radius-lg);
	background: var(--color-button-bg);
}

@keyframes translation-float-in {
	from {
		opacity: 0;
		transform: translateY(12px);
	}
	to {
		opacity: 1;
		transform: translateY(0);
	}
}

</style>
