<script setup lang="ts">
import { ChevronDownIcon, SearchIcon, XIcon } from '@modrinth/assets'
import { defineMessages, type MessageDescriptor, ProgressBar, useVIntl } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { platform as getOsPlatform, version as getOsVersion } from '@tauri-apps/plugin-os'
import { computed, nextTick, ref, watch } from 'vue'

import {
	getVisibleSettingsCategories,
	getVisibleSettingsGroups,
	type SettingsCategory,
	settingsPageTitle,
} from '@/components/ui/settings/settings-registry'
import {
	filterSettingsSearchDocuments,
	normalizeSettingsSearchText,
} from '@/components/ui/settings/settings-search'
import {
	getSettingsSearchTargetId,
	type SettingsSearchEntry,
} from '@/components/ui/settings/settings-search-index'
import { AxolotlBrandConfig } from '@/config'
import { get, set } from '@/helpers/settings'
import { injectAppUpdateDownloadProgress } from '@/providers/download-progress'
import { useTheming } from '@/store/state'

interface SettingsSearchResult {
	category: SettingsCategory
	entry?: SettingsSearchEntry
	label: string
	breadcrumb: string
}

const themeStore = useTheming()
const { formatMessage } = useVIntl()
const { progress, version: downloadingVersion } = injectAppUpdateDownloadProgress()

const version = await getVersion()
const osPlatform = getOsPlatform()
const osVersion = getOsVersion()
const settings = ref(await get())
const devModeCounter = ref(0)
const searchQuery = ref('')
const selectedCategoryId = ref('interface')
const contentContainer = ref<HTMLElement | null>(null)
const searchHighlightTarget = ref<HTMLElement | null>(null)
const expandedGroups = ref<Record<string, boolean>>({
	launcher: true,
	game: true,
	'data-privacy': true,
	support: true,
	developer: false,
})
const hasSearchQuery = computed(() => !!normalizeSettingsSearchText(searchQuery.value))
let searchHighlightTimer: ReturnType<typeof window.setTimeout> | undefined

const messages = defineMessages({
	search: {
		id: 'app.settings.search.placeholder',
		defaultMessage: 'Search settings',
	},
	clearSearch: {
		id: 'app.settings.search.clear',
		defaultMessage: 'Clear settings search',
	},
	noResults: {
		id: 'app.settings.search.empty',
		defaultMessage: 'No settings match your search.',
	},
	results: {
		id: 'app.settings.search.results',
		defaultMessage: 'Search results',
	},
	downloading: {
		id: 'app.settings.downloading',
		defaultMessage: 'Downloading v{version}',
	},
	developerModeEnabled: {
		id: 'app.settings.developer-mode-enabled',
		defaultMessage: 'Developer mode enabled.',
	},
})

const visibleCategories = computed(() => getVisibleSettingsCategories(!!themeStore.devMode))
const visibleGroups = computed(() => getVisibleSettingsGroups(!!themeStore.devMode))
const activeCategory = computed(
	() =>
		visibleCategories.value.find((category) => category.id === selectedCategoryId.value) ??
		visibleCategories.value[0],
)
const searchResults = computed<SettingsSearchResult[]>(() => {
	const documents = visibleGroups.value.flatMap((group) =>
		group.categories.flatMap((category) => {
			const categoryLabel = categoryName(category)
			const groupLabel = formatMessage(group.name)

			return [
				{
					item: {
						category,
						label: categoryLabel,
						breadcrumb: groupLabel,
					},
					text: searchTextVariants([group.name, category.name]),
				},
				...category.entries.map((entry) => ({
					item: {
						category,
						entry,
						label: entryName(entry),
						breadcrumb: `${groupLabel} > ${categoryLabel}`,
					},
					text: [
						...searchTextVariants([
							group.name,
							category.name,
							entry.label,
							...(entry.keywords ?? []),
						]),
						...messageSearchTexts(entry.description),
					],
				})),
			]
		}),
	)

	return filterSettingsSearchDocuments(searchQuery.value, documents).map(({ item }) => item)
})

watch(
	settings,
	async () => {
		await set(settings.value)
	},
	{ deep: true },
)

watch(visibleCategories, (categories) => {
	if (!categories.some((category) => category.id === selectedCategoryId.value)) {
		selectedCategoryId.value = categories[0]?.id ?? 'interface'
	}
})

function selectCategory(categoryId: string) {
	selectedCategoryId.value = categoryId
	const category = visibleCategories.value.find((item) => item.id === categoryId)
	if (category) expandedGroups.value[category.group] = true
	contentContainer.value?.scrollTo({ top: 0 })
}

function toggleGroup(groupId: string) {
	expandedGroups.value[groupId] = !expandedGroups.value[groupId]
}

async function selectSearchResult(result: SettingsSearchResult) {
	selectedCategoryId.value = result.category.id
	expandedGroups.value[result.category.group] = true
	searchQuery.value = ''
	contentContainer.value?.scrollTo({ top: 0 })

	if (!result.entry) return

	const targetId = getSettingsSearchTargetId(result.entry)
	for (let attempt = 0; attempt < 12; attempt++) {
		await nextTick()
		const target = contentContainer.value?.querySelector<HTMLElement>(`#${targetId}`)
		if (target) {
			target.scrollIntoView({ behavior: 'smooth', block: 'center' })
			flashSearchTarget(target)
			return
		}
		await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))
	}
}

function toggleDeveloperMode() {
	devModeCounter.value++
	if (devModeCounter.value <= 5) return

	themeStore.devMode = !themeStore.devMode
	settings.value.developer_mode = !!themeStore.devMode
	devModeCounter.value = 0
}

function categoryName(category: SettingsCategory): string {
	return formatMessage(category.name)
}

function entryName(entry: SettingsSearchEntry): string {
	return formatMessage(entry.label)
}

function messageSearchTexts(message?: MessageDescriptor): string[] {
	if (!message) return []

	const texts = [formatMessage(message), message.defaultMessage].filter(
		(text): text is string => !!text,
	)
	return [...new Set(texts)]
}

function searchTextVariants(messages: Array<MessageDescriptor | undefined>): string[] {
	return messages.flatMap(messageSearchTexts)
}

function searchResultKey(result: SettingsSearchResult): string {
	return result.entry?.id ?? `category-${result.category.id}`
}

function searchMatchSegments(text: string) {
	const query = normalizeSettingsSearchText(searchQuery.value)
	if (!query) return [{ text, matched: false }]

	const index = text.toLocaleLowerCase().indexOf(query)
	if (index < 0) return [{ text, matched: false }]

	return [
		{ text: text.slice(0, index), matched: false },
		{ text: text.slice(index, index + query.length), matched: true },
		{ text: text.slice(index + query.length), matched: false },
	].filter((segment) => segment.text)
}

function flashSearchTarget(target: HTMLElement) {
	const highlightTarget = target.closest<HTMLElement>('.settings-row') ?? target
	searchHighlightTarget.value?.classList.remove('settings-search-result-highlight')
	if (searchHighlightTimer) window.clearTimeout(searchHighlightTimer)

	highlightTarget.classList.add('settings-search-result-highlight')
	searchHighlightTarget.value = highlightTarget
	searchHighlightTimer = window.setTimeout(() => {
		highlightTarget.classList.remove('settings-search-result-highlight')
		if (searchHighlightTarget.value === highlightTarget) searchHighlightTarget.value = null
	}, 1800)
}

function platformName() {
	return osPlatform === 'macos' ? 'macOS' : osPlatform.charAt(0).toUpperCase() + osPlatform.slice(1)
}

const pageTitle: MessageDescriptor = settingsPageTitle
</script>

<template>
	<div class="settings-fixed-render h-full min-h-0 p-6">
		<div class="settings-layout h-full min-h-0">
			<aside class="settings-sidebar">
				<div class="relative shrink-0">
					<SearchIcon
						class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-secondary"
					/>
					<input
						v-model="searchQuery"
						type="search"
						:placeholder="formatMessage(messages.search)"
						:aria-label="formatMessage(messages.search)"
						class="w-full rounded-lg border border-surface-4 bg-surface-3 py-2 pl-9 pr-9 text-sm text-contrast outline-none transition-colors placeholder:text-secondary focus:border-surface-5"
						@keydown.escape="searchQuery = ''"
					/>
					<button
						v-if="searchQuery"
						type="button"
						class="absolute right-1.5 top-1/2 flex size-7 -translate-y-1/2 items-center justify-center rounded-md border-0 bg-transparent text-secondary transition-colors hover:bg-surface-4 hover:text-contrast"
						:aria-label="formatMessage(messages.clearSearch)"
						@click="searchQuery = ''"
					>
						<XIcon class="size-4" />
					</button>
				</div>

				<div
					v-if="hasSearchQuery"
					class="settings-sidebar-list"
					:aria-label="formatMessage(messages.results)"
				>
					<button
						v-for="result in searchResults"
						:key="searchResultKey(result)"
						type="button"
						class="settings-search-result items-center gap-2 p-2 hover:bg-surface-3 hover:text-contrast"
						@click="selectSearchResult(result)"
					>
						<component :is="result.category.icon" class="size-4 shrink-0 text-secondary" />
						<span class="settings-search-result-copy">
							<span class="settings-search-result-label">
								<template v-for="segment in searchMatchSegments(result.label)" :key="segment.text">
									<mark v-if="segment.matched" class="settings-search-match">{{
										segment.text
									}}</mark>
									<span v-else>{{ segment.text }}</span>
								</template>
							</span>
							<span class="truncate text-xs text-secondary">{{ result.breadcrumb }}</span>
						</span>
					</button>
					<p v-if="searchResults.length === 0" class="m-0 px-3 py-4 text-sm text-secondary">
						{{ formatMessage(messages.noResults) }}
					</p>
				</div>

				<nav v-else class="settings-sidebar-list" :aria-label="formatMessage(pageTitle)">
					<section v-for="group in visibleGroups" :key="group.id" class="settings-nav-group">
						<button
							type="button"
							class="settings-group-button hover:bg-surface-3 hover:text-contrast"
							:aria-expanded="expandedGroups[group.id]"
							@click="toggleGroup(group.id)"
						>
							<component :is="group.icon" class="size-3.5 shrink-0" />
							<span class="truncate">{{ formatMessage(group.name) }}</span>
							<ChevronDownIcon
								class="ml-auto size-3.5 shrink-0 transition-transform"
								:class="expandedGroups[group.id] ? 'rotate-180' : ''"
							/>
						</button>
						<div v-show="expandedGroups[group.id]" class="settings-nav-items">
							<button
								v-for="category in group.categories"
								:key="category.id"
								type="button"
								:data-onboarding-id="category.onboardingId"
								class="settings-category-button hover:bg-surface-3 hover:text-contrast"
								:class="{ 'is-active': activeCategory?.id === category.id }"
								@click="selectCategory(category.id)"
							>
								<component :is="category.icon" class="size-4 shrink-0" />
								<span class="truncate">{{ categoryName(category) }}</span>
							</button>
						</div>
					</section>
				</nav>

				<footer class="mt-auto shrink-0 pt-4 text-sm text-secondary">
					<div v-if="progress > 0 && progress < 1" class="mb-4">
						<p class="m-0 mb-2">
							{{ formatMessage(messages.downloading, { version: downloadingVersion }) }}
						</p>
						<ProgressBar :progress="progress" />
					</div>
					<p v-if="themeStore.devMode" class="m-0 mb-3 text-brand font-semibold">
						{{ formatMessage(messages.developerModeEnabled) }}
					</p>
					<div class="flex items-center gap-3">
						<button
							type="button"
							class="m-0 flex size-9 shrink-0 items-center justify-center rounded-lg border-0 bg-transparent p-0 transition-colors hover:bg-surface-3"
							:class="themeStore.devMode ? 'text-brand' : 'text-secondary'"
							@click="toggleDeveloperMode"
						>
							<img class="size-8 object-contain" src="@/assets/axolotl.png" alt="" />
						</button>
						<div class="min-w-0">
							<p class="m-0 truncate">{{ AxolotlBrandConfig.productName }} {{ version }}</p>
							<p class="m-0 truncate">{{ platformName() }} {{ osVersion }}</p>
						</div>
					</div>
				</footer>
			</aside>

			<section class="settings-content" :aria-label="formatMessage(pageTitle)">
				<header class="settings-content-header">
					<component :is="activeCategory?.icon" class="size-5 text-secondary" />
					<h1 class="m-0 text-xl font-semibold text-contrast">
						{{ activeCategory ? categoryName(activeCategory) : formatMessage(pageTitle) }}
					</h1>
				</header>
				<div
					ref="contentContainer"
					class="settings-content-scroll min-h-0 flex-1"
					:class="activeCategory?.flushContent ? 'overflow-hidden' : 'overflow-y-auto'"
				>
					<div
						v-if="activeCategory"
						:id="`settings-category-${activeCategory.id}`"
						class="min-h-0"
						:class="activeCategory.flushContent ? 'h-full' : 'mx-auto max-w-5xl px-6 pb-6'"
						tabindex="-1"
					>
						<Suspense>
							<component :is="activeCategory.content" />
						</Suspense>
					</div>
				</div>
			</section>
		</div>
	</div>
</template>

<style scoped>
.settings-layout {
	--settings-divider: color-mix(in srgb, var(--surface-4) 55%, transparent);
	--settings-card-border: color-mix(in srgb, var(--surface-4) 72%, transparent);
	display: grid;
	grid-template-columns: minmax(14rem, 16rem) minmax(0, 1fr);
	min-height: 0;
	overflow: hidden;
}

.settings-sidebar {
	display: flex;
	height: 100%;
	flex-direction: column;
	gap: 0.75rem;
	min-height: 0;
	overflow: hidden;
	padding: var(--gap-lg);
}

.settings-sidebar-list {
	display: flex;
	min-height: 0;
	flex: 1;
	flex-direction: column;
	gap: var(--gap-lg);
	overflow-y: auto;
}

.settings-nav-group {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: var(--gap-xs);
}

.settings-nav-items {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: var(--gap-xs);
}

.settings-group-button {
	display: flex;
	width: 100%;
	align-items: center;
	gap: var(--gap-sm);
	min-height: 1.75rem;
	padding: 0 var(--gap-sm);
	border: 0;
	border-radius: var(--radius-sm);
	background: transparent;
	color: var(--color-secondary);
	font-size: 0.75rem;
	font-weight: 600;
	text-align: left;
	cursor: pointer;
	transition:
		background-color 120ms ease,
		color 120ms ease;
}

.settings-category-button,
.settings-search-result {
	display: flex;
	width: 100%;
	border: 0;
	border-radius: var(--radius-sm);
	background: transparent;
	color: var(--color-text-primary);
	text-align: left;
	cursor: pointer;
	transition:
		background-color 120ms ease,
		color 120ms ease;
}

.settings-category-button {
	align-items: center;
	gap: 0.625rem;
	min-height: 2.25rem;
	gap: var(--gap-sm);
	padding: 0 var(--gap-sm);
	font-size: 0.875rem;
	font-weight: 600;
}

.settings-category-button.is-active {
	background: var(--color-button-bg-selected);
	color: var(--color-button-text-selected);
}

.settings-search-result-copy {
	display: flex;
	min-width: 0;
	flex: 1;
	flex-direction: column;
	gap: 0.125rem;
}

.settings-search-result-label {
	overflow: hidden;
	color: var(--color-contrast);
	font-size: 0.875rem;
	font-weight: 600;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.settings-search-match {
	background: transparent;
	color: var(--color-brand);
	padding: 0;
}

.settings-content {
	display: flex;
	min-width: 0;
	flex-direction: column;
	min-height: 0;
	overflow: hidden;
}

.settings-content-header {
	display: flex;
	flex-shrink: 0;
	align-items: center;
	gap: var(--gap-md);
	padding: var(--gap-xs) var(--gap-xl) var(--gap-lg);
}

.settings-content-scroll :deep([id^='settings-target-']),
.settings-content-scroll :deep([id^='settings-category-']) {
	scroll-margin-top: 1.5rem;
}

.settings-content-scroll :deep(.settings-search-result-highlight) {
	border-radius: var(--radius-sm);
	animation: settings-search-result-highlight 0.9s ease-in-out 2;
}

@keyframes settings-search-result-highlight {
	0%,
	100% {
		background: transparent;
	}

	50% {
		background: var(--surface-3);
	}
}

@media (max-width: 800px) {
	.settings-layout {
		grid-template-columns: minmax(0, 1fr);
		grid-template-rows: auto minmax(0, 1fr);
	}

	.settings-sidebar {
		height: auto;
		border-right: 0;
		padding-bottom: var(--gap-lg);
	}

	.settings-sidebar-list {
		max-height: 18rem;
		overflow-y: auto;
	}

	.settings-content-header {
		padding-inline: var(--gap-lg);
	}
}
</style>

<style>
.app-viewport:has(.settings-fixed-render) {
	overflow: hidden;
	scrollbar-gutter: auto;
}

.app-viewport:has(.settings-fixed-render) .page-transition-grid,
.app-viewport:has(.settings-fixed-render) .page-transition-layer {
	height: 100%;
	min-height: 0;
}
</style>
