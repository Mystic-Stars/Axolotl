import type {
	AbstractWebNotificationManager,
	CreationFlowContextValue,
	CreationFlowModal,
} from '@modrinth/ui'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { confirm } from '@tauri-apps/plugin-dialog'
import { provide, ref, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'
import { useRouter } from 'vue-router'

import type UnknownPackWarningModal from '@/components/ui/install_flow/UnknownPackWarningModal.vue'
import type ModpackAlreadyInstalledModal from '@/components/ui/modal/ModpackAlreadyInstalledModal.vue'
import { trackEvent } from '@/helpers/analytics'
import { get_project_versions, get_search_results } from '@/helpers/cache.js'
import { import_instance } from '@/helpers/import.js'
import { check_symlink_capability, restart_as_admin } from '@/helpers/instance'
import {
	type CreatePackLocation,
	install_create_instance,
	install_create_modpack_instance,
	install_get_modpack_preview,
} from '@/helpers/install'
import { list } from '@/helpers/instance'
import { get_loader_versions as getLoaderManifest } from '@/helpers/metadata.js'
import type { InstanceLoader } from '@/helpers/types'
import { useTheming } from '@/store/state'

const symlinkMessages = defineMessages({
	unsupportedTitle: {
		id: 'app.symlink-capability.unsupported.title',
	},
	unsupportedBody: {
		id: 'app.symlink-capability.unsupported',
	},
	requiresAdminTitle: {
		id: 'app.symlink-capability.requires-admin.title',
	},
	requiresAdminDescription: {
		id: 'app.symlink-capability.requires-admin.description',
	},
	requiresAdminRestartButton: {
		id: 'app.symlink-capability.requires-admin.restart-button',
	},
	cancel: {
		id: 'app.symlink-capability.cancel',
	},
	ok: {
		id: 'app.symlink-capability.ok',
	},
})

export function setupCreationModal(notificationManager: AbstractWebNotificationManager) {
	const { formatMessage } = useVIntl()
	const { handleError } = notificationManager
	const router = useRouter()
	const themeStore = useTheming()

	const installationModal =
		useTemplateRef<ComponentExposed<typeof CreationFlowModal>>('installationModal')
	const unknownPackWarningModal =
		useTemplateRef<InstanceType<typeof UnknownPackWarningModal>>('unknownPackWarningModal')
	const modpackAlreadyInstalledModal = ref<InstanceType<typeof ModpackAlreadyInstalledModal>>()

	function setModpackAlreadyInstalledModal(
		modal: InstanceType<typeof ModpackAlreadyInstalledModal>,
	) {
		modpackAlreadyInstalledModal.value = modal
	}

	async function fetchExistingInstanceNames(): Promise<string[]> {
		const instances = await list().catch(handleError)
		return instances?.map((i) => i.name) ?? []
	}

	provide('showCreationModal', () => {
		installationModal.value?.show()
	})

	async function proceedWithModpackCreation(
		projectId: string,
		versionId: string,
		name: string,
		iconUrl?: string,
	) {
		await install_create_modpack_instance({
			type: 'fromVersionId',
			project_id: projectId,
			version_id: versionId,
			title: name,
			icon_url: iconUrl,
		}).catch(handleError)
		trackEvent('InstanceCreate', { source: 'CreationModalModpack' })
	}

	async function handleCreate(config: CreationFlowContextValue) {
		try {
			if (config.modpackSelection.value) {
				const { projectId, versionId, name, iconUrl } = config.modpackSelection.value

				const instances = await list().catch(handleError)
				const existingInstance = instances?.find((i) => i.link?.project_id === projectId)

				if (existingInstance && !themeStore.getFeatureFlag('skip_non_essential_warnings')) {
					pendingModpackCreation.value = { projectId, versionId, name, iconUrl }
					installationModal.value?.hide()
					modpackAlreadyInstalledModal.value?.show(existingInstance.name, existingInstance.id)
					return
				}
			}

			installationModal.value?.hide()

			if (config.isImportMode.value) {
				if (config.importAsSymlink.value) {
					const capability = await check_symlink_capability()
					if (capability === 'unsupported') {
						notificationManager.addNotification({
							type: 'error',
							title: formatMessage(symlinkMessages.unsupportedTitle),
							text: formatMessage(symlinkMessages.unsupportedBody),
						})
						return
					}
					if (capability === 'requires_admin') {
						const confirmed = await confirm(
							formatMessage(symlinkMessages.requiresAdminDescription),
							{
								title: formatMessage(symlinkMessages.requiresAdminTitle),
								okLabel: formatMessage(symlinkMessages.ok),
								cancelLabel: formatMessage(symlinkMessages.cancel),
							},
						)
						if (confirmed) {
							restart_as_admin()
						}
						return
					}
				}

				for (const [launcherName, instanceSet] of Object.entries(
					config.importSelectedInstances.value,
				)) {
					const launcher = config.importLaunchers.value.find((l) => l.name === launcherName)
					if (!launcher || instanceSet.size === 0) continue
					for (const name of instanceSet) {
						await import_instance(
							launcher.launcherType ?? launcher.name,
							launcher.path,
							name,
							config.importAsSymlink.value,
						).catch(handleError)
					}
				}
				trackEvent('InstanceCreate', { source: 'CreationModalImport' })
				return
			}

			if (config.modpackSelection.value) {
				const { projectId, versionId, name, iconUrl } = config.modpackSelection.value
				await proceedWithModpackCreation(projectId, versionId, name, iconUrl)
				return
			}

			if (config.modpackFilePath.value) {
				const location: CreatePackLocation = {
					type: 'fromFile',
					path: config.modpackFilePath.value,
				}
				const preview = await install_get_modpack_preview(location)

				if (preview.unknownFile) {
					const splitPath = config.modpackFilePath.value.split(/[\\/]/)
					const fileName = splitPath
						? splitPath[splitPath.length - 1]
						: config.modpackFilePath.value
					if (unknownPackWarningModal.value) {
						unknownPackWarningModal.value?.show(
							() => install_create_modpack_instance(location).then(() => undefined),
							fileName,
						)
					} else {
						await install_create_modpack_instance(location)
					}
				} else {
					await install_create_modpack_instance(location)
				}
				trackEvent('InstanceCreate', { source: 'CreationModalModpackFile' })
				return
			}

			// Custom/vanilla setup
			const loader = config.hideLoaderChips.value
				? 'vanilla'
				: (config.selectedLoader.value ?? 'vanilla')
			const loaderVersion = config.hideLoaderVersion.value
				? null
				: (config.selectedLoaderVersion.value ?? config.loaderVersionType.value)
			const iconPath = config.instanceIconPath.value ?? null
			const name = config.instanceName.value.trim() || config.autoInstanceName.value

			await install_create_instance({
				name,
				gameVersion: config.selectedGameVersion.value!,
				loader: loader as InstanceLoader,
				loaderVersion,
				iconPath,
			}).catch(handleError)

			trackEvent('InstanceCreate', {
				source: 'CreationModal',
			})
		} catch (err) {
			handleError(err as Error)
		}
	}

	const pendingModpackCreation = ref<{
		projectId: string
		versionId: string
		name: string
		iconUrl?: string
	} | null>(null)

	async function handleModpackDuplicateCreateAnyway() {
		if (!pendingModpackCreation.value) return
		const { projectId, versionId, name, iconUrl } = pendingModpackCreation.value
		pendingModpackCreation.value = null
		await proceedWithModpackCreation(projectId, versionId, name, iconUrl)
	}

	function handleModpackDuplicateGoToInstance(instanceId: string) {
		pendingModpackCreation.value = null
		router.push(`/instance/${encodeURIComponent(instanceId)}/`)
	}

	function handleBrowseModpacks() {
		installationModal.value?.hide()
		router.push('/browse/modpack')
	}

	async function searchModpacks(query: string, limit: number = 10) {
		const params = [`facets=[["project_type:modpack"]]`, `limit=${limit}`]
		if (query) {
			params.push(`query=${encodeURIComponent(query)}`)
		}
		const raw = await get_search_results(`?${params.join('&')}`)
		if (raw?.result) return raw.result
		return { hits: [], offset: 0, limit, total_hits: 0 }
	}

	async function getProjectVersions(projectId: string) {
		const versions = await get_project_versions(projectId)
		return versions ?? []
	}

	return {
		installationModal,
		unknownPackWarningModal,
		fetchExistingInstanceNames,
		handleCreate,
		handleBrowseModpacks,
		searchModpacks,
		getProjectVersions,
		getLoaderManifest,
		setModpackAlreadyInstalledModal,
		handleModpackDuplicateCreateAnyway,
		handleModpackDuplicateGoToInstance,
	}
}
