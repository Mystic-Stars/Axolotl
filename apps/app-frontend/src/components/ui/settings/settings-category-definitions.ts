import { defineMessage, type MessageDescriptor } from '@modrinth/ui'

export type SettingsCategoryId =
	| 'interface'
	| 'home-navigation'
	| 'language-translation'
	| 'ai'
	| 'java-performance'
	| 'launch-defaults'
	| 'content-downloads'
	| 'network-multiplayer'
	| 'storage-backups'
	| 'privacy-data'
	| 'updates'
	| 'about'
	| 'feature-flags'

export type SettingsGroupId = 'launcher' | 'game' | 'data-privacy' | 'support' | 'developer'

export interface SettingsCategoryDefinition {
	id: SettingsCategoryId
	name: MessageDescriptor
	group: SettingsGroupId
	flushContent?: boolean
	developerOnly?: boolean
	onboardingId?: string
}

export const settingsCategoryDefinitions: SettingsCategoryDefinition[] = [
	{
		id: 'interface',
		name: defineMessage({
			id: 'app.settings.tabs.interface',
			defaultMessage: 'Interface & appearance',
		}),
		group: 'launcher',
		onboardingId: 'settings-tab-interface',
	},
	{
		id: 'home-navigation',
		name: defineMessage({
			id: 'app.settings.tabs.home-navigation',
			defaultMessage: 'Home & navigation',
		}),
		group: 'launcher',
		onboardingId: 'settings-tab-home-navigation',
	},
	{
		id: 'language-translation',
		name: defineMessage({
			id: 'app.settings.tabs.language-translation',
			defaultMessage: 'Language & translation',
		}),
		group: 'launcher',
		onboardingId: 'settings-tab-language-translation',
	},
	{
		id: 'ai',
		name: defineMessage({ id: 'app.settings.tabs.ai', defaultMessage: 'AI' }),
		group: 'launcher',
		flushContent: true,
		onboardingId: 'settings-tab-ai',
	},
	{
		id: 'java-performance',
		name: defineMessage({
			id: 'app.settings.tabs.java-performance',
			defaultMessage: 'Java & performance',
		}),
		group: 'game',
		onboardingId: 'settings-tab-java-performance',
	},
	{
		id: 'launch-defaults',
		name: defineMessage({
			id: 'app.settings.tabs.launch-defaults',
			defaultMessage: 'Launch & instance defaults',
		}),
		group: 'game',
		onboardingId: 'settings-tab-launch-defaults',
	},
	{
		id: 'content-downloads',
		name: defineMessage({
			id: 'app.settings.tabs.content-downloads',
			defaultMessage: 'Content & downloads',
		}),
		group: 'game',
		onboardingId: 'settings-tab-content-downloads',
	},
	{
		id: 'network-multiplayer',
		name: defineMessage({
			id: 'app.settings.tabs.network-multiplayer',
			defaultMessage: 'Network & multiplayer',
		}),
		group: 'game',
		onboardingId: 'settings-tab-network-multiplayer',
	},
	{
		id: 'storage-backups',
		name: defineMessage({
			id: 'app.settings.tabs.storage-backups',
			defaultMessage: 'Storage & backups',
		}),
		group: 'data-privacy',
		onboardingId: 'settings-tab-storage-backups',
	},
	{
		id: 'privacy-data',
		name: defineMessage({
			id: 'app.settings.tabs.privacy-data',
			defaultMessage: 'Privacy & data sharing',
		}),
		group: 'data-privacy',
		onboardingId: 'settings-tab-privacy-data',
	},
	{
		id: 'updates',
		name: defineMessage({ id: 'app.settings.tabs.updates', defaultMessage: 'Updates' }),
		group: 'support',
		onboardingId: 'settings-tab-updates',
	},
	{
		id: 'about',
		name: defineMessage({ id: 'app.settings.tabs.about', defaultMessage: 'About' }),
		group: 'support',
	},
	{
		id: 'feature-flags',
		name: defineMessage({
			id: 'settings.feature-flags.title',
			defaultMessage: 'Feature flags',
		}),
		group: 'developer',
		developerOnly: true,
	},
]

export function getVisibleSettingsCategoryDefinitions(developerMode: boolean) {
	return settingsCategoryDefinitions.filter((category) => !category.developerOnly || developerMode)
}
