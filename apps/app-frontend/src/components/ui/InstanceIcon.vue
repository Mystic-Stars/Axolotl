<script setup lang="ts">
import { Avatar } from '@modrinth/ui'
import { computed } from 'vue'

import { getDisplayInstanceIcon } from '@/helpers/instance-icons'

const props = withDefaults(
    defineProps<{
        iconPath?: string | null
        instanceId?: string | null
        loader?: string | null
    }>(),
    {
        iconPath: null,
        instanceId: null,
        loader: null,
    },
)

const displayIcon = computed(() => getDisplayInstanceIcon(props.iconPath, props.loader))
</script>

<template>
    <Avatar
        :src="displayIcon.url"
        :tint-by="instanceId"
        :class="{ '!border-0 !rounded-none !bg-transparent !shadow-none': displayIcon.frameless }"
    />
</template>
