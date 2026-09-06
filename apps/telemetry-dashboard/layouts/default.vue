<script setup lang="ts">
import {
	Activity,
	BarChart3,
	Database,
	Menu,
	RefreshCw,
	Search,
	Server,
	ShieldCheck,
	X,
} from 'lucide-vue-next'

import AppState from '~/components/AppState.vue'
import Badge from '~/components/ui/Badge.vue'
import Button from '~/components/ui/Button.vue'
import Command from '~/components/ui/Command.vue'
import DropdownMenu from '~/components/ui/DropdownMenu.vue'
import Skeleton from '~/components/ui/Skeleton.vue'
import Tabs from '~/components/ui/Tabs.vue'
import Tooltip from '~/components/ui/Tooltip.vue'
import type { AdminRange, AdminSessionDto } from '~/shared/types/telemetry'
import { statusCode } from '~/utils/format'

const route = useRoute()
const requestFetch = useRequestFetch()
const range = useDashboardRange()
const refreshEpoch = useDashboardRefresh()
const mobileOpen = ref(false)
const commandOpen = ref(false)

const {
	data: session,
	status,
	error,
	refresh: refreshSession,
} = await useAsyncData('admin-session', () => requestFetch<AdminSessionDto>('/api/admin/session'))

const nav = [
	{ label: '数据总览', path: '/', icon: BarChart3 },
	{ label: '系统状态', path: '/system', icon: Server },
]
const ranges = [
	{ value: '7d', label: '7 天' },
	{ value: '30d', label: '30 天' },
	{ value: '90d', label: '90 天' },
	{ value: '365d', label: '1 年' },
]

function setRange(value: string): void {
	range.value = value as AdminRange
}

function refresh(): void {
	refreshEpoch.value += 1
}

function keydown(event: KeyboardEvent): void {
	if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
		event.preventDefault()
		commandOpen.value = !commandOpen.value
	}
}

onMounted(() => window.addEventListener('keydown', keydown))
onBeforeUnmount(() => window.removeEventListener('keydown', keydown))
watch(
	() => route.path,
	() => {
		mobileOpen.value = false
	},
)
</script>

<template>
	<div class="min-h-dvh bg-surface-1">
		<template v-if="status === 'pending'">
			<div
				class="fixed inset-y-0 left-0 hidden w-60 border-r border-surface-4 bg-surface-2 p-4 md:block"
			>
				<Skeleton class="h-10 w-44" />
				<div class="mt-8 space-y-2">
					<Skeleton v-for="index in 3" :key="index" class="h-10 w-full" />
				</div>
			</div>
			<div class="md:pl-60">
				<div class="h-16 border-b border-surface-4 bg-surface-2"></div>
				<div class="space-y-4 p-5">
					<Skeleton class="h-9 w-48" />
					<Skeleton class="h-36 w-full" />
				</div>
			</div>
		</template>
		<div v-else-if="error || !session" class="flex min-h-dvh items-center justify-center">
			<AppState
				:kind="
					statusCode(error) === 403
						? 'forbidden'
						: statusCode(error) === 401
							? 'unauthenticated'
							: 'error'
				"
				@retry="refreshSession"
			/>
		</div>
		<template v-else>
			<Transition name="fade">
				<div
					v-if="mobileOpen"
					class="fixed inset-0 z-30 bg-black/45 backdrop-blur-[1px] md:hidden"
					aria-hidden="true"
					@click="mobileOpen = false"
				></div>
			</Transition>
			<aside
				class="fixed inset-y-0 left-0 z-40 flex w-60 -translate-x-full flex-col border-r border-surface-4 bg-surface-2 transition-transform duration-200 md:translate-x-0"
				:class="mobileOpen && 'translate-x-0'"
			>
				<div class="flex h-16 items-center gap-3 border-b border-surface-4 px-4">
					<div
						class="flex size-9 items-center justify-center rounded-md bg-primary text-primary-foreground shadow-sm"
					>
						<Activity class="size-[18px]" />
					</div>
					<div class="min-w-0">
						<p class="truncate text-sm font-semibold">Axolotl 遥测中心</p>
						<p class="mt-0.5 text-[11px] text-muted-foreground">只读管理控制台</p>
					</div>
					<Button
						variant="ghost"
						size="icon"
						class="ml-auto md:hidden"
						aria-label="关闭导航"
						@click="mobileOpen = false"
					>
						<X class="size-4" />
					</Button>
				</div>
				<div class="px-3 pb-2 pt-4 text-[11px] font-medium text-muted-foreground">工作区</div>
				<nav class="space-y-1 px-3" aria-label="主导航">
					<NuxtLink
						v-for="item in nav"
						:key="item.path"
						:to="item.path"
						class="flex h-10 cursor-pointer items-center gap-3 rounded-md px-3 text-sm font-medium text-muted-foreground transition-colors hover:bg-surface-3 hover:text-foreground active:bg-surface-4"
						:class="route.path === item.path && 'bg-primary/10 text-primary'"
					>
						<component :is="item.icon" class="size-4" />
						{{ item.label }}
					</NuxtLink>
				</nav>

				<div class="mt-auto p-3">
					<div class="rounded-md border border-surface-4 bg-surface-1 p-3">
						<div class="flex items-center justify-between gap-2">
							<span class="inline-flex items-center gap-2 text-xs font-medium">
								<Database class="size-3.5 text-primary" />数据源
							</span>
							<Badge :variant="session.dataSource === 'production' ? 'success' : 'warning'">
								{{ session.dataSource === 'production' ? '生产数据' : '模拟数据' }}
							</Badge>
						</div>
						<p class="mt-2 text-[11px] leading-5 text-muted-foreground">
							统计范围仅包含主动同意遥测的匿名安装。
						</p>
					</div>
					<div class="mt-3 flex items-center gap-2 px-1 text-[11px] text-muted-foreground">
						<ShieldCheck class="size-3.5 text-emerald-600" />
						Cloudflare Access 已保护
					</div>
				</div>
			</aside>

			<div class="min-w-0 md:pl-60">
				<header
					class="sticky top-0 z-20 flex h-16 items-center gap-2 border-b border-surface-4 bg-surface-2/95 px-3 backdrop-blur md:px-5"
				>
					<Button
						variant="ghost"
						size="icon"
						class="md:hidden"
						aria-label="打开导航"
						@click="mobileOpen = true"
					>
						<Menu class="size-4" />
					</Button>
					<button
						type="button"
						class="hidden h-9 min-w-52 cursor-pointer items-center gap-2 rounded-md border border-surface-4 bg-surface-1 px-3 text-sm text-muted-foreground transition-colors hover:border-surface-5 hover:bg-surface-3 lg:flex"
						@click="commandOpen = true"
					>
						<Search class="size-4" />
						<span class="flex-1 text-left">快速跳转</span>
						<kbd class="rounded border border-surface-4 bg-surface-2 px-1.5 py-0.5 text-[10px]"
							>Ctrl K</kbd
						>
					</button>
					<div class="ml-auto flex min-w-0 items-center gap-2">
						<Tabs
							:model-value="range"
							label="UTC 时间范围"
							:items="ranges"
							class="hidden sm:inline-flex"
							@update:model-value="setRange"
						/>
						<Tooltip text="刷新当前数据">
							<Button variant="outline" size="icon" aria-label="刷新当前数据" @click="refresh">
								<RefreshCw class="size-4" />
							</Button>
						</Tooltip>
						<DropdownMenu :session="session" />
					</div>
				</header>
				<div class="border-b border-surface-4 bg-surface-2 px-3 py-2 sm:hidden">
					<Tabs
						:model-value="range"
						label="UTC 时间范围"
						:items="ranges"
						@update:model-value="setRange"
					/>
				</div>
				<main class="mx-auto min-w-0 max-w-[1680px] p-4 md:p-5 lg:p-6">
					<slot />
				</main>
			</div>
			<Command v-model:open="commandOpen" />
		</template>
	</div>
</template>
