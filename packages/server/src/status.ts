import type { ServerStatus, ServerStatusInput } from './types.ts'

export function computeServerStatus(input: ServerStatusInput): ServerStatus {
	if (input.isRunning) return 'running'
	if (input.isStarting) return 'starting'
	if (input.lastExitWasCrash) return 'crashed'
	if (!input.eulaAccepted) return input.eulaFileExists ? 'eula_pending' : 'created'
	return 'ready'
}
