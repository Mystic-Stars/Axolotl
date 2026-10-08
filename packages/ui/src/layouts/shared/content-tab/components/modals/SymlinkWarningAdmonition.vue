<template>
    <Admonition v-if="symlinkTarget" type="warning" :header="formatMessage(messages.header)">
        {{ formatMessage(messages.body, { path: symlinkTarget }) }}
    </Admonition>
</template>

<script setup lang="ts">
import Admonition from '#ui/components/base/Admonition.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

const { formatMessage } = useVIntl()

const messages = defineMessages({
    header: {
        id: 'app.symlink-warning.write.header',
        defaultMessage: 'Shared instance',
    },
    body: {
        id: 'app.symlink-warning.write.body',
        defaultMessage:
            'This instance is linked to "{path}". Changes will also affect the original files.',
    },
})

defineProps<{
    symlinkTarget?: string
}>()
</script>
