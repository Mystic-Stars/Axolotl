<script setup lang="ts">
import { PlusIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NavTabs,
	useVIntl,
} from '@modrinth/ui'
import { inject, onUnmounted, shallowRef } from 'vue'
import { useRoute } from 'vue-router'

import { NewInstanceImage } from '@/assets/icons'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { instance_listener } from '@/helpers/events.js'
import { list } from '@/helpers/instance'
import { useBreadcrumbs } from '@/store/breadcrumbs.js'

const { handleError } = injectNotificationManager()
const showCreationModal = inject('showCreationModal')
const route = useRoute()
const breadcrumbs = useBreadcrumbs()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	library: { id: 'app.library.title', defaultMessage: 'Library' },
	allInstances: { id: 'app.library.tabs.all-instances', defaultMessage: 'All instances' },
	modpacks: { id: 'app.library.tabs.modpacks', defaultMessage: 'Modpacks' },
	servers: { id: 'app.library.tabs.servers', defaultMessage: 'Servers' },
	custom: { id: 'app.library.tabs.custom', defaultMessage: 'Custom' },
	shared: { id: 'app.library.tabs.shared', defaultMessage: 'Shared with me' },
	saved: { id: 'app.library.tabs.saved', defaultMessage: 'Saved' },
	noInstances: { id: 'app.library.no-instances', defaultMessage: 'No instances found' },
	createInstance: {
		id: 'app.library.create-instance',
		defaultMessage: 'Create new instance',
	},
})

breadcrumbs.setRootContext({ name: formatMessage(messages.library), link: route.path })

const instances = shallowRef(await list().catch(handleError))

const { offline } = useNetworkStatus()

const unlistenInstance = await instance_listener(async () => {
	instances.value = await list().catch(handleError)
})
onUnmounted(() => {
	unlistenInstance()
})
</script>

<template>
	<div class="p-6 flex flex-col gap-3">
		<h1 class="m-0 text-2xl hidden">{{ formatMessage(messages.library) }}</h1>
		<NavTabs
			:links="[
				{ label: formatMessage(messages.allInstances), href: `/library` },
				{ label: formatMessage(messages.modpacks), href: `/library/modpacks` },
				{ label: formatMessage(messages.servers), href: `/library/servers` },
				{ label: formatMessage(messages.custom), href: `/library/custom` },
				{ label: formatMessage(messages.shared), href: `/library/shared`, shown: false },
				{ label: formatMessage(messages.saved), href: `/library/saved`, shown: false },
			]"
		/>
		<template v-if="instances && instances.length > 0">
			<RouterView v-if="route.path.startsWith('/library')" :instances="instances" />
		</template>
		<div v-else class="no-instance">
			<div class="icon">
				<NewInstanceImage />
			</div>
			<h3>{{ formatMessage(messages.noInstances) }}</h3>
			<ButtonStyled color="brand">
				<button :disabled="offline" @click="showCreationModal?.()">
					<PlusIcon />
					{{ formatMessage(messages.createInstance) }}
				</button>
			</ButtonStyled>
		</div>
	</div>
</template>

<style lang="scss" scoped>
.no-instance {
	display: flex;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	height: 100%;
	gap: var(--gap-md);

	p,
	h3 {
		margin: 0;
	}

	.icon {
		svg {
			width: 10rem;
			height: 10rem;
		}
	}
}
</style>
