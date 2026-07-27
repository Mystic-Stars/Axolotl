import type {
	AbstractPopupNotificationManager,
	AbstractWebNotificationManager,
	CreationFlowContextValue,
	CreationFlowModal,
} from '@modrinth/ui'
import { defineMessages, useVIntl } from '@modrinth/ui'
import SymlinkMethodCards from '@modrinth/ui/src/components/flows/drop/SymlinkMethodCards.vue'
import { confirm } from '@tauri-apps/plugin-dialog'
import { inject, provide, ref, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'
import { useRouter } from 'vue-router'

import type UnknownPackWarningModal from '@/components/ui/install_flow/UnknownPackWarningModal.vue'
import type ModpackAlreadyInstalledModal from '@/components/ui/modal/ModpackAlreadyInstalledModal.vue'
import { trackEvent } from '@/helpers/analytics'
import { get_project_versions, get_search_results } from '@/helpers/cache.js'
import {
	type ClassificationResult,
	classifyDroppedItem,
	classifyDroppedItemWithExtraction,
} from '@/helpers/drop'
import { import_instance } from '@/helpers/import.js'
import {
	type CreatePackLocation,
	type InstallJobSnapshot,
	install_create_instance,
	install_create_modpack_instance,
	install_get_modpack_preview,
	wait_for_install_job,
} from '@/helpers/install'
import { check_symlink_capability, list, restart_as_admin } from '@/helpers/instance'
import { install_job_listener } from '@/helpers/events.js'
import { get_loader_versions as getLoaderManifest } from '@/helpers/metadata.js'
import type { InstanceLoader } from '@/helpers/types'
import { useTheming } from '@/store/state'

const symlinkMessages = defineMessages({
	unsupportedTitle: {
		id: 'app.symlink-capability.unsupported.title',
		defaultMessage: 'Shared instances are unavailable',
	},
	unsupportedBody: {
		id: 'app.symlink-capability.unsupported',
		defaultMessage: 'This system does not support creating symbolic links.',
	},
	requiresAdminTitle: {
		id: 'app.symlink-capability.requires-admin.title',
		defaultMessage: 'Administrator permission required',
	},
	requiresAdminDescription: {
		id: 'app.symlink-capability.requires-admin.description',
		defaultMessage:
			'Windows Developer Mode is disabled, so the launcher must restart as administrator to create a shared instance.',
	},
	requiresAdminRestartButton: {
		id: 'app.symlink-capability.requires-admin.restart-button',
		defaultMessage: 'Restart as administrator',
	},
	cancel: {
		id: 'app.symlink-capability.cancel',
		defaultMessage: 'Cancel',
	},
})

export function setupCreationModal(
	notificationManager: AbstractWebNotificationManager,
	popupNotificationManager: AbstractPopupNotificationManager,
) {
	const { formatMessage } = useVIntl()
	const { handleError } = notificationManager
	const { addPopupNotification } = popupNotificationManager
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

	provide('showCreationModalWithOptions', (options?: {
		skipSetupType?: boolean
		initialMode?: 'custom' | 'import'
		onBack?: () => void
	}) => {
		installationModal.value?.show(options)
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
				// Collect all instances to import
				const instanceEntries: Array<{
					launcherType: string
					launcherName: string
					path: string
					instanceName: string
					instancePath: string
				}> = []
				for (const [launcherName, instanceSet] of Object.entries(
					config.importSelectedInstances.value,
				)) {
					const launcher = config.importLaunchers.value.find((l) => l.name === launcherName)
					if (!launcher || instanceSet.size === 0) continue
					for (const name of instanceSet) {
						const instanceData = launcher.instances.find((i) => i.name === name)
						instanceEntries.push({
							launcherType: launcher.launcherType ?? launcher.name,
							launcherName: launcher.name,
							path: launcher.path,
							instanceName: name,
							instancePath: instanceData?.path ?? '',
						})
					}
				}

				if (instanceEntries.length === 0) return

				// Show SymlinkMethodCards for user to choose copy vs symlink
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
							okLabel: formatMessage(symlinkMessages.requiresAdminRestartButton),
							cancelLabel: formatMessage(symlinkMessages.cancel),
						},
					)
					if (confirmed) {
						restart_as_admin()
					}
					return
				}

				const chooseImportMethod: (options: {
					instanceNames: string[]
					symlinkCapable: 'supported' | 'requires_admin' | 'unsupported'
				}) => Promise<boolean> = inject('chooseImportMethod')!

				const useSymlink = await chooseImportMethod({
					instanceNames: instanceEntries.map((e) => e.instanceName),
					symlinkCapable: capability,
				})

				for (const entry of instanceEntries) {
					try {
						const job = await import_instance(
							entry.launcherType,
							entry.path,
							entry.instanceName,
							useSymlink,
							entry.instancePath,
						)
						await wait_for_install_job(job.job_id)
					} catch (error) {
						handleError(error)
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
				// Fallback: called when modpack is imported via the creation flow
				// (not via onImportFileReceived, which has its own install path).
				const location: CreatePackLocation = {
					type: 'fromFile',
					path: config.modpackFilePath.value,
				}
				const preview = await install_get_modpack_preview(location).catch(handleError)
				if (!preview) return

				if (preview.unknownFile) {
					const splitPath = config.modpackFilePath.value.split(/[\\/]/)
					const fileName = splitPath
						? splitPath[splitPath.length - 1]
						: config.modpackFilePath.value
					if (unknownPackWarningModal.value) {
						unknownPackWarningModal.value?.show(
							() => doInstallModpackFile(location),
							fileName,
						)
					} else {
						await doInstallModpackFile(location)
					}
				} else {
					await doInstallModpackFile(location)
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

	async function doInstallModpackFile(location: CreatePackLocation) {
		const installingNotify = notificationManager.addNotification({
			title: `Installing modpack...`,
			type: 'info',
			autoCloseMs: 1000 * 10,
		})

		const job = await install_create_modpack_instance(location).catch((e) => {
			notificationManager.removeNotification(installingNotify.id)
			handleError(e)
			return null
		})
		if (!job) return

		// Single-use listener that auto-cleans up when the job reaches a terminal state
		const unlisten = await install_job_listener((updatedJob: InstallJobSnapshot) => {
			if (updatedJob.job_id !== job.job_id) return

			if (updatedJob.status === 'succeeded') {
				notificationManager.removeNotification(installingNotify.id)
				notificationManager.addNotification({
					title: 'Modpack installed successfully',
					type: 'success',
				})
				unlisten()
			} else if (['failed', 'canceled', 'interrupted'].includes(updatedJob.status)) {
				notificationManager.removeNotification(installingNotify.id)
				unlisten()
			}
		})
	}

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

let currentFlowCtx: CreationFlowContextValue | null = null

	/** Show a popup notification prompting the user to force-analyse an unclassified file. */
	function showForceAnalysisPopup(classification: ClassificationResult) {
		addPopupNotification({
			title: `Unknown file type`,
			text: `This file couldn't be identified from its contents. Perform a deep analysis?`,
			type: 'info',
			autoCloseMs: null,
			buttons: [
				{
					label: 'Force Analysis',
					action: async () => {
						const filePath = classification.file_path ?? classification.base_path
						if (!filePath) return

						// ── Processing notification during extraction ──
						const extractingNotify = notificationManager.addNotification({
							title: 'Analyzing file...',
							type: 'info',
							autoCloseMs: null,
						})

						try {
							const result = await classifyDroppedItemWithExtraction(filePath)
							notificationManager.removeNotification(extractingNotify.id)

							if (result.item_type === 'unknown') {
								notificationManager.addNotification({
									title: 'Could not identify file',
									text: result.reason ?? 'Deep analysis was unable to determine the file type.',
									type: 'error',
								})
								return
							}

							if (result.item_type === 'modpack') {
								const fileName = filePath.split(/[/\\]/).pop() || 'file'
								await installModpackFromPath(filePath, fileName)
								return
							}

							// Unexpected type from force analysis
							notificationManager.addNotification({
								title: `Unexpected type: ${result.item_type}`,
								type: 'error',
							})
						} catch (e) {
							notificationManager.removeNotification(extractingNotify.id)
							handleError(e as Error)
						}
					},
					color: 'brand',
				},
			],
		})
	}

	/** Install a modpack file with continuous feedback notifications. */
	async function installModpackFromPath(filePath: string, fileName: string) {
		let currentNotify = notificationManager.addNotification({
			title: `Installing ${fileName}...`,
			type: 'info',
			autoCloseMs: null,
		})

		const location: CreatePackLocation = { type: 'fromFile', path: filePath }

		try {
			const isMrpack = fileName?.toLowerCase().endsWith('.mrpack')

			if (!isMrpack) {
				// .zip needs preview to determine manifest
				const preview = await install_get_modpack_preview(location).catch((e) => {
					notificationManager.removeNotification(currentNotify.id)
					handleError(e)
					return null
				})
				if (!preview) return

				if (preview.unknownFile) {
					notificationManager.removeNotification(currentNotify.id)
					unknownPackWarningModal.value?.show(
						async () => {
							await doInstallModpackFile(location)
						},
						fileName,
					)
					return
				}
			}

			await doInstallModpackFile(location)
		} catch (e) {
			notificationManager.removeNotification(currentNotify?.id)
			handleError(e as Error)
		}
	}

		provide('setCreationFlowCtx', (ctx: CreationFlowContextValue) => {
		currentFlowCtx = ctx
	})

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
