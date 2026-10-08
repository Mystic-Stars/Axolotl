<script setup lang="ts">
import {
    Admonition,
    Button,
    Checkbox,
    commonMessages,
    defineMessages,
    NewModal,
    useVIntl,
} from '@modrinth/ui'
import { computed, nextTick, ref } from 'vue'

const { formatMessage } = useVIntl()
const messages = defineMessages({
    title: {
        id: 'app.privileged-consent.title',
        defaultMessage: 'Enable privileged link actions?',
    },
    warning: {
        id: 'app.privileged-consent.warning',
        defaultMessage: 'These actions can change security-sensitive settings and stop games',
    },
    danger: {
        id: 'app.privileged-consent.danger',
        defaultMessage:
            'External links will be able to change settings (theme, launch commands, directories, network options, etc.) and stop running games. You will still confirm each action before it executes.',
    },
    acknowledge: {
        id: 'app.privileged-consent.acknowledge',
        defaultMessage: 'I understand the risks and want to enable privileged link actions',
    },
    confirm: { id: 'app.privileged-consent.confirm', defaultMessage: 'Enable' },
})

const modal = ref<InstanceType<typeof NewModal>>()
const cancelWrap = ref<HTMLElement>()
const acknowledged = ref(false)
const secondsLeft = ref(0)
const settled = ref(true)
let resolver: ((ok: boolean) => void) | undefined
let countdownTimer: ReturnType<typeof setInterval> | undefined

const title = computed(() => formatMessage(messages.title))
const confirmDisabled = computed(() => secondsLeft.value > 0 || !acknowledged.value)
const confirmLabel = computed(() => {
    if (secondsLeft.value > 0) {
        return `${formatMessage(messages.confirm)} (${secondsLeft.value})`
    }
    return formatMessage(messages.confirm)
})

function stopCountdown() {
    if (countdownTimer) {
        clearInterval(countdownTimer)
        countdownTimer = undefined
    }
}

function finish(ok: boolean) {
    if (settled.value) return
    settled.value = true
    stopCountdown()
    modal.value?.hide()
    const resolve = resolver
    resolver = undefined
    resolve?.(ok)
}

// 启用开关前的强制阅读确认
function request(): Promise<boolean> {
    return new Promise<boolean>((resolve) => {
        acknowledged.value = false
        secondsLeft.value = 10
        settled.value = false
        resolver = resolve
        stopCountdown()
        countdownTimer = setInterval(() => {
            secondsLeft.value -= 1
            if (secondsLeft.value <= 0) stopCountdown()
        }, 1000)
        modal.value?.show()
        void nextTick(() => cancelWrap.value?.querySelector('button')?.focus())
    })
}

defineExpose({ request })
</script>

<template>
    <NewModal
        ref="modal"
        :header="title"
        fade="danger"
        max-width="42rem"
        width="min(42rem, calc(95vw - 10rem))"
        :on-hide="() => finish(false)"
    >
        <div class="flex w-full flex-col gap-4">
            <Admonition type="critical" :header="formatMessage(messages.warning)">
                {{ formatMessage(messages.danger) }}
            </Admonition>
            <Checkbox v-model="acknowledged" :label="formatMessage(messages.acknowledge)" />
        </div>
        <template #actions>
            <div ref="cancelWrap" class="flex w-full flex-row justify-end gap-2">
                <Button @click="finish(false)">{{
                    formatMessage(commonMessages.cancelButton)
                }}</Button>
                <Button
                    type="colored"
                    color="red"
                    :disabled="confirmDisabled"
                    @click="finish(true)"
                >
                    {{ confirmLabel }}
                </Button>
            </div>
        </template>
    </NewModal>
</template>
