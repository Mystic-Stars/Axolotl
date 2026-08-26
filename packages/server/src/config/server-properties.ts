import { inferFieldKindFromValue, type ConfigFileDefinition } from './types.ts'
import { registerConfigFile } from './registry.ts'

const boolean = (key: string) => ({ key, kind: 'boolean' as const })
const integer = (key: string, min?: number, max?: number) => ({
	key,
	kind: 'integer' as const,
	min,
	max,
})
const string = (key: string) => ({ key, kind: 'string' as const })
const enumeration = (key: string, options: string[]) => ({ key, kind: 'enum' as const, options })

/**
 * Known server.properties fields. Keys not listed here fall back to automatic
 * type inference from the current value.
 */
export const serverPropertiesDefinition: ConfigFileDefinition = {
	id: 'server-properties',
	filename: 'server.properties',
	inferFieldKind: (_key, value) => inferFieldKindFromValue(value),
	fields: [
		integer('server-port', 1, 65535),
		enumeration('difficulty', ['peaceful', 'easy', 'normal', 'hard']),
		enumeration('gamemode', ['survival', 'creative', 'adventure', 'spectator']),
		enumeration('level-type', [
			'minecraft:normal',
			'minecraft:flat',
			'minecraft:large_biomes',
			'minecraft:amplified',
			'minecraft:single_biome',
			'minecraft:debug',
			'normal',
			'flat',
			'largeBiomes',
			'amplified',
			'default',
		]),
		integer('max-players', 0, 1000),
		integer('view-distance', 2, 32),
		integer('simulation-distance', 2, 32),
		integer('max-tick-time', -1),
		integer('max-world-size', 1, 29999984),
		integer('op-permission-level', 0, 4),
		integer('function-permission-level', 0, 4),
		integer('spawn-protection', 0, 256),
		integer('player-idle-timeout', 0),
		integer('network-compression-threshold', -1, 1024),
		integer('rate-limit', 0),
		integer('query.port', 1, 65535),
		integer('rcon.port', 1, 65535),
		string('level-name'),
		string('level-seed'),
		string('motd'),
		string('resource-pack'),
		string('resource-pack-sha1'),
		string('resource-pack-prompt'),
		string('rcon.password'),
		string('server-ip'),
		string('text-filtering-config'),
		string('initial-enabled-packs'),
		boolean('online-mode'),
		boolean('white-list'),
		boolean('enforce-whitelist'),
		boolean('enforce-secure-profile'),
		boolean('prevent-proxy-connections'),
		boolean('allow-flight'),
		boolean('allow-nether'),
		boolean('spawn-animals'),
		boolean('spawn-monsters'),
		boolean('spawn-npcs'),
		boolean('pvp'),
		boolean('enable-command-block'),
		boolean('enable-status'),
		boolean('enable-query'),
		boolean('enable-rcon'),
		boolean('enable-jmx-monitoring'),
		boolean('force-gamemode'),
		boolean('hardcore'),
		boolean('announce-player-achievements'),
		boolean('log-ips'),
		boolean('hide-online-players'),
		boolean('require-resource-pack'),
		boolean('sync-chunk-writes'),
		boolean('use-native-transport'),
		boolean('allow-end'),
		boolean('generate-structures'),
		boolean('enable-lan'),
	],
}

registerConfigFile(serverPropertiesDefinition)
