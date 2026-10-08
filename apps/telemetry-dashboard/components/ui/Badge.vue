<script setup lang="ts">
import { cn } from '~/lib/utils'

const props = withDefaults(
    defineProps<{
        variant?: 'default' | 'secondary' | 'success' | 'warning' | 'destructive'
        class?: string
    }>(),
    { variant: 'secondary', class: undefined },
)

const variants = {
    default: 'bg-primary text-primary-foreground',
    secondary: 'bg-secondary text-secondary-foreground',
    success: 'bg-emerald-500/15 text-emerald-700 dark:text-emerald-300',
    warning: 'bg-amber-500/15 text-amber-800 dark:text-amber-300',
    destructive: 'bg-red-500/15 text-red-700 dark:text-red-300',
}
</script>

<template>
    <span
        :class="
            cn(
                'inline-flex items-center rounded px-2 py-0.5 text-xs font-medium',
                variants[variant],
                props.class,
            )
        "
    >
        <slot />
    </span>
</template>
