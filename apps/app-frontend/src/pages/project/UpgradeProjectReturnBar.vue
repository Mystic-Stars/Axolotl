<template>
	<FloatingActionBar
		v-if="snapshot"
		:shown="true"
		aria-label="Return to instance upgrade"
		hide-when-modal-open
	>
		<ButtonStyled color="brand" size="large">
			<button @click="returnToUpgrade">
				<ArrowLeftIcon aria-hidden="true" />
				{{ formatMessage(messages.returnAction) }}
			</button>
		</ButtonStyled>
	</FloatingActionBar>
</template>

<script setup lang="ts">
import { ArrowLeftIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, FloatingActionBar, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'
import { useRouter } from 'vue-router'

import { peekUpgradeFlow } from '@/helpers/upgrade-return-state'

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
