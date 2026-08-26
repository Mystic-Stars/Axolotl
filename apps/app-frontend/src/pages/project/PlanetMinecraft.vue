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
	import_planet_minecraft_content,
	install_planet_minecraft_content,
	type PlanetMinecraftContentInstallRequest,
} from '@/helpers/instance'
import { getPlanetMinecraftProject, type PlanetMinecraftVersion } from '@/helpers/planet-minecraft'
import { injectContentSelection } from '@/providers/content-selection'
import { useBreadcrumbs } from '@/store/breadcrumbs'

const route = useRoute()
const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()
const contentSelection = injectContentSelection()
const breadcrumbs = useBreadcrumbs()
const instanceSelector = ref<InstanceType<typeof BrowseInstanceSelector>>()
const project = ref<Awaited<ReturnType<typeof getPlanetMinecraftProject>> | null>(null)
const loading = ref(false)
const busyVersionId = ref<string | null>(null)
const manualDownload = ref<{
	request: PlanetMinecraftContentInstallRequest
	pageUrl: string
	fileName: string | null
} | null>(null)

const messages = defineMessages({
	loading: {
		id: 'app.project.planet-minecraft.loading',
		defaultMessage: 'Loading Planet Minecraft project…',
	},
	chooseInstance: { id: 'app.browse.choose-instance', defaultMessage: 'Choose instance' },
	versions: { id: 'app.project.planet-minecraft.versions', defaultMessage: 'Downloads' },
	install: { id: 'app.project.planet-minecraft.install', defaultMessage: 'Install' },
	manualTitle: {
		id: 'app.project.planet-minecraft.manual.title',
		defaultMessage: 'Manual download required',
	},
	manualDescription: {
		id: 'app.project.planet-minecraft.manual.description',
		defaultMessage: '{fileName} must be downloaded from Planet Minecraft, then imported here.',
	},
	unknownFile: {
		id: 'app.project.planet-minecraft.unknown-file',
		defaultMessage: 'Downloaded file',
	},
	openSource: {
		id: 'app.project.planet-minecraft.manual.open-source',
		defaultMessage: 'Open source page',
	},
	importFile: {
		id: 'app.project.planet-minecraft.manual.import-file',
		defaultMessage: 'Import downloaded file',
	},
	pickFile: {
		id: 'app.project.planet-minecraft.manual.pick-file',
		defaultMessage: 'Choose downloaded archive',
	},
	archiveFilter: {
		id: 'app.project.planet-minecraft.manual.archive-filter',
		defaultMessage: 'Minecraft archives',
	},
	installed: {
		id: 'app.project.planet-minecraft.installed',
		defaultMessage: 'Installed {fileName}',
	},
})

const selectedInstance = computed(() => contentSelection.targetInstance.value)

function createRequest(version: PlanetMinecraftVersion): PlanetMinecraftContentInstallRequest {
	if (!project.value) throw new Error('Planet Minecraft project is unavailable')
	return { projectId: project.value.id, versionId: version.id, projectType: 'mod' }
}

async function install(version: PlanetMinecraftVersion) {
	if (!selectedInstance.value) {
		await contentSelection.refreshInstances(
			typeof route.query.i === 'string' ? route.query.i : undefined,
		)
		instanceSelector.value?.show()
		return
	}
	busyVersionId.value = version.id
	try {
		const request = createRequest(version)
		const result = await install_planet_minecraft_content(selectedInstance.value.id, request)
		if (result.state === 'manual_download') {
			manualDownload.value = { request, pageUrl: result.pageUrl, fileName: result.fileName }
			return
		}
		addNotification({
			type: 'success',
			title: formatMessage(messages.installed, {
				fileName: version.download.fileName ?? version.name,
			}),
		})
	} catch (error) {
		handleError(error)
	} finally {
		busyVersionId.value = null
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
	busyVersionId.value = manualDownload.value.request.versionId
	try {
		const result = await import_planet_minecraft_content(
			selectedInstance.value.id,
			manualDownload.value.request,
			path,
		)
		const fileName =
			manualDownload.value.fileName ?? (result.state === 'installed' ? result.relativePath : '')
		addNotification({ type: 'success', title: formatMessage(messages.installed, { fileName }) })
		manualDownload.value = null
	} catch (error) {
		handleError(error)
	} finally {
		busyVersionId.value = null
	}
}

watch(
	() => route.params.id,
	async (id) => {
		if (typeof id !== 'string' || !id) return
		loading.value = true
		try {
			project.value = await getPlanetMinecraftProject(id)
			breadcrumbs.setName('Project', project.value.title)
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
		<SpinnerIcon class="animate-spin" /> {{ formatMessage(messages.loading) }}
	</div>
	<div v-else-if="project" class="mx-auto flex w-full max-w-5xl flex-col gap-5 p-6">
		<section class="flex flex-wrap items-start justify-between gap-3">
			<div class="min-w-0">
				<h1 class="m-0 text-2xl font-bold text-contrast">{{ project.title }}</h1>
				<p v-if="project.summary" class="mb-0 mt-2 text-secondary">{{ project.summary }}</p>
			</div>
			<ButtonStyled
				><button class="flex items-center gap-2" @click="instanceSelector?.show()">
					<InstanceIcon
						v-if="selectedInstance"
						size="1.25rem"
						:icon-path="selectedInstance.icon_path"
						:instance-id="selectedInstance.id"
						:loader="selectedInstance.loader"
					/>
					{{ selectedInstance?.name ?? formatMessage(messages.chooseInstance) }}
				</button></ButtonStyled
			>
		</section>
		<Card v-if="manualDownload" class="flex flex-wrap items-center justify-between gap-3">
			<div class="min-w-0">
				<h2 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.manualTitle) }}
				</h2>
				<p class="mb-0 mt-1 text-sm text-secondary">
					{{
						formatMessage(messages.manualDescription, {
							fileName: manualDownload.fileName ?? formatMessage(messages.unknownFile),
						})
					}}
				</p>
			</div>
			<div class="flex gap-2">
				<ButtonStyled type="outlined"
					><button @click="openUrl(manualDownload!.pageUrl)">
						<ExternalIcon /> {{ formatMessage(messages.openSource) }}
					</button></ButtonStyled
				>
				<ButtonStyled
					><button :disabled="busyVersionId !== null" @click="importDownloadedFile">
						<FileArchiveIcon /> {{ formatMessage(messages.importFile) }}
					</button></ButtonStyled
				>
			</div>
		</Card>
		<section class="flex flex-col gap-3">
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.versions) }}
			</h2>
			<Card
				v-for="version in project.versions"
				:key="version.id"
				class="flex flex-wrap items-center justify-between gap-3"
			>
				<div class="min-w-0">
					<div class="font-semibold text-contrast">{{ version.name }}</div>
					<div class="text-sm text-secondary">{{ version.gameVersions.join(', ') }}</div>
				</div>
				<ButtonStyled
					><button :disabled="busyVersionId !== null" @click="install(version)">
						<SpinnerIcon v-if="busyVersionId === version.id" class="animate-spin" /><DownloadIcon
							v-else
						/>
						{{ formatMessage(messages.install) }}
					</button></ButtonStyled
				>
			</Card>
		</section>
	</div>
	<BrowseInstanceSelector
		ref="instanceSelector"
		:instances="contentSelection.instances.value"
		:selected-instance="selectedInstance"
		:selected-count="0"
		:install-current="async () => true"
		:clear-current="() => {}"
		@select="contentSelection.setTarget"
	/>
</template>
