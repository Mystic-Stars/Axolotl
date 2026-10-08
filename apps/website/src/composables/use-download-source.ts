export type DownloadSource = 'auto' | 'update-server' | 'github'
export type ResolvedDownloadSource = Exclude<DownloadSource, 'auto'>

const DOWNLOAD_SOURCE_STORAGE_KEY = 'axolotl-download-source'
function isDownloadSource(value: string | null): value is DownloadSource {
    return value === 'auto' || value === 'update-server' || value === 'github'
}

export function useDownloadSource() {
    const selectedSource = useState<DownloadSource>('axolotl-download-source', () => 'auto')

    const resolvedSource = computed<ResolvedDownloadSource>(() => {
        if (selectedSource.value !== 'auto') return selectedSource.value
        return 'update-server'
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
    })

    return {
        selectedSource,
        resolvedSource,
        setDownloadSource,
    }
}
