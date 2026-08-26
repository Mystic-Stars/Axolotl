import {
	install_job_get,
	install_job_list,
	installJobInstanceId,
	type InstallJobSnapshot,
} from '@/helpers/install'
import { execute_instance_upgrade } from '@/helpers/instance-upgrade'

import {
	isInstanceUpgradeJobWith,
	selectRecoverableUpgradeJobWith,
	submitInstanceUpgradeWith,
	type UpgradeJobSelectionContext,
	type UpgradeSubmissionLock,
	type UpgradeSubmissionRequest,
	type UpgradeSubmissionResult,
} from './install-job-core'

export { isRecoverableUpgradeStatus } from './install-job-core'

export function isInstanceUpgradeJob(job: InstallJobSnapshot, instanceId: string): boolean {
	return isInstanceUpgradeJobWith(job, instanceId, installJobInstanceId)
}

export function selectRecoverableUpgradeJob(
	jobs: InstallJobSnapshot[],
	instanceId: string,
	context: UpgradeJobSelectionContext = {},
): InstallJobSnapshot | null {
	return selectRecoverableUpgradeJobWith(jobs, instanceId, context, installJobInstanceId)
}

export async function recoverInstanceUpgradeJob(
	instanceId: string,
	context: UpgradeJobSelectionContext = {},
): Promise<InstallJobSnapshot | null> {
	if (context.knownJobId) {
		const known = await install_job_get(context.knownJobId).catch(() => null)
		if (known && isInstanceUpgradeJob(known, instanceId)) return known
	}

	const jobs = await install_job_list(true)
	return selectRecoverableUpgradeJob(jobs, instanceId, context)
}

export function submitInstanceUpgrade(
	request: UpgradeSubmissionRequest,
	lock: UpgradeSubmissionLock,
): Promise<UpgradeSubmissionResult | null> {
	return submitInstanceUpgradeWith(request, lock, {
		listJobs: install_job_list,
		execute: execute_instance_upgrade,
		instanceIdOf: installJobInstanceId,
	})
}
