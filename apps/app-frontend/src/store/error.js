import { defineStore } from 'pinia'

import { findMinecraftAuthError } from '@/components/ui/minecraft-auth-error-modal/minecraft-auth-errors'

export const useError = defineStore('errorsStore', {
	state: () => ({
		errorModal: null,
		minecraftAuthErrorModal: null,
		minecraftLaunchErrorHandler: null,
	}),
	actions: {
		setErrorModal(ref) {
			this.errorModal = ref
		},
		setMinecraftAuthErrorModal(ref) {
			this.minecraftAuthErrorModal = ref
		},
		setMinecraftLaunchErrorHandler(handler) {
			this.minecraftLaunchErrorHandler = handler
		},
		showError(error, context, closable = true, source = null) {
			if (this.minecraftLaunchErrorHandler?.(error, context)) return
			if (
				error.message &&
				(error.message.includes('Minecraft authentication error:') ||
					findMinecraftAuthError(error.message)) &&
				this.minecraftAuthErrorModal
			) {
				this.minecraftAuthErrorModal.show(error)
				return
			}
			this.errorModal.show(error, context, closable, source)
		},
	},
})

export const handleSevereError = (err, context) => {
	const error = useError()
	error.showError(err, context)
	console.error(err)
}
