export type ModTranslationPhase =
	'prepare' | 'research' | 'language' | 'repair' | 'class' | 'validation' | 'packaging'

export type ModTranslationLevel = 'info' | 'warn' | 'error'
export type ModTranslationTaskStatus = 'running' | 'completed' | 'failed'

export interface ModTranslationQuote {
	estimatedInputTokens: number
	estimatedOutputTokens: number
	estimatedTokens: number
	estimatedCalls: number
	languageBatches: number
	classBatches: number
	points: number
	characters: number
	entries: number
}

export interface ModTranslationLanguageSourceSummary {
	kind: string
	namespace: string
	sourcePath: string
	targetPath: string
	entries: number
	characters: number
	required: number
}

export interface ModTranslationClassCandidateSummary {
	id: string
	path: string
	text: string
	occurrences: number
}

export interface ModTranslationAnalysis {
	analysisId: string
	inputHash: string
	loader: string
	modIds: string[]
	projectNames: string[]
	modVersion?: string
	minecraftVersionRange?: string
	signed: boolean
	warnings: string[]
	languageSources: ModTranslationLanguageSourceSummary[]
	languageEntries: number
	languageCharacters: number
	requiredEntries: number
	classCandidates: ModTranslationClassCandidateSummary[]
	quote: ModTranslationQuote
}

export interface ModTranslationProgress {
	taskId: string
	phase: ModTranslationPhase
	message: string
	completed: number
	total: number
	weightVerified: number
	weightTotal: number
	sample?: { source: string; translation: string } | null
	level: ModTranslationLevel
	finished: boolean
	ok: boolean
	report?: string | null
}

export interface ModTranslationActivity {
	taskId: string
	pass: number
	kind: string
	status: string
	title: string
	summary: string
	count: number
	issueIds: string[]
	debug?: unknown
}

export interface ModTranslationFailure {
	code: string
	message: string
	details?: unknown
}

export interface ModTranslationReport {
	taskId: string
	ok: boolean
	outputPath: string
	modName?: { name?: string; source?: string } | null
	languageAttempted: number
	languageAccepted: number
	classResolved: number
	classTotal: number
	classChangedFiles?: string[]
	warnings: string[]
	error?: string | null
}

export interface ModTranslationTaskEvent {
	eventId: string
	taskId: string
	sequence: number
	occurredAt: string
	eventType: 'progress' | 'activity' | 'finished'
	status: ModTranslationTaskStatus
	progress?: ModTranslationProgress | null
	activity?: ModTranslationActivity | null
	report?: ModTranslationReport | null
	error?: ModTranslationFailure | null
}

export interface ModTranslationTaskSnapshot {
	taskId: string
	inputPath: string
	outputPath: string
	inputHash: string
	startedAt: string
	updatedAt: string
	status: ModTranslationTaskStatus
	sequence: number
	progress?: ModTranslationProgress | null
	activities: ModTranslationActivity[]
	report?: ModTranslationReport | null
	error?: ModTranslationFailure | null
	events: ModTranslationTaskEvent[]
}

export interface ModTranslationOptions {
	batchSize: number
	deepBatchSize: number
	generateModName: boolean
	repairEnabled: boolean
	classTextEnabled: boolean
	maxClassBatch: number
}

export interface ModTranslationTimelineEntry {
	id: string
	sequence: number
	time: string
	phase: ModTranslationPhase
	pass?: number
	kind: string
	status: string
	title: string
	summary?: string
	count?: number
	issueIds: string[]
	debug?: unknown
}

export interface ModTranslationJob {
	taskId: string
	inputPath: string
	outputPath: string
	inputHash: string
	startedAt: string
	updatedAt: string
	status: ModTranslationTaskStatus
	lastSequence: number
	phase: ModTranslationPhase
	message: string
	percent: number
	completed: number
	total: number
	weightVerified: number
	weightTotal: number
	sample?: { source: string; translation: string }
	level: ModTranslationLevel
	events: ModTranslationTaskEvent[]
	timeline: ModTranslationTimelineEntry[]
	report?: ModTranslationReport
	error?: ModTranslationFailure
}
