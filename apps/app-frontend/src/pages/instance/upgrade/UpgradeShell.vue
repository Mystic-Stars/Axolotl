<template>
	<div class="mx-auto w-full" :class="wideCompatibilityLayout ? 'max-w-[96rem]' : 'max-w-5xl'">
		<RouterView v-if="instanceMatchesRoute" />
	</div>
	<UpgradeFlowFloatingBar v-if="instanceMatchesRoute" />
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, toRef, watch } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

import type { GameInstance } from '@/helpers/types'
import { parkUpgradeFlow, restoreUpgradeFlow } from '@/helpers/upgrade-return-state'

import {
	attachUpgradeJobToFlow,
	isUpgradeRouteAvailable,
	isUpgradeRouteRecoveryPending,
	provideInstanceUpgradeFlow,
	type UpgradeRouteRequirement,
} from './flow'
import { isRecoverableUpgradeStatus, recoverInstanceUpgradeJob } from './install-job'
import UpgradeFlowFloatingBar from './UpgradeFlowFloatingBar.vue'

const props = defineProps<{ instance: GameInstance }>()
const route = useRoute()
const router = useRouter()
const flow = provideInstanceUpgradeFlow(toRef(props, 'instance'))
const routeInstanceId = computed(() =>
	Array.isArray(route.params.id) ? route.params.id[0] : route.params.id,
)
const instanceMatchesRoute = computed(() => routeInstanceId.value === props.instance.id)
const wideCompatibilityLayout = computed(() => route.path.endsWith('/upgrade/compatibility'))
const restoredSnapshot = restoreUpgradeFlow(props.instance.id, route.fullPath, flow.hydrate)

async function recoverUpgradeJob() {
	const instanceId = props.instance.id
	const requirement = route.meta.upgradeRequirement as UpgradeRouteRequirement | undefined
	if (requirement === 'result') {
		flow.setJobRecoveryState('ready')
		return
	}
	flow.setJobRecoveryState('loading')
	try {
		const job = await recoverInstanceUpgradeJob(instanceId, {
			knownJobId: flow.activeJobId.value,
			continuation: requirement === 'job',
		})
		if (props.instance.id !== instanceId || !job) return
		const downloadsLocation = attachUpgradeJobToFlow(flow, job)
		if (isRecoverableUpgradeStatus(job.status) && requirement !== 'job') {
			await router.replace(downloadsLocation)
		}
	} catch {
		return
	} finally {
		if (props.instance.id === instanceId) flow.setJobRecoveryState('ready')
	}
}

void recoverUpgradeJob()

onMounted(async () => {
	if (restoredSnapshot?.scrollTop === undefined) return
	await nextTick()
	const viewport = document.querySelector('.app-viewport')
	if (viewport) viewport.scrollTop = restoredSnapshot.scrollTop
})

onBeforeRouteLeave((to) => {
	if (to.path.startsWith('/project/')) {
		parkUpgradeFlow({
			instanceId: props.instance.id,
			returnFullPath: route.fullPath,
			targetEnvironment: flow.targetEnvironment.value,
			plan: flow.plan.value,
			createFullBackup: flow.createFullBackup.value,
			directFullBackupPreference: flow.directFullBackupPreference.value,
			sharedUpgradeMode: flow.sharedUpgradeMode.value,
			activeJobId: flow.activeJobId.value,
			result: flow.result.value,
			initialBlockingPlanId: flow.initialBlockingPlanId.value,
			initialBlockingIssues: flow.initialBlockingIssues.value,
			customizeActiveStrategy: flow.customizeActiveStrategy.value,
			scrollTop: document.querySelector('.app-viewport')?.scrollTop,
		})
	}
})

function safeEntryPath(instanceId: string) {
	return `/instance/${encodeURIComponent(instanceId)}/upgrade`
}

function requirementFallback(instanceId: string, requirement: UpgradeRouteRequirement | undefined) {
	if ((requirement === 'unblocked-plan' || requirement === 'selection') && flow.plan.value) {
		return `${safeEntryPath(instanceId)}/compatibility`
	}
	return safeEntryPath(instanceId)
}

watch(
	[
		() => route.fullPath,
		() => props.instance.id,
		flow.activeJobId,
		flow.jobRecoveryState,
		flow.plan,
		flow.result,
	],
	async () => {
		if (!instanceMatchesRoute.value) return
		const requirement = route.meta.upgradeRequirement as UpgradeRouteRequirement | undefined
		if (requirement === 'result') return
		if (isUpgradeRouteRecoveryPending(requirement, flow)) return
		if (requirement === 'job' && route.name === 'InstanceUpgradeProgress') return
		if (!isUpgradeRouteAvailable(requirement, flow)) {
			await router.replace(requirementFallback(props.instance.id, requirement))
		}
	},
	{ immediate: true },
)
</script>
