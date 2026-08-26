<script setup lang="ts">
import {
	CheckIcon,
	ChevronDownIcon,
	CopyIcon,
	EditIcon,
	ExternalIcon,
	GithubIcon,
	GlobeIcon,
	HeartHandshakeIcon,
	IssuesIcon,
	ScaleIcon,
	UsersIcon,
} from '@modrinth/assets'
import { Avatar, defineMessages, NewButton as Button, useVIntl } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { inject, ref } from 'vue'

import AfdianIcon from '@/assets/external/afdian.png'
import QqIcon from '@/assets/external/qq.svg?component'
import { AxolotlBrandConfig } from '@/config'
import { contributors, teamMembers } from '@/data/about'

import QqChannelIcon from './QqChannelIcon.vue'

const { formatMessage } = useVIntl()
const version = await getVersion()
const copied = ref(false)
const replayOnboarding = inject<(mode: 'main' | 'instance') => Promise<void>>('replayOnboarding')

const licenseUrl = `${AxolotlBrandConfig.repositoryUrl}/blob/main/LICENSE`
const thirdPartyLicensesUrl = `${AxolotlBrandConfig.repositoryUrl}/tree/main/third-party/licenses`

async function copyQqGroupNumber() {
	await navigator.clipboard.writeText(AxolotlBrandConfig.qqGroupNumber)
	copied.value = true
	setTimeout(() => {
		copied.value = false
	}, 3000)
}

const messages = defineMessages({
	productTitle: {
		id: 'app.settings.about.product-title',
		defaultMessage: 'About {productName}',
	},
	productDescription: {
		id: 'app.settings.about.description',
		defaultMessage: 'Your last launcher.',
	},
	version: {
		id: 'app.settings.about.version',
		defaultMessage: 'Version {version}',
	},
	replayOnboarding: {
		id: 'app.settings.about.replay-onboarding',
		defaultMessage: 'Replay tour',
	},
	developmentTeam: {
		id: 'app.settings.about.development-team',
		defaultMessage: 'Development team',
	},
	communitySupport: {
		id: 'app.settings.about.community-support',
		defaultMessage: 'Project & community',
	},
	projectWebsite: {
		id: 'app.settings.about.project-website',
		defaultMessage: 'Project website',
	},
	repository: {
		id: 'app.settings.about.repository',
		defaultMessage: 'Source code',
	},
	reportIssue: {
		id: 'app.settings.about.report-issue',
		defaultMessage: 'Issues & feedback',
	},
	qqGroup: {
		id: 'app.settings.about.qq-group',
		defaultMessage: 'Player QQ group',
	},
	qqChannel: {
		id: 'app.settings.about.qq-channel',
		defaultMessage: 'QQ channel',
	},
	copyQqGroup: {
		id: 'app.settings.about.copy-qq-group',
		defaultMessage: 'Copy group number',
	},
	copiedQqGroup: {
		id: 'app.settings.about.copied-qq-group',
		defaultMessage: 'Group number copied',
	},
	afdian: {
		id: 'app.settings.about.afdian',
		defaultMessage: 'Support on Afdian',
	},
	afdianDescription: {
		id: 'app.settings.about.afdian-description',
		defaultMessage: 'Help support continued development',
	},
	survey: {
		id: 'app.settings.about.survey',
		defaultMessage: 'Community survey',
	},
	surveyDescription: {
		id: 'app.settings.about.survey-description',
		defaultMessage: 'Help us improve Axolotl Launcher',
	},
	licenseAttribution: {
		id: 'app.settings.about.license-attribution',
		defaultMessage: 'License & attribution',
	},
	attribution: {
		id: 'app.settings.about.attribution',
		defaultMessage: 'Axolotl Launcher is a modified version of the open-source Modrinth codebase.',
	},
	notAffiliated: {
		id: 'app.settings.about.not-affiliated',
		defaultMessage:
			'Modrinth is a trademark of Rinth, Inc. Axolotl Launcher is not affiliated with or endorsed by Rinth, Inc.',
	},
	originalSource: {
		id: 'app.settings.about.original-source',
		defaultMessage: 'Original Modrinth source',
	},
	projectLicense: {
		id: 'app.settings.about.project-license',
		defaultMessage: 'Project license (GPL-3.0)',
	},
	thirdPartyLicenses: {
		id: 'app.settings.about.third-party-licenses',
		defaultMessage: 'Third-party licenses',
	},
	contributors: {
		id: 'app.settings.about.contributors',
		defaultMessage: 'Contributors',
	},
	contributorsCount: {
		id: 'app.settings.about.contributors-count',
		defaultMessage: '{count, plural, one {# contributor} other {# contributors}}',
	},
})

const projectLinks = [
	{
		href: AxolotlBrandConfig.website,
		label: messages.projectWebsite,
		icon: GlobeIcon,
	},
	{
		href: AxolotlBrandConfig.repositoryUrl,
		label: messages.repository,
		icon: GithubIcon,
	},
	{
		href: AxolotlBrandConfig.supportUrl,
		label: messages.reportIssue,
		icon: IssuesIcon,
	},
	{
		href: AxolotlBrandConfig.qqChannelUrl,
		label: messages.qqChannel,
		icon: QqChannelIcon,
	},
]
</script>

<template>
	<div class="about-page flex flex-col gap-6">
		<section id="settings-target-about-product" tabindex="-1" class="about-panel">
			<div class="flex items-center gap-4">
				<img class="size-20 object-contain" src="@/assets/axolotl.png" alt="" />
				<div class="min-w-0">
					<h2 class="m-0 text-xl font-semibold text-contrast">
						{{
							formatMessage(messages.productTitle, {
								productName: AxolotlBrandConfig.productName,
							})
						}}
					</h2>
					<p class="m-0 mt-1 text-secondary">
						{{ formatMessage(messages.version, { version }) }}
					</p>
				</div>
			</div>
			<p class="m-0 mt-4 text-primary">
				{{ formatMessage(messages.productDescription) }}
			</p>
		</section>

		<section>
			<h3 class="m-0 mb-3 flex items-center gap-2 text-base font-semibold text-contrast">
				<UsersIcon class="size-5 text-secondary" />
				{{ formatMessage(messages.developmentTeam) }}
			</h3>
			<ul class="m-0 grid list-none grid-cols-2 gap-3 p-0 sm:grid-cols-3">
				<li v-for="member in teamMembers" :key="member.name" class="min-w-0">
					<component
						:is="member.url ? 'a' : 'div'"
						:href="member.url"
						:target="member.url ? '_blank' : undefined"
						:rel="member.url ? 'noopener noreferrer' : undefined"
						class="flex min-w-0 flex-col items-center gap-3 rounded-xl bg-surface-4 p-4"
						:class="member.url ? 'transition-colors hover:bg-surface-5' : 'cursor-default'"
					>
						<Avatar :src="member.avatarUrl" :alt="member.name" size="4rem" circle no-shadow />
						<span class="block truncate text-center font-semibold text-contrast">{{
							member.name
						}}</span>
					</component>
				</li>
			</ul>
		</section>

		<section>
			<h3 class="m-0 mb-3 flex items-center gap-2 text-base font-semibold text-contrast">
				<HeartHandshakeIcon class="size-5 text-secondary" />
				{{ formatMessage(messages.communitySupport) }}
			</h3>
			<div class="grid gap-3 sm:grid-cols-2">
				<a
					v-for="link in projectLinks"
					:key="link.label"
					:href="link.href"
					target="_blank"
					rel="noopener noreferrer"
					class="flex min-w-0 items-center gap-3 rounded-xl bg-surface-4 p-4 transition-colors hover:bg-surface-5"
				>
					<span
						class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-surface-2 text-contrast"
					>
						<component :is="link.icon" class="size-6" />
					</span>
					<span class="min-w-0 flex-1 font-semibold text-contrast">
						{{ formatMessage(link.label) }}
					</span>
					<ExternalIcon class="size-5 shrink-0 text-secondary" />
				</a>

				<button
					type="button"
					:disabled="copied"
					:aria-label="
						copied ? formatMessage(messages.copiedQqGroup) : formatMessage(messages.copyQqGroup)
					"
					class="flex min-w-0 items-center gap-3 rounded-xl bg-surface-4 p-4 text-left transition-colors hover:bg-surface-5 disabled:cursor-default"
					@click="copyQqGroupNumber"
				>
					<span
						class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-surface-2 text-contrast"
					>
						<QqIcon class="size-6" />
					</span>
					<span class="min-w-0 flex-1">
						<span class="block font-semibold text-contrast">
							{{ formatMessage(messages.qqGroup) }}
						</span>
						<span class="block text-sm text-secondary">
							{{ AxolotlBrandConfig.qqGroupNumber }}
						</span>
					</span>
					<span class="shrink-0" aria-live="polite">
						<CheckIcon v-if="copied" class="size-5 text-green" />
						<CopyIcon v-else class="size-5 text-secondary" />
						<span class="sr-only">
							{{
								copied ? formatMessage(messages.copiedQqGroup) : formatMessage(messages.copyQqGroup)
							}}
						</span>
					</span>
				</button>

				<a
					:href="AxolotlBrandConfig.sponsorUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="flex min-w-0 items-center gap-3 rounded-xl bg-surface-4 p-4 transition-colors hover:bg-surface-5"
				>
					<span class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-surface-2">
						<img :src="AfdianIcon" alt="" class="size-7 object-contain" />
					</span>
					<span class="min-w-0 flex-1">
						<span class="block font-semibold text-contrast">
							{{ formatMessage(messages.afdian) }}
						</span>
						<span class="block text-sm text-secondary">
							{{ formatMessage(messages.afdianDescription) }}
						</span>
					</span>
					<ExternalIcon class="size-5 shrink-0 text-secondary" />
				</a>

				<a
					:href="AxolotlBrandConfig.surveyUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="flex min-w-0 items-center gap-3 rounded-xl bg-surface-4 p-4 transition-colors hover:bg-surface-5 sm:col-span-2"
				>
					<span
						class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-surface-2 text-contrast"
					>
						<EditIcon class="size-6" />
					</span>
					<span class="min-w-0 flex-1">
						<span class="block font-semibold text-contrast">
							{{ formatMessage(messages.survey) }}
						</span>
						<span class="block text-sm text-secondary">
							{{ formatMessage(messages.surveyDescription) }}
						</span>
					</span>
					<ExternalIcon class="size-5 shrink-0 text-secondary" />
				</a>
			</div>
		</section>

		<section>
			<h3 class="m-0 mb-3 flex items-center gap-2 text-base font-semibold text-contrast">
				<ScaleIcon class="size-5 text-secondary" />
				{{ formatMessage(messages.licenseAttribution) }}
			</h3>
			<div class="about-panel about-panel-compact">
				<p class="m-0 text-primary">
					{{ formatMessage(messages.attribution) }}
				</p>
				<p class="m-0 mt-2 text-sm text-secondary">
					{{ formatMessage(messages.notAffiliated) }}
				</p>
			</div>
			<div class="mt-3 flex flex-wrap gap-2">
				<a
					:href="licenseUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-contrast transition-colors hover:bg-surface-5"
				>
					{{ formatMessage(messages.projectLicense) }}
					<ExternalIcon class="size-4 text-secondary" />
				</a>
				<a
					:href="thirdPartyLicensesUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-contrast transition-colors hover:bg-surface-5"
				>
					{{ formatMessage(messages.thirdPartyLicenses) }}
					<ExternalIcon class="size-4 text-secondary" />
				</a>
				<a
					href="https://github.com/modrinth/code"
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-contrast transition-colors hover:bg-surface-5"
				>
					{{ formatMessage(messages.originalSource) }}
					<ExternalIcon class="size-4 text-secondary" />
				</a>
			</div>
		</section>

		<details class="group pt-4 about-settings-details">
			<summary
				class="flex cursor-pointer list-none items-center gap-2 text-base font-semibold text-contrast [&::-webkit-details-marker]:hidden"
			>
				<UsersIcon class="size-5 text-secondary" />
				<span>{{ formatMessage(messages.contributors) }}</span>
				<span class="rounded-full bg-surface-4 px-2 py-0.5 text-xs text-secondary">
					{{ formatMessage(messages.contributorsCount, { count: contributors.length }) }}
				</span>
				<ChevronDownIcon
					class="ml-auto size-5 text-secondary transition-transform group-open:rotate-180"
				/>
			</summary>
			<div class="mt-3 flex flex-wrap gap-2">
				<a
					v-for="contributor in contributors"
					:key="contributor.name"
					:href="contributor.url"
					target="_blank"
					rel="noopener noreferrer"
					class="flex min-w-0 items-center gap-1.5 rounded-full bg-surface-4 py-1 pl-1 pr-2.5 transition-colors hover:bg-surface-5"
				>
					<Avatar
						:src="contributor.avatarUrl"
						:alt="contributor.name"
						size="1.5rem"
						circle
						no-shadow
						loading="lazy"
					/>
					<span class="truncate text-sm text-primary">{{ contributor.name }}</span>
				</a>
			</div>
		</details>

		<div id="settings-target-about-replay-tour" tabindex="-1" class="flex flex-wrap gap-2">
			<Button type="base" @click="replayOnboarding?.('main')">
				{{ formatMessage(messages.replayOnboarding) }}
			</Button>
		</div>
	</div>
</template>

<style scoped>
.about-settings-details {
	border-top: 1px solid
		var(--settings-divider, color-mix(in srgb, var(--surface-4) 55%, transparent));
}

.about-panel {
	padding: 1.25rem;
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
}

.about-panel-compact {
	padding: var(--gap-lg);
}

.about-page :deep(.rounded-xl.bg-surface-4) {
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
}

.about-page :deep(.rounded-xl.bg-surface-2) {
	border-radius: var(--radius-sm);
}
</style>
