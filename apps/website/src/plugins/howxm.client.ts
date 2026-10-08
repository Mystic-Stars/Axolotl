import { initHowxm } from 'howxm-js'

const HOWXM_APP_ID = '0b1c924f-5bca-4865-a52f-e3416fd26e46'

export default defineNuxtPlugin((nuxtApp) => {
    nuxtApp.hook('app:mounted', () => {
        initHowxm(HOWXM_APP_ID)
    })
})
