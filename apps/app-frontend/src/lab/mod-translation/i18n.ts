import {
	BracesIcon,
	FileArchiveIcon,
	LanguagesIcon,
	PackageIcon,
	SearchIcon,
	ShieldCheckIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { defineMessages } from '@modrinth/ui'
import type { Component } from 'vue'

import type { ModTranslationPhase } from './types.ts'

export const modTranslationMessages = defineMessages({
	title: { id: 'app.lab.mod-translation.title', defaultMessage: 'Mod translation' },
	description: {
		id: 'app.lab.mod-translation.description',
		defaultMessage: 'Translate any Minecraft mod JAR into Simplified Chinese.',
	},
	inputSection: {
		id: 'app.lab.mod-translation.input-section',
		defaultMessage: 'Input',
	},
	aiSection: {
		id: 'app.lab.mod-translation.ai-section',
		defaultMessage: 'AI and options',
	},
	jobsSection: {
		id: 'app.lab.mod-translation.jobs-section',
		defaultMessage: 'Jobs',
	},
	startHint: {
		id: 'app.lab.mod-translation.start-hint',
		defaultMessage: 'Choose a JAR and an AI model to start.',
	},
	outputPath: {
		id: 'app.lab.mod-translation.output-path',
		defaultMessage: 'Output: {path}',
	},
	selectFile: {
		id: 'app.lab.mod-translation.select-file',
		defaultMessage: 'Choose a mod JAR',
	},
	analyze: {
		id: 'app.lab.mod-translation.analyze',
		defaultMessage: 'Analyze',
	},
	analyzing: {
		id: 'app.lab.mod-translation.analyzing',
		defaultMessage: 'Analyzing…',
	},
	analyzingElapsed: {
		id: 'app.lab.mod-translation.analyzing-elapsed',
		defaultMessage: 'Analyzing for {seconds}s',
	},
	analyzingHint: {
		id: 'app.lab.mod-translation.analyzing-hint',
		defaultMessage: 'Local inspection: unpacking and scanning the mod. No AI tokens are consumed.',
	},
	analysis: {
		id: 'app.lab.mod-translation.analysis',
		defaultMessage: 'Analysis',
	},
	loader: {
		id: 'app.lab.mod-translation.loader',
		defaultMessage: 'Loader',
	},
	languageEntries: {
		id: 'app.lab.mod-translation.language-entries',
		defaultMessage: 'Language entries',
	},
	languageCharacters: {
		id: 'app.lab.mod-translation.language-characters',
		defaultMessage: 'Characters',
	},
	classCandidates: {
		id: 'app.lab.mod-translation.class-candidates',
		defaultMessage: 'Class text candidates',
	},
	estimatedQuote: {
		id: 'app.lab.mod-translation.estimated-quote',
		defaultMessage: 'Estimated usage',
	},
	estimatedTokens: {
		id: 'app.lab.mod-translation.estimated-tokens',
		defaultMessage: '{tokens} tokens',
	},
	estimatedTokensDetail: {
		id: 'app.lab.mod-translation.estimated-tokens-detail',
		defaultMessage: '~{calls} calls · {input} in / {output} out',
	},
	points: {
		id: 'app.lab.mod-translation.points',
		defaultMessage: '{points} points',
	},
	provider: {
		id: 'app.lab.mod-translation.provider',
		defaultMessage: 'AI provider',
	},
	model: {
		id: 'app.lab.mod-translation.model',
		defaultMessage: 'Text model',
	},
	aiNotConfigured: {
		id: 'app.lab.mod-translation.ai-not-configured',
		defaultMessage: 'AI is not configured. Open the AI settings to enable a provider and model.',
	},
	options: {
		id: 'app.lab.mod-translation.options',
		defaultMessage: 'Options',
	},
	batchSize: {
		id: 'app.lab.mod-translation.batch-size',
		defaultMessage: 'Batch size',
	},
	start: {
		id: 'app.lab.mod-translation.start',
		defaultMessage: 'Start translation',
	},
	cancel: {
		id: 'app.lab.mod-translation.cancel',
		defaultMessage: 'Cancel',
	},
	cancelling: {
		id: 'app.lab.mod-translation.cancelling',
		defaultMessage: 'Cancelling…',
	},
	noJobs: {
		id: 'app.lab.mod-translation.no-jobs',
		defaultMessage: 'Started jobs will appear here.',
	},
	openOutput: {
		id: 'app.lab.mod-translation.open-output',
		defaultMessage: 'Open output folder',
	},
	done: {
		id: 'app.lab.mod-translation.done',
		defaultMessage: 'Done',
	},
	failed: {
		id: 'app.lab.mod-translation.failed',
		defaultMessage: 'Failed',
	},
	signedMod: {
		id: 'app.lab.mod-translation.signed-mod',
		defaultMessage: 'This mod is signed and cannot be modified.',
	},
	phasePrepare: {
		id: 'app.lab.mod-translation.phase.prepare',
		defaultMessage: 'Preparing',
	},
	phaseResearch: {
		id: 'app.lab.mod-translation.phase.research',
		defaultMessage: 'Name generation',
	},
	phaseLanguage: {
		id: 'app.lab.mod-translation.phase.language',
		defaultMessage: 'Language',
	},
	phaseRepair: {
		id: 'app.lab.mod-translation.phase.repair',
		defaultMessage: 'Quality check',
	},
	phaseClass: {
		id: 'app.lab.mod-translation.phase.class',
		defaultMessage: 'Class text',
	},
	phaseValidation: {
		id: 'app.lab.mod-translation.phase.validation',
		defaultMessage: 'Validation',
	},
	phasePackaging: {
		id: 'app.lab.mod-translation.phase.packaging',
		defaultMessage: 'Packaging',
	},
	operationFailed: {
		id: 'app.lab.mod-translation.operation-failed',
		defaultMessage: 'The mod translation operation failed.',
	},
	languageSources: {
		id: 'app.lab.mod-translation.language-sources',
		defaultMessage: 'Language sources',
	},
	preparing: {
		id: 'app.lab.mod-translation.preparing',
		defaultMessage: 'Preparing…',
	},
	backgroundRunning: {
		id: 'app.lab.mod-translation.background-running',
		defaultMessage: 'Running in background',
	},
	technicalDetails: {
		id: 'app.lab.mod-translation.technical-details',
		defaultMessage: 'Technical details',
	},
	copyDiagnostics: {
		id: 'app.lab.mod-translation.copy-diagnostics',
		defaultMessage: 'Copy diagnostics',
	},
	copyDiagnosticsInfo: {
		id: 'app.lab.mod-translation.copy-diagnostics-info',
		defaultMessage: 'Copy diagnostic information',
	},
})

export const modTranslationPhaseSteps: readonly {
	id: ModTranslationPhase
	icon: Component
	label: (typeof modTranslationMessages)[keyof typeof modTranslationMessages]
}[] = [
	{ id: 'prepare', icon: FileArchiveIcon, label: modTranslationMessages.phasePrepare },
	{ id: 'research', icon: SearchIcon, label: modTranslationMessages.phaseResearch },
	{ id: 'language', icon: LanguagesIcon, label: modTranslationMessages.phaseLanguage },
	{ id: 'repair', icon: WrenchIcon, label: modTranslationMessages.phaseRepair },
	{ id: 'class', icon: BracesIcon, label: modTranslationMessages.phaseClass },
	{ id: 'validation', icon: ShieldCheckIcon, label: modTranslationMessages.phaseValidation },
	{ id: 'packaging', icon: PackageIcon, label: modTranslationMessages.phasePackaging },
]
