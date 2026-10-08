<script setup lang="ts">
import { Check, ChevronDown, LogOut, Monitor, Moon, Sun } from 'lucide-vue-next'
import {
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuPortal,
    DropdownMenuRoot,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from 'reka-ui'

import type { AdminSessionDto } from '~/shared/types/telemetry'

import { type ThemeMode, useTheme } from '../../composables/use-theme'

defineProps<{ session: AdminSessionDto }>()

const { mode } = useTheme()
const themes: Array<{ value: ThemeMode; label: string; icon: typeof Sun }> = [
    { value: 'light', label: '浅色', icon: Sun },
    { value: 'dark', label: '深色', icon: Moon },
    { value: 'system', label: '跟随系统', icon: Monitor },
]
</script>

<template>
    <DropdownMenuRoot>
        <DropdownMenuTrigger
            class="inline-flex h-9 min-w-0 cursor-pointer items-center gap-2 rounded-md border border-surface-4 bg-surface-2 px-2.5 text-sm outline-none transition-colors hover:border-surface-5 hover:bg-surface-3 focus:ring-2 focus:ring-ring"
        >
            <span
                class="flex size-6 shrink-0 items-center justify-center rounded bg-primary/15 text-xs font-semibold text-primary"
            >
                {{ session.identity.name.slice(0, 1).toUpperCase() }}
            </span>
            <span class="hidden max-w-32 truncate sm:block">{{ session.identity.name }}</span>
            <ChevronDown class="size-3.5 text-muted-foreground" />
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
            <DropdownMenuContent
                align="end"
                :side-offset="6"
                class="z-50 w-64 rounded-lg border border-surface-5 bg-popover p-1 text-popover-foreground shadow-xl outline-none"
            >
                <DropdownMenuLabel class="px-2 py-1.5">
                    <p class="truncate text-sm font-medium">{{ session.identity.name }}</p>
                    <p class="truncate text-xs font-normal text-muted-foreground">
                        {{ session.identity.email || 'Cloudflare Access 身份' }}
                    </p>
                </DropdownMenuLabel>
                <DropdownMenuSeparator class="my-1 h-px bg-border" />
                <DropdownMenuLabel class="px-2 py-1 text-xs text-muted-foreground"
                    >界面主题</DropdownMenuLabel
                >
                <DropdownMenuItem
                    v-for="theme in themes"
                    :key="theme.value"
                    class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-sm outline-none hover:bg-surface-3 focus:bg-surface-3"
                    @click="mode = theme.value"
                >
                    <component :is="theme.icon" class="size-4 text-muted-foreground" />
                    <span class="flex-1">{{ theme.label }}</span>
                    <Check v-if="mode === theme.value" class="size-4" />
                </DropdownMenuItem>
                <DropdownMenuSeparator class="my-1 h-px bg-border" />
                <DropdownMenuItem as-child>
                    <a
                        :href="session.logoutUrl"
                        class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-sm outline-none hover:bg-surface-3 focus:bg-surface-3"
                    >
                        <LogOut class="size-4 text-muted-foreground" />
                        退出登录
                    </a>
                </DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenuPortal>
    </DropdownMenuRoot>
</template>
