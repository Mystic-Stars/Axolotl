export interface ServerLogSignal {
	eulaRequired?: boolean
	started?: boolean
	stopping?: boolean
}

const EULA_RE = /you need to agree to the eula|eula\.txt/i
const DONE_RE = /Done \([\d.]+s\)!/
const STOPPING_RE = /Stopping (the )?server|Stopping singleplayer server/i

/** Classifies a dedicated server log line for status tracking. */
export function classifyServerLogLine(line: string): ServerLogSignal {
	const signals: ServerLogSignal = {}
	if (EULA_RE.test(line)) signals.eulaRequired = true
	if (DONE_RE.test(line)) signals.started = true
	if (STOPPING_RE.test(line)) signals.stopping = true
	return signals
}

export interface ServerExitSummary {
	crashed: boolean
	eulaRequired: boolean
}

/**
 * Summarizes the outcome of a server process run from its log lines and exit
 * code. A clean exit right after the EULA notice is the expected first-run
 * behavior, not a crash.
 */
export function summarizeServerExit(lines: string[], exitCode: number | null): ServerExitSummary {
	const eulaRequired = lines.some((line) => classifyServerLogLine(line).eulaRequired)
	return {
		crashed: exitCode !== null && exitCode !== 0 && !eulaRequired,
		eulaRequired,
	}
}
