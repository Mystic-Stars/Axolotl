import 'floating-vue/dist/style.css'
import 'overlayscrollbars/overlayscrollbars.css'
import '@/assets/stylesheets/global.css'

import { VueQueryPlugin } from '@tanstack/vue-query'
import FloatingVue from 'floating-vue'
import { createPinia } from 'pinia'
import { createApp } from 'vue'

import App from '@/App.vue'
import { overlayScrollbarsDirective } from '@/directives/overlayScrollbars'
import { installTelemetryHandlers } from '@/helpers/telemetry'
import i18nPlugin from '@/plugins/i18n'
import i18nDebugPlugin from '@/plugins/i18n-debug'
import router from '@/routes'

const pinia = createPinia()

const app = createApp(App)

installTelemetryHandlers()

app.use(VueQueryPlugin)
app.use(router)
app.use(pinia)
app.use(FloatingVue, {
	themes: {
		'ribbit-popout': {
			$extend: 'dropdown',
			placement: 'bottom-end',
			instantMove: true,
			distance: 8,
		},
		'dismissable-prompt': {
			$extend: 'dropdown',
			placement: 'bottom-start',
		},
	},
})
app.use(i18nPlugin)
app.use(i18nDebugPlugin)
app.directive('overlay-scrollbars', overlayScrollbarsDirective)

app.mount('#app')
