export function formatNumber(value: number): string {
    return new Intl.NumberFormat('zh-CN').format(value)
}

export function formatUtcDay(value: string): string {
    const parsed = new Date(`${value}T00:00:00.000Z`)
    return Number.isNaN(parsed.getTime())
        ? value
        : new Intl.DateTimeFormat('zh-CN', {
              month: 'numeric',
              day: 'numeric',
              timeZone: 'UTC',
          }).format(parsed)
}

export function formatUtcTimestamp(value: string): string {
    const parsed = new Date(value)
    return Number.isNaN(parsed.getTime())
        ? value
        : `${new Intl.DateTimeFormat('zh-CN', {
              month: '2-digit',
              day: 'numeric',
              year: 'numeric',
              hour: '2-digit',
              minute: '2-digit',
              hour12: false,
              timeZone: 'UTC',
          }).format(parsed)}（UTC）`
}

export function statusCode(error: unknown): number | null {
    if (!error || typeof error !== 'object') return null
    const value = error as { statusCode?: number; status?: number }
    return value.statusCode ?? value.status ?? null
}
