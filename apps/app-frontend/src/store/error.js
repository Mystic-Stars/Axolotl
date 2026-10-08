import { defineStore } from 'pinia'

import { findMinecraftAuthError } from '@/components/ui/minecraft-auth-error-modal/minecraft-auth-errors'
import { isOfflineAccountRestrictedError } from '@/composables/useAntiPiracyStatus'

export const useError = defineStore('errorsStore', {
    state: () => ({
        errorModal: null,
        minecraftAuthErrorModal: null,
        minecraftLaunchErrorHandler: null,
        antiPiracyNoticeModal: null,
    }),
    actions: {
        setErrorModal(ref) {
            this.errorModal = ref
        },
        setMinecraftAuthErrorModal(ref) {
            this.minecraftAuthErrorModal = ref
        },
        setAntiPiracyNoticeModal(ref) {
            this.antiPiracyNoticeModal = ref
        },
        showAntiPiracyNotice() {
            this.antiPiracyNoticeModal?.show()
        },
        setMinecraftLaunchErrorHandler(handler) {
            this.minecraftLaunchErrorHandler = handler
        },
        showError(error, context, closable = true, source = null) {
            if (isOfflineAccountRestrictedError(error)) {
                this.showAntiPiracyNotice()
                return
            }
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
