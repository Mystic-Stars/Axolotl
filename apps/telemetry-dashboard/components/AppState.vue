<script setup lang="ts">
import { Ban, LogIn, RefreshCw, ServerOff } from 'lucide-vue-next'
import { computed } from 'vue'

import Button from '~/components/ui/Button.vue'

const props = defineProps<{
    kind: 'error' | 'unauthenticated' | 'forbidden'
    compact?: boolean
}>()

const emit = defineEmits<{ retry: [] }>()

const content = computed(() => {
    if (props.kind === 'unauthenticated') {
        return {
            icon: LogIn,
            title: '需要登录',
            detail: '请使用 Axolotl-Launcher 组织成员账号通过 Cloudflare Access 登录。',
            action: '前往登录',
        }
    }
    if (props.kind === 'forbidden') {
        return {
            icon: Ban,
            title: '无权访问',
            detail: '当前 GitHub 身份不符合 Cloudflare Access 组织策略。',
            action: null,
        }
    }
    return {
        icon: ServerOff,
        title: '遥测数据暂不可用',
        detail: '只读管理 API 未返回可用结果，请稍后重试。',
        action: '重新加载',
    }
})
</script>

<template>
    <div
        :class="compact ? 'py-10' : 'min-h-[60vh]'"
        class="flex flex-col items-center justify-center px-4 text-center"
        :data-state="kind"
    >
        <div class="flex size-10 items-center justify-center rounded-lg bg-muted">
            <component :is="content.icon" class="size-5 text-muted-foreground" />
        </div>
        <h2 class="mt-4 text-base font-semibold">{{ content.title }}</h2>
        <p class="mt-1 max-w-md text-sm text-muted-foreground">{{ content.detail }}</p>
        <Button v-if="content.action" class="mt-4" variant="outline" @click="emit('retry')">
            <RefreshCw v-if="kind === 'error'" class="size-4" />
            <LogIn v-else class="size-4" />
            {{ content.action }}
        </Button>
    </div>
</template>
