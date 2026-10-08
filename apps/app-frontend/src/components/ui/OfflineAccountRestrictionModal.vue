<script setup lang="ts">
import { ExternalIcon, LogInIcon } from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, NewModal, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ref } from 'vue'

const emit = defineEmits<{ signIn: [] }>()
const modal = ref<InstanceType<typeof NewModal> | null>(null)
const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const openingStore = ref(false)
const messages = defineMessages({
    title: {
        id: 'minecraft-account.restriction.title',
        defaultMessage: 'Offline accounts are unavailable',
    },
    description: {
        id: 'minecraft-account.restriction.description',
        defaultMessage:
            'Sign in with an official Minecraft account on this device before creating or using offline accounts. If you do not own Minecraft yet, you can purchase it from the official website.',
    },
    signIn: {
        id: 'minecraft-account.restriction.sign-in',
        defaultMessage: 'Sign in with Minecraft',
    },
    buy: {
        id: 'minecraft-account.restriction.buy',
        defaultMessage: 'Buy Minecraft',
    },
    openFailed: {
        id: 'minecraft-account.restriction.open-failed',
        defaultMessage: 'Could not open the Minecraft store',
    },
    openFailedDescription: {
        id: 'minecraft-account.restriction.open-failed-description',
        defaultMessage:
            'Try again, or open https://www.minecraft.net in your browser to purchase Minecraft.',
    },
})

async function buyMinecraft() {
    if (openingStore.value) return
    openingStore.value = true
    try {
        await openUrl('https://www.minecraft.net/en-us/store/minecraft-java-bedrock-edition-pc')
    } catch {
        addNotification({
            title: formatMessage(messages.openFailed),
            text: formatMessage(messages.openFailedDescription),
            type: 'error',
        })
    } finally {
        openingStore.value = false
    }
}

function signIn() {
    modal.value?.hide()
    emit('signIn')
}

defineExpose({ show: () => modal.value?.show() })
</script>

<template>
    <NewModal ref="modal" :header="formatMessage(messages.title)" max-width="min(28rem, 95vw)">
        <p class="m-0 text-[var(--color-text-tertiary)]">
            {{ formatMessage(messages.description) }}
        </p>
        <template #actions>
            <div class="flex w-full flex-wrap justify-end gap-2">
                <Button type="base" :disabled="openingStore" @click="buyMinecraft">
                    <ExternalIcon /> {{ formatMessage(messages.buy) }}
                </Button>
                <Button type="colored" color="brand" @click="signIn">
                    <LogInIcon /> {{ formatMessage(messages.signIn) }}
                </Button>
            </div>
        </template>
    </NewModal>
</template>
