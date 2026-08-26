import { defineStore } from 'pinia'

export const useBreadcrumbs = defineStore('breadcrumbsStore', {
	state: () => ({
		names: new Map(),
		icons: new Map(),
		context: null,
		rootContext: null,
	}),
	actions: {
		getName(route) {
			return this.names.get(route) ?? ''
		},
		setName(route, title) {
			this.names.set(route, title)
		},
		setNameIcon(route, iconUrl) {
			if (iconUrl) this.icons.set(route, iconUrl)
			else this.icons.delete(route)
		},
		getIcon(route) {
			return this.icons.get(route) ?? null
		},
		// resets breadcrumbs to only included ones as to not have stale breadcrumbs
		resetToNames(breadcrumbs) {
			if (!breadcrumbs) return
			// names is an array of every breadcrumb.name that starts with a ?
			const names = breadcrumbs
				.filter((breadcrumb) => breadcrumb.name.charAt(0) === '?')
				.map((breadcrumb) => breadcrumb.name.slice(1))
			// remove all names that are not in the names array
			for (const [route] of this.names) {
				if (!names.includes(route)) {
					this.names.delete(route)
				}
			}
			for (const route of this.icons.keys()) {
				if (!names.includes(route)) this.icons.delete(route)
			}
		},
		setContext(context) {
			this.context = context
		},
		setContextIcon(iconUrl) {
			if (this.context) this.context = { ...this.context, iconUrl }
		},
		setRootContext(context) {
			this.rootContext = context
		},
		setRootContextIcon(iconUrl) {
			if (this.rootContext) this.rootContext = { ...this.rootContext, iconUrl }
		},
	},
})
