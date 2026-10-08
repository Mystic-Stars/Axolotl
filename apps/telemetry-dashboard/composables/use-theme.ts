export type ThemeMode = 'light' | 'dark' | 'system'

export function useTheme() {
    const mode = useState<ThemeMode>('theme-mode', () => 'system')

    function apply(value: ThemeMode): void {
        if (!import.meta.client) return
        const dark =
            value === 'dark' ||
            (value === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
        document.documentElement.classList.toggle('dark', dark)
        localStorage.setItem('telemetry-admin-theme', value)
    }

    onMounted(() => {
        const saved = localStorage.getItem('telemetry-admin-theme') as ThemeMode | null
        if (saved === 'light' || saved === 'dark' || saved === 'system') mode.value = saved
        apply(mode.value)
    })

    watch(mode, apply)
    return { mode }
}
