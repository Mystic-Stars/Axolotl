<script setup lang="ts">
import { MoreVerticalIcon, PinIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, OverflowMenu, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import Instance from '@/components/ui/Instance.vue'
import type { GameInstance } from '@/helpers/types'

const props = defineProps<{
	instance: GameInstance
	pinned: boolean
}>()

const emit = defineEmits<{
	'pinned-change': [instance: GameInstance, pinned: boolean]
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	pin: { id: 'app.home.instances.pin', defaultMessage: 'Pin to Home' },
	unpin: { id: 'app.home.instances.unpin', defaultMessage: 'Unpin from Home' },
})

const menuOptions = computed(() => [
	{
		id: props.pinned ? 'unpin' : 'pin',
		action: () => emit('pinned-change', props.instance, !props.pinned),
	},
])
</script>

<template>
	<div class="relative min-w-0">
		<Instance :instance="props.instance" />
		<div class="absolute right-2 top-2 z-10" @click.stop>
			<ButtonStyled circular size="small" type="transparent">
				<OverflowMenu
					:options="menuOptions"
					:tooltip="formatMessage(props.pinned ? messages.unpin : messages.pin)"
				>
					<MoreVerticalIcon />
					<template #pin> <PinIcon /> {{ formatMessage(messages.pin) }} </template>
					<template #unpin>
						<PinIcon class="rotate-45" /> {{ formatMessage(messages.unpin) }}
					</template>
				</OverflowMenu>
			</ButtonStyled>
		</div>
	</div>
</template>
