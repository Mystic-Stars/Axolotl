import {
	ArchiveIcon,
	BotIcon,
	CoffeeIcon,
	CpuIcon,
	GameIcon,
	GaugeIcon,
	InfoIcon,
	LanguagesIcon,
	LayoutTemplateIcon,
	PaintbrushIcon,
	RefreshCwIcon,
	ShieldIcon,
	ToggleRightIcon,
	UsersIcon,
} from '@modrinth/assets'
import { commonMessages, defineMessages, type MessageDescriptor } from '@modrinth/ui'
import type { Component } from 'vue'

import AboutSettings from './AboutSettings.vue'
import AISettings from './AISettings.vue'
import AppearanceSettings from './AppearanceSettings.vue'
import ContentDownloadSettings from './ContentDownloadSettings.vue'
import DefaultInstanceSettings from './DefaultInstanceSettings.vue'
import FeatureFlagSettings from './FeatureFlagSettings.vue'
import HomeNavigationSettings from './HomeNavigationSettings.vue'
import JavaSettings from './JavaSettings.vue'
import LanguageTranslationSettings from './LanguageTranslationSettings.vue'
import NetworkMultiplayerSettings from './NetworkMultiplayerSettings.vue'
import PrivacySettings from './PrivacySettings.vue'
import {
	getVisibleSettingsCategoryDefinitions,
	type SettingsCategoryDefinition,
	settingsCategoryDefinitions,
	type SettingsCategoryId,
	type SettingsGroupId,
} from './settings-category-definitions'
import { settingsSearchEntries, type SettingsSearchEntry } from './settings-search-index'
import StorageBackupSettings from './StorageBackupSettings.vue'
import UpdateSettings from './UpdateSettings.vue'

export interface SettingsCategory extends SettingsCategoryDefinition {
	icon: Component
	content: Component
	entries: SettingsSearchEntry[]
}

export interface SettingsGroup {
	id: SettingsGroupId
	name: MessageDescriptor
	icon: Component
	categories: SettingsCategory[]
}

const categoryContent: Record<SettingsCategoryId, Pick<SettingsCategory, 'icon' | 'content'>> = {
	interface: { icon: PaintbrushIcon, content: AppearanceSettings },
	'home-navigation': { icon: LayoutTemplateIcon, content: HomeNavigationSettings },
	'language-translation': { icon: LanguagesIcon, content: LanguageTranslationSettings },
	ai: { icon: BotIcon, content: AISettings },
	'java-performance': { icon: CoffeeIcon, content: JavaSettings },
	'launch-defaults': { icon: GameIcon, content: DefaultInstanceSettings },
	'content-downloads': { icon: GaugeIcon, content: ContentDownloadSettings },
	'network-multiplayer': { icon: UsersIcon, content: NetworkMultiplayerSettings },
	'storage-backups': { icon: ArchiveIcon, content: StorageBackupSettings },
	'privacy-data': { icon: ShieldIcon, content: PrivacySettings },
	updates: { icon: RefreshCwIcon, content: UpdateSettings },
	about: { icon: InfoIcon, content: AboutSettings },
	'feature-flags': { icon: ToggleRightIcon, content: FeatureFlagSettings },
}

const messages = defineMessages({
	launcher: { id: 'app.settings.groups.launcher', defaultMessage: 'Launcher' },
	game: { id: 'app.settings.groups.game', defaultMessage: 'Game' },
	dataPrivacy: { id: 'app.settings.groups.data-privacy', defaultMessage: 'Data & privacy' },
	support: { id: 'app.settings.groups.support', defaultMessage: 'App & support' },
	developer: { id: 'app.settings.groups.developer', defaultMessage: 'Developer' },
})

export const settingsCategories: SettingsCategory[] = settingsCategoryDefinitions.map(
	(definition) => ({
		...definition,
		...categoryContent[definition.id],
		entries: settingsSearchEntries.filter((entry) => entry.categoryId === definition.id),
	}),
)

const settingsGroupDefinitions: Array<{
	id: SettingsGroupId
	name: MessageDescriptor
	icon: Component
}> = [
	{
		id: 'launcher',
		name: messages.launcher,
		icon: GaugeIcon,
	},
	{
		id: 'game',
		name: messages.game,
		icon: GameIcon,
	},
	{
		id: 'data-privacy',
		name: messages.dataPrivacy,
		icon: ShieldIcon,
	},
	{
		id: 'support',
		name: messages.support,
		icon: InfoIcon,
	},
	{
		id: 'developer',
		name: messages.developer,
		icon: CpuIcon,
	},
]

export function getVisibleSettingsCategories(developerMode: boolean): SettingsCategory[] {
	const visibleIds = new Set(
		getVisibleSettingsCategoryDefinitions(developerMode).map((category) => category.id),
	)
	return settingsCategories.filter((category) => visibleIds.has(category.id))
}

export function getVisibleSettingsGroups(developerMode: boolean): SettingsGroup[] {
	const categories = getVisibleSettingsCategories(developerMode)
	return settingsGroupDefinitions
		.map((group) => ({
			...group,
			categories: categories.filter((category) => category.group === group.id),
		}))
		.filter((group) => group.categories.length > 0)
}

export const settingsPageTitle: MessageDescriptor = commonMessages.settingsLabel
