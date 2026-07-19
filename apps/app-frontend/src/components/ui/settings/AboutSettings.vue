<script setup lang="ts">
import { CheckIcon, CopyIcon, ExternalIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { ref } from 'vue'

import AfdianIcon from '@/assets/external/afdian.png'
import QqIcon from '@/assets/external/qq.svg?component'
import { AxolotlBrandConfig } from '@/config'

const { formatMessage } = useVIntl()
const version = await getVersion()
const copied = ref(false)

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
	version: {
		id: 'app.settings.about.version',
		defaultMessage: 'Version {version}',
	},
	developer: {
		id: 'app.settings.about.developer',
		defaultMessage: 'Developed by {developerName} at {organizationName}.',
	},
	attribution: {
		id: 'app.settings.about.attribution',
		defaultMessage: 'This application is a modified version of the open-source Modrinth project.',
	},
	communitySupport: {
		id: 'app.settings.about.community-support',
		defaultMessage: 'Community & support',
	},
	qqGroup: {
		id: 'app.settings.about.qq-group',
		defaultMessage: 'Player QQ group',
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
	originalSource: {
		id: 'app.settings.about.original-source',
		defaultMessage: 'View the original Modrinth source code',
	},
	projectWebsite: {
		id: 'app.settings.about.project-website',
		defaultMessage: 'Visit the project website',
	},
	contentSearchAttribution: {
		id: 'app.settings.about.content-search-attribution',
		defaultMessage:
			'Chinese content search uses project-name data from Plain Craft Launcher and MC Encyclopedia.',
	},
	pclSource: {
		id: 'app.settings.about.pcl-source',
		defaultMessage: 'View the Plain Craft Launcher source and license',
	},
	mcModWebsite: {
		id: 'app.settings.about.mcmod-website',
		defaultMessage: 'Visit MC Encyclopedia',
	},
})
</script>

<template>
	<div class="flex flex-col gap-6">
		<div class="flex items-center gap-4">
			<img class="size-20 object-contain" src="@/assets/axolotl.png" alt="" />
			<div>
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

		<div class="rounded-xl bg-surface-4 p-4">
			<p class="m-0 text-primary">
				{{
					formatMessage(messages.developer, {
						developerName: AxolotlBrandConfig.developerName,
						organizationName: AxolotlBrandConfig.organizationName,
					})
				}}
			</p>
			<p class="m-0 mt-3 text-primary">
				{{ formatMessage(messages.attribution) }}
			</p>
			<p class="m-0 mt-3 text-primary">
				{{ formatMessage(messages.contentSearchAttribution) }}
			</p>
		</div>

		<div>
			<h3 class="m-0 mb-3 text-base font-semibold text-contrast">
				{{ formatMessage(messages.communitySupport) }}
			</h3>
			<div class="grid gap-3 sm:grid-cols-2">
				<button
					type="button"
					:disabled="copied"
					:aria-label="
						copied
							? formatMessage(messages.copiedQqGroup)
							: formatMessage(messages.copyQqGroup)
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
								copied
									? formatMessage(messages.copiedQqGroup)
									: formatMessage(messages.copyQqGroup)
							}}
						</span>
					</span>
				</button>

				<a
					:href="AxolotlBrandConfig.sponsorUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="flex min-w-0 items-center gap-3 rounded-xl bg-surface-4 p-4 text-left transition-colors hover:bg-surface-5"
				>
					<span
						class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-surface-2"
					>
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
			</div>
		</div>

		<div class="flex flex-col items-start gap-3">
			<a
				href="https://github.com/modrinth/code"
				target="_blank"
				rel="noopener noreferrer"
				class="inline-flex items-center gap-2 font-semibold text-brand hover:underline"
			>
				{{ formatMessage(messages.originalSource) }}
				<ExternalIcon class="size-4" />
			</a>
			<a
				:href="AxolotlBrandConfig.website"
				target="_blank"
				rel="noopener noreferrer"
				class="inline-flex items-center gap-2 font-semibold text-brand hover:underline"
			>
				{{ formatMessage(messages.projectWebsite) }}
				<ExternalIcon class="size-4" />
			</a>
			<a
				href="https://github.com/Meloong-Git/PCL/tree/fd7b722346523d9574678a8a4a02928d31cd1e0c"
				target="_blank"
				rel="noopener noreferrer"
				class="inline-flex items-center gap-2 font-semibold text-brand hover:underline"
			>
				{{ formatMessage(messages.pclSource) }}
				<ExternalIcon class="size-4" />
			</a>
			<a
				href="https://www.mcmod.cn/"
				target="_blank"
				rel="noopener noreferrer"
				class="inline-flex items-center gap-2 font-semibold text-brand hover:underline"
			>
				{{ formatMessage(messages.mcModWebsite) }}
				<ExternalIcon class="size-4" />
			</a>
		</div>
	</div>
</template>
