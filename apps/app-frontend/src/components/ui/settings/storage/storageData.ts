export type StorageNodeType =
	| 'instances'
	| 'cache'
	| 'meta'
	| 'database'
	| 'other'
	| 'instance'
	| 'mods'
	| 'replay'
	| 'resourcepacks'
	| 'saves'
	| 'world'
	| 'schematics'
	| 'screenshots'
	| 'shaderpacks'
	| 'minimap'
	| 'distant-horizons'
	| 'db-file'
	| 'db-backup'

export interface StoragePath {
	path: string
	kind: 'file' | 'directory'
}

export interface StorageSize {
	actual: number
	symlink: number
}

export interface StorageNode {
	id: string
	type: StorageNodeType
	name?: string
	instance_id?: string
	size: StorageSize
	count?: number
	paths: StoragePath[]
	children?: StorageNode[]
}

export interface StorageTree {
	version?: number
	scannedAt?: string
	total: StorageSize
	categories: StorageNode[]
	rootOther: StorageNode | null
}

export function sortStorageChildren(children: StorageNode[] | undefined): StorageNode[] {
	if (!children) return []
	return [...children]
		.filter((child) => child.size.actual + child.size.symlink > 0)
		.sort((a, b) => {
			const aIsOther = a.type === 'other' ? 1 : 0
			const bIsOther = b.type === 'other' ? 1 : 0
			if (aIsOther !== bIsOther) return aIsOther - bIsOther
			const aTotal = a.size.actual + a.size.symlink
			const bTotal = b.size.actual + b.size.symlink
			return bTotal - aTotal
		})
}

const gb = (value: number) => Math.round(value * 1024 ** 3)
const mb = (value: number) => Math.round(value * 1024 ** 2)

const appData = 'C:/Users/You/AppData/Roaming/red.ghs.axolotl'

const instanceAPath = `${appData}/profiles/红石生电优化【Redstone Survival Optimization】`
const instanceBPath = `${appData}/profiles/Fabric 1.21.11`
const instanceCPath = `${appData}/profiles/Vanilla 26.2`
const instanceDPath = `${appData}/profiles/CurseForge Pack`

const worldNode = (
	id: string,
	name: string,
	actual: number,
	symlink = 0,
	parentPath: string,
): StorageNode => ({
	id,
	type: 'world',
	name,
	size: { actual, symlink },
	paths: [{ path: `${parentPath}/saves/${name}`, kind: 'directory' }],
})

const instanceANode: StorageNode = {
	id: 'instance-a',
	type: 'instance',
	name: '红石生电优化【Redstone Survival Optimization】',
	size: { actual: gb(20), symlink: gb(1.5) },
	paths: [{ path: instanceAPath, kind: 'directory' }],
	children: [
		{
			id: 'instance-a-mods',
			type: 'mods',
			size: { actual: gb(1.6), symlink: gb(0.2) },
			count: 42,
			paths: [{ path: `${instanceAPath}/mods`, kind: 'directory' }],
		},
		{
			id: 'instance-a-replay',
			type: 'replay',
			size: { actual: gb(5.2), symlink: 0 },
			paths: [
				{ path: `${instanceAPath}/flashback`, kind: 'directory' },
				{ path: `${instanceAPath}/replay_recordings`, kind: 'directory' },
			],
		},
		{
			id: 'instance-a-resourcepacks',
			type: 'resourcepacks',
			size: { actual: gb(0.8), symlink: 0 },
			count: 4,
			paths: [{ path: `${instanceAPath}/resourcepacks`, kind: 'directory' }],
		},
		{
			id: 'instance-a-saves',
			type: 'saves',
			size: { actual: gb(9.6), symlink: 0 },
			count: 2,
			paths: [{ path: `${instanceAPath}/saves`, kind: 'directory' }],
			children: [
				worldNode('instance-a-world1', 'world1', gb(8.4), 0, instanceAPath),
				worldNode('instance-a-world2', 'world2', gb(1.2), 0, instanceAPath),
			],
		},
		{
			id: 'instance-a-schematics',
			type: 'schematics',
			size: { actual: gb(0.15), symlink: 0 },
			count: 86,
			paths: [{ path: `${instanceAPath}/schematics`, kind: 'directory' }],
		},
		{
			id: 'instance-a-screenshots',
			type: 'screenshots',
			size: { actual: gb(0.6), symlink: 0 },
			count: 214,
			paths: [{ path: `${instanceAPath}/screenshots`, kind: 'directory' }],
		},
		{
			id: 'instance-a-shaderpacks',
			type: 'shaderpacks',
			size: { actual: gb(0.9), symlink: gb(0.1) },
			count: 3,
			paths: [{ path: `${instanceAPath}/shaderpacks`, kind: 'directory' }],
		},
		{
			id: 'instance-a-minimap',
			type: 'minimap',
			size: { actual: gb(0.05), symlink: gb(0.8) },
			paths: [
				{ path: `${instanceAPath}/voxelmap`, kind: 'directory' },
				{ path: `${instanceAPath}/xaero`, kind: 'directory' },
				{ path: `${instanceAPath}/XaeroWaypoints_BACKUP`, kind: 'directory' },
			],
		},
		{
			id: 'instance-a-distant-horizons',
			type: 'distant-horizons',
			size: { actual: gb(0.3), symlink: gb(0.4) },
			paths: [
				{ path: `${instanceAPath}/.voxy`, kind: 'directory' },
				{ path: `${instanceAPath}/Distant_Horizons_server_data`, kind: 'directory' },
			],
		},
		{
			id: 'instance-a-other',
			type: 'other',
			size: { actual: gb(0.8), symlink: 0 },
			paths: [{ path: instanceAPath, kind: 'directory' }],
		},
	],
}

const instanceBNode: StorageNode = {
	id: 'instance-b',
	type: 'instance',
	name: 'Fabric 1.21.11',
	size: { actual: gb(9.8), symlink: 0 },
	paths: [{ path: instanceBPath, kind: 'directory' }],
	children: [
		{
			id: 'instance-b-mods',
			type: 'mods',
			size: { actual: gb(2.4), symlink: 0 },
			count: 57,
			paths: [{ path: `${instanceBPath}/mods`, kind: 'directory' }],
		},
		{
			id: 'instance-b-replay',
			type: 'replay',
			size: { actual: gb(0.1), symlink: 0 },
			paths: [
				{ path: `${instanceBPath}/flashback`, kind: 'directory' },
				{ path: `${instanceBPath}/replay_recordings`, kind: 'directory' },
			],
		},
		{
			id: 'instance-b-resourcepacks',
			type: 'resourcepacks',
			size: { actual: gb(0.2), symlink: 0 },
			count: 3,
			paths: [{ path: `${instanceBPath}/resourcepacks`, kind: 'directory' }],
		},
		{
			id: 'instance-b-saves',
			type: 'saves',
			size: { actual: gb(5.6), symlink: 0 },
			count: 2,
			paths: [{ path: `${instanceBPath}/saves`, kind: 'directory' }],
			children: [
				worldNode('instance-b-world1', 'world1', gb(3.2), 0, instanceBPath),
				worldNode('instance-b-world2', 'world2', gb(2.4), 0, instanceBPath),
			],
		},
		{
			id: 'instance-b-schematics',
			type: 'schematics',
			size: { actual: gb(0.05), symlink: 0 },
			count: 12,
			paths: [{ path: `${instanceBPath}/schematics`, kind: 'directory' }],
		},
		{
			id: 'instance-b-screenshots',
			type: 'screenshots',
			size: { actual: gb(0.3), symlink: 0 },
			count: 76,
			paths: [{ path: `${instanceBPath}/screenshots`, kind: 'directory' }],
		},
		{
			id: 'instance-b-shaderpacks',
			type: 'shaderpacks',
			size: { actual: gb(0.4), symlink: 0 },
			count: 2,
			paths: [{ path: `${instanceBPath}/shaderpacks`, kind: 'directory' }],
		},
		{
			id: 'instance-b-minimap',
			type: 'minimap',
			size: { actual: gb(0.02), symlink: 0 },
			paths: [
				{ path: `${instanceBPath}/voxelmap`, kind: 'directory' },
				{ path: `${instanceBPath}/xaero`, kind: 'directory' },
				{ path: `${instanceBPath}/XaeroWaypoints_BACKUP`, kind: 'directory' },
			],
		},
		{
			id: 'instance-b-distant-horizons',
			type: 'distant-horizons',
			size: { actual: gb(0.1), symlink: 0 },
			paths: [
				{ path: `${instanceBPath}/.voxy`, kind: 'directory' },
				{ path: `${instanceBPath}/Distant_Horizons_server_data`, kind: 'directory' },
			],
		},
		{
			id: 'instance-b-other',
			type: 'other',
			size: { actual: gb(0.63), symlink: 0 },
			paths: [{ path: instanceBPath, kind: 'directory' }],
		},
	],
}

const instanceCNode: StorageNode = {
	id: 'instance-c',
	type: 'instance',
	name: 'Vanilla 26.2',
	size: { actual: gb(0.5), symlink: gb(12) },
	paths: [
		{ path: `${appData}/profiles/Vanilla 26.2`, kind: 'directory' },
		{ path: `${appData}/.minecraft/versions/Vanilla 26.2`, kind: 'directory' },
	],
	children: [
		{
			id: 'instance-c-mods',
			type: 'mods',
			size: { actual: gb(0.05), symlink: gb(0.5) },
			count: 1,
			paths: [{ path: `${instanceCPath}/mods`, kind: 'directory' }],
		},
		{
			id: 'instance-c-replay',
			type: 'replay',
			size: { actual: 0, symlink: 0 },
			paths: [
				{ path: `${instanceCPath}/flashback`, kind: 'directory' },
				{ path: `${instanceCPath}/replay_recordings`, kind: 'directory' },
			],
		},
		{
			id: 'instance-c-resourcepacks',
			type: 'resourcepacks',
			size: { actual: gb(0.02), symlink: gb(0.3) },
			count: 1,
			paths: [{ path: `${instanceCPath}/resourcepacks`, kind: 'directory' }],
		},
		{
			id: 'instance-c-saves',
			type: 'saves',
			size: { actual: gb(0.4), symlink: gb(9) },
			count: 1,
			paths: [{ path: `${instanceCPath}/saves`, kind: 'directory' }],
			children: [worldNode('instance-c-world1', 'world1', gb(0.4), gb(9), instanceCPath)],
		},
		{
			id: 'instance-c-schematics',
			type: 'schematics',
			size: { actual: 0, symlink: 0 },
			count: 0,
			paths: [{ path: `${instanceCPath}/schematics`, kind: 'directory' }],
		},
		{
			id: 'instance-c-screenshots',
			type: 'screenshots',
			size: { actual: 0, symlink: 0 },
			count: 0,
			paths: [{ path: `${instanceCPath}/screenshots`, kind: 'directory' }],
		},
		{
			id: 'instance-c-shaderpacks',
			type: 'shaderpacks',
			size: { actual: gb(0.01), symlink: gb(2) },
			count: 1,
			paths: [{ path: `${instanceCPath}/shaderpacks`, kind: 'directory' }],
		},
		{
			id: 'instance-c-minimap',
			type: 'minimap',
			size: { actual: gb(0.01), symlink: gb(0.1) },
			paths: [
				{ path: `${instanceCPath}/voxelmap`, kind: 'directory' },
				{ path: `${instanceCPath}/xaero`, kind: 'directory' },
				{ path: `${instanceCPath}/XaeroWaypoints_BACKUP`, kind: 'directory' },
			],
		},
		{
			id: 'instance-c-distant-horizons',
			type: 'distant-horizons',
			size: { actual: gb(0.01), symlink: gb(0.1) },
			paths: [
				{ path: `${instanceCPath}/.voxy`, kind: 'directory' },
				{ path: `${instanceCPath}/Distant_Horizons_server_data`, kind: 'directory' },
			],
		},
		{
			id: 'instance-c-other',
			type: 'other',
			size: { actual: 0, symlink: 0 },
			paths: [{ path: instanceCPath, kind: 'directory' }],
		},
	],
}

const instanceDNode: StorageNode = {
	id: 'instance-d',
	type: 'instance',
	name: 'CurseForge Pack',
	size: { actual: gb(4), symlink: gb(0.2) },
	paths: [{ path: instanceDPath, kind: 'directory' }],
	children: [
		{
			id: 'instance-d-mods',
			type: 'mods',
			size: { actual: gb(2), symlink: gb(0.1) },
			count: 35,
			paths: [{ path: `${instanceDPath}/mods`, kind: 'directory' }],
		},
		{
			id: 'instance-d-replay',
			type: 'replay',
			size: { actual: gb(0.05), symlink: 0 },
			paths: [
				{ path: `${instanceDPath}/flashback`, kind: 'directory' },
				{ path: `${instanceDPath}/replay_recordings`, kind: 'directory' },
			],
		},
		{
			id: 'instance-d-resourcepacks',
			type: 'resourcepacks',
			size: { actual: gb(0.3), symlink: 0 },
			count: 5,
			paths: [{ path: `${instanceDPath}/resourcepacks`, kind: 'directory' }],
		},
		{
			id: 'instance-d-saves',
			type: 'saves',
			size: { actual: gb(1), symlink: 0 },
			count: 2,
			paths: [{ path: `${instanceDPath}/saves`, kind: 'directory' }],
			children: [
				worldNode('instance-d-world1', 'world1', gb(0.6), 0, instanceDPath),
				worldNode('instance-d-world2', 'world2', gb(0.4), 0, instanceDPath),
			],
		},
		{
			id: 'instance-d-schematics',
			type: 'schematics',
			size: { actual: gb(0.1), symlink: 0 },
			count: 24,
			paths: [{ path: `${instanceDPath}/schematics`, kind: 'directory' }],
		},
		{
			id: 'instance-d-screenshots',
			type: 'screenshots',
			size: { actual: gb(0.2), symlink: 0 },
			count: 88,
			paths: [{ path: `${instanceDPath}/screenshots`, kind: 'directory' }],
		},
		{
			id: 'instance-d-shaderpacks',
			type: 'shaderpacks',
			size: { actual: gb(0.2), symlink: 0 },
			count: 4,
			paths: [{ path: `${instanceDPath}/shaderpacks`, kind: 'directory' }],
		},
		{
			id: 'instance-d-minimap',
			type: 'minimap',
			size: { actual: gb(0.03), symlink: 0 },
			paths: [
				{ path: `${instanceDPath}/voxelmap`, kind: 'directory' },
				{ path: `${instanceDPath}/xaero`, kind: 'directory' },
				{ path: `${instanceDPath}/XaeroWaypoints_BACKUP`, kind: 'directory' },
			],
		},
		{
			id: 'instance-d-distant-horizons',
			type: 'distant-horizons',
			size: { actual: gb(0.02), symlink: 0 },
			paths: [
				{ path: `${instanceDPath}/.voxy`, kind: 'directory' },
				{ path: `${instanceDPath}/Distant_Horizons_server_data`, kind: 'directory' },
			],
		},
		{
			id: 'instance-d-other',
			type: 'other',
			size: { actual: gb(0.1), symlink: gb(0.1) },
			paths: [{ path: instanceDPath, kind: 'directory' }],
		},
	],
}

const instancesCategory: StorageNode = {
	id: 'category-instances',
	type: 'instances',
	size: { actual: gb(34.7), symlink: gb(13.7) },
	count: 4,
	paths: [{ path: `${appData}/profiles`, kind: 'directory' }],
	children: [
		{
			id: 'profiles-root-other',
			type: 'other',
			size: { actual: gb(0.4), symlink: 0 },
			paths: [{ path: `${appData}/profiles`, kind: 'directory' }],
		},
		instanceANode,
		instanceBNode,
		instanceCNode,
		instanceDNode,
	],
}

const cacheCategory: StorageNode = {
	id: 'category-cache',
	type: 'cache',
	size: { actual: gb(2.1), symlink: 0 },
	count: 128,
	paths: [{ path: `${appData}/caches`, kind: 'directory' }],
}

const metaCategory: StorageNode = {
	id: 'category-meta',
	type: 'meta',
	size: { actual: gb(6.3), symlink: 0 },
	count: 3421,
	paths: [{ path: `${appData}/meta`, kind: 'directory' }],
}

const databaseCategory: StorageNode = {
	id: 'category-database',
	type: 'database',
	size: { actual: gb(0.5), symlink: 0 },
	count: 2,
	paths: [{ path: `${appData}`, kind: 'directory' }],
	children: [
		{
			id: 'database-app-db',
			type: 'db-file',
			size: { actual: mb(20), symlink: 0 },
			paths: [{ path: `${appData}/app.db`, kind: 'file' }],
		},
		{
			id: 'database-backups',
			type: 'db-backup',
			size: { actual: mb(480), symlink: 0 },
			paths: [{ path: `${appData}/Backups/app-db`, kind: 'directory' }],
		},
	],
}

const rootOther: StorageNode = {
	id: 'root-other',
	type: 'other',
	size: { actual: gb(0.3), symlink: 0 },
	paths: [
		{ path: `${appData}/launcher_logs`, kind: 'directory' },
		{ path: `${appData}/app-window-state.json`, kind: 'file' },
		{ path: `${appData}/download.log`, kind: 'file' },
		{ path: `${appData}/download-reputation.json`, kind: 'file' },
	],
}

export const storageTree: StorageTree = {
	total: { actual: gb(43.9), symlink: gb(13.7) },
	categories: [instancesCategory, cacheCategory, metaCategory, databaseCategory, rootOther],
	rootOther,
}
