export const ADMIN_RANGES = ['7d', '30d', '90d', '365d'] as const
export type AdminRange = (typeof ADMIN_RANGES)[number]

export type Availability = 'available' | 'degraded' | 'unavailable'

export interface AdminSessionDto {
    identity: {
        name: string
        email: string | null
    }
    organization: 'Axolotl-Launcher'
    logoutUrl: string
    mock: boolean
    dataSource: 'production' | 'fixture'
}

export interface MetricDto {
    value: number
    label: string
}

export interface OverviewDto {
    range: AdminRange
    generatedAt: string
    metrics: {
        totalInstallations: MetricDto
        dau: MetricDto
        wau: MetricDto
        mau: MetricDto
        newInstallationsToday: MetricDto
        errorOccurrences: MetricDto
        distinctErrorGroups: MetricDto
        r2SamplesToday: MetricDto
    }
}

export interface DailyPointDto {
    day: string
    activeInstallations: number
    newInstallations: number
    errorOccurrences: number
}

export interface ActivityDto {
    range: AdminRange
    points: DailyPointDto[]
}

export interface DistributionItemDto {
    label: string
    value: number
}

export interface DistributionsDto {
    range: AdminRange
    versions: DistributionItemDto[]
    platforms: DistributionItemDto[]
    architectures: DistributionItemDto[]
}

export type ErrorSort = 'lastSeen' | 'firstSeen' | 'occurrences' | 'installations'
export type SortDirection = 'asc' | 'desc'

export interface ErrorFiltersDto {
    versions: string[]
    platforms: string[]
    errorTypes: string[]
}

export interface ErrorRowDto {
    fingerprint: string
    errorType: string
    latestMessage: string
    appVersion: string
    firstSeen: string
    lastSeen: string
    occurrenceCount: number
    affectedInstallations: number
    hasSample: boolean
}

export interface ErrorsPageDto {
    items: ErrorRowDto[]
    page: number
    pageSize: number
    total: number
    totalPages: number
    filters: ErrorFiltersDto
}

export interface ErrorDetailDto extends ErrorRowDto {
    route: string | null
    command: string | null
    stack: string | null
}

export interface ErrorSampleDto {
    fingerprint: string
    occurredAt: string
    appVersion: string
    platform: string
    architecture: string
    errorType: string
    message: string
    stack: string | null
    route: string | null
    command: string | null
    context: string | null
}

export interface ServiceCheckDto {
    status: Availability
    label: string
    detail: string
}

export interface SystemDto {
    generatedAt: string
    publicWorker: ServiceCheckDto
    d1: ServiceCheckDto
    r2: ServiceCheckDto
    storeErrorContext: boolean
    r2Budget: { used: number; limit: 2000 }
    limits: {
        samplesPerGroup: 3
        dailyActiveRetentionDays: 35
        errorReportsRetentionDays: 30
        r2RetentionDays: 30
        errorAggregatesRetentionDays: 365
    }
    latestDataDay: string | null
    cron: ServiceCheckDto
    accountUsage: ServiceCheckDto
}

export interface ApiErrorDto {
    error: {
        code: string
        message: string
    }
}
