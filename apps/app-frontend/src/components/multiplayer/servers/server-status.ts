import type { ServerStatus } from '@modrinth/server'
import { defineMessages } from '@modrinth/ui'

export const serverStatusMessages = defineMessages({
	created: { id: 'app.servers.status.created', defaultMessage: 'Not set up' },
	eulaPending: { id: 'app.servers.status.eula-pending', defaultMessage: 'EULA pending' },
	ready: { id: 'app.servers.status.ready', defaultMessage: 'Ready' },
	starting: { id: 'app.servers.status.starting', defaultMessage: 'Starting' },
	running: { id: 'app.servers.status.running', defaultMessage: 'Running' },
	crashed: { id: 'app.servers.status.crashed', defaultMessage: 'Crashed' },
})

export interface ServerStatusMeta {
	label: (typeof serverStatusMessages)[keyof typeof serverStatusMessages]
	color: string
}

export const SERVER_STATUS_META: Record<ServerStatus, ServerStatusMeta> = {
	created: { label: serverStatusMessages.created, color: 'text-secondary' },
	eula_pending: { label: serverStatusMessages.eulaPending, color: 'text-orange' },
	ready: { label: serverStatusMessages.ready, color: 'text-brand' },
	starting: { label: serverStatusMessages.starting, color: 'text-orange' },
	running: { label: serverStatusMessages.running, color: 'text-green' },
	crashed: { label: serverStatusMessages.crashed, color: 'text-red' },
}

/** Idle/closed states that should not render a status tag. */
export function isServerStatusVisible(status: ServerStatus): boolean {
	return status !== 'created' && status !== 'ready'
}
