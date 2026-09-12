<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import { SearchIcon, SpinnerIcon } from '@modrinth/assets'
import { computed, ref, toValue } from 'vue'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import Combobox, { type ComboboxOption } from '#ui/components/base/Combobox.vue'
import EmptyState from '#ui/components/base/EmptyState.vue'
import NavTabs from '#ui/components/base/NavTabs.vue'
import Pagination from '#ui/components/base/Pagination.vue'
import PopoutMenu from '#ui/components/base/PopoutMenu.vue'
import StyledInput from '#ui/components/base/StyledInput.vue'
import ProjectCard from '#ui/components/project/card/ProjectCard.vue'
import ContentCardReveal from '#ui/components/project/ContentCardReveal.vue'
import ProjectCardList from '#ui/components/project/ProjectCardList.vue'
import ProjectCardSkeleton from '#ui/components/project/ProjectCardSkeleton.vue'
import SearchFilterControl from '#ui/components/search/SearchFilterControl.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { useStickyObserver } from '#ui/composables/sticky-observer'
import { commonMessages, formatProjectTypeSentence } from '#ui/utils/common-messages'
import type { SortType } from '#ui/utils/search'

import SelectedProjectsFloatingBar from './components/SelectedProjectsFloatingBar.vue'
import BrowseInstallHeader from './header.vue'
import { injectBrowseManager } from './providers/browse-manager'

const ctx = injectBrowseManager()
const { formatMessage } = useVIntl()
const lockedMessages = computed(() => toValue(ctx.lockedFilterMessages))
const stickyChromeRef = ref<HTMLElement | null>(null)
const { isStuck: isChromeStuck } = useStickyObserver(stickyChromeRef, 'BrowseToolbarChrome')
const showInstallHeader = computed(
	() =>
		!!ctx.installContext?.value &&
		ctx.installContext.value.showInstallHeader !== false &&
		ctx.variant !== 'web',
)

const maxResultsOptions = computed<ComboboxOption<number>[]>(() =>
	(ctx.maxResultsOptions?.value ?? [5, 10, 15, 20, 50, 100]).map((n) => ({
		value: n,
		label: String(n),
	})),
)

const messages = defineMessages({
	searchPlaceholder: {
		id: 'browse.search.placeholder',
		defaultMessage: 'Search {projectType}...',
	},
	viewPrefix: {
		id: 'browse.view-prefix',
		defaultMessage: 'View:',
	},
	filterResults: {
		id: 'browse.filter-results',
		defaultMessage: 'Filter results...',
	},
	offline: {
		id: 'browse.offline',
		defaultMessage: 'You are currently offline. Connect to the internet to browse Modrinth!',
	},
	offlineHeading: {
		id: 'browse.offline.heading',
		defaultMessage: 'No connection',
	},
	loadingLabel: {
		id: 'browse.loading-results',
		defaultMessage: 'Loading results…',
	},
	noResults: {
		id: 'browse.no-results',
		defaultMessage: 'No results found for your query!',
	},
	noResultsHeading: {
		id: 'browse.no-results.heading',
		defaultMessage: 'Nothing here yet',
	},
	clearSearchLabel: {
		id: 'browse.clear-search',
		defaultMessage: 'Clear search',
	},
	retryLabel: {
		id: 'browse.retry',
		defaultMessage: 'Try again',
	},
	sortRelevance: { id: 'browse.sort.relevance', defaultMessage: 'Relevance' },
	sortDownloads: { id: 'browse.sort.downloads', defaultMessage: 'Downloads' },
	sortFollowers: { id: 'browse.sort.followers', defaultMessage: 'Followers' },
	sortDatePublished: { id: 'browse.sort.date-published', defaultMessage: 'Date published' },
	sortDateUpdated: { id: 'browse.sort.date-updated', defaultMessage: 'Date updated' },
	sortVerifiedPlays: { id: 'browse.sort.verified-plays', defaultMessage: 'Verified plays' },
	sortPlayers: { id: 'browse.sort.players', defaultMessage: 'Players' },
})

function formatSortType(sortType: SortType): string {
	const sortMessages = {
		relevance: messages.sortRelevance,
		downloads: messages.sortDownloads,
		follows: messages.sortFollowers,
		newest: messages.sortDatePublished,
		updated: messages.sortDateUpdated,
		'minecraft_java_server.verified_plays_2w': messages.sortVerifiedPlays,
		'minecraft_java_server.ping.data.players_online': messages.sortPlayers,
		date_created: messages.sortDatePublished,
		date_modified: messages.sortDateUpdated,
	}

	const message = sortMessages[sortType.name as keyof typeof sortMessages]
	return message ? formatMessage(message) : sortType.display
}

const sortOptions = computed<ComboboxOption<SortType>[]>(() =>
	ctx.effectiveSortTypes.value.map((sortType) => ({
		value: sortType,
		label: formatSortType(sortType),
	})),
)

const selectedDisplayMode = computed(() =>
	ctx.displayModeOptions?.value.find((option) => option.id === ctx.displayMode?.value),
)

const skeletonCount = computed(() => {
	const layout = ctx.effectiveLayout.value
	if (layout === 'grid' || layout === 'gallery') return 6
	// Match the visible list density without stacking a full viewport of rows.
	return Math.min(ctx.maxResults?.value ?? 8, 8)
})
</script>

<template>
	<div
		ref="stickyChromeRef"
		class="browse-toolbar-chrome sticky top-0 z-20 flex flex-col gap-3 border-b border-solid border-surface-5 bg-surface-1"
		:class="[
			isChromeStuck && ctx.variant !== 'web' ? 'border-t' : '',
			ctx.variant === 'web'
				? 'mb-3 rounded-xl p-3'
				: '-mx-6 -mt-6 rounded-tl-[--radius-xl] rounded-tr-[--radius-xl] border-0',
		]"
	>
		<div v-if="showInstallHeader" class="p-3 pb-0">
			<BrowseInstallHeader />
		</div>
		<div class="flex flex-col gap-3" :class="ctx.variant === 'web' ? '' : 'p-3'">
			<div class="flex min-w-0 items-center gap-2">
				<NavTabs
					v-if="ctx.showProjectTypeTabs.value"
					:links="ctx.selectableProjectTypes.value"
					class="min-w-0 flex-1"
				/>
				<div class="shrink-0">
					<slot name="nav-tabs-actions" />
				</div>
			</div>

			<div class="flex items-center gap-2">
				<StyledInput
					v-model="ctx.query.value"
					:icon="SearchIcon"
					type="text"
					autocomplete="off"
					:placeholder="
						formatMessage(messages.searchPlaceholder, {
							projectType: formatProjectTypeSentence(formatMessage, ctx.projectType.value, 2),
						})
					"
					clearable
					wrapper-class="flex-1"
					:input-class="ctx.variant === 'web' ? '!h-12' : 'h-12'"
					@clear="ctx.clearSearch()"
				/>
				<slot name="search-bar-actions" />
			</div>

			<div class="flex flex-wrap items-center gap-2">
				<Combobox
					:model-value="ctx.effectiveCurrentSortType.value"
					:options="sortOptions"
					:class="
						ctx.variant === 'web'
							? '!w-[16rem] min-w-max max-w-full flex-grow md:flex-grow-0'
							: '!w-[16rem] min-w-max max-w-full'
					"
					@update:model-value="(val: SortType) => (ctx.effectiveCurrentSortType.value = val)"
				>
					<template #prefix>
						<span class="font-semibold text-primary">{{
							formatMessage(commonMessages.sortByLabel)
						}}</span>
					</template>
				</Combobox>

				<Combobox
					:model-value="ctx.maxResults.value"
					:options="maxResultsOptions"
					:class="
						ctx.variant === 'web'
							? '!w-[9rem] min-w-max max-w-full flex-grow md:flex-grow-0'
							: '!w-[9rem] min-w-max max-w-full'
					"
					:placeholder="formatMessage(commonMessages.viewLabel)"
					@update:model-value="(val: number) => (ctx.maxResults.value = val)"
				>
					<template #prefix>
						<span class="font-semibold text-primary">{{ formatMessage(messages.viewPrefix) }}</span>
					</template>
				</Combobox>

				<div v-if="ctx.filtersMenuOpen && !ctx.filtersMenuOpen.value" class="lg:hidden">
					<ButtonStyled>
						<button @click="ctx.filtersMenuOpen.value = true">
							{{ formatMessage(messages.filterResults) }}
						</button>
					</ButtonStyled>
				</div>

				<PopoutMenu
					v-if="ctx.displayMode && ctx.displayModeOptions?.value.length && ctx.setDisplayMode"
					:tooltip="ctx.displayModeTooltip?.value"
					placement="bottom-end"
				>
					<ButtonStyled circular>
						<button :aria-label="ctx.displayModeTooltip?.value">
							<component :is="selectedDisplayMode?.icon" />
						</button>
					</ButtonStyled>
					<template #menu>
						<div class="flex w-44 flex-col gap-1 p-1">
							<ButtonStyled
								v-for="option in ctx.displayModeOptions.value"
								:key="option.id"
								:type="ctx.displayMode.value === option.id ? 'filled' : 'transparent'"
							>
								<button
									class="flex w-full items-center gap-2 !justify-start text-left"
									:aria-pressed="ctx.displayMode.value === option.id"
									@click="ctx.setDisplayMode!(option.id)"
								>
									<component :is="option.icon" class="h-4 w-4" />
									<span>{{ option.label }}</span>
								</button>
							</ButtonStyled>
						</div>
					</template>
				</PopoutMenu>

				<Pagination
					:page="ctx.currentPage.value"
					:count="ctx.pageCount.value"
					:loading="ctx.loading.value"
					:class="ctx.variant === 'web' ? 'mx-auto sm:ml-auto sm:mr-0' : 'ml-auto'"
					@switch-page="ctx.setPage"
				/>
			</div>

			<SearchFilterControl
				v-if="ctx.isServerType.value"
				v-model:selected-filters="ctx.serverCurrentFilters.value"
				:filters="ctx.serverFilterTypes.value"
				:provided-filters="[]"
				:overridden-provided-filter-types="[]"
				:project-type="ctx.projectType.value"
			/>
			<SearchFilterControl
				v-else
				v-model:selected-filters="ctx.currentFilters.value"
				:filters="
					ctx.filters.value.filter(
						(f) => f.display !== 'none' && !(ctx.hiddenFilterTypes?.value ?? []).includes(f.id),
					)
				"
				:provided-filters="ctx.providedFilters?.value ?? []"
				:overridden-provided-filter-types="ctx.overriddenProvidedFilterTypes.value"
				:project-type="ctx.projectType.value"
				:provided-message="lockedMessages?.providedBy"
			/>
		</div>
	</div>

	<SelectedProjectsFloatingBar v-if="ctx.installContext?.value && ctx.variant !== 'web'" />

	<slot name="above-results" />

	<div class="search relative">
		<section v-if="ctx.loading.value" class="py-1" aria-busy="true" aria-live="polite">
			<div class="flex items-center justify-center gap-2 pb-3 text-sm font-medium text-secondary">
				<SpinnerIcon class="size-4 animate-spin" />
				{{ formatMessage(messages.loadingLabel) }}
			</div>
			<ProjectCardList :layout="ctx.effectiveLayout.value">
				<ProjectCardSkeleton
					v-for="index in skeletonCount"
					:key="`skeleton-${index}`"
					:layout="ctx.effectiveLayout.value"
				/>
			</ProjectCardList>
		</section>
		<section v-else-if="ctx.offline?.value && ctx.totalHits.value === 0" class="py-8">
			<EmptyState type="offline" compact :heading="formatMessage(messages.offlineHeading)">
				<template #description>{{ formatMessage(messages.offline) }}</template>
				<template #actions>
					<ButtonStyled>
						<button type="button" @click="ctx.refreshSearch()">
							{{ formatMessage(messages.retryLabel) }}
						</button>
					</ButtonStyled>
				</template>
			</EmptyState>
		</section>
		<section
			v-else-if="
				ctx.isServerType.value
					? ctx.serverHits.value.length === 0
					: ctx.projectHits.value.length === 0
			"
			class="py-8"
		>
			<EmptyState
				type="no-results"
				compact
				:heading="formatMessage(messages.noResultsHeading)"
				:description="formatMessage(messages.noResults)"
			>
				<template v-if="ctx.query.value" #actions>
					<ButtonStyled>
						<button type="button" @click="ctx.clearSearch()">
							{{ formatMessage(messages.clearSearchLabel) }}
						</button>
					</ButtonStyled>
				</template>
			</EmptyState>
		</section>

		<ProjectCardList v-else :layout="ctx.effectiveLayout.value">
			<template v-if="ctx.isServerType.value">
				<ContentCardReveal
					v-for="result in ctx.serverHits.value"
					:key="`server-card-${result.project_id}`"
				>
					<ProjectCard
						:title="result.name"
						:icon-url="result.icon_url || undefined"
						:summary="result.summary"
						:tags="result.categories"
						:link="ctx.getServerProjectLink(result)"
						:server-online-players="result.minecraft_java_server?.ping?.data?.players_online ?? 0"
						:server-region="result.minecraft_server?.region"
						:server-recent-plays="result.minecraft_java_server?.verified_plays_2w ?? 0"
						:server-modpack-content="ctx.getServerModpackContent?.(result)"
						:server-ping="ctx.serverPings?.value?.[result.project_id]"
						:server-status-online="!!result.minecraft_java_server?.ping?.data"
						:hide-online-players-label="ctx.variant === 'app'"
						:hide-recent-plays-label="ctx.variant === 'app'"
						:layout="ctx.effectiveLayout.value"
						:max-tags="2"
						is-server-project
						exclude-loaders
						:color="result.color ?? undefined"
						:banner="result.featured_gallery ?? undefined"
						@contextmenu.prevent.stop="(event: MouseEvent) => ctx.onContextMenu?.(event, result)"
						@mouseenter="ctx.onServerProjectHover?.(result)"
						@mouseleave="ctx.onProjectHoverEnd?.()"
					>
						<template v-if="ctx.getCardActions?.(result, ctx.projectType.value)?.length" #actions>
							<div class="flex gap-2">
								<ButtonStyled
									v-for="action in ctx.getCardActions(result, ctx.projectType.value)"
									:key="action.key"
									:color="action.color"
									:type="action.type"
									:size="ctx.effectiveLayout.value === 'compact' ? 'small' : 'standard'"
									:circular="action.circular"
								>
									<button
										v-tooltip="action.tooltip"
										:disabled="action.disabled"
										@click.stop="action.onClick"
									>
										<component :is="action.icon" :class="action.iconClass" />
										<template v-if="!action.circular">{{
											ctx.effectiveLayout.value === 'compact'
												? (action.compactLabel ?? action.label)
												: action.label
										}}</template>
									</button>
								</ButtonStyled>
							</div>
						</template>
					</ProjectCard>
				</ContentCardReveal>
			</template>
			<template v-else>
				<ContentCardReveal
					v-for="result in ctx.projectHits.value"
					:key="`${result.provider}:${result.project_id}`"
				>
					<ProjectCard
						:link="ctx.getProjectLink(result)"
						:title="result.title"
						:icon-url="result.icon_url"
						:author="{
							name: result.organization == null ? result.author : result.organization,
							link:
								result.provider === 'curseforge'
									? result.author_url
									: result.provider === 'modrinth'
										? result.organization_id == null
											? ctx.variant === 'web'
												? `/user/${result.author_id ?? result.author}`
												: `https://modrinth.com/user/${result.author_id ?? result.author}`
											: ctx.variant === 'web'
												? `/organization/${result.organization_id}`
												: `https://modrinth.com/organization/${result.organization_id}`
										: undefined,
						}"
						:date-updated="result.date_modified"
						:date-published="result.date_created"
						:displayed-date="
							ctx.effectiveCurrentSortType.value.name === 'newest' ? 'published' : 'updated'
						"
						:downloads="result.downloads"
						:summary="result.description"
						:tags="result.display_categories"
						:all-tags="result.categories"
						:deprioritized-tags="ctx.deprioritizedTags.value"
						:exclude-loaders="ctx.excludeLoaders.value"
						:banner="result.featured_gallery ?? undefined"
						:color="result.color ?? undefined"
						:provider="result.provider"
						:environment="
							['mod', 'modpack'].includes(ctx.projectType.value)
								? {
										clientSide: result.client_side as Labrinth.Projects.v2.Environment,
										serverSide: result.server_side as Labrinth.Projects.v2.Environment,
									}
								: undefined
						"
						:layout="ctx.effectiveLayout.value"
						@contextmenu.prevent.stop="(event: MouseEvent) => ctx.onContextMenu?.(event, result)"
						@mouseenter="ctx.onProjectHover?.(result)"
						@mouseleave="ctx.onProjectHoverEnd?.()"
					>
						<template v-if="ctx.getCardActions?.(result, ctx.projectType.value)?.length" #actions>
							<div class="flex gap-2">
								<ButtonStyled
									v-for="action in ctx.getCardActions(result, ctx.projectType.value)"
									:key="action.key"
									:color="action.color"
									:type="action.type"
									:size="ctx.effectiveLayout.value === 'compact' ? 'small' : 'standard'"
									:circular="action.circular"
								>
									<button
										v-tooltip="action.tooltip"
										:disabled="action.disabled"
										@click.stop="action.onClick"
									>
										<component :is="action.icon" :class="action.iconClass" />
										<template v-if="!action.circular">{{
											ctx.effectiveLayout.value === 'compact'
												? (action.compactLabel ?? action.label)
												: action.label
										}}</template>
									</button>
								</ButtonStyled>
							</div>
						</template>
					</ProjectCard>
				</ContentCardReveal>
			</template>
		</ProjectCardList>

		<div :class="ctx.variant === 'web' ? 'pagination-after mt-3' : 'flex justify-end mt-3'">
			<Pagination
				:page="ctx.currentPage.value"
				:count="ctx.pageCount.value"
				:loading="ctx.loading.value"
				:class="ctx.variant === 'web' ? 'justify-end' : 'pagination-after'"
				@switch-page="ctx.setPage"
			/>
		</div>
	</div>

	<slot name="after" />
</template>
