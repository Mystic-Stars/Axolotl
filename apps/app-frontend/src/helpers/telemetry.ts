import { invoke } from '@tauri-apps/api/core'

export function installTelemetryHandlers(): void {
	window.addEventListener('online', () => {
		void invoke('plugin:telemetry|notify_online').catch(() => undefined)
	})
}
