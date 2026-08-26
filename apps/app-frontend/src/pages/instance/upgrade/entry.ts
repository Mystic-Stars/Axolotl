import type { InstallJobSnapshot } from '@/helpers/install'
import type { GameInstance } from '@/helpers/types'

export function isUnmanagedUpgradeEligible(instance: GameInstance): boolean {
	return (
		instance.install_stage === 'installed' &&
		Boolean(instance.game_version && instance.loader) &&
		(instance.link == null ||
			instance.link.type === 'shared_instance' ||
			Boolean(instance.symlink_target))
	)
}

export function isActiveUpgradeJobForInstance(
	job: InstallJobSnapshot,
	instanceId: string,
): boolean {
	return (
		job.kind === 'upgrade_unmanaged_instance' &&
		['queued', 'running', 'canceling', 'waiting_for_user'].includes(job.status) &&
		(job.source_instance_id ?? job.instance_id) === instanceId
	)
}
