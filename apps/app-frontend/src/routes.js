import { createRouter, createWebHistory } from 'vue-router'

import {
	hasBrowseReturnSnapshot,
	isBrowseReturnNavigation,
	prepareBrowseReturnNavigation,
} from '@/helpers/browse-return-state.ts'
import { clearUpgradeFlow, peekUpgradeFlow } from '@/helpers/upgrade-return-state'

/**
 * Configures application routing. Add page to pages/index and then add to route table here.
 */
export default new createRouter({
	history: createWebHistory(),
	routes: [
		{
			path: '/',
			name: 'Home',
			component: () => import('@/pages/Index.vue'),
			meta: {
				breadcrumb: [{ name: 'Home' }],
				discordActivity: 'Idling...',
			},
		},
		{
			path: '/worlds',
			name: 'Worlds',
			component: () => import('@/pages/Worlds.vue'),
			meta: {
				breadcrumb: [{ name: 'Worlds' }],
			},
		},
		{
			path: '/create',
			name: 'Create',
			component: () => import('@/pages/Create.vue'),
			meta: { useRootContext: false },
		},
		{
			path: '/downloads',
			name: 'Downloads',
			component: () => import('@/pages/Downloads.vue'),
			meta: {
				breadcrumb: [{ name: 'Downloads' }],
				discordActivity: 'Idling...',
			},
		},
		{
			path: '/settings',
			name: 'Settings',
			component: () => import('@/pages/Settings.vue'),
			meta: {
				breadcrumb: [{ name: 'Settings' }],
				discordActivity: 'Idling...',
			},
		},
		{
			path: '/browse/favorites',
			name: 'Favorites',
			component: () => import('@/pages/Favorites.vue'),
			meta: {
				useContext: true,
				breadcrumb: [{ name: '?FavoritesTitle' }],
				discordActivity: 'Browsing favorites...',
				pageTransitionGroup: 'browse',
			},
		},
		{
			path: '/browse/:projectType',
			name: 'Discover content',
			component: () => import('@/pages/Browse.vue'),
			meta: {
				useContext: true,
				breadcrumb: [{ name: '?BrowseTitle' }],
				discordActivity: 'Browsing mods...',
				pageTransitionGroup: 'browse',
			},
		},
		{
			path: '/help/drop',
			name: 'DropHelp',
			component: () => import('@/pages/help/DropHelp.vue'),
			meta: {
				breadcrumb: [{ name: 'Drop help' }],
			},
		},
		{
			path: '/skins',
			name: 'Skin selector',
			component: () => import('@/pages/Skins.vue'),
			meta: {
				breadcrumb: [{ name: 'Skin selector' }],
				discordActivity: 'Changing skins...',
			},
		},
		{
			path: '/multiplayer',
			name: 'Multiplayer',
			component: () => import('@/pages/Multiplayer.vue'),
			meta: {
				breadcrumb: [{ name: 'Multiplayer' }],
				discordActivity: 'Idling...',
				pageTransitionGroup: 'multiplayer',
			},
			children: [
				{
					path: '',
					redirect: { name: 'MultiplayerServers' },
				},
				{
					path: 'servers',
					name: 'MultiplayerServers',
					component: () => import('@/components/multiplayer/servers/ServersOverview.vue'),
				},
				{
					path: 'servers/:id',
					name: 'MultiplayerServerDetail',
					component: () => import('@/components/multiplayer/servers/ServerDetail.vue'),
				},
				{
					path: 'servers/:id/studio',
					name: 'MultiplayerServerFileStudio',
					component: () => import('@/components/multiplayer/servers/ServerFileStudio.vue'),
					meta: {
						renderMode: 'fixed',
						breadcrumb: [{ name: 'Multiplayer', link: '/multiplayer/servers' }, { name: 'Studio' }],
					},
				},
				{
					path: 'rooms',
					name: 'MultiplayerRooms',
					component: () => import('@/components/multiplayer/MultiplayerRooms.vue'),
				},
			],
		},
		{
			path: '/lab',
			name: 'Lab',
			component: () => import('@/pages/Lab.vue'),
			meta: {
				breadcrumb: [{ name: 'Lab' }],
				discordActivity: 'Messing with labs...',
			},
		},
		{
			path: '/lab/skin-editor',
			name: 'Skin editor',
			component: () => import('@/pages/LabSkinEditor.vue'),
			meta: {
				breadcrumb: [{ name: 'Lab', link: '/lab' }, { name: 'Skin editor' }],
				discordActivity: 'Editing a skin...',
			},
		},
		{
			path: '/lab/gradient-text',
			name: 'Gradient text generator',
			component: () => import('@/pages/LabGradientText.vue'),
			meta: {
				breadcrumb: [{ name: 'Lab', link: '/lab' }, { name: 'Gradient text generator' }],
				discordActivity: 'Messing with labs...',
			},
		},
		{
			path: '/lab/recipe-generator',
			name: 'Recipe generator',
			component: () => import('@/pages/LabRecipeGenerator.vue'),
			meta: {
				breadcrumb: [{ name: 'Lab', link: '/lab' }, { name: 'Recipe generator' }],
				discordActivity: 'Messing with labs...',
			},
		},
		{
			path: '/lab/seed-map',
			name: 'Seed map',
			component: () => import('@/pages/LabSeedMap.vue'),
			meta: {
				breadcrumb: [{ name: 'Lab', link: '/lab' }, { name: 'Seed map' }],
				discordActivity: 'Messing with labs...',
			},
		},
		{
			path: '/lab/schematic-preview',
			name: 'Schematic workshop',
			component: () => import('@/pages/LabSchematicPreview.vue'),
			meta: {
				breadcrumb: [{ name: 'Lab', link: '/lab' }, { name: 'Schematic workshop' }],
				discordActivity: 'Messing with labs...',
			},
		},
		{
			path: '/lab/mod-translation',
			name: 'Mod translation',
			component: () => import('@/pages/LabModTranslation.vue'),
			meta: {
				breadcrumb: [{ name: 'Lab', link: '/lab' }, { name: 'Mod translation' }],
				discordActivity: 'Messing with labs...',
			},
		},
		{
			path: '/library',
			name: 'Library',
			component: () => import('@/pages/library/Index.vue'),
			meta: {
				breadcrumb: [{ name: 'Library' }],
				discordActivity: 'Browsing instances...',
				pageTransitionGroup: 'library',
			},
			children: [
				{
					path: '',
					name: 'Overview',
					component: () => import('@/pages/library/Overview.vue'),
				},
				{
					path: 'downloaded',
					name: 'Downloaded',
					component: () => import('@/pages/library/Downloaded.vue'),
				},
				{
					path: 'modpacks',
					name: 'Modpacks',
					component: () => import('@/pages/library/Modpacks.vue'),
				},
				{
					path: 'servers',
					name: 'LibraryServers',
					component: () => import('@/pages/library/Servers.vue'),
				},
				{
					path: 'custom',
					name: 'Custom',
					component: () => import('@/pages/library/Custom.vue'),
				},
			],
		},
		{
			path: '/:projectType(mod|plugin|datapack|resourcepack|shader|modpack)/:id/:rest(.*)*',
			redirect: (to) => {
				const rest = to.params.rest ? `/${[].concat(to.params.rest).join('/')}` : ''
				return `/project/${to.params.id}${rest}${to.hash}`
			},
		},
		{
			path: '/project/curseforge/:id',
			name: 'CurseForgeProject',
			component: () => import('@/pages/project/CurseForge.vue'),
			props: true,
			meta: {
				useContext: true,
				breadcrumb: [{ name: '?Project' }],
				discordActivity: 'Browsing mods...',
				pageTransitionGroup: 'curseforge-project',
			},
		},
		{
			path: '/project/curseforge/:id/versions',
			name: 'CurseForgeProjectVersions',
			component: () => import('@/pages/project/CurseForge.vue'),
			props: true,
			meta: {
				useContext: true,
				breadcrumb: [{ name: '?Project', link: '/project/curseforge/{id}' }, { name: 'Versions' }],
				discordActivity: 'Browsing mods...',
				pageTransitionGroup: 'curseforge-project',
			},
		},
		{
			path: '/project/curseforge/:id/gallery',
			name: 'CurseForgeProjectGallery',
			component: () => import('@/pages/project/CurseForge.vue'),
			props: true,
			meta: {
				useContext: true,
				breadcrumb: [{ name: '?Project', link: '/project/curseforge/{id}' }, { name: 'Gallery' }],
				discordActivity: 'Browsing mods...',
				pageTransitionGroup: 'curseforge-project',
			},
		},
		{
			path: '/project/mcarchive/:slug',
			name: 'McArchiveProject',
			component: () => import('@/pages/project/McArchive.vue'),
			props: true,
			meta: {
				useContext: true,
				breadcrumb: [{ name: '?Project' }],
				discordActivity: 'Browsing mods...',
				pageTransitionGroup: 'mcarchive-project',
			},
		},
		{
			path: '/project/planet-minecraft/:id',
			name: 'PlanetMinecraftProject',
			component: () => import('@/pages/project/PlanetMinecraft.vue'),
			props: true,
			meta: {
				useContext: true,
				breadcrumb: [{ name: '?Project' }],
				discordActivity: 'Browsing mods...',
				pageTransitionGroup: 'planet-minecraft-project',
			},
		},
		{
			path: '/project/:id',
			name: 'Project',
			component: () => import('@/pages/project/Index.vue'),
			props: true,
			meta: {
				discordActivity: 'Browsing mods...',
				pageTransitionGroup: 'project',
			},
			children: [
				{
					path: '',
					name: 'Description',
					component: () => import('@/pages/project/Description.vue'),
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?Project' }],
					},
				},
				{
					path: 'versions',
					name: 'Versions',
					component: () => import('@/pages/project/Versions.vue'),
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?Project', link: '/project/{id}/' }, { name: 'Versions' }],
					},
				},
				{
					path: 'version/:version',
					name: 'Version',
					component: () => import('@/pages/project/Version.vue'),
					props: true,
					meta: {
						useContext: true,
						breadcrumb: [
							{ name: '?Project', link: '/project/{id}/' },
							{ name: 'Versions', link: '/project/{id}/versions' },
							{ name: '?Version' },
						],
					},
				},
				{
					path: 'gallery',
					name: 'Gallery',
					component: () => import('@/pages/project/Gallery.vue'),
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?Project', link: '/project/{id}/' }, { name: 'Gallery' }],
					},
				},
			],
		},
		{
			path: '/instance/:id',
			name: 'Instance',
			component: () => import('@/pages/instance/Index.vue'),
			props: true,
			meta: {
				discordActivity: 'Browsing instances...',
				pageTransitionGroup: 'instance',
			},
			children: [
				{
					path: 'upgrade',
					component: () => import('@/pages/instance/upgrade/UpgradeShell.vue'),
					meta: {
						useRootContext: true,
						hideInstanceTabs: true,
						breadcrumb: [{ name: 'Upgrade' }],
					},
					children: [
						{
							path: '',
							name: 'InstanceUpgrade',
							component: () => import('@/pages/instance/upgrade/Select.vue'),
						},
						{
							path: 'compatibility',
							name: 'InstanceUpgradeCompatibility',
							component: () => import('@/pages/instance/upgrade/Compatibility.vue'),
							meta: { upgradeRequirement: 'plan' },
						},
						{
							path: 'customize',
							name: 'InstanceUpgradeCustomize',
							component: () => import('@/pages/instance/upgrade/Customize.vue'),
							meta: { upgradeRequirement: 'unblocked-plan' },
						},
						{
							path: 'confirm',
							name: 'InstanceUpgradeConfirm',
							component: () => import('@/pages/instance/upgrade/Confirm.vue'),
							meta: { upgradeRequirement: 'selection' },
						},
						{
							path: 'progress',
							name: 'InstanceUpgradeProgress',
							component: () => import('@/pages/instance/upgrade/Progress.vue'),
							meta: { upgradeRequirement: 'job' },
						},
						{
							path: 'result',
							name: 'InstanceUpgradeResult',
							component: () => import('@/pages/instance/upgrade/Result.vue'),
							meta: { upgradeRequirement: 'result' },
						},
					],
				},
				// {
				//   path: '',
				//   name: 'Overview',
				//   component: Instance.Overview,
				//   meta: {
				//     useRootContext: true,
				//   },
				// },
				{
					path: 'worlds',
					name: 'InstanceWorlds',
					component: () => import('@/pages/instance/Worlds.vue'),
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: 'Worlds' }],
					},
				},
				{
					path: 'worlds/:world/edit',
					name: 'InstanceWorldEditor',
					component: () => import('@/pages/instance/WorldEditor.vue'),
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: 'Worlds', link: '/instance/{id}/worlds' }, { name: 'Edit world' }],
					},
				},
				{
					path: 'screenshots',
					name: 'InstanceScreenshots',
					component: () => import('@/pages/instance/Screenshots.vue'),
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: 'Screenshots' }],
					},
				},
				{
					path: '',
					name: 'Mods',
					component: () => import('@/pages/instance/Mods.vue'),
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: 'Content' }],
					},
				},
				{
					path: 'projects/:type',
					name: 'ModsFilter',
					component: () => import('@/pages/instance/Mods.vue'),
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: 'Content' }],
					},
				},
				{
					path: 'files',
					name: 'Files',
					component: () => import('@/pages/instance/Files.vue'),
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: 'Files' }],
					},
				},
				{
					path: 'files/studio',
					name: 'FileStudio',
					component: () => import('@/pages/instance/FileStudio.vue'),
					meta: {
						renderMode: 'fixed',
						useRootContext: true,
						breadcrumb: [{ name: 'Files', link: '/instance/{id}/files' }, { name: 'Studio' }],
					},
				},
				{
					path: 'logs',
					name: 'Logs',
					component: () => import('@/pages/instance/Logs.vue'),
					meta: {
						renderMode: 'fixed',
						useRootContext: true,
						breadcrumb: [{ name: 'Logs' }],
					},
				},
			],
		},
	],
	linkActiveClass: 'router-link-active',
	linkExactActiveClass: 'router-link-exact-active',
	beforeEach(to, from) {
		const parkedUpgrade = peekUpgradeFlow()
		if (
			parkedUpgrade &&
			!to.path.startsWith('/project/') &&
			!to.fullPath.startsWith(parkedUpgrade.returnFullPath)
		) {
			clearUpgradeFlow()
		}
		if (to.path.startsWith('/browse/')) {
			prepareBrowseReturnNavigation(to.fullPath, from.path)
		}
	},
	scrollBehavior(to, from) {
		if (
			to.path.startsWith('/browse/') &&
			(isBrowseReturnNavigation(to.fullPath) || hasBrowseReturnSnapshot(to.fullPath))
		) {
			return false
		}
		if (to.path === from.path) return
		// Sometimes Vue's scroll behavior is not working as expected, so we need to manually scroll to top (especially on Linux)
		document.querySelector('.app-viewport')?.scrollTo(0, 0)
		return {
			el: '.app-viewport',
			top: 0,
		}
	},
})
