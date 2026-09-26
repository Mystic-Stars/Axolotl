<script setup lang="ts">
import { Avatar, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

const emit = defineEmits<{
	openGame: []
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'app.settings.about.easteregg.contributors-title',
		defaultMessage: '贡献者彩蛋',
	},
	clickHint: {
		id: 'app.settings.about.easteregg.click-hint',
		defaultMessage: 'Click to open a hidden Mini Game',
	},
})

const modal = ref<InstanceType<typeof NewModal> | null>(null)

const contributors = [
	{
		name: 'cyf112233',
		avatarUrl: `${window.location.origin}/easteregg/avatars/cyf112233.jpg`,
	},
]

function show() {
	modal.value?.show()
}

function selectContributor() {
	modal.value?.hide()
	emit('openGame')
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		width="min(480px, calc(100vw - 2rem))"
		max-width="480px"
	>
		<ul class="m-0 list-none p-0">
			<li>
				<button
					type="button"
					class="flex w-full items-center gap-3 rounded-xl bg-surface-4 p-4 text-left transition-colors hover:bg-surface-5"
					@click="selectContributor"
				>
					<Avatar
						:src="contributors[0].avatarUrl"
						:alt="contributors[0].name"
						size="4rem"
						circle
						no-shadow
					/>
					<span class="min-w-0">
						<span class="block font-semibold text-[var(--color-text-primary)]">{{
							contributors[0].name
						}}</span>
						<span class="block text-sm text-[var(--color-text-tertiary)]">
							{{ formatMessage(messages.clickHint) }}
						</span>
					</span>
				</button>
			</li>
		</ul>
	</NewModal>
</template>
