<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'
import { Admonition, BulletDivider, ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed } from 'vue'

import {
	ANNOUNCEMENT_CHANGE_TYPES,
	type AnnouncementChangeType,
	getLocalizedAnnouncementText,
	type LauncherAnnouncement,
} from '@/announcements/catalog'
import i18n from '@/i18n.config'

const props = withDefaults(
	defineProps<{
		announcement?: LauncherAnnouncement
		version?: string | null
		externalUrl?: string
		showHeader?: boolean
	}>(),
	{
		announcement: undefined,
		version: null,
		externalUrl: undefined,
		showHeader: true,
	},
)

const { formatMessage } = useVIntl()

const messages = defineMessages({
	unknownTitle: {
		id: 'app.update-announcement.unknown-title',
		defaultMessage: 'Axolotl Launcher was updated',
	},
	unknownBody: {
		id: 'app.update-announcement.unknown-body',
		defaultMessage:
			'This version does not have a bundled announcement yet. Visit the website for the full changelog.',
	},
	version: {
		id: 'app.update-announcement.version',
		defaultMessage: 'Version {version}',
	},
	openChangelog: {
		id: 'app.update-announcement.open-changelog',
		defaultMessage: 'Open full changelog',
	},
	notes: {
		id: 'app.update-announcement.notes',
		defaultMessage: 'Notes',
	},
	added: {
		id: 'app.update-announcement.category.added',
		defaultMessage: 'Added',
	},
	changed: {
		id: 'app.update-announcement.category.changed',
		defaultMessage: 'Changed',
	},
	deprecated: {
		id: 'app.update-announcement.category.deprecated',
		defaultMessage: 'Deprecated',
	},
	removed: {
		id: 'app.update-announcement.category.removed',
		defaultMessage: 'Removed',
	},
	fixed: {
		id: 'app.update-announcement.category.fixed',
		defaultMessage: 'Fixed',
	},
	security: {
		id: 'app.update-announcement.category.security',
		defaultMessage: 'Security',
	},
})

const categoryClasses: Record<AnnouncementChangeType, string> = {
	added: 'bg-brand-green',
	changed: 'bg-brand-blue',
	deprecated: 'bg-brand-orange',
	removed: 'bg-brand-red',
	fixed: 'bg-brand-purple',
	security: 'bg-brand-orange',
}

const locale = computed(() => i18n.global.locale.value)
const title = computed(() =>
	props.announcement
		? getLocalizedAnnouncementText(props.announcement.title, locale.value)
		: formatMessage(messages.unknownTitle),
)
const versionLabel = computed(() =>
	formatMessage(messages.version, { version: props.announcement?.version ?? props.version ?? '—' }),
)
const categoryRows = computed(() =>
	ANNOUNCEMENT_CHANGE_TYPES.flatMap((type) => {
		const changes = props.announcement?.changes[type]
		if (!changes?.length) return []
		return [
			{
				type,
				label: formatMessage(messages[type]),
				className: categoryClasses[type],
				changes: changes.map((change) => getLocalizedAnnouncementText(change, locale.value)),
			},
		]
	}),
)

async function openChangelog() {
	if (props.externalUrl) await openUrl(props.externalUrl)
}
</script>

<template>
	<div class="flex min-w-0 flex-col gap-5 text-primary">
		<header v-if="showHeader" class="flex min-w-0 flex-col gap-2">
			<h2 class="m-0 break-words text-xl font-semibold text-contrast">{{ title }}</h2>
			<div class="flex flex-wrap items-center gap-2 text-sm text-secondary">
				<span>{{ versionLabel }}</span>
				<BulletDivider v-if="announcement?.publishedAt" />
				<time v-if="announcement?.publishedAt" :datetime="announcement.publishedAt">
					{{ announcement.publishedAt }}
				</time>
			</div>
		</header>

		<div v-if="categoryRows.length" class="flex flex-col">
			<section
				v-for="(category, index) in categoryRows"
				:key="category.type"
				class="announcement-category grid grid-cols-1 gap-2 py-4 sm:grid-cols-[7rem_minmax(0,1fr)] sm:gap-5"
				:class="{ 'border-t-0 pt-0': index === 0 }"
			>
				<h3 class="m-0 flex items-center gap-2 text-sm font-semibold text-secondary">
					<span
						class="size-2 shrink-0 rounded-full"
						:class="category.className"
						aria-hidden="true"
					/>
					{{ category.label }}
				</h3>
				<ul class="m-0 flex list-disc flex-col gap-2 pl-5 leading-relaxed text-primary">
					<li v-for="change in category.changes" :key="change">{{ change }}</li>
				</ul>
			</section>
		</div>
		<Admonition v-else type="info" :body="formatMessage(messages.unknownBody)" />

		<div v-if="announcement?.notes" class="announcement-notes">
			<h3 class="m-0 mb-2 text-sm font-semibold text-secondary">
				{{ formatMessage(messages.notes) }}
			</h3>
			<p class="m-0 leading-relaxed text-primary">
				{{ getLocalizedAnnouncementText(announcement.notes, locale) }}
			</p>
		</div>

		<ButtonStyled v-if="externalUrl" color="brand" type="outlined" class="self-start">
			<button type="button" @click="openChangelog">
				<ExternalIcon />
				{{ formatMessage(messages.openChangelog) }}
			</button>
		</ButtonStyled>
	</div>
</template>

<style scoped>
.announcement-category {
	border-top: 1px solid
		var(--settings-divider, color-mix(in srgb, var(--surface-4) 55%, transparent));
}

.announcement-notes {
	padding-top: var(--gap-md);
	border-top: 1px solid
		var(--settings-divider, color-mix(in srgb, var(--surface-4) 55%, transparent));
}
</style>
