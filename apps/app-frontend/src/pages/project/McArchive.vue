<script setup lang="ts">
import { DownloadIcon, ExternalIcon, FileArchiveIcon, SpinnerIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Card,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import BrowseInstanceSelector from '@/components/browse/BrowseInstanceSelector.vue'
import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import {
	import_mcarchive_content,
	install_mcarchive_content,
	type McArchiveContentInstallRequest,
} from '@/helpers/instance'
import {
	getMcArchiveModBySlug,
	type McArchiveFile,
	type McArchiveModVersion,
} from '@/helpers/mcarchive'
import { injectContentSelection } from '@/providers/content-selection'
import { useBreadcrumbs } from '@/store/breadcrumbs'

const route = useRoute()
const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()
const contentSelection = injectContentSelection()
const breadcrumbs = useBreadcrumbs()
const instanceSelector = ref<InstanceType<typeof BrowseInstanceSelector>>()
const project = ref<Awaited<ReturnType<typeof getMcArchiveModBySlug>> | null>(null)
const loading = ref(false)
const busyFileId = ref<string | null>(null)
const manualDownload = ref<{
	request: McArchiveContentInstallRequest
	fileName: string
	pageUrl: string | null
	expectedSha256: string | null
} | null>(null)

const messages = defineMessages({
	loading: {
		id: 'app.project.mcarchive.loading',
		defaultMessage: 'Loading MCArchive project…',
	},
	versions: {
		id: 'app.project.mcarchive.versions',
		defaultMessage: 'Versions',
	},
	chooseInstance: {
		id: 'app.browse.choose-instance',
		defaultMessage: 'Choose instance',
	},
	install: {
		id: 'app.project.mcarchive.install',
		defaultMessage: 'Install',
	},
	installed: {
		id: 'app.project.mcarchive.installed',
		defaultMessage: 'Installed {fileName}',
	},
	manualTitle: {
		id: 'app.project.mcarchive.manual.title',
		defaultMessage: 'Manual download required',
	},
	manualDescription: {
		id: 'app.project.mcarchive.manual.description',
		defaultMessage:
			'{fileName} has no verifiable direct download. Download it from the source page, then import the downloaded file.',
	},
	openSource: {
		id: 'app.project.mcarchive.manual.open-source',
		defaultMessage: 'Open source page',
	},
	importFile: {
		id: 'app.project.mcarchive.manual.import-file',
		defaultMessage: 'Import downloaded file',
	},
	pickFile: {
		id: 'app.project.mcarchive.manual.pick-file',
		defaultMessage: 'Choose downloaded archive',
	},
	archiveFilter: {
		id: 'app.project.mcarchive.manual.archive-filter',
		defaultMessage: 'Minecraft archives',
	},
	source: {
		id: 'app.project.mcarchive.source',
		defaultMessage: 'MCArchive source',
	},
	noFiles: {
		id: 'app.project.mcarchive.no-files',
		defaultMessage: 'No files are available for this release.',
	},
	sha256: {
		id: 'app.project.mcarchive.sha256',
		defaultMessage: 'SHA-256',
	},
})

const selectedInstance = computed(() => contentSelection.targetInstance.value)

function createRequest(version: McArchiveModVersion, file: McArchiveFile) {
	if (!project.value) throw new Error('MCArchive project is unavailable')
	return {
		projectId: project.value.uuid,
		projectSlug: project.value.slug,
		versionId: version.uuid,
		fileId: file.uuid,
		projectType: 'mod',
	} satisfies McArchiveContentInstallRequest
}

async function install(version: McArchiveModVersion, file: McArchiveFile) {
	if (!selectedInstance.value) {
		await contentSelection.refreshInstances(
			typeof route.query.i === 'string' ? route.query.i : undefined,
		)
		instanceSelector.value?.show()
		return
	}
	busyFileId.value = file.uuid
	try {
		const request = createRequest(version, file)
		const result = await install_mcarchive_content(selectedInstance.value.id, request)
		if (result.state === 'manual_download') {
			manualDownload.value = {
				request,
				fileName: result.fileName,
				pageUrl: result.pageUrl,
				expectedSha256: result.expectedSha256,
			}
			return
		}
		addNotification({
			type: 'success',
			title: formatMessage(messages.installed, { fileName: file.name }),
		})
	} catch (error) {
		handleError(error)
	} finally {
		busyFileId.value = null
	}
}

async function importDownloadedFile() {
	if (!manualDownload.value || !selectedInstance.value) return
	const path = await open({
		multiple: false,
		title: formatMessage(messages.pickFile),
		filters: [{ name: formatMessage(messages.archiveFilter), extensions: ['jar', 'zip'] }],
	})
	if (!path || Array.isArray(path)) return
	busyFileId.value = manualDownload.value.request.fileId
	try {
		const result = await import_mcarchive_content(
			selectedInstance.value.id,
			manualDownload.value.request,
			path,
		)
		if (result.state === 'manual_download') {
			manualDownload.value = {
				request: manualDownload.value.request,
				fileName: result.fileName,
				pageUrl: result.pageUrl,
				expectedSha256: result.expectedSha256,
			}
			return
		}
		addNotification({
			type: 'success',
			title: formatMessage(messages.installed, { fileName: manualDownload.value.fileName }),
		})
		manualDownload.value = null
	} catch (error) {
		handleError(error)
	} finally {
		busyFileId.value = null
	}
}

async function selectInstance(instance: (typeof contentSelection.instances.value)[number]) {
	contentSelection.setTarget(instance)
}

watch(
	() => route.params.slug,
	async (slug) => {
		if (typeof slug !== 'string' || !slug) return
		loading.value = true
		project.value = null
		manualDownload.value = null
		try {
			project.value = await getMcArchiveModBySlug(slug)
			breadcrumbs.setName('Project', project.value.name)
			breadcrumbs.setNameIcon('Project', project.value.icon_url ?? project.value.iconUrl ?? null)
		} catch (error) {
			handleError(error)
		} finally {
			loading.value = false
		}
	},
	{ immediate: true },
)

void contentSelection
	.refreshInstances(typeof route.query.i === 'string' ? route.query.i : undefined)
	.then(() => {
		if (typeof route.query.i !== 'string' || contentSelection.targetInstance.value) return
		const requested = contentSelection.instances.value.find(
			(instance) => instance.id === route.query.i,
		)
		if (requested) contentSelection.setTarget(requested)
	})
	.catch(handleError)
</script>

<template>
	<div v-if="loading" class="flex min-h-64 items-center justify-center gap-3 p-6 text-secondary">
		<SpinnerIcon class="animate-spin" />
		{{ formatMessage(messages.loading) }}
	</div>
	<div v-else-if="project" class="mx-auto flex w-full max-w-5xl flex-col gap-5 p-6">
		<section class="flex flex-col gap-3">
			<div class="flex flex-wrap items-start justify-between gap-3">
				<div class="min-w-0">
					<h1 class="m-0 text-2xl font-bold text-contrast">{{ project.name }}</h1>
					<p v-if="project.summary" class="mb-0 mt-2 text-secondary">{{ project.summary }}</p>
				</div>
				<ButtonStyled size="standard" type="standard">
					<button class="flex min-w-0 items-center gap-2" @click="instanceSelector?.show()">
						<InstanceIcon
							v-if="selectedInstance"
							class="shrink-0"
							size="1.25rem"
							:icon-path="selectedInstance.icon_path"
							:instance-id="selectedInstance.id"
							:loader="selectedInstance.loader"
						/>
						<span class="max-w-48 truncate font-medium">
							{{ selectedInstance?.name ?? formatMessage(messages.chooseInstance) }}
						</span>
					</button>
				</ButtonStyled>
			</div>
			<p v-if="project.description" class="m-0 whitespace-pre-wrap text-sm text-secondary">
				{{ project.description }}
			</p>
		</section>

		<Card v-if="manualDownload" class="flex flex-col gap-3">
			<div>
				<h2 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.manualTitle) }}
				</h2>
				<p class="mb-0 mt-1 text-sm text-secondary">
					{{ formatMessage(messages.manualDescription, { fileName: manualDownload.fileName }) }}
				</p>
			</div>
			<div class="flex flex-wrap gap-2">
				<ButtonStyled v-if="manualDownload.pageUrl" type="outlined">
					<button @click="openUrl(manualDownload!.pageUrl!)">
						<ExternalIcon />
						{{ formatMessage(messages.openSource) }}
					</button>
				</ButtonStyled>
				<ButtonStyled>
					<button :disabled="busyFileId !== null" @click="importDownloadedFile">
						<FileArchiveIcon />
						{{ formatMessage(messages.importFile) }}
					</button>
				</ButtonStyled>
			</div>
		</Card>

		<section class="flex flex-col gap-3">
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.versions) }}
			</h2>
			<Card v-for="version in project.modVersions" :key="version.uuid" class="flex flex-col gap-3">
				<div class="flex flex-wrap items-center justify-between gap-2">
					<span class="font-semibold text-contrast">{{ version.name }}</span>
					<span class="text-sm text-secondary">
						{{ version.gameVersions.map((gameVersion) => gameVersion.name).join(', ') }}
					</span>
				</div>
				<div
					v-if="version.files.length"
					class="divide-y divide-surface-4 border-y border-surface-4"
				>
					<div
						v-for="file in version.files"
						:key="file.uuid"
						class="flex min-w-0 items-center gap-3 py-3"
					>
						<div class="min-w-0 flex-1">
							<div class="truncate text-sm font-medium text-contrast">{{ file.name }}</div>
							<div v-if="file.sha256" class="truncate text-xs text-secondary">
								{{ formatMessage(messages.sha256) }}: {{ file.sha256 }}
							</div>
						</div>
						<ButtonStyled size="standard">
							<button :disabled="busyFileId !== null" @click="install(version, file)">
								<SpinnerIcon v-if="busyFileId === file.uuid" class="animate-spin" />
								<DownloadIcon v-else />
								{{ formatMessage(messages.install) }}
							</button>
						</ButtonStyled>
					</div>
				</div>
				<p v-else class="m-0 text-sm text-secondary">{{ formatMessage(messages.noFiles) }}</p>
			</Card>
		</section>
	</div>
	<div v-else class="p-6">
		<Card class="text-secondary">{{ formatMessage(messages.loading) }}</Card>
	</div>
	<BrowseInstanceSelector
		ref="instanceSelector"
		:instances="contentSelection.instances.value"
		:selected-instance="selectedInstance"
		:selected-count="0"
		:install-current="async () => true"
		:clear-current="() => {}"
		@select="selectInstance"
	/>
</template>
