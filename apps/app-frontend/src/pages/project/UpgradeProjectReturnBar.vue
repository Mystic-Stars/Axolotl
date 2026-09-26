<template>
	<FloatingActionBar
		v-if="snapshot"
		:shown="true"
		aria-label="Return to instance upgrade"
		hide-when-modal-open
	>
		<Button type="colored" color="brand" size="xl" @click="returnToUpgrade"
			><ArrowLeftIcon aria-hidden="true" />
			{{ formatMessage(messages.returnAction) }}
		</Button>
	</FloatingActionBar>
</template>

<script setup lang="ts">
import { ArrowLeftIcon } from '@modrinth/assets'
import { Button, defineMessages, FloatingActionBar, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'
import { useRouter } from 'vue-router'

import { peekUpgradeFlow } from '@/store/navigation-return'

const messages = defineMessages({
	returnAction: { id: 'instance.upgrade.return', defaultMessage: 'Return to instance upgrade' },
})
const router = useRouter()
const { formatMessage } = useVIntl()
const snapshot = computed(() => peekUpgradeFlow())
async function returnToUpgrade() {
	if (snapshot.value) await router.push(snapshot.value.returnFullPath)
}
</script>
