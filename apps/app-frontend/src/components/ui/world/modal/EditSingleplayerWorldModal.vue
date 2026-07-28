<script setup lang="ts">
import { ChevronRightIcon, SaveIcon, UndoIcon, XIcon } from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import SymlinkInstanceWarning from '@/components/ui/SymlinkInstanceWarning.vue'
import type { GameInstance } from '@/helpers/types'
import type { SingleplayerWorld } from '@/helpers/worlds.ts'
import { rename_world, reset_world_icon } from '@/helpers/worlds.ts'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const emit = defineEmits<{
	submit: [path: string, name: string, removeIcon: boolean]
}>()

const props = defineProps<{
	instance: GameInstance
}>()

const modal = ref()

const icon = ref()
const name = ref()
const path = ref()
const removeIcon = ref(false)
async function saveWorld() {
	await rename_world(props.instance.id, path.value, name.value).catch(handleError)

	if (removeIcon.value) {
		await reset_world_icon(props.instance.id, path.value).catch(handleError)
	}
	emit('submit', path.value, name.value, removeIcon.value)
	hide()
}

function show(world: SingleplayerWorld) {
	name.value = world.name
	path.value = world.path
	icon.value = world.icon
	removeIcon.value = false
	modal.value.show()
}

function hide() {
	modal.value.hide()
}

defineExpose({ show })

const messages = defineMessages({
	title: {
		id: 'instance.edit-world.title',
		defaultMessage: 'Edit world',
	},
	name: {
		id: 'instance.edit-world.name',
		defaultMessage: 'Name',
	},
	placeholderName: {
		id: 'instance.edit-world.placeholder-name',
		defaultMessage: 'Minecraft World',
	},
	resetIcon: {
		id: 'instance.edit-world.reset-icon',
		defaultMessage: 'Reset icon',
	},
})
</script>
<template>
	<ModalWrapper ref="modal">
		<template #title>
			<Avatar :src="removeIcon || !icon ? undefined : icon" size="24px" />
			{{ instance.name }} <ChevronRightIcon />
			<span class="font-extrabold text-lg text-contrast">{{ formatMessage(messages.title) }}</span>
		</template>
		<SymlinkInstanceWarning
			v-if="instance?.symlink_target"
			:symlink-target="instance.symlink_target"
		/>
		<div class="w-[450px]">
			<h2 class="text-lg font-extrabold text-contrast mt-0 mb-1">
				{{ formatMessage(messages.name) }}
			</h2>
			<StyledInput
				v-model="name"
				:placeholder="formatMessage(messages.placeholderName)"
				autocomplete="off"
				wrapper-class="w-full"
			/>
		</div>
		<div class="flex gap-2 mt-4">
			<ButtonStyled color="brand">
				<button @click="saveWorld">
					<SaveIcon />
					{{ formatMessage(commonMessages.saveChangesButton) }}
				</button>
			</ButtonStyled>
			<ButtonStyled>
				<button :disabled="removeIcon || !icon" @click="removeIcon = true">
					<UndoIcon />
					{{ formatMessage(messages.resetIcon) }}
				</button>
			</ButtonStyled>
			<ButtonStyled>
				<button @click="hide()">
					<XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</button>
			</ButtonStyled>
		</div>
	</ModalWrapper>
</template>
