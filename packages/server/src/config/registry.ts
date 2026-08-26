import type { ConfigFileDefinition } from './types.ts'

const registry = new Map<string, ConfigFileDefinition>()

export function registerConfigFile(definition: ConfigFileDefinition): void {
	registry.set(definition.filename, definition)
}

export function getConfigFile(filename: string): ConfigFileDefinition | undefined {
	return registry.get(filename)
}

export function listConfigFiles(): ConfigFileDefinition[] {
	return [...registry.values()]
}
