<script setup lang="ts">
import {
	Activity,
	CalendarDays,
	Clock3,
	Database,
	PackagePlus,
	Users,
} from 'lucide-vue-next'

import AppState from '~/components/AppState.vue'
import DistributionChart from '~/components/charts/DistributionChart.vue'
import TrendChart from '~/components/charts/TrendChart.vue'
import MetricCard from '~/components/MetricCard.vue'
import PageHeader from '~/components/PageHeader.vue'
import Alert from '~/components/ui/Alert.vue'
import Badge from '~/components/ui/Badge.vue'
import Card from '~/components/ui/Card.vue'
import Skeleton from '~/components/ui/Skeleton.vue'
import type { ActivityDto, DistributionsDto, OverviewDto } from '~/shared/types/telemetry'
import { formatUtcTimestamp } from '~/utils/format'

const range = useDashboardRange()
const refreshEpoch = useDashboardRefresh()
const requestFetch = useRequestFetch()

const { data, status, error, refresh } = await useAsyncData(
	'overview-data',
	async () => {
		const query = { range: range.value }
		const [overview, activity, distributions] = await Promise.all([
			requestFetch<OverviewDto>('/api/admin/overview', { query }),
			requestFetch<ActivityDto>('/api/admin/activity', { query }),
			requestFetch<DistributionsDto>('/api/admin/distributions', { query }),
		])
		return { overview, activity, distributions }
	},
	{ watch: [range, refreshEpoch] },
)

const metricDefinitions = [
	{ key: 'totalInstallations', title: '累计同意遥测安装', icon: Database, tone: 'green' },
	{ key: 'dau', title: '日活跃安装（DAU）', icon: Activity, tone: 'cyan' },
	{ key: 'wau', title: '周活跃安装（WAU）', icon: CalendarDays, tone: 'cyan' },
	{ key: 'mau', title: '月活跃安装（MAU）', icon: Users, tone: 'cyan' },
	{ key: 'newInstallationsToday', title: '今日新增安装', icon: PackagePlus, tone: 'green' },
] as const

const trendCharts = [
	{
		title: '每日活跃安装趋势',
		description: '按 UTC 自然日统计的唯一活跃安装。',
		series: [
			{
				key: 'activeInstallations' as const,
				label: '活跃安装',
				color: 'var(--chart-green)',
			},
		],
	},
	{
		title: '每日新增安装趋势',
		description: '按 UTC 自然日首次出现的同意遥测安装。',
		series: [{ key: 'newInstallations' as const, label: '新增安装', color: 'var(--chart-cyan)' }],
	},
]

const distributionCharts = computed(() => [
	{
		title: '启动器版本',
		description: '按上报版本统计的唯一活跃安装。',
		items: data.value?.distributions.versions ?? [],
		color: 'var(--chart-green)',
	},
	{
		title: '操作系统',
		description: '按上报平台统计的唯一活跃安装。',
		items: data.value?.distributions.platforms ?? [],
		color: 'var(--chart-gold)',
	},
	{
		title: 'CPU 架构',
		description: '按上报架构统计的唯一活跃安装。',
		items: data.value?.distributions.architectures ?? [],
		color: 'var(--chart-cyan)',
	},
])
</script>

<template>
	<div>
		<PageHeader
			title="数据总览"
			description="查看匿名启动器遥测的活跃度、增长和运行环境分布。"
		>
			<Badge v-if="data" variant="secondary" class="gap-1.5 px-2.5 py-1">
				<Clock3 class="size-3.5" />更新于 {{ formatUtcTimestamp(data.overview.generatedAt) }}
			</Badge>
		</PageHeader>
		<Alert title="统计口径" class="mb-5">
			所有统计仅包含主动同意遥测的匿名安装，时间均按 UTC 计算。
		</Alert>

		<div v-if="status === 'pending' && !data" data-state="loading">
			<div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
				<Skeleton v-for="index in 5" :key="index" class="h-32" />
			</div>
			<div class="mt-5 grid gap-4 xl:grid-cols-2">
				<Skeleton v-for="index in 4" :key="index" class="h-80" />
			</div>
		</div>
		<AppState v-else-if="error || !data" compact kind="error" @retry="refresh" />
		<template v-else>
			<div
				class="grid grid-cols-2 gap-3 lg:grid-cols-4"
				:class="status === 'pending' && 'opacity-65'"
				:aria-busy="status === 'pending'"
			>
				<MetricCard
					v-for="metric in metricDefinitions"
					:key="metric.key"
					:title="metric.title"
					:value="data.overview.metrics[metric.key].value"
					:detail="data.overview.metrics[metric.key].label"
					:icon="metric.icon"
					:tone="metric.tone"
				/>
			</div>

			<section class="mt-6">
				<div class="mb-3">
					<h2 class="text-sm font-semibold">使用者趋势</h2>
					<p class="mt-0.5 text-xs text-muted-foreground">当前范围 {{ range }}，截止今日 UTC。</p>
				</div>
				<div class="grid gap-4 xl:grid-cols-2">
					<Card v-for="chart in trendCharts" :key="chart.title" class="min-w-0 p-4">
						<h3 class="text-sm font-semibold">{{ chart.title }}</h3>
						<p class="mt-1 text-xs text-muted-foreground">{{ chart.description }}</p>
						<TrendChart class="mt-3" :data="data.activity.points" :series="chart.series" />
					</Card>
				</div>
			</section>

			<section class="mt-6">
				<div class="mb-3">
					<h2 class="text-sm font-semibold">运行环境分布</h2>
					<p class="mt-0.5 text-xs text-muted-foreground">所选时间范围内的唯一活跃安装。</p>
				</div>
				<div class="grid gap-4 lg:grid-cols-3">
					<Card v-for="chart in distributionCharts" :key="chart.title" class="min-w-0 p-4">
						<h3 class="text-sm font-semibold">{{ chart.title }}</h3>
						<p class="mt-1 text-xs text-muted-foreground">{{ chart.description }}</p>
						<DistributionChart class="mt-4" :items="chart.items" :color="chart.color" />
					</Card>
				</div>
			</section>
		</template>
	</div>
</template>
