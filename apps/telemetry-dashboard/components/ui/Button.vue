<script setup lang="ts">
import type { ButtonHTMLAttributes } from 'vue'

import { cn } from '~/lib/utils'

const props = withDefaults(
    defineProps<{
        variant?: 'default' | 'secondary' | 'ghost' | 'outline' | 'destructive'
        size?: 'default' | 'sm' | 'icon'
        class?: ButtonHTMLAttributes['class']
    }>(),
    { variant: 'default', size: 'default', class: undefined },
)

const variants = {
    default:
        'bg-primary text-primary-foreground shadow-sm hover:brightness-95 active:brightness-90',
    secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/75',
    ghost: 'text-foreground hover:bg-surface-3 active:bg-surface-4',
    outline:
        'border-surface-4 bg-surface-2 text-foreground hover:border-surface-5 hover:bg-surface-3 active:bg-surface-4',
    destructive: 'bg-destructive text-destructive-foreground hover:brightness-95',
}

const sizes = {
    default: 'h-9 px-3.5',
    sm: 'h-8 px-2.5 text-xs',
    icon: 'size-9 shrink-0 p-0',
}
</script>

<template>
    <button
        :data-variant="variant"
        :class="
            cn(
                'inline-flex cursor-pointer items-center justify-center gap-2 rounded-md text-sm font-medium transition-[color,background-color,border-color,box-shadow,filter] duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-auto disabled:cursor-not-allowed disabled:opacity-50',
                variants[props.variant],
                sizes[props.size],
                props.class,
            )
        "
    >
        <slot />
    </button>
</template>
