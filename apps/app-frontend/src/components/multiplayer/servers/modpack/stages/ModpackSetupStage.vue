<script setup lang="ts">
import { ServerIcon } from '@modrinth/assets'
import { requiredJavaMajorVersion } from '@modrinth/server'
import {
	Admonition,
	Avatar,
	defineMessages,
	Slider,
	StyledInput,
	TagItem,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted } from 'vue'

import JavaSelector from '@/components/ui/JavaSelector.vue'

import { injectModpackServerFlow } from '../create-modpack-server-flow'

const { formatMessage } = useVIntl()
const ctx = injectModpackServerFlow()

const messages = defineMessages({
	name: { id: 'app.servers.wizard.name', defaultMessage: 'Server name' },
	java: { id: 'app.servers.settings.java', defaultMessage: 'Java' },
	memory: { id: 'app.servers.settings.memory', defaultMessage: 'Memory' },
	memoryValue: { id: 'app.servers.wizard.memory-value', defaultMessage: '{value} MB' },
	unsupportedLoaderTitle: {
		id: 'app.servers.modpack.unsupported-loader-title',
		defaultMessage: '{loader} servers are not supported yet',
	},
	unsupportedLoaderDescription: {
		id: 'app.servers.modpack.unsupported-loader-description',
		defaultMessage:
			'This modpack uses {loader}, but Axolotl can only start modpack servers with vanilla, Fabric, or Quilt. Support for {loader} is coming soon.',
	},
})

const requiredJava = computed(() =>
	requiredJavaMajorVersion(ctx.selectedGameVersion.value || '1.21'),
)

onMounted(() => {
	void ctx.loadDefaultJava()
})
</script>

<template>
	<div class="flex flex-col gap-5">
		<div
			class="flex items-center gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-3"
		>
			<Avatar
				:src="ctx.modpackIconUrl.value"
				:alt="ctx.modpackTitle.value"
				size="56px"
				class="shrink-0"
			/>
			<div class="min-w-0 flex-1">
				<p class="m-0 truncate text-base font-bold leading-tight text-[var(--color-text-primary)]">
					{{ ctx.modpackTitle.value }}
				</p>
				<p class="m-0 mt-0.5 truncate text-sm font-medium text-[var(--color-text-tertiary)]">
					{{ ctx.modpackVersionNumber.value }}
				</p>
				<div class="mt-1.5 flex flex-wrap items-center gap-1.5">
					<TagItem>
						<span class="font-semibold">{{ ctx.loaderLabel.value }}</span>
					</TagItem>
					<TagItem v-if="ctx.gameVersionLabel.value">
						<span class="font-semibold">{{ ctx.gameVersionLabel.value }}</span>
					</TagItem>
				</div>
			</div>
		</div>

		<Admonition
			v-if="!ctx.loaderSupported.value"
			type="critical"
			:header="
				formatMessage(messages.unsupportedLoaderTitle, {
					loader: ctx.loaderLabel.value,
				})
			"
		>
			{{
				formatMessage(messages.unsupportedLoaderDescription, {
					loader: ctx.loaderLabel.value,
				})
			}}
		</Admonition>

		<label class="flex min-w-0 flex-col gap-2" for="modpack-server-name">
			<span class="font-semibold text-[var(--color-text-primary)]">{{
				formatMessage(messages.name)
			}}</span>
			<StyledInput
				id="modpack-server-name"
				v-model="ctx.name.value"
				:icon="ServerIcon"
				:placeholder="ctx.modpackTitle.value"
			/>
		</label>

		<div class="flex min-w-0 flex-col gap-2">
			<span class="font-semibold text-[var(--color-text-primary)]">{{
				formatMessage(messages.java)
			}}</span>
			<JavaSelector
				id="modpack-java-selector"
				v-model="ctx.selectedJava.value"
				:version="requiredJava"
				select-all-versions
			/>
		</div>

		<div class="flex min-w-0 flex-col gap-2">
			<div class="flex items-center justify-between gap-3">
				<span class="font-semibold text-[var(--color-text-primary)]">{{
					formatMessage(messages.memory)
				}}</span>
				<span
					class="rounded-md border border-solid border-surface-5 bg-surface-3 px-2 py-1 text-xs font-semibold leading-none text-[var(--color-text-primary)]"
				>
					{{ formatMessage(messages.memoryValue, { value: ctx.memoryMb.value }) }}
				</span>
			</div>
			<Slider v-model="ctx.memoryMb.value" :min="1024" :max="ctx.maxMemoryMb.value" :step="512" />
		</div>
	</div>
</template>
