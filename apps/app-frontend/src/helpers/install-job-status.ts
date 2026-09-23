export type InstallJobStatus =
	| 'queued'
	| 'running'
	| 'canceling'
	| 'waiting_for_user'
	| 'succeeded'
	| 'failed'
	| 'interrupted'
	| 'canceled'

export const ACTIVE_INSTALL_JOB_STATUSES: ReadonlySet<InstallJobStatus> = new Set([
	'queued',
	'running',
	'canceling',
	'waiting_for_user',
])

export function isActiveInstallJobStatus(status: InstallJobStatus): boolean {
	return ACTIVE_INSTALL_JOB_STATUSES.has(status)
}
