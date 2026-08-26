<template>
	<div class="py-2">
		<LoadingIndicator v-if="loading" class="pt-8" />
		<Admonition
			v-else-if="errorMessage"
			type="critical"
			:header="formatMessage(messages.unavailable)"
		>
			{{ errorMessage }}
		</Admonition>
		<UpgradeResultDetails v-else-if="job?.upgrade_result" :result="job.upgrade_result" />
	</div>
</template>

<script setup lang="ts">
import { Admonition, defineMessages, LoadingIndicator, useVIntl } from '@modrinth/ui'
import { onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { install_job_get, type InstallJobSnapshot } from '@/helpers/install'

import { isSuccessfulUpgradeJob } from './result'
import UpgradeResultDetails from './UpgradeResultDetails.vue'

const route = useRoute()
const router = useRouter()
const { formatMessage } = useVIntl()
const loading = ref(true)
const errorMessage = ref<string | null>(null)
const job = ref<InstallJobSnapshot | null>(null)
const messages = defineMessages({
	unavailable: {
		id: 'instance.upgrade.result.unavailable',
		defaultMessage: 'Upgrade result unavailable',
	},
	missing: {
		id: 'instance.upgrade.result.missing',
		defaultMessage: 'This persisted upgrade result could not be loaded.',
	},
})

onMounted(async () => {
	const jobId = typeof route.query.job === 'string' ? route.query.job : null
	if (!jobId) {
		await router.replace('/downloads')
		return
	}
	try {
		const persisted = await install_job_get(jobId)
		const routeInstanceId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
		if (
			!isSuccessfulUpgradeJob(persisted) ||
			persisted.upgrade_result?.sourceInstanceId !== routeInstanceId
		) {
			errorMessage.value = formatMessage(messages.missing)
			return
		}
		job.value = persisted
	} catch {
		errorMessage.value = formatMessage(messages.missing)
	} finally {
		loading.value = false
	}
})
</script>
