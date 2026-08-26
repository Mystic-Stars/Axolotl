<template>
	<span
		ref="trigger"
		class="relative inline-flex max-w-full"
		tabindex="0"
		@mouseenter="setOwnership('triggerHovered', true)"
		@mouseleave="setOwnership('triggerHovered', false)"
		@focus="setOwnership('triggerFocused', true)"
		@blur="setOwnership('triggerFocused', false)"
	>
		<span class="cursor-help underline decoration-dotted underline-offset-2">{{ label }}</span>
		<Teleport to="body">
			<div
				v-if="visible"
				ref="popup"
				:style="popupStyle"
				class="fixed z-[200] flex max-h-[min(28rem,calc(100dvh-2rem))] w-96 max-w-[calc(100vw-2rem)] flex-col rounded-lg border border-solid border-surface-5 p-3 text-left shadow-xl"
				@mouseenter="setOwnership('popupHovered', true)"
				@mouseleave="setOwnership('popupHovered', false)"
				@focusin="setOwnership('popupFocused', true)"
				@focusout="setOwnership('popupFocused', false)"
			>
				<span v-if="loading" class="text-sm text-secondary">{{
					formatMessage(messages.loading)
				}}</span>
				<template v-else-if="metadata">
					<div class="flex items-start justify-between gap-3">
						<div class="min-w-0">
							<strong class="block truncate text-sm text-contrast">{{ metadata.version }}</strong>
							<span v-if="metadata.channel" class="mt-1 block text-xs uppercase text-secondary">{{
								metadata.channel
							}}</span>
						</div>
						<ButtonStyled v-if="metadata.changelog" type="transparent" size="small">
							<button :disabled="translationLoading" @click="toggleTranslation">
								<SpinnerIcon v-if="translationLoading" class="animate-spin" aria-hidden="true" />
								{{ formatMessage(showTranslation ? messages.showOriginal : messages.translate) }}
							</button>
						</ButtonStyled>
					</div>
					<p v-if="translationError" class="mb-0 mt-2 text-sm text-red">{{ translationError }}</p>
					<!-- eslint-disable vue/no-v-html -->
					<div
						v-if="metadata.changelog"
						class="markdown-body mt-2 min-h-0 overflow-y-auto text-sm text-secondary"
						@click="openExternalLink"
						v-html="renderedChangelog"
					/>
					<!-- eslint-enable vue/no-v-html -->
					<span v-else class="mt-2 block text-sm text-secondary">{{
						formatMessage(messages.empty)
					}}</span>
				</template>
				<span v-else class="text-sm text-secondary">{{ formatMessage(messages.unavailable) }}</span>
			</div>
		</Teleport>
	</span>
</template>

<script setup lang="ts">
import { SpinnerIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { renderHighlightedString } from '@modrinth/utils'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

import {
	getTranslationErrorKind,
	getTranslationSettings,
	prepareDescription,
	renderTranslatedDescription,
	translateInBatches,
	validateTranslatedDescription,
} from '@/helpers/translation'
import {
	getUpgradeChangelogTranslation,
	setUpgradeChangelogTranslation,
	shouldUpgradeChangelogStayOpen,
	upgradeChangelogTranslationCacheKey,
	upgradeExternalChangelogUrl,
} from '@/helpers/upgrade-changelog'
import { loadUpgradeVersionMetadata } from '@/helpers/upgrade-version-metadata'
import i18n from '@/i18n.config'

const props = defineProps<{
	label: string
	provider: string | null
	projectId: string | null
	releaseId: string | null
}>()
const messages = defineMessages({
	loading: { id: 'instance.upgrade.changelog.loading', defaultMessage: 'Loading release details…' },
	empty: {
		id: 'instance.upgrade.changelog.empty',
		defaultMessage: 'No changelog was provided for this version.',
	},
	unavailable: {
		id: 'instance.upgrade.changelog.unavailable',
		defaultMessage: 'Release details unavailable.',
	},
	translate: { id: 'instance.upgrade.changelog.translate', defaultMessage: 'Translate' },
	showOriginal: { id: 'instance.upgrade.changelog.show-original', defaultMessage: 'Show original' },
	translationRateLimited: {
		id: 'instance.upgrade.changelog.translation.rate-limited',
		defaultMessage: 'Translation is temporarily rate limited.',
	},
	translationAuthentication: {
		id: 'instance.upgrade.changelog.translation.authentication',
		defaultMessage: 'Translation provider authentication failed.',
	},
	translationTooLong: {
		id: 'instance.upgrade.changelog.translation.too-long',
		defaultMessage: 'This changelog is too long to translate.',
	},
	translationNetwork: {
		id: 'instance.upgrade.changelog.translation.network',
		defaultMessage: 'Translation network request failed.',
	},
	translationFailed: {
		id: 'instance.upgrade.changelog.translation.failed',
		defaultMessage: 'Changelog translation failed.',
	},
})
const { formatMessage } = useVIntl()
const visible = ref(false)
const trigger = ref<HTMLElement | null>(null)
const popup = ref<HTMLElement | null>(null)
const popupStyle = ref<Record<string, string>>({ backgroundColor: 'var(--color-tooltip-bg)' })
const loading = ref(false)
const metadata = ref<Awaited<ReturnType<typeof loadUpgradeVersionMetadata>> | null>(null)
const translationLoading = ref(false)
const translationError = ref<string | null>(null)
const translatedChangelog = ref<string | null>(null)
const showTranslation = ref(false)
const ownership = ref({
	triggerHovered: false,
	triggerFocused: false,
	popupHovered: false,
	popupFocused: false,
})
let closeTimer: ReturnType<typeof setTimeout> | undefined
let loaded = false

const renderedChangelog = computed(() => {
	if (showTranslation.value && translatedChangelog.value) return translatedChangelog.value
	return renderHighlightedString(metadata.value?.changelog ?? '')
})

function cancelClose() {
	if (closeTimer) clearTimeout(closeTimer)
}

function positionPopup() {
	if (!trigger.value || !popup.value) return
	const anchor = trigger.value.getBoundingClientRect()
	if (anchor.bottom < 0 || anchor.top > window.innerHeight) {
		visible.value = false
		return
	}
	const popupRect = popup.value.getBoundingClientRect()
	const gap = 8
	const margin = 8
	const placeAbove =
		window.innerHeight - anchor.bottom < popupRect.height + gap &&
		anchor.top > popupRect.height + gap
	const desiredTop = placeAbove ? anchor.top - popupRect.height - gap : anchor.bottom + gap
	popupStyle.value = {
		backgroundColor: 'var(--color-tooltip-bg)',
		left: `${Math.max(margin, Math.min(anchor.left, window.innerWidth - popupRect.width - margin))}px`,
		top: `${Math.max(margin, Math.min(desiredTop, window.innerHeight - popupRect.height - margin))}px`,
	}
}

async function open() {
	cancelClose()
	visible.value = true
	await nextTick()
	positionPopup()
	if (loaded || !props.provider || !props.projectId || !props.releaseId) return
	loaded = true
	loading.value = true
	try {
		metadata.value = await loadUpgradeVersionMetadata(
			props.provider,
			props.projectId,
			props.releaseId,
		)
	} catch {
		metadata.value = null
	} finally {
		loading.value = false
		await nextTick()
		positionPopup()
	}
}

function setOwnership(key: keyof typeof ownership.value, active: boolean) {
	ownership.value = { ...ownership.value, [key]: active }
	if (shouldUpgradeChangelogStayOpen(ownership.value)) {
		void open()
	} else {
		close()
	}
}

function translationFailureMessage(error: unknown) {
	return formatMessage(
		{
			'rate-limited': messages.translationRateLimited,
			authentication: messages.translationAuthentication,
			'content-too-long': messages.translationTooLong,
			network: messages.translationNetwork,
			provider: messages.translationFailed,
		}[getTranslationErrorKind(error)],
	)
}

async function toggleTranslation() {
	if (showTranslation.value) {
		showTranslation.value = false
		await nextTick()
		positionPopup()
		return
	}
	if (!metadata.value?.changelog || !props.provider || !props.projectId || !props.releaseId) return
	translationLoading.value = true
	translationError.value = null
	try {
		const settings = await getTranslationSettings()
		const targetLanguage = settings.target_language || i18n.global.locale.value || 'en-US'
		const key = upgradeChangelogTranslationCacheKey(
			props.provider,
			props.projectId,
			props.releaseId,
			targetLanguage,
		)
		const cached = getUpgradeChangelogTranslation(key)
		if (cached) {
			translatedChangelog.value = cached
		} else {
			const prepared = prepareDescription(metadata.value.changelog)
			const accumulated: Record<string, string> = {}
			await translateInBatches(
				{
					source_language: 'auto',
					target_language: targetLanguage,
					segments: prepared.segments,
					context: { title: metadata.value.version, description: '' },
				},
				(response) => {
					for (const segment of response.segments) accumulated[segment.id] = segment.text
				},
			)
			validateTranslatedDescription(prepared, accumulated)
			const translated = renderTranslatedDescription(
				prepared,
				accumulated,
				'translation-only',
				settings.style,
			)
			setUpgradeChangelogTranslation(key, translated)
			translatedChangelog.value = translated
		}
		showTranslation.value = true
	} catch (error) {
		translationError.value = translationFailureMessage(error)
	} finally {
		translationLoading.value = false
		await nextTick()
		positionPopup()
	}
}

async function openExternalLink(event: MouseEvent) {
	const target = event.target instanceof Element ? event.target.closest('a') : null
	if (!target) return
	event.preventDefault()
	const url = upgradeExternalChangelogUrl(target.getAttribute('href') ?? '')
	if (!url) return
	await openUrl(url)
}

function handleViewportChange() {
	if (visible.value) positionPopup()
}

function close() {
	cancelClose()
	closeTimer = setTimeout(() => {
		if (!shouldUpgradeChangelogStayOpen(ownership.value)) visible.value = false
	}, 160)
}

function forceClose() {
	cancelClose()
	ownership.value = {
		triggerHovered: false,
		triggerFocused: false,
		popupHovered: false,
		popupFocused: false,
	}
	visible.value = false
}

function handleDocumentPointerDown(event: PointerEvent) {
	const target = event.target
	if (!(target instanceof Node)) return
	if (trigger.value?.contains(target) || popup.value?.contains(target)) return
	forceClose()
}

function handleDocumentKeydown(event: KeyboardEvent) {
	if (event.key === 'Escape') forceClose()
}

onMounted(() => {
	window.addEventListener('resize', handleViewportChange)
	window.addEventListener('scroll', handleViewportChange, true)
	document.addEventListener('pointerdown', handleDocumentPointerDown)
	document.addEventListener('keydown', handleDocumentKeydown)
})
onBeforeUnmount(() => {
	if (closeTimer) clearTimeout(closeTimer)
	window.removeEventListener('resize', handleViewportChange)
	window.removeEventListener('scroll', handleViewportChange, true)
	document.removeEventListener('pointerdown', handleDocumentPointerDown)
	document.removeEventListener('keydown', handleDocumentKeydown)
})
</script>

<style scoped>
:deep(.markdown-body a) {
	color: var(--color-brand);
	cursor: pointer;
	text-decoration: underline;
	text-underline-offset: 2px;
}
</style>
