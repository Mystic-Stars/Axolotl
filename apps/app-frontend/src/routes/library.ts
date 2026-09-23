import './meta'

import type { RouteRecordRaw } from 'vue-router'

export const libraryRoutes: RouteRecordRaw[] = [
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
]
