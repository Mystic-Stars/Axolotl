<script setup lang="ts">
import { ServerIcon } from '@modrinth/assets'
import { requiredJavaMajorVersion } from '@modrinth/server'
import { defineMessages, Slider, StyledInput, useVIntl } from '@modrinth/ui'
import { computed, onMounted } from 'vue'

import JavaSelector from '@/components/ui/JavaSelector.vue'

import { injectCreateServerFlow } from '../create-server-flow'

const { formatMessage } = useVIntl()
const ctx = injectCreateServerFlow()

const messages = defineMessages({
	name: { id: 'app.servers.wizard.name', defaultMessage: 'Server name' },
	namePlaceholder: {
		id: 'app.servers.wizard.name-placeholder',
		defaultMessage: 'Survival server',
	},
	java: { id: 'app.servers.settings.java', defaultMessage: 'Java' },
	memory: { id: 'app.servers.settings.memory', defaultMessage: 'Memory' },
	memoryValue: { id: 'app.servers.wizard.memory-value', defaultMessage: '{value} MB' },
})

const requiredJava = computed(() =>
	requiredJavaMajorVersion(ctx.selectedGameVersion.value || '1.21'),
)

function suggestName() {
	const type = ctx.serverType.value
	const version = ctx.selectedGameVersion.value
	const flag = Math.random().toString(16).slice(2, 6)
	const segments = [type, version]
	if (ctx.selectedLoaderVersion.value) segments.push(ctx.selectedLoaderVersion.value)
	segments.push(flag)
	ctx.name.value = segments.filter(Boolean).join('-')
}

onMounted(() => {
	void ctx.loadDefaultJava()
	if (!ctx.name.value.trim() && ctx.selectedGameVersion.value) {
		suggestName()
	}
})
</script>

<template>
	<div class="flex flex-col gap-5">
		<label class="flex min-w-0 flex-col gap-2" for="wizard-server-name">
			<span class="font-semibold text-contrast">{{ formatMessage(messages.name) }}</span>
			<StyledInput
				id="wizard-server-name"
				v-model="ctx.name.value"
				:icon="ServerIcon"
				:placeholder="formatMessage(messages.namePlaceholder)"
			/>
		</label>

		<div class="flex min-w-0 flex-col gap-2">
			<span class="font-semibold text-contrast">{{ formatMessage(messages.java) }}</span>
			<JavaSelector
				id="wizard-java-selector"
				v-model="ctx.selectedJava.value"
				:version="requiredJava"
				select-all-versions
			/>
		</div>

		<div class="flex min-w-0 flex-col gap-2">
			<div class="flex items-center justify-between gap-3">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.memory) }}</span>
				<span
					class="rounded-md border border-solid border-surface-5 bg-surface-3 px-2 py-1 text-xs font-semibold leading-none text-contrast"
				>
					{{ formatMessage(messages.memoryValue, { value: ctx.memoryMb.value }) }}
				</span>
			</div>
			<Slider v-model="ctx.memoryMb.value" :min="1024" :max="ctx.maxMemoryMb.value" :step="512" />
		</div>
	</div>
</template>
