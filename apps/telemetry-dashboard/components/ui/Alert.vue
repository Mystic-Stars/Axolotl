<script setup lang="ts">
import { AlertCircle, CircleCheck, Info } from 'lucide-vue-next'

import { cn } from '~/lib/utils'

const props = withDefaults(
    defineProps<{ variant?: 'info' | 'success' | 'destructive'; title: string; class?: string }>(),
    { variant: 'info', class: undefined },
)

const icons = { info: Info, success: CircleCheck, destructive: AlertCircle }
const styles = {
    info: 'border-border bg-muted/40',
    success: 'border-emerald-500/30 bg-emerald-500/10',
    destructive: 'border-red-500/30 bg-red-500/10',
}
</script>

<template>
    <div
        role="alert"
        :class="cn('flex gap-3 rounded-lg border p-3.5 text-sm', styles[variant], props.class)"
    >
        <component :is="icons[variant]" class="mt-0.5 size-4 shrink-0" />
        <div class="min-w-0">
            <p class="font-medium">{{ title }}</p>
            <div class="mt-0.5 text-muted-foreground"><slot /></div>
        </div>
    </div>
</template>
