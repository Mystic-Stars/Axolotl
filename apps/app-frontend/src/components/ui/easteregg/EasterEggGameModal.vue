<script setup lang="ts">
import { defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
    title: {
        id: 'app.settings.about.easteregg.game-title',
        defaultMessage: 'Mini Game',
    },
})

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const gameVisible = ref(false)

const gameUrl = `${window.location.origin}/easteregg/games/game.html`

function show() {
    gameVisible.value = true
    modal.value?.show()
}

function onHide() {
    gameVisible.value = false
}

defineExpose({ show })
</script>

<template>
    <NewModal
        ref="modal"
        :header="formatMessage(messages.title)"
        width="min(832px, calc(100vw - 2rem))"
        max-width="832px"
        noblur
        :on-hide="onHide"
    >
        <div class="flex flex-col items-center py-2">
            <iframe
                v-if="gameVisible"
                :src="gameUrl"
                title="Mini Game"
                class="h-[600px] w-[800px] max-w-full rounded-xl border-none bg-black"
            />
        </div>
    </NewModal>
</template>
