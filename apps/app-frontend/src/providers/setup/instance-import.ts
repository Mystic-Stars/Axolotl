import type { AbstractWebNotificationManager } from '@modrinth/ui'
import { provideInstanceImport } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'

import { import_plan_listener } from '@/helpers/events.js'
import {
	cancel_import_plan,
	get_default_launcher_path,
	get_importable_instances,
	import_instance,
	start_import_plan,
} from '@/helpers/import.js'
import { wait_for_install_job } from '@/helpers/install'
import { get_loader_versions } from '@/helpers/metadata.js'
import { openPath } from '@/helpers/utils.js'

export function setupInstanceImportProvider(notificationManager: AbstractWebNotificationManager) {
	const { handleError } = notificationManager

	provideInstanceImport({
		async getDetectedLaunchers() {
			const launcherNames = [
				'ModrinthApp',
				'MultiMC',
				'PCL2',
				'PCL2CE',
				'HMCL',
				'GDLauncher',
				'ATLauncher',
				'Curseforge',
				'PrismLauncher',
				'Generic',
			]
			const launchers = []
			for (const name of launcherNames) {
				try {
					const path = await get_default_launcher_path(name)
					if (!path) continue
					const instances = await get_importable_instances(name, path)
					if (instances?.length > 0) {
						launchers.push({ name, path, instances })
					}
				} catch {
					// Skip launchers that fail detection
				}
			}
			return launchers
		},
		async getImportableInstances(launcherName: string, path: string) {
			return (await get_importable_instances(launcherName, path)) ?? []
		},
		async getLoaderVersions(loader: string, gameVersion: string) {
			const manifest = await get_loader_versions(loader, gameVersion)
			const gameVersions = manifest?.gameVersions ?? manifest?.game_versions ?? []
			const versionGroups = manifest?.versionGroups ?? manifest?.version_groups ?? []
			const entry = gameVersions.find(
				(version) =>
					(version.id ?? '').replace('${modrinth.gameVersion}', gameVersion) === gameVersion,
			)
			let loaders: Array<string | { id?: string }> = []
			if (entry) {
				if (entry.versionGroup) {
					loaders = versionGroups.find((group) => group.id === entry.versionGroup)?.loaders ?? []
				} else {
					loaders = entry.loaders ?? entry.loader_versions ?? []
				}
			}
			const versions = loaders.map((loaderVersion) =>
				typeof loaderVersion === 'string' ? loaderVersion : loaderVersion.id,
			)
			console.debug('[InstanceImport] loader versions', loader, gameVersion, versions.length)
			return [...new Set(versions.filter((version): version is string => !!version))]
		},
		openPath: (path) => openPath(path),
		startImportPlan: (request) => start_import_plan(request),
		cancelImportPlan: (requestId) => cancel_import_plan(requestId),
		listenImportPlan: (callback) => import_plan_listener(callback),
		async importInstances(selections) {
			for (const sel of selections) {
				for (let i = 0; i < sel.instanceNames.length; i++) {
					const instanceName = sel.instanceNames[i]
					const instancePath = sel.instancePaths?.[i]
					try {
						const job = await import_instance(
							sel.launcherType ?? sel.launcher,
							sel.path,
							instanceName,
							sel.symlink ?? false,
							instancePath,
							sel.gameVersion,
							sel.loader,
							sel.loaderVersion,
							sel.gameDirOverride ?? null,
						)
						await wait_for_install_job(job.job_id)
					} catch (error) {
						handleError(error)
					}
				}
			}
		},
		async selectDirectory() {
			const result = await open({ multiple: false, directory: true })
			return result?.toString() ?? null
		},
		async selectDirectories() {
			const result = await open({ multiple: true, directory: true })
			if (!result) return null
			if (Array.isArray(result)) return result.map((p) => p.toString())
			return [result.toString()]
		},
	})
}
