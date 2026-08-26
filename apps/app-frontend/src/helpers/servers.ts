import type { ServerTypeId } from '@modrinth/server'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface ServerManifestData {
	id: string
	name: string
	serverType: ServerTypeId
	gameVersion: string
	loaderVersion?: string
	jarName?: string
	iconPath?: string
	javaPath?: string
	memoryMb?: number
	jvmArgs: string[]
	createdAt: string
	lastStartedAt?: string
	lastExitCrashed: boolean
}

export interface ServerInfoData extends ServerManifestData {
	path: string
	running: boolean
	eulaExists: boolean
	eulaAccepted: boolean
	port: number | null
}

export type ServerEventPayload =
	| { event: 'log'; line: string }
	| { event: 'download_progress'; downloaded: number; total?: number }
	| { event: 'started' }
	| { event: 'stopped'; crashed: boolean }

export interface PortProcessInfoData {
	pid: number
	name?: string | null
}

const command = (name: string) => `plugin:servers|${name}`

export const servers = {
	list: () => invoke<ServerInfoData[]>(command('servers_list')),
	get: (serverId: string) => invoke<ServerInfoData>(command('servers_get'), { serverId }),
	create: (options: {
		name: string
		serverType: ServerTypeId
		gameVersion: string
		loaderVersion?: string
		javaPath?: string
		memoryMb?: number
	}) => invoke<ServerManifestData>(command('servers_create'), options),
	updateSettings: (
		serverId: string,
		options: {
			name?: string
			javaPath?: string
			memoryMb?: number
			jvmArgs?: string[]
		},
	) => invoke<ServerManifestData>(command('servers_update_settings'), { serverId, ...options }),
	setIcon: (serverId: string, iconPath: string | null) =>
		invoke<ServerManifestData>(command('servers_set_icon'), { serverId, iconPath }),
	delete: (serverId: string) => invoke<void>(command('servers_delete'), { serverId }),
	readFile: (serverId: string, file: string) =>
		invoke<string>(command('servers_read_file'), { serverId, file }),
	writeFile: (serverId: string, file: string, contents: string) =>
		invoke<void>(command('servers_write_file'), { serverId, file, contents }),
	downloadFile: (serverId: string, url: string, filename: string, expectedSha1?: string) =>
		invoke<void>(command('servers_download_file'), {
			serverId,
			url,
			filename,
			expectedSha1,
		}),
	start: (
		serverId: string,
		options?: { javaPath?: string; memoryMb?: number; jvmArgs?: string[] },
	) => invoke<void>(command('servers_start'), { serverId, ...options }),
	sendCommand: (serverId: string, commandText: string) =>
		invoke<void>(command('servers_send_command'), { serverId, command: commandText }),
	stop: (serverId: string) => invoke<void>(command('servers_stop'), { serverId }),
	kill: (serverId: string) => invoke<void>(command('servers_kill'), { serverId }),
	killPortProcess: (port: number) => invoke<void>(command('servers_kill_port_process'), { port }),
	portProcess: (port: number) =>
		invoke<PortProcessInfoData | null>(command('servers_port_process'), { port }),
	getLogBuffer: (serverId: string) =>
		invoke<string[]>(command('servers_get_log_buffer'), { serverId }),
	clearLog: (serverId: string) => invoke<void>(command('servers_clear_log'), { serverId }),
}

export async function serverEventListener(
	callback: (serverId: string, payload: ServerEventPayload) => void,
): Promise<() => void> {
	const unlisten = await listen<{ serverId: string; event: string } & ServerEventPayload>(
		'server',
		(event) => callback(event.payload.serverId, event.payload),
	)
	return unlisten
}
