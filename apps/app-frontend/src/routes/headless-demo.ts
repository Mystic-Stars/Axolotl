import './meta'

import type { RouteRecordRaw } from 'vue-router'

/**
 * Isolated reka-ui token-mapping demo (dev scaffold). Not linked from production nav.
 */
export const headlessDemoRoutes: RouteRecordRaw[] = [
	{
		path: '/headless-demo',
		name: 'HeadlessDemo',
		component: () => import('@/pages/headless/HeadlessDemo.vue'),
		meta: {
			breadcrumb: [{ name: 'Headless demo' }],
		},
	},
]
