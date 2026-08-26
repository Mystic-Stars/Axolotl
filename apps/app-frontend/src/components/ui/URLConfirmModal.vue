<script setup>
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	ProjectCard,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import { get_project_v3, get_version } from '@/helpers/cache.js'
import { injectContentInstall } from '@/providers/content-install'

const { handleError } = injectNotificationManager()
const { install: installVersion } = injectContentInstall()
const { formatMessage } = useVIntl()
const messages = defineMessages({
	installProject: { id: 'app.url-install.title', defaultMessage: 'Install {project}' },
	installingVersion: {
		id: 'app.url-install.version',
		defaultMessage: 'Installing version {version}',
	},
})

const confirmModal = ref(null)
const project = ref(null)
const version = ref(null)

defineExpose({
	async show(event) {
		if (event.event === 'InstallVersion') {
			version.value = await get_version(event.id, 'must_revalidate').catch(handleError)
			project.value = await get_project_v3(version.value.project_id, 'must_revalidate').catch(
				handleError,
			)
		} else {
			project.value = await get_project_v3(event.id, 'must_revalidate').catch(handleError)
			version.value = await get_version(
				project.value.versions[project.value.versions.length - 1],
				'must_revalidate',
			).catch(handleError)
		}
		confirmModal.value.show()
	},
})

async function install() {
	confirmModal.value.hide()
	await installVersion(
		project.value.id,
		version.value.id,
		null,
		'URLConfirmModal',
		() => {},
		() => {},
	).catch(handleError)
}
</script>

<template>
	<ModalWrapper
		ref="confirmModal"
		:header="formatMessage(messages.installProject, { project: project?.name })"
	>
		<div class="modal-body flex flex-col items-center justify-center gap-3">
			<ProjectCard
				:title="project.name"
				:link="() => confirmModal.hide()"
				:icon-url="project.icon_url"
				:summary="project.summary"
				:tags="project.display_categories"
				:all-tags="project.categories"
				:downloads="project.downloads"
				:date-updated="project.date_modified"
				:banner="project.featured_gallery ?? undefined"
				:color="project.color ?? undefined"
				layout="list"
				class="project-card bg-bg w-full"
			/>
			<div class="flex w-full flex-row justify-between items-center gap-3">
				<div class="markdown-body">
					<p>
						{{ formatMessage(messages.installingVersion, { version: version.id }) }}
					</p>
				</div>
				<div class="flex flex-row gap-2">
					<ButtonStyled color="brand">
						<button @click="install">{{ formatMessage(commonMessages.installButton) }}</button>
					</ButtonStyled>
				</div>
			</div>
		</div>
	</ModalWrapper>
</template>

<style scoped lang="scss">
.project-card {
	:deep(.badge) {
		border: 1px solid var(--color-raised-bg);
		background-color: var(--color-accent-contrast);
	}
}
</style>
