<script setup lang="ts">
import { X } from 'lucide-vue-next'
import {
    DialogClose,
    DialogContent,
    DialogDescription,
    DialogOverlay,
    DialogPortal,
    DialogRoot,
    DialogTitle,
} from 'reka-ui'

defineProps<{ open: boolean; title: string; description?: string }>()
defineEmits<{ 'update:open': [value: boolean] }>()
</script>

<template>
    <DialogRoot :open="open" @update:open="$emit('update:open', $event)">
        <DialogPortal>
            <DialogOverlay
                class="sheet-overlay fixed inset-0 z-40 bg-black/45 backdrop-blur-[1px]"
            />
            <DialogContent
                class="sheet-panel fixed inset-y-0 right-0 z-50 flex w-full max-w-xl flex-col border-l border-surface-5 bg-surface-1 shadow-2xl focus:outline-none"
            >
                <header class="flex shrink-0 items-start justify-between gap-4 border-b px-5 py-4">
                    <div class="min-w-0">
                        <DialogTitle class="text-base font-semibold">{{ title }}</DialogTitle>
                        <DialogDescription
                            v-if="description"
                            class="mt-1 text-sm text-muted-foreground"
                        >
                            {{ description }}
                        </DialogDescription>
                    </div>
                    <DialogClose
                        class="flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground hover:bg-surface-3 hover:text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                        aria-label="关闭详情"
                    >
                        <X class="size-4" />
                    </DialogClose>
                </header>
                <div class="min-h-0 flex-1 overflow-y-auto px-5 py-5">
                    <slot />
                </div>
            </DialogContent>
        </DialogPortal>
    </DialogRoot>
</template>
