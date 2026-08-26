<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	defineMessages,
	injectModrinthClient,
	injectNotificationManager,
	NewModal,
	shareLogs,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import CrashAIExplanationModal from '@/components/ui/CrashAIExplanationModal.vue'
import CrashModChangesModal from '@/components/ui/CrashModChangesModal.vue'
import {
	clearCrashAnalysis,
	type CrashAnalysisResult,
	refreshCrashAnalysis,
} from '@/composables/useCrashAnalysis'
import type { MinecraftLaunchErrorPayload } from '@/composables/useMinecraftLaunchError'
import { getAIState } from '@/helpers/ai'
import { process_listener } from '@/helpers/events.js'
import { get as getInstance } from '@/helpers/instance'
import { get_crash_analysis_ai_settings } from '@/helpers/logs.js'
import { shouldShowMinecraftCrash } from '@/helpers/process.js'

interface CrashModalPayload extends MinecraftLaunchErrorPayload {
	title?: string
	summary?: string
	body?: string
	hint?: string
}

interface ProcessEvent {
	instance_id: string
	uuid: string
	event: 'launched' | 'finished'
	crashed?: boolean
}

interface CrashWarningPayload extends MinecraftLaunchErrorPayload {
	kind: 'minecraft_crash'
}

type Unlisten = () => void

const { formatMessage } = useVIntl()
const client = injectModrinthClient()
const { addNotification } = injectNotificationManager()
const modal = ref<InstanceType<typeof NewModal>>()
const aiModal = ref<InstanceType<typeof CrashAIExplanationModal>>()
const modChangesModal = ref<InstanceType<typeof CrashModChangesModal>>()
const payload = ref<Partial<CrashModalPayload>>({})
const sharing = ref(false)
let lastAnalysis: CrashAnalysisResult | null = null
const modChangesAvailable = ref(false)
const activeRuns = new Map<string, string>()
const lastShownAt = new Map<string, number>()
let unlistenProcess: Unlisten | undefined
let mounted = false
let analysisVersion = 0
const aiAvailable = ref(false)

const messages = defineMessages({
	title: {
		id: 'app.minecraft-crash.title',
		defaultMessage: '{instanceName} crashed',
	},
	body: {
		id: 'app.minecraft-crash.body',
		defaultMessage:
			'Do not send a screenshot of this window when asking for help. Export the error report instead so the crash report, game logs, debug log, and JVM details can be checked together.',
	},
	summary: {
		id: 'app.minecraft-crash.summary',
		defaultMessage: 'Minecraft stopped unexpectedly.',
	},
	supportHint: {
		id: 'app.minecraft-crash.support-hint',
		defaultMessage:
			'When asking for help, send the exported ZIP. Do not send only a screenshot of this window because it does not contain the diagnostic evidence.',
	},
	previewInstance: {
		id: 'app.minecraft-crash.preview-instance',
		defaultMessage: 'Minecraft test instance',
	},
	launchFailedTitle: {
		id: 'app.minecraft-crash.launch-failed-title',
		defaultMessage: '{instanceName} could not start',
	},
	launchFailedSummary: {
		id: 'app.minecraft-crash.launch-failed-summary',
		defaultMessage: 'Minecraft failed during launch preparation.',
	},
	exitedBeforeInitialization: {
		id: 'app.minecraft-crash.exited-before-initialization',
		defaultMessage:
			'The Java process exited before it could connect to the launcher. The selected Java version is probably incompatible with this Minecraft or Mod loader version. Select the Java version required by the instance, then try again.',
	},
	initializationTimedOut: {
		id: 'app.minecraft-crash.initialization-timed-out',
		defaultMessage:
			'The Java process started but did not connect to the launcher within 15 seconds. Check the selected Java version and any wrapper command, then try again.',
	},
	preparationTimedOut: {
		id: 'app.minecraft-crash.preparation-timed-out',
		defaultMessage:
			'Launch preparation did not finish within 60 seconds and was cancelled. Check the Java path, launch hooks, wrapper command, and network connection, then try again.',
	},
	launchFailureHint: {
		id: 'app.minecraft-crash.launch-failure-hint',
		defaultMessage:
			'Open the Minecraft logs to view the captured Java output. When asking for help, export and send the complete Minecraft diagnostic package.',
	},
	analyzing: {
		id: 'app.minecraft-crash.analyzing',
		defaultMessage: 'Analyzing the logs from this launch...',
	},
	evidence: {
		id: 'app.minecraft-crash.evidence',
		defaultMessage: 'Reference evidence: {evidence}',
	},
	viewModChanges: {
		id: 'app.minecraft-crash.view-mod-changes',
		defaultMessage: 'View Mod changes',
	},
	modChangesTitle: {
		id: 'app.minecraft-crash.mod-changes-title',
		defaultMessage: 'Possible issue: Mod files changed since the last successful launch',
	},
	modChangesAction: {
		id: 'app.minecraft-crash.mod-changes-action',
		defaultMessage:
			'Review the changed Mod files and restore the previous setup manually if the crash started after those changes.',
	},
	jvmArgumentsTitle: {
		id: 'app.minecraft-crash.diagnosis.jvm-arguments.title',
		defaultMessage: 'Possible issue: the JVM arguments are invalid',
	},
	jvmArgumentsAction: {
		id: 'app.minecraft-crash.diagnosis.jvm-arguments.action',
		defaultMessage:
			'You can try removing the JVM argument shown below from the instance settings, then launch again.',
	},
	javaTooNewTitle: {
		id: 'app.minecraft-crash.diagnosis.java-too-new.title',
		defaultMessage: 'Possible issue: the selected Java version is too new',
	},
	javaTooNewAction: {
		id: 'app.minecraft-crash.diagnosis.java-too-new.action',
		defaultMessage:
			'You can try selecting the Java major version required by this Minecraft and Mod loader version, then launch again.',
	},
	javaIncompatibleTitle: {
		id: 'app.minecraft-crash.diagnosis.java-incompatible.title',
		defaultMessage: 'Possible issue: the Java version is incompatible',
	},
	javaIncompatibleAction: {
		id: 'app.minecraft-crash.diagnosis.java-incompatible.action',
		defaultMessage:
			'You can try selecting the Java version requested in the error below, or using a compatible build of the affected Mod.',
	},
	java32BitTitle: {
		id: 'app.minecraft-crash.diagnosis.java-32bit.title',
		defaultMessage: 'Possible issue: 32-bit Java cannot allocate enough memory',
	},
	java32BitAction: {
		id: 'app.minecraft-crash.diagnosis.java-32bit.action',
		defaultMessage:
			'You can try installing and selecting a 64-bit Java runtime, then launch again.',
	},
	java11RequiredTitle: {
		id: 'app.minecraft-crash.diagnosis.java-11-required.title',
		defaultMessage: 'Possible issue: a Mod requires Java 11',
	},
	java11RequiredAction: {
		id: 'app.minecraft-crash.diagnosis.java-11-required.action',
		defaultMessage:
			'You can try selecting Java 11, or installing a build of the affected Mod that supports the current Java version.',
	},
	openJ9Title: {
		id: 'app.minecraft-crash.diagnosis.openj9.title',
		defaultMessage: 'Possible issue: OpenJ9 is not compatible with this instance',
	},
	openJ9Action: {
		id: 'app.minecraft-crash.diagnosis.openj9.action',
		defaultMessage:
			'You can try selecting a HotSpot-based Java runtime, such as the bundled Minecraft runtime or Eclipse Temurin.',
	},
	jdkRuntimeTitle: {
		id: 'app.minecraft-crash.diagnosis.jdk-runtime.title',
		defaultMessage: 'Possible issue: the selected JDK is not compatible',
	},
	jdkRuntimeAction: {
		id: 'app.minecraft-crash.diagnosis.jdk-runtime.action',
		defaultMessage:
			'You can try selecting a standard HotSpot Java runtime for this Minecraft version.',
	},
	forgeJavaTitle: {
		id: 'app.minecraft-crash.diagnosis.forge-java.title',
		defaultMessage: 'Possible issue: Forge is not compatible with the selected Java version',
	},
	forgeJavaAction: {
		id: 'app.minecraft-crash.diagnosis.forge-java.action',
		defaultMessage:
			'You can try using the Java version expected by this Forge release, or updating Forge.',
	},
	outOfMemoryTitle: {
		id: 'app.minecraft-crash.diagnosis.out-of-memory.title',
		defaultMessage: 'Possible issue: Minecraft ran out of memory',
	},
	outOfMemoryAction: {
		id: 'app.minecraft-crash.diagnosis.out-of-memory.action',
		defaultMessage:
			'You can try increasing the instance memory allocation, or removing memory-heavy Mods and resource packs.',
	},
	diskSpaceTitle: {
		id: 'app.minecraft-crash.diagnosis.disk-space.title',
		defaultMessage: 'Possible issue: the disk ran out of free space',
	},
	diskSpaceAction: {
		id: 'app.minecraft-crash.diagnosis.disk-space.action',
		defaultMessage:
			'Free space on the drive containing this instance, then launch Minecraft again.',
	},
	fileInUseTitle: {
		id: 'app.minecraft-crash.diagnosis.file-in-use.title',
		defaultMessage: 'Possible issue: another process is using a required file',
	},
	fileInUseAction: {
		id: 'app.minecraft-crash.diagnosis.file-in-use.action',
		defaultMessage:
			'Close the program named in the log, including other launchers, backup tools, or antivirus scans, then launch again.',
	},
	knownFailureTitle: {
		id: 'app.minecraft-crash.diagnosis.known-failure.title',
		defaultMessage: 'Possible issue: a specific launch problem was detected',
	},
	knownFailureAction: {
		id: 'app.minecraft-crash.diagnosis.known-failure.action',
		defaultMessage:
			'This is an automatic guess, not a guaranteed diagnosis. Open the log analysis for the full context before applying the suggested fix.',
	},
	shareDiagnostic: {
		id: 'app.minecraft-crash.share-diagnostic',
		defaultMessage: 'Share diagnostic',
	},
	sharingDiagnostic: {
		id: 'app.minecraft-crash.sharing-diagnostic',
		defaultMessage: 'Sharing diagnostic...',
	},
	shareFailed: {
		id: 'app.minecraft-crash.share-failed',
		defaultMessage: 'Failed to share the diagnostic',
	},
	shareTruncated: {
		id: 'app.minecraft-crash.share-truncated',
		defaultMessage: 'The diagnostic log is too large, so only the last 9 MB was uploaded.',
	},
	shareCopied: {
		id: 'app.minecraft-crash.share-copied',
		defaultMessage: 'Diagnostic link copied to your clipboard',
	},
	shareReady: {
		id: 'app.minecraft-crash.share-ready',
		defaultMessage: 'Diagnostic link is ready to share',
	},
	copyLink: {
		id: 'app.minecraft-crash.copy-link',
		defaultMessage: 'Copy link',
	},
	aiAnalyze: {
		id: 'app.crash-analysis.ai.action',
		defaultMessage: 'Use AI to explain',
	},
	noLogContent: {
		id: 'app.minecraft-crash.no-log-content',
		defaultMessage:
			'No log content was found to share or analyze. Make sure the instance has logs generated in the last few minutes.',
	},
})

const diagnosisMessages = {
	jvm_arguments: [messages.jvmArgumentsTitle, messages.jvmArgumentsAction],
	java_too_new: [messages.javaTooNewTitle, messages.javaTooNewAction],
	java_incompatible: [messages.javaIncompatibleTitle, messages.javaIncompatibleAction],
	java_32bit: [messages.java32BitTitle, messages.java32BitAction],
	java_11_required: [messages.java11RequiredTitle, messages.java11RequiredAction],
	openj9: [messages.openJ9Title, messages.openJ9Action],
	jdk_runtime: [messages.jdkRuntimeTitle, messages.jdkRuntimeAction],
	forge_java_incompatible: [messages.forgeJavaTitle, messages.forgeJavaAction],
	out_of_memory: [messages.outOfMemoryTitle, messages.outOfMemoryAction],
	disk_space: [messages.diskSpaceTitle, messages.diskSpaceAction],
	file_in_use: [messages.fileInUseTitle, messages.fileInUseAction],
} as const

const title = computed(
	() =>
		payload.value.title ||
		formatMessage(messages.title, {
			instanceName: payload.value.instance_name || 'Minecraft',
		}),
)
const summary = computed(() => payload.value.summary || formatMessage(messages.summary))
const body = computed(() => payload.value.body || formatMessage(messages.body))
const hint = computed(() => payload.value.hint || formatMessage(messages.supportHint))
const showSupportHint = computed(() => hint.value !== formatMessage(messages.supportHint))

function applyAnalysis(
	modalPayload: CrashModalPayload,
	analysis: CrashAnalysisResult | null,
): CrashModalPayload {
	const finding = analysis?.findings[0]
	const modChanges = analysis?.mod_changes ?? []
	if (!finding && modChanges.length === 0) return modalPayload

	const diagnosis = finding
		? diagnosisMessages[finding.id as keyof typeof diagnosisMessages]
		: undefined
	const [titleMessage, actionMessage] = diagnosis ?? [
		messages.knownFailureTitle,
		messages.knownFailureAction,
	]
	const resolvedTitleMessage = finding ? titleMessage : messages.modChangesTitle
	const resolvedActionMessage = finding ? actionMessage : messages.modChangesAction
	const evidence = finding?.evidence[0]
	return {
		...modalPayload,
		summary: formatMessage(resolvedTitleMessage),
		body: formatMessage(resolvedActionMessage),
		hint: evidence
			? formatMessage(messages.evidence, {
					evidence: `${evidence.filename}:${evidence.line} - ${evidence.text}`,
				})
			: modalPayload.hint,
		/*
		...(false
			? {
					hint: `${formatMessage(messages.modChanges, {
						changes: modChanges.map((change) => `${change.kind}: ${change.filename}`).join('; '),
					})}${
						evidence
							? ` ${formatMessage(messages.evidence, {
									evidence: `${evidence.filename}:${evidence.line} - ${evidence.text}`,
								})}`
							: ''
					}`,
				}
			: {}),
		*/
	}
}

function show(modalPayload: CrashModalPayload, isPreview = false): boolean {
	if (!isPreview) {
		const now = Date.now()
		const lastShown = lastShownAt.get(modalPayload.instance_id) ?? 0
		if (now - lastShown < 5000) return false
		lastShownAt.set(modalPayload.instance_id, now)
	}
	analysisVersion += 1
	payload.value = modalPayload
	modal.value?.show()
	return true
}

function openModChanges(): void {
	if (lastAnalysis?.mod_changes.length) modChangesModal.value?.show(lastAnalysis)
}

function launchErrorText(error: unknown): string {
	if (typeof error === 'string') return error
	if (error && typeof error === 'object') {
		const record = error as Record<string, unknown>
		const values = [record.message, record.error, record.cause]
			.filter((value): value is string => typeof value === 'string')
			.join('\n')
		if (values) return values
		try {
			return JSON.stringify(error)
		} catch {
			return ''
		}
	}
	return String(error)
}

function launchFailureBody(error: unknown): string | null {
	const errorText = launchErrorText(error)
	if (errorText.includes('Minecraft exited before launcher initialization completed')) {
		return formatMessage(messages.exitedBeforeInitialization)
	}
	if (errorText.includes('Minecraft launcher initialization did not respond')) {
		return formatMessage(messages.initializationTimedOut)
	}
	if (errorText.includes('Minecraft launch preparation timed out')) {
		return formatMessage(messages.preparationTimedOut)
	}
	return null
}

function isLaunchFailure(error: unknown): boolean {
	return launchFailureBody(error) !== null
}

async function analyzeAndUpdate(
	modalPayload: CrashModalPayload,
	fallbackHint?: string,
): Promise<CrashAnalysisResult | null> {
	const version = analysisVersion
	const analysis = await refreshCrashAnalysis(modalPayload.instance_id).catch((error) => {
		console.error('Failed to analyze Minecraft crash', error)
		return null
	})
	lastAnalysis = analysis
	modChangesAvailable.value = !!analysis?.mod_changes.length
	if (mounted && version === analysisVersion) {
		payload.value = applyAnalysis(modalPayload, analysis)
		if (!analysis?.findings.length && fallbackHint) payload.value.hint = fallbackHint
	}
	return analysis
}

async function handleLaunchError(
	error: unknown,
	launchPayload: MinecraftLaunchErrorPayload,
): Promise<boolean> {
	const failureBody = launchFailureBody(error)
	if (!failureBody) return false

	const instanceName = launchPayload.instance_name || 'Minecraft'
	const modalPayload: CrashModalPayload = {
		...launchPayload,
		title: formatMessage(messages.launchFailedTitle, { instanceName }),
		summary: formatMessage(messages.launchFailedSummary),
		body: failureBody,
		hint: formatMessage(messages.analyzing),
	}
	if (!show(modalPayload)) return true
	await analyzeAndUpdate(modalPayload, formatMessage(messages.launchFailureHint))
	return true
}

async function handleWarning(warning: CrashWarningPayload): Promise<void> {
	const modalPayload = { ...warning, hint: formatMessage(messages.analyzing) }
	if (!show(modalPayload)) return
	await analyzeAndUpdate(modalPayload)
}

function showPreview(): void {
	show(
		{
			instance_id: 'preview',
			instance_name: formatMessage(messages.previewInstance),
		},
		true,
	)
}

const shareUrl = ref('')

function notifyNoLogContent(): void {
	addNotification({
		title: formatMessage(messages.noLogContent),
		type: 'warning',
	})
}

async function shareDiagnostic(): Promise<void> {
	if (sharing.value) return
	if (!lastAnalysis?.combined_log) {
		notifyNoLogContent()
		return
	}
	sharing.value = true
	shareUrl.value = ''
	try {
		const result = await shareLogs(client, lastAnalysis.combined_log)
		if (result.truncated) {
			addNotification({
				title: formatMessage(messages.shareTruncated),
				type: 'warning',
			})
		}
		shareUrl.value = result.url
		try {
			await navigator.clipboard.writeText(result.url)
			addNotification({
				title: formatMessage(messages.shareCopied),
				type: 'success',
			})
		} catch (error) {
			console.error('Failed to copy shared diagnostic URL', error)
			addNotification({
				title: formatMessage(messages.shareReady),
				type: 'success',
			})
		}
	} catch (error) {
		console.error('Failed to share crash diagnostic', error)
		addNotification({
			title: formatMessage(messages.shareFailed),
			type: 'error',
		})
	} finally {
		sharing.value = false
	}
}

async function copyShareUrl(): Promise<void> {
	if (!shareUrl.value) return
	try {
		await navigator.clipboard.writeText(shareUrl.value)
		addNotification({
			title: formatMessage(messages.shareCopied),
			type: 'success',
		})
	} catch (error) {
		console.error('Failed to copy share URL', error)
	}
}

function _openAIAnalysis(): void {
	if (!lastAnalysis?.combined_log) {
		notifyNoLogContent()
		return
	}
	aiModal.value?.show(payload.value.instance_id!)
}

async function refreshAIAvailability(): Promise<void> {
	try {
		const [settings, state] = await Promise.all([get_crash_analysis_ai_settings(), getAIState()])
		const provider = state.providers.find((item) => item.provider_id === settings.provider_id)
		aiAvailable.value =
			settings.enabled &&
			state.settings.enabled &&
			!!provider?.enabled &&
			provider.models.some((model) => model.id === settings.model_id && model.enabled)
	} catch {
		aiAvailable.value = false
	}
}

async function handleProcessEvent(event: ProcessEvent): Promise<void> {
	if (event.event === 'launched') {
		activeRuns.set(event.instance_id, event.uuid)
		clearCrashAnalysis(event.instance_id)
		modChangesAvailable.value = false
		return
	}
	if (event.event !== 'finished' || activeRuns.get(event.instance_id) !== event.uuid) return
	if (!shouldShowMinecraftCrash(event.crashed)) {
		activeRuns.delete(event.instance_id)
		return
	}

	await new Promise((resolve) => setTimeout(resolve, 2000))
	if (!mounted || activeRuns.get(event.instance_id) !== event.uuid) return

	try {
		const analysis = await refreshCrashAnalysis(event.instance_id).catch((error) => {
			console.error('Failed to analyze finished Minecraft process', error)
			return null
		})
		lastAnalysis = analysis
		modChangesAvailable.value = !!analysis?.mod_changes.length
		if (!mounted) return

		const instance = await getInstance(event.instance_id).catch(() => null)
		if (!mounted) return
		show(
			applyAnalysis(
				{
					instance_id: event.instance_id,
					instance_name: instance?.name || 'Minecraft',
				},
				analysis,
			),
		)
	} finally {
		if (activeRuns.get(event.instance_id) === event.uuid) activeRuns.delete(event.instance_id)
	}
}

onMounted(async () => {
	mounted = true
	void refreshAIAvailability()
	const unlisten = await process_listener((event: ProcessEvent) => void handleProcessEvent(event))
	if (!mounted) {
		unlisten()
		return
	}
	unlistenProcess = unlisten
})

onUnmounted(() => {
	mounted = false
	analysisVersion += 1
	activeRuns.clear()
	unlistenProcess?.()
})

defineExpose({ handleLaunchError, handleWarning, isLaunchFailure, showPreview })
</script>

<template>
	<NewModal ref="modal" :header="title" fade="danger" max-width="560px">
		<div class="flex flex-col gap-4">
			<Admonition type="critical" :header="summary">
				{{ body }}
			</Admonition>
			<p class="m-0 text-secondary">
				{{ hint }}
			</p>
			<p v-if="showSupportHint" class="m-0 text-secondary">
				{{ formatMessage(messages.supportHint) }}
			</p>
			<div v-if="shareUrl" class="flex items-center gap-2 rounded-lg bg-surface-2 p-3">
				<ExternalIcon class="h-4 w-4 shrink-0 text-secondary" />
				<a
					:href="shareUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="min-w-0 flex-1 truncate text-primary underline"
				>
					{{ shareUrl }}
				</a>
				<ButtonStyled type="outlined">
					<button @click="copyShareUrl">
						{{ formatMessage(messages.copyLink) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
		<template #actions>
			<div class="flex flex-wrap justify-end gap-2">
				<ButtonStyled type="outlined">
					<button :disabled="sharing" @click="shareDiagnostic">
						{{
							sharing
								? formatMessage(messages.sharingDiagnostic)
								: formatMessage(messages.shareDiagnostic)
						}}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="aiAvailable" color="brand">
					<button @click="openAIAnalysis">
						{{ formatMessage(messages.aiAnalyze) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="modChangesAvailable" type="outlined">
					<button @click="openModChanges">
						{{ formatMessage(messages.viewModChanges) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
	<CrashAIExplanationModal ref="aiModal" />
	<CrashModChangesModal ref="modChangesModal" />
</template>
