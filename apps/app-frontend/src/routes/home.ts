import './meta'

import type { RouteRecordRaw } from 'vue-router'

export const homeRoutes: RouteRecordRaw[] = [
	{
		path: '/',
		name: 'Home',
		component: () => import('@/pages/Index.vue'),
		meta: {
			breadcrumb: [{ name: 'Home' }],
			discordActivity: 'Idling...',
		},
	},
]
