export type DownloadSource = 'auto' | 'cnb' | 'github'
export type ResolvedDownloadSource = Exclude<DownloadSource, 'auto'>

const DOWNLOAD_SOURCE_STORAGE_KEY = 'axolotl-download-source'
const MAINLAND_CHINA_TIMEZONES = new Set([
	'Asia/Chongqing',
	'Asia/Harbin',
	'Asia/Kashgar',
	'Asia/Shanghai',
	'Asia/Urumqi',
])

function isDownloadSource(value: string | null): value is DownloadSource {
	return value === 'auto' || value === 'cnb' || value === 'github'
}

function isMainlandChinaBrowser() {
	const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone
	if (MAINLAND_CHINA_TIMEZONES.has(timezone)) return true

	const languages = navigator.languages.length > 0 ? navigator.languages : [navigator.language]
	return languages.some((language) => /^zh(?:-Hans)?-CN$/i.test(language))
}

export function useDownloadSource() {
	const selectedSource = useState<DownloadSource>('axolotl-download-source', () => 'auto')
	const browserInformationReady = useState('axolotl-download-source-browser-ready', () => false)

	const resolvedSource = computed<ResolvedDownloadSource>(() => {
		if (selectedSource.value !== 'auto') return selectedSource.value
		if (!browserInformationReady.value) return 'cnb'
		return isMainlandChinaBrowser() ? 'cnb' : 'github'
	})

	function setDownloadSource(source: DownloadSource) {
		selectedSource.value = source
		if (import.meta.client) {
			localStorage.setItem(DOWNLOAD_SOURCE_STORAGE_KEY, source)
		}
	}

	onMounted(() => {
		const savedSource = localStorage.getItem(DOWNLOAD_SOURCE_STORAGE_KEY)
		if (isDownloadSource(savedSource)) {
			selectedSource.value = savedSource
		}
		browserInformationReady.value = true
	})

	return {
		selectedSource,
		resolvedSource,
		setDownloadSource,
	}
}
