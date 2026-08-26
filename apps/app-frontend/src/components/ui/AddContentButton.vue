<script setup lang="ts">
import { DropdownIcon, FolderOpenIcon, PlusIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	OverflowMenu,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { useRouter } from 'vue-router'

import { add_project_from_path } from '@/helpers/instance'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const messages = defineMessages({
	installContent: { id: 'app.content.install-content', defaultMessage: 'Install content' },
	addFromFile: { id: 'app.content.add-from-file', defaultMessage: 'Add from file' },
})

const props = defineProps({
	instance: {
		type: Object,
		required: true,
	},
})

const router = useRouter()

const handleAddContentFromFile = async () => {
	const newProject = await open({ multiple: true })
	if (!newProject) return

	for (const project of newProject) {
		await add_project_from_path(props.instance.id, project.path ?? project).catch(handleError)
	}
}

const handleSearchContent = async () => {
	await router.push({
		path: `/browse/${props.instance.loader === 'vanilla' ? 'resourcepack' : 'mod'}`,
		query: { i: props.instance.id },
	})
}
</script>

<template>
	<div class="joined-buttons">
		<ButtonStyled>
			<button @click="handleSearchContent">
				<PlusIcon />
				{{ formatMessage(messages.installContent) }}
			</button>
		</ButtonStyled>
		<ButtonStyled>
			<OverflowMenu
				:options="[
					{
						id: 'from_file',
						action: handleAddContentFromFile,
					},
				]"
			>
				<DropdownIcon />
				<template #from_file>
					<FolderOpenIcon />
					<span class="whitespace-nowrap">{{ formatMessage(messages.addFromFile) }}</span>
				</template>
			</OverflowMenu>
		</ButtonStyled>
	</div>
</template>
