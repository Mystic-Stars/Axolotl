<script setup lang="ts">
import type { HTMLAttributes } from 'vue'

import { cn } from '~/lib/utils'

const props = defineProps<{ class?: HTMLAttributes['class'] }>()
</script>

<template>
    <section
        :class="
            cn(
                'rounded-lg border border-surface-4 bg-surface-2 text-card-foreground shadow-[0_1px_2px_rgba(15,23,42,0.04)]',
                props.class,
            )
        "
    >
        <slot />
    </section>
</template>
