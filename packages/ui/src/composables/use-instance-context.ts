import { computed, type ComputedRef } from 'vue'
import { useRoute } from 'vue-router'

export interface InstanceContext {
    isInInstance: ComputedRef<boolean>
    instanceId: ComputedRef<string | null>
    currentPage: ComputedRef<string | null>
}

export function useInstanceContext(): InstanceContext {
    const route = useRoute()

    const instanceId = computed<string | null>(() => {
        const param = route.params.id
        if (param && typeof param === 'string') return param

        const query = route.query.i
        if (query && typeof query === 'string') return query

        return null
    })

    const isInInstance = computed<boolean>(() => {
        // /instance/:id and all subroutes (Mods, Files, Worlds, Screenshots, Logs)
        if (route.path.startsWith('/instance/')) return true

        // /browse/:type?i=:id
        if (route.path.startsWith('/browse/') && route.query.i) return true

        // /project/:id?i=:id
        if (route.path.startsWith('/project/') && route.query.i) return true

        return false
    })

    const currentPage = computed<string | null>(() => {
        const segments = route.path.split('/').filter(Boolean)
        return segments[0] ?? null
    })

    return { isInInstance, instanceId, currentPage }
}
