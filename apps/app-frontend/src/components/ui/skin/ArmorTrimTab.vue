<script setup lang="ts">
import { CheckIcon, RotateCounterClockwiseIcon, XIcon } from '@modrinth/assets'
import {
    type ArmorPreviewConfig,
    ArmorTrimPanel,
    Button,
    defineMessages,
    useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted } from 'vue'

const messages = defineMessages({
    title: { id: 'app.skins.armor.title', defaultMessage: 'Armour and trims' },
    description: {
        id: 'app.skins.armor.description',
        defaultMessage: 'Try armour and trims on your skin. Nothing is stored until you save it.',
    },
    unsaved: { id: 'app.skins.armor.unsaved', defaultMessage: 'Unsaved changes' },
    resetToSaved: { id: 'app.skins.armor.reset-to-saved', defaultMessage: 'Reset to saved' },
    restoreDefaults: {
        id: 'app.skins.armor.restore-defaults',
        defaultMessage: 'Restore defaults',
    },
    save: { id: 'app.skins.armor.save', defaultMessage: 'Save' },
    close: { id: 'app.skins.armor.close', defaultMessage: 'Close' },
})

const draft = defineModel<ArmorPreviewConfig>({ required: true })
const props = defineProps<{
    saved: ArmorPreviewConfig
    /** Stable id shared with the toggle so it can point `aria-controls` here. */
    panelId: string
}>()
const emit = defineEmits<{
    save: []
    reset: []
    defaults: []
    close: []
}>()

const { formatMessage } = useVIntl()

/**
 * The tab is mounted only while it is showing, so this listens exactly for that
 * window. It listens in the capture phase because the app's own global
 * shortcuts listen on `window` the same way, and a bubble-phase listener here
 * would be skipped as soon as anything closer stopped the event. A modal on top
 * of the page owns Escape while it is up, and focus sits inside it, so the key
 * is left to it.
 */
function handleKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return
    if (
        event.target instanceof Element &&
        event.target.closest('[role="dialog"], [role="alertdialog"], [aria-modal="true"]')
    ) {
        return
    }

    emit('close')
}

onMounted(() => window.addEventListener('keydown', handleKeydown, true))
onUnmounted(() => window.removeEventListener('keydown', handleKeydown, true))

const isDirty = computed(() =>
    Object.entries(draft.value).some(([slot, piece]) => {
        const applied = props.saved[slot as keyof ArmorPreviewConfig]
        return (
            !applied ||
            applied.material !== piece.material ||
            applied.trimPattern !== piece.trimPattern ||
            applied.trimMaterial !== piece.trimMaterial
        )
    }),
)
</script>

<template>
    <div
        :id="panelId"
        role="region"
        :aria-label="formatMessage(messages.title)"
        class="flex min-h-[24rem] flex-col gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-2 p-4"
    >
        <header class="flex flex-wrap items-start justify-between gap-3">
            <div class="min-w-0">
                <h2 class="m-0 text-lg font-bold text-[var(--color-text-primary)]">
                    {{ formatMessage(messages.title) }}
                </h2>
                <p class="m-0 mt-1 text-sm leading-6 text-[var(--color-text-tertiary)]">
                    {{ formatMessage(messages.description) }}
                </p>
            </div>
            <div class="flex shrink-0 flex-wrap items-center gap-2">
                <span
                    v-if="isDirty"
                    class="text-sm font-semibold text-[var(--color-text-tertiary)]"
                >
                    {{ formatMessage(messages.unsaved) }}
                </span>
                <Button type="outlined" @click="emit('close')"
                    ><XIcon />{{ formatMessage(messages.close) }}</Button
                >
            </div>
        </header>

        <ArmorTrimPanel v-model="draft" />

        <footer
            class="mt-auto flex flex-wrap items-center justify-between gap-2 border-0 border-t border-solid border-surface-5 pt-3"
        >
            <div class="flex flex-wrap items-center gap-2">
                <Button type="outlined" :disabled="!isDirty" @click="emit('reset')"
                    ><RotateCounterClockwiseIcon />{{
                        formatMessage(messages.resetToSaved)
                    }}</Button
                >
                <Button type="outlined" @click="emit('defaults')">{{
                    formatMessage(messages.restoreDefaults)
                }}</Button>
            </div>
            <Button type="colored" color="brand" :disabled="!isDirty" @click="emit('save')"
                ><CheckIcon />{{ formatMessage(messages.save) }}</Button
            >
        </footer>
    </div>
</template>
