import type { AdminRange } from '~/shared/types/telemetry'

export function useDashboardRange() {
    return useState<AdminRange>('dashboard-range', () => '30d')
}

export function useDashboardRefresh() {
    return useState<number>('dashboard-refresh', () => 0)
}
