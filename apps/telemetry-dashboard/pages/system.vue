<script setup lang="ts">
import { Database, ShieldCheck } from 'lucide-vue-next'

import AppState from '~/components/AppState.vue'
import PageHeader from '~/components/PageHeader.vue'
import StatusRow from '~/components/system/StatusRow.vue'
import Card from '~/components/ui/Card.vue'
import Skeleton from '~/components/ui/Skeleton.vue'
import type { SystemDto } from '~/shared/types/telemetry'
import { formatUtcTimestamp } from '~/utils/format'

const refreshEpoch = useDashboardRefresh()
const requestFetch = useRequestFetch()
const { data, status, error, refresh } = await useAsyncData(
	'system-data',
	() => requestFetch<SystemDto>('/api/admin/system'),
	{ watch: [refreshEpoch] },
)
</script>

<template>
	<div>
		<PageHeader title="系统状态" description="检查采集服务、聚合任务和数据状态。" />
		<div v-if="status === 'pending' && !data" class="grid gap-4 lg:grid-cols-2" data-state="loading">
			<Skeleton v-for="index in 3" :key="index" class="h-48" />
		</div>
		<AppState v-else-if="error || !data" compact kind="error" @retry="refresh" />
		<template v-else>
			<div class="grid gap-4 lg:grid-cols-2" :class="status === 'pending' && 'opacity-65'">
				<Card class="overflow-hidden">
					<div class="border-b px-4 py-3">
						<h2 class="text-sm font-semibold">服务检查</h2>
						<p class="mt-0.5 text-xs text-muted-foreground">检查时间 {{ formatUtcTimestamp(data.generatedAt) }}</p>
					</div>
					<StatusRow name="公开遥测 Worker" :check="data.publicWorker" />
					<StatusRow name="D1 只读查询" :check="data.d1" />
					<StatusRow name="Cron 聚合任务" :check="data.cron" />
					<StatusRow name="Cloudflare 账户用量" :check="data.accountUsage" />
				</Card>
				<Card class="p-4">
					<h2 class="text-sm font-semibold">数据状态</h2>
					<p class="mt-1 text-xs text-muted-foreground">每日聚合的最近可用数据。</p>
					<div class="mt-7 flex items-center justify-between gap-4 text-sm">
						<span class="text-muted-foreground">最新数据日期</span>
						<span class="flex items-center gap-2 tabular-nums"><Database class="size-4" />{{ data.latestDataDay ? `${data.latestDataDay}（UTC）` : '暂无' }}</span>
					</div>
					<div class="mt-7 flex items-center gap-2 border-t pt-4 text-xs text-muted-foreground"><ShieldCheck class="size-4 text-emerald-600" />管理 API 只读访问</div>
				</Card>
			</div>
		</template>
	</div>
</template>
