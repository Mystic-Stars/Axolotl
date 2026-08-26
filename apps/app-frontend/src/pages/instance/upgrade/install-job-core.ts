import type { InstallJobSnapshot, InstallJobStatus } from '@/helpers/install'
import type { InstanceUpgradeDisplayNames, SharedUpgradeMode } from '@/helpers/instance-upgrade'

const RECOVERABLE_UPGRADE_STATUSES = new Set<InstallJobStatus>([
	'queued',
	'running',
	'canceling',
	'waiting_for_user',
])

export type InstallJobInstanceIdResolver = (job: InstallJobSnapshot) => string | null

export interface UpgradeJobSelectionContext {
	knownJobId?: string | null
	continuation?: boolean
}

export interface UpgradeSubmissionRequest {
	instanceId: string
	planId: string
	createFullBackup: boolean
	sharedUpgradeMode: SharedUpgradeMode
	displayNames: InstanceUpgradeDisplayNames
}

export interface UpgradeSubmissionLock {
	value: boolean
}

export interface UpgradeSubmissionResult {
	job: InstallJobSnapshot
	attached: boolean
}

export interface UpgradeSubmissionDependencies {
	listJobs: (includeFinished: boolean) => Promise<InstallJobSnapshot[]>
	execute: (
		planId: string,
		createFullBackup: boolean,
		sharedUpgradeMode: SharedUpgradeMode,
		displayNames: InstanceUpgradeDisplayNames,
	) => Promise<InstallJobSnapshot>
	instanceIdOf: InstallJobInstanceIdResolver
}

export function isRecoverableUpgradeStatus(status: InstallJobStatus): boolean {
	return RECOVERABLE_UPGRADE_STATUSES.has(status)
}

export function isInstanceUpgradeJobWith(
	job: InstallJobSnapshot,
	instanceId: string,
	instanceIdOf: InstallJobInstanceIdResolver,
): boolean {
	if (job.kind !== 'upgrade_unmanaged_instance') return false
	return (job.source_instance_id ?? instanceIdOf(job)) === instanceId
}

function compareJobFreshness(a: InstallJobSnapshot, b: InstallJobSnapshot): number {
	return (
		b.modified.localeCompare(a.modified) ||
		b.created.localeCompare(a.created) ||
		b.job_id.localeCompare(a.job_id)
	)
}

export function selectRecoverableUpgradeJobWith(
	jobs: InstallJobSnapshot[],
	instanceId: string,
	context: UpgradeJobSelectionContext,
	instanceIdOf: InstallJobInstanceIdResolver,
): InstallJobSnapshot | null {
	const matching = jobs.filter((job) => isInstanceUpgradeJobWith(job, instanceId, instanceIdOf))
	if (context.knownJobId) {
		const known = matching.find((job) => job.job_id === context.knownJobId)
		if (known) return known
	}

	const active = matching.filter((job) => isRecoverableUpgradeStatus(job.status))
	if (active.length) return [...active].sort(compareJobFreshness)[0]

	if (!context.continuation) return null
	const completed = matching.filter(
		(job) =>
			job.status === 'succeeded' && job.upgrade_result !== null && job.upgrade_result !== undefined,
	)
	return completed.length ? [...completed].sort(compareJobFreshness)[0] : null
}

export async function submitInstanceUpgradeWith(
	request: UpgradeSubmissionRequest,
	lock: UpgradeSubmissionLock,
	dependencies: UpgradeSubmissionDependencies,
): Promise<UpgradeSubmissionResult | null> {
	if (lock.value) return null
	lock.value = true
	try {
		const jobs = await dependencies.listJobs(false)
		const active = selectRecoverableUpgradeJobWith(
			jobs,
			request.instanceId,
			{},
			dependencies.instanceIdOf,
		)
		if (active) return { job: active, attached: true }

		const job = await dependencies.execute(
			request.planId,
			request.createFullBackup,
			request.sharedUpgradeMode,
			request.displayNames,
		)
		if (!isInstanceUpgradeJobWith(job, request.instanceId, dependencies.instanceIdOf)) {
			throw new Error(
				'Upgrade execution returned an Install Job for a different instance or job kind',
			)
		}
		return { job, attached: false }
	} finally {
		lock.value = false
	}
}
