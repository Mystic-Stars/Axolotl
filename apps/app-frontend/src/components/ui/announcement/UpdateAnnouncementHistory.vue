<script setup lang="ts">
import { CalendarIcon, HistoryIcon } from '@modrinth/assets'
import { Accordion, defineMessages, TagItem, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import {
	getAnnouncementByVersion,
	getAnnouncements,
	getLocalizedAnnouncementText,
} from '@/announcements/catalog'
import { AxolotlBrandConfig } from '@/config'
import i18n from '@/i18n.config'

import UpdateAnnouncementContent from './UpdateAnnouncementContent.vue'

const props = defineProps<{
	currentVersion: string
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'app.settings.updates.announcements.title',
		defaultMessage: 'Update announcements',
	},
	description: {
		id: 'app.settings.updates.announcements.description',
		defaultMessage: 'See what changed in this version and browse previous releases.',
	},
	history: {
		id: 'app.settings.updates.announcements.history',
		defaultMessage: 'Version history',
	},
	empty: {
		id: 'app.settings.updates.announcements.empty',
		defaultMessage: 'No bundled update announcements are available.',
	},
})

const locale = computed(() => i18n.global.locale.value)
const launcherAnnouncements = getAnnouncements()
const currentAnnouncement = computed(() => getAnnouncementByVersion(props.currentVersion))
const historyAnnouncements = computed(() =>
	launcherAnnouncements.filter((announcement) => announcement.id !== currentAnnouncement.value?.id),
)
</script>

<template>
	<section class="update-announcement-history">
		<div class="flex min-w-0 flex-col gap-1">
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.title) }}
			</h2>
			<p class="m-0 leading-relaxed text-secondary">
				{{ formatMessage(messages.description) }}
			</p>
		</div>

		<div class="min-w-0">
			<UpdateAnnouncementContent
				:announcement="currentAnnouncement"
				:version="currentVersion"
				:external-url="currentAnnouncement?.externalUrl ?? AxolotlBrandConfig.website"
			/>
		</div>

		<div class="flex min-w-0 flex-col gap-3">
			<h3 class="m-0 flex items-center gap-2 text-base font-semibold text-contrast">
				<HistoryIcon aria-hidden="true" class="size-4 text-secondary" />
				{{ formatMessage(messages.history) }}
			</h3>
			<p v-if="historyAnnouncements.length === 0" class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.empty) }}
			</p>
			<div v-else class="flex min-w-0 flex-col gap-2">
				<Accordion
					v-for="announcement in historyAnnouncements"
					:key="announcement.id"
					class="update-announcement-history-item hover:border-surface-4 focus-within:border-surface-4"
					button-class="group flex w-full cursor-pointer items-center gap-3 border-0 bg-transparent px-4 py-3 text-left"
				>
					<template #title>
						<div class="flex min-w-0 flex-1 items-center gap-3">
							<div class="flex min-w-0 flex-1 flex-col gap-1">
								<span
									class="truncate font-semibold text-primary transition-colors group-hover:text-contrast"
								>
									{{ getLocalizedAnnouncementText(announcement.title, locale) }}
								</span>
								<div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-secondary">
									<TagItem class="px-1.5 py-0.5 text-xs">v{{ announcement.version }}</TagItem>
									<span class="flex items-center gap-1">
										<CalendarIcon aria-hidden="true" class="size-3.5" />
										<time :datetime="announcement.publishedAt">{{ announcement.publishedAt }}</time>
									</span>
								</div>
							</div>
						</div>
					</template>
					<div class="update-announcement-history-item-content">
						<UpdateAnnouncementContent
							:announcement="announcement"
							:show-header="false"
							:external-url="announcement.externalUrl"
						/>
					</div>
				</Accordion>
			</div>
		</div>
	</section>
</template>

<style scoped>
.update-announcement-history {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: var(--gap-xl);
	padding: var(--gap-xl);
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
}

.update-announcement-history-item {
	min-width: 0;
	overflow: hidden;
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-sm);
	background: var(--surface-3);
	transition: border-color 120ms ease;
}

.update-announcement-history-item-content {
	padding: var(--gap-lg);
	border-top: 1px solid
		var(--settings-divider, color-mix(in srgb, var(--surface-4) 55%, transparent));
}
</style>
