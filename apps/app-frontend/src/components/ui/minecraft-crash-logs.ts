export interface CrashLogFile {
	age: number
	filename: string
	log_type: 'InfoLog' | 'CrashReport' | 'JvmCrash' | 'LauncherLog'
	output?: string | null
}

const RUN_ASSOCIATION_WINDOW_SECONDS = 3 * 60

export function shouldUseLogShareAutoAnalysis(settings: {
	ai_source: 'logshare' | 'custom'
	auto_upload: boolean
	no_storage?: boolean
}): boolean {
	return settings.ai_source === 'logshare' && settings.auto_upload
}

function isRelevantLog(file: CrashLogFile): boolean {
	const lower = file.filename.toLowerCase()
	if (file.log_type === 'CrashReport') return lower.endsWith('.txt')
	if (file.log_type === 'JvmCrash') return lower.startsWith('hs_err') && lower.endsWith('.log')
	return ['latest.log', 'debug.log', 'launcher_log.txt', 'latest_stdout.log'].includes(lower)
}

export function selectCrashLogFiles(files: CrashLogFile[]): CrashLogFile[] {
	const sorted = files.filter(isRelevantLog).sort((left, right) => right.age - left.age)
	const anchor = sorted[0]?.age
	if (anchor === undefined) return []

	const singletonTypes = new Set<CrashLogFile['log_type']>()
	return sorted.filter((file) => {
		if (anchor - file.age > RUN_ASSOCIATION_WINDOW_SECONDS) return false
		if (file.log_type !== 'CrashReport' && file.log_type !== 'JvmCrash') return true
		if (singletonTypes.has(file.log_type)) return false
		singletonTypes.add(file.log_type)
		return true
	})
}

export function crashLogKey(file: CrashLogFile): string {
	return `${file.log_type}:${file.filename}`
}

export function crashLogLabel(file: CrashLogFile): string {
	return file.log_type === 'CrashReport' ? `crash-reports/${file.filename}` : file.filename
}

export function preferredCrashLogKey(files: CrashLogFile[]): string {
	const preferred =
		files.find((file) => file.log_type === 'CrashReport') ??
		files.find((file) => file.log_type === 'JvmCrash') ??
		files.find((file) => file.filename.toLowerCase() === 'latest.log') ??
		files[0]
	return preferred ? crashLogKey(preferred) : ''
}

export function combineCrashLogs(files: CrashLogFile[]): string {
	return files
		.filter((file) => file.output)
		.map((file) => `===== ${crashLogLabel(file)} =====\n${file.output}`)
		.join('\n\n')
}
