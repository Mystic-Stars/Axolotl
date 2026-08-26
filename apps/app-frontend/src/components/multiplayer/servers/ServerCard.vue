<script setup lang="ts">
import { PlayIcon, StopCircleIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, TagItem, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import {
	isServerStatusVisible,
	SERVER_STATUS_META,
} from '@/components/multiplayer/servers/server-status'
import ServerIcon from '@/components/multiplayer/servers/ServerIcon.vue'
import type { ServerView } from '@/composables/useServers'

const props = defineProps<{
	server: ServerView
	variant: 'standard' | 'library'
}>()

const emit = defineEmits<{
	open: []
	'start-stop': []
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	start: { id: 'app.servers.action.start', defaultMessage: 'Start' },
	stop: { id: 'app.servers.action.stop', defaultMessage: 'Stop' },
})

const statusMeta = computed(() => SERVER_STATUS_META[props.server.status])
const showStatus = computed(() => isServerStatusVisible(props.server.status))
</script>

<template>
	<div
		v-if="variant === 'library'"
		data-onboarding-id="server-card"
		role="button"
		tabindex="0"
		class="group relative flex w-full cursor-pointer select-none flex-col items-start justify-end gap-3 overflow-clip rounded-[20px] border border-solid border-surface-4 bg-surface-3 p-3 text-left transition-[border-color,filter,transform] hover:border-surface-5 hover:brightness-110 active:scale-[0.98]"
		@click="emit('open')"
		@keydown.enter="emit('open')"
		@keydown.space.prevent="emit('open')"
	>
		<div
			class="relative flex aspect-square w-full shrink-0 items-center justify-center overflow-clip rounded-2xl bg-surface-2"
		>
			<ServerIcon
				:icon-path="server.iconPath"
				:server-type="server.serverType"
				:server-id="server.id"
				size="96px"
			/>
			<TagItem v-if="showStatus" class="absolute left-3 top-3">
				<span :class="'font-semibold ' + statusMeta.color">
					{{ formatMessage(statusMeta.label) }}
				</span>
			</TagItem>
			<div class="absolute bottom-1.5 right-1.5" @click.stop @keydown.stop>
				<ButtonStyled v-if="server.status !== 'running'" color="brand" size="large" circular>
					<button
						v-tooltip="formatMessage(messages.start)"
						type="button"
						class="scale-75 opacity-0 transition-all group-hover:scale-100 group-hover:opacity-100 group-focus-within:scale-100 group-focus-within:opacity-100"
						@click="emit('start-stop')"
					>
						<PlayIcon class="translate-x-[1px]" />
					</button>
				</ButtonStyled>
				<ButtonStyled v-else color="red" size="large" circular>
					<button
						v-tooltip="formatMessage(messages.stop)"
						type="button"
						class="scale-75 opacity-0 transition-all group-hover:scale-100 group-hover:opacity-100 group-focus-within:scale-100 group-focus-within:opacity-100"
						@click="emit('start-stop')"
					>
						<StopCircleIcon />
					</button>
				</ButtonStyled>
			</div>
		</div>
		<div class="flex w-full min-w-0 flex-col items-start justify-center gap-1 px-0.5">
			<p class="m-0 w-full truncate text-base font-semibold leading-5 text-contrast">
				{{ server.name }}
			</p>
			<p class="m-0 w-full truncate text-sm font-medium leading-[18px] text-primary">
				{{ server.serverType }} {{ server.gameVersion }}
			</p>
		</div>
	</div>
	<div
		v-else
		data-onboarding-id="server-card"
		role="button"
		tabindex="0"
		class="group button-base flex w-full cursor-pointer select-none gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-4 text-left transition-[border-color,filter,transform] hover:border-surface-5 hover:brightness-110 active:scale-[0.98]"
		@click="emit('open')"
		@keydown.enter="emit('open')"
		@keydown.space.prevent="emit('open')"
	>
		<div class="relative flex size-12 shrink-0 items-center justify-center">
			<ServerIcon
				:icon-path="server.iconPath"
				:server-type="server.serverType"
				:server-id="server.id"
				size="48px"
				class="transition-all group-hover:brightness-75"
			/>
			<div class="absolute inset-0 flex items-center justify-center" @click.stop @keydown.stop>
				<ButtonStyled v-if="server.status !== 'running'" color="brand" size="large" circular>
					<button
						v-tooltip="formatMessage(messages.start)"
						type="button"
						class="origin-bottom scale-75 opacity-0 transition-all group-hover:scale-100 group-hover:opacity-100 group-focus-within:scale-100 group-focus-within:opacity-100"
						@click="emit('start-stop')"
					>
						<PlayIcon class="translate-x-[1px]" />
					</button>
				</ButtonStyled>
				<ButtonStyled v-else color="red" size="large" circular>
					<button
						v-tooltip="formatMessage(messages.stop)"
						type="button"
						class="origin-bottom scale-75 opacity-0 transition-all group-hover:scale-100 group-hover:opacity-100 group-focus-within:scale-100 group-focus-within:opacity-100"
						@click="emit('start-stop')"
					>
						<StopCircleIcon />
					</button>
				</ButtonStyled>
			</div>
		</div>
		<div class="min-w-0 flex-1">
			<div class="flex min-w-0 items-center gap-2">
				<p class="m-0 min-w-0 truncate text-base font-bold leading-tight text-contrast">
					{{ server.name }}
				</p>
				<TagItem v-if="showStatus" class="shrink-0">
					<span :class="'font-semibold ' + statusMeta.color">
						{{ formatMessage(statusMeta.label) }}
					</span>
				</TagItem>
			</div>
			<p class="m-0 mt-1 truncate text-sm font-semibold text-secondary">
				{{ server.serverType }} {{ server.gameVersion }}
			</p>
		</div>
	</div>
</template>
