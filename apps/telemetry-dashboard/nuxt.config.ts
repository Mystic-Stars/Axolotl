import { fileURLToPath } from 'node:url'

const remoteBindingsEnabled =
	process.env.NODE_ENV === 'development' && process.env.TELEMETRY_ADMIN_REMOTE_BINDINGS === 'true'

export default defineNuxtConfig({
	compatibilityDate: '2025-08-13',
	devtools: { enabled: false },
	telemetry: false,
	css: ['~/assets/styles/tailwind.css'],
	app: {
		head: {
			htmlAttrs: { lang: 'zh-CN' },
			title: 'Axolotl 遥测中心',
			meta: [
				{ name: 'robots', content: 'noindex, nofollow' },
				{ name: 'color-scheme', content: 'light dark' },
			],
			script: [
				{
					key: 'theme-init',
					innerHTML:
						"(function(){try{var t=localStorage.getItem('telemetry-admin-theme')||'system';var d=document.documentElement;var dark=t==='dark'||(t==='system'&&matchMedia('(prefers-color-scheme: dark)').matches);d.classList.toggle('dark',dark)}catch(e){}})()",
				},
			],
		},
		pageTransition: { name: 'dashboard-page', mode: 'out-in' },
	},
	runtimeConfig: {
		accessTeamDomain: '',
		accessAudience: '',
		mockAuth: false,
		mockScenario: 'normal',
		publicWorkerHealthUrl: 'https://telemetry.axlmc.org/health',
		public: {
			adminOrigin: 'https://admin.axlmc.org',
		},
	},
	postcss: {
		plugins: {
			tailwindcss: {},
			autoprefixer: {},
		},
	},
	hooks: {
		'vite:extendConfig'(config, { isClient }) {
			if (process.env.NODE_ENV !== 'development') return
			if (!config.server) return
			const port = isClient ? 24679 : 24680
			config.server.hmr = { port, clientPort: port }
		},
	},
	nitro: {
		preset:
			process.env.NITRO_PRESET ||
			(remoteBindingsEnabled
				? 'node-server'
				: process.env.VERCEL === '1'
					? 'vercel'
					: 'cloudflare_module'),
		externals: remoteBindingsEnabled ? { external: ['wrangler'] } : undefined,
		plugins: remoteBindingsEnabled
			? [fileURLToPath(new URL('./server/dev/remote-bindings.ts', import.meta.url))]
			: [],
	},
	typescript: {
		strict: true,
		typeCheck: false,
	},
})
