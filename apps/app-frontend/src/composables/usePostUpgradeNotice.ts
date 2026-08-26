import { useQuery } from '@tanstack/vue-query'
import { computed, type MaybeRefOrGetter, toValue } from 'vue'

import { get_post_upgrade_notice } from '@/helpers/instance'

export function postUpgradeNoticeQueryKey(instanceId: string) {
	return ['post-upgrade-notice', instanceId] as const
}

export function usePostUpgradeNotice(instanceId: MaybeRefOrGetter<string>) {
	return useQuery({
		queryKey: computed(() => postUpgradeNoticeQueryKey(toValue(instanceId))),
		queryFn: async () => {
			try {
				return await get_post_upgrade_notice(toValue(instanceId))
			} catch (error) {
				if (import.meta.env.DEV) {
					console.error('Failed to load post-upgrade notice', error)
				}
				throw error
			}
		},
		enabled: computed(() => toValue(instanceId).length > 0),
		staleTime: 0,
		refetchOnMount: 'always',
	})
}
