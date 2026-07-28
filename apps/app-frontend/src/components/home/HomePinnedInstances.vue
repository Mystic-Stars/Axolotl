<script setup lang="ts">
import { RightArrowIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import HomeInstanceCard from '@/components/home/HomeInstanceCard.vue'
import { set_pinned } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

const props = defineProps<{
	instances: GameInstance[]
}>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const messages = defineMessages({
	pinnedInstances: {
		id: 'app.home.instances.pinned',
		defaultMessage: 'Pinned instances',
	},
	emptyPinned: {
		id: 'app.home.instances.pinned-empty',
		defaultMessage: 'Pin an instance from its card menu or the library to keep it here.',
	},
	viewAllInstances: {
		id: 'app.home.instances.view-all',
		defaultMessage: 'View all instances',
	},
})

const pinnedInstances = computed(() =>
	props.instances
		.filter((instance) => instance.pinned_at)
		.slice()
		.sort((a, b) => new Date(b.pinned_at ?? 0).getTime() - new Date(a.pinned_at ?? 0).getTime()),
)

async function updatePinned(instance: GameInstance, pinned: boolean) {
	await set_pinned(instance.id, pinned).catch(handleError)
}
</script>

<template>
	<section class="flex flex-col gap-3">
		<div class="flex items-center gap-3">
			<h2 class="m-0 text-lg font-bold text-contrast">
				{{ formatMessage(messages.pinnedInstances) }}
			</h2>
			<ButtonStyled type="transparent" size="small" class="ml-auto">
				<router-link to="/library">
					{{ formatMessage(messages.viewAllInstances) }}
					<RightArrowIcon aria-hidden="true" />
				</router-link>
			</ButtonStyled>
		</div>
		<div
			v-if="pinnedInstances.length > 0"
			class="grid grid-cols-[repeat(auto-fill,minmax(16rem,1fr))] gap-3"
		>
			<HomeInstanceCard
				v-for="instance in pinnedInstances"
				:key="instance.id"
				:instance="instance"
				:pinned="true"
				@pinned-change="updatePinned"
			/>
		</div>
		<p v-else class="m-0 text-sm text-secondary">
			{{ formatMessage(messages.emptyPinned) }}
		</p>
	</section>
</template>
