import { defineMessages } from '@modrinth/ui'

export const storageMessages = defineMessages({
	total: {
		id: 'app.settings.storage.total',
		defaultMessage: 'Total size',
	},
	totalDescription: {
		id: 'app.settings.storage.total-description',
		defaultMessage: 'Total size including symbolic links and junctions',
	},
	instanceData: {
		id: 'app.settings.storage.instance-data',
		defaultMessage: 'Instance data',
	},
	cacheData: {
		id: 'app.settings.storage.cache-data',
		defaultMessage: 'Cache data',
	},
	metaData: {
		id: 'app.settings.storage.meta-data',
		defaultMessage: 'Meta data',
	},
	database: {
		id: 'app.settings.storage.database',
		defaultMessage: 'Database',
	},
	other: {
		id: 'app.settings.storage.other',
		defaultMessage: 'Other',
	},
	instance: {
		id: 'app.settings.storage.instance',
		defaultMessage: 'Instance',
	},
	mods: {
		id: 'app.settings.storage.mods',
		defaultMessage: 'Mods',
	},
	replay: {
		id: 'app.settings.storage.replay',
		defaultMessage: 'Replay recordings',
	},
	resourcepacks: {
		id: 'app.settings.storage.resourcepacks',
		defaultMessage: 'Resource packs',
	},
	saves: {
		id: 'app.settings.storage.saves',
		defaultMessage: 'Saves',
	},
	world: {
		id: 'app.settings.storage.world',
		defaultMessage: 'World',
	},
	schematics: {
		id: 'app.settings.storage.schematics',
		defaultMessage: 'Schematics',
	},
	screenshots: {
		id: 'app.settings.storage.screenshots',
		defaultMessage: 'Screenshots',
	},
	shaderpacks: {
		id: 'app.settings.storage.shaderpacks',
		defaultMessage: 'Shader packs',
	},
	minimap: {
		id: 'app.settings.storage.minimap',
		defaultMessage: 'Minimap data',
	},
	distantHorizons: {
		id: 'app.settings.storage.distant-horizons',
		defaultMessage: 'Distant Horizons cache',
	},
	dbFile: {
		id: 'app.settings.storage.db-file',
		defaultMessage: 'App database',
	},
	dbBackup: {
		id: 'app.settings.storage.db-backup',
		defaultMessage: 'Database backups',
	},
	itemCount: {
		id: 'app.settings.storage.item-count',
		defaultMessage: '{count} items',
	},
	instanceCount: {
		id: 'app.settings.storage.instance-count',
		defaultMessage: '{count} instances',
	},
	actualSizeTooltip: {
		id: 'app.settings.storage.actual-size-tooltip',
		defaultMessage: 'Actual size: {size}',
	},
	symlinkSizeTooltip: {
		id: 'app.settings.storage.symlink-size-tooltip',
		defaultMessage: 'Symbolic link or junction referenced size: {size}',
	},
	openAction: {
		id: 'app.settings.storage.open-action',
		defaultMessage: 'Open in launcher or open location',
	},
	symlinkLabel: {
		id: 'app.settings.storage.symlink-label',
		defaultMessage: 'symlink',
	},
	symlinkHelp: {
		id: 'app.settings.storage.symlink-help',
		defaultMessage: 'What is a symlink?',
	},
	symlinkHelpTooltip: {
		id: 'app.settings.storage.symlink-help-tooltip',
		defaultMessage:
			'Symbolic links reference Minecraft resources from other locations — these files may actually live in another launcher\u2019s directory.\nFor example, \u201c20MB + 1.2GB\u201d means the launcher folder contains 20MB of files and references an external 1.2GB of files.',
	},
	update: {
		id: 'app.settings.storage.update',
		defaultMessage: 'Update',
	},
	updating: {
		id: 'app.settings.storage.updating',
		defaultMessage: 'Updating…',
	},
	lastUpdatedLabel: {
		id: 'app.settings.storage.last-updated-label',
		defaultMessage: 'Last updated:',
	},
	scanning: {
		id: 'app.settings.storage.scanning',
		defaultMessage: 'Scanning storage…',
	},
	storageTab: {
		id: 'app.settings.tabs.storage',
		defaultMessage: 'Storage',
	},
})
