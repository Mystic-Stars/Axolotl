import type {
    ActivityDto,
    AdminRange,
    DistributionsDto,
    ErrorDetailDto,
    ErrorSampleDto,
    ErrorsPageDto,
    OverviewDto,
    SystemDto,
} from '../../shared/types/telemetry'
import type { TelemetryAdminApi } from './admin-api'
import { unavailable } from './errors'
import type { ErrorQuery } from './validation'
import { rangeDays } from './validation'

const FIXTURE_NOW = new Date('2026-08-14T10:00:00.000Z')

const fixtureErrors: ErrorDetailDto[] = [
    {
        fingerprint: 'fixture-render-thread-01',
        errorType: 'RenderInitializationError',
        latestMessage: 'Fixture renderer could not initialize the selected backend.',
        appVersion: '9.4.0-fixture',
        firstSeen: '2026-08-03',
        lastSeen: '2026-08-14',
        occurrenceCount: 184,
        affectedInstallations: 41,
        hasSample: true,
        route: null,
        command: null,
        stack: null,
    },
    {
        fingerprint: 'fixture-metadata-timeout-02',
        errorType: 'MetadataTimeout',
        latestMessage: 'Fixture metadata request exceeded its deadline.',
        appVersion: '9.3.2-fixture',
        firstSeen: '2026-08-09',
        lastSeen: '2026-08-13',
        occurrenceCount: 57,
        affectedInstallations: 19,
        hasSample: false,
        route: null,
        command: null,
        stack: null,
    },
    {
        fingerprint: 'fixture-runtime-path-03',
        errorType: 'RuntimePathError',
        latestMessage: 'Fixture Java runtime path was not available.',
        appVersion: '9.4.0-fixture',
        firstSeen: '2026-08-12',
        lastSeen: '2026-08-12',
        occurrenceCount: 12,
        affectedInstallations: 8,
        hasSample: true,
        route: null,
        command: null,
        stack: null,
    },
]

export class MockTelemetryAdminApi implements TelemetryAdminApi {
    constructor(private readonly scenario = 'normal') {}

    private assertAvailable(): void {
        if (this.scenario === 'api-error') throw unavailable()
    }

    async overview(range: AdminRange): Promise<OverviewDto> {
        this.assertAvailable()
        const empty = this.scenario === 'empty'
        const value = (normal: number, label: string) => ({ value: empty ? 0 : normal, label })
        return {
            range,
            generatedAt: FIXTURE_NOW.toISOString(),
            metrics: {
                totalInstallations: value(18_426, '历史累计主动同意遥测的安装'),
                dau: value(2_184, '今日 UTC 唯一活跃安装'),
                wau: value(7_932, '最近 7 天唯一活跃安装'),
                mau: value(14_806, '最近 30 天唯一活跃安装'),
                newInstallationsToday: value(318, '今日 UTC 首次出现'),
                errorOccurrences: value(476, `${range} 范围内发生次数`),
                distinctErrorGroups: value(39, `${range} 范围内错误指纹`),
                r2SamplesToday: value(
                    this.scenario === 'budget-reached' ? 2_000 : 614,
                    '今日 UTC 已存储样本',
                ),
            },
        }
    }

    async activity(range: AdminRange): Promise<ActivityDto> {
        this.assertAvailable()
        const points = Array.from({ length: rangeDays(range) }, (_, index) => {
            const day = new Date(FIXTURE_NOW)
            day.setUTCDate(day.getUTCDate() - rangeDays(range) + index + 1)
            const wave = Math.round(Math.sin(index / 3) * 90)
            return {
                day: day.toISOString().slice(0, 10),
                activeInstallations: this.scenario === 'empty' ? 0 : 1_820 + index * 9 + wave,
                newInstallations: this.scenario === 'empty' ? 0 : 230 + (index % 6) * 17,
                errorOccurrences: this.scenario === 'empty' ? 0 : 20 + (index % 5) * 8,
            }
        })
        return { range, points }
    }

    async distributions(range: AdminRange): Promise<DistributionsDto> {
        this.assertAvailable()
        if (this.scenario === 'empty')
            return { range, versions: [], platforms: [], architectures: [] }
        return {
            range,
            versions: [
                { label: '9.4.0-fixture', value: 8_214 },
                { label: '9.3.2-fixture', value: 4_981 },
                { label: '9.3.1-fixture', value: 1_760 },
            ],
            platforms: [
                { label: 'windows-fixture', value: 10_840 },
                { label: 'linux-fixture', value: 2_934 },
                { label: 'macos-fixture', value: 1_181 },
            ],
            architectures: [
                { label: 'x86_64-fixture', value: 12_902 },
                { label: 'aarch64-fixture', value: 2_053 },
            ],
        }
    }

    async errors(query: ErrorQuery): Promise<ErrorsPageDto> {
        this.assertAvailable()
        let rows = this.scenario === 'empty' ? [] : [...fixtureErrors]
        if (this.scenario === 'no-sample') rows = rows.map((row) => ({ ...row, hasSample: false }))
        const search = query.search.toLowerCase()
        rows = rows.filter(
            (row) =>
                (!search ||
                    `${row.fingerprint} ${row.errorType} ${row.latestMessage}`
                        .toLowerCase()
                        .includes(search)) &&
                (!query.version || row.appVersion === query.version) &&
                (!query.errorType || row.errorType === query.errorType) &&
                (query.hasSample === null || row.hasSample === query.hasSample),
        )
        const keys: Record<ErrorQuery['sort'], keyof ErrorDetailDto> = {
            lastSeen: 'lastSeen',
            firstSeen: 'firstSeen',
            occurrences: 'occurrenceCount',
            installations: 'affectedInstallations',
        }
        rows.sort((left, right) => {
            const a = left[keys[query.sort]]
            const b = right[keys[query.sort]]
            const compared =
                typeof a === 'number' && typeof b === 'number'
                    ? a - b
                    : String(a).localeCompare(String(b))
            return query.direction === 'asc' ? compared : -compared
        })
        const total = rows.length
        const start = (query.page - 1) * query.pageSize
        return {
            items: rows.slice(start, start + query.pageSize),
            page: query.page,
            pageSize: query.pageSize,
            total,
            totalPages: Math.max(1, Math.ceil(total / query.pageSize)),
            filters: {
                versions: ['9.4.0-fixture', '9.3.2-fixture', '9.3.1-fixture'],
                platforms: ['windows-fixture', 'linux-fixture', 'macos-fixture'],
                errorTypes: ['MetadataTimeout', 'RenderInitializationError', 'RuntimePathError'],
            },
        }
    }

    async errorDetail(fingerprint: string): Promise<ErrorDetailDto | null> {
        this.assertAvailable()
        return fixtureErrors.find((row) => row.fingerprint === fingerprint) ?? null
    }

    async errorSample(fingerprint: string): Promise<ErrorSampleDto | null> {
        this.assertAvailable()
        const error = fixtureErrors.find((row) => row.fingerprint === fingerprint)
        if (!error?.hasSample || this.scenario === 'no-sample') return null
        return {
            fingerprint,
            occurredAt: '2026-08-14T08:42:11.000Z',
            appVersion: error.appVersion,
            platform: 'windows-fixture',
            architecture: 'x86_64-fixture',
            errorType: error.errorType,
            message: error.latestMessage,
            stack: 'FixtureError: synthetic stack\n    at fixture.operation (fixture.ts:14:2)',
            route: '/fixture/library',
            command: 'fixture-command',
            context: '{"fixture":true,"note":"synthetic telemetry context"}',
        }
    }

    async system(): Promise<SystemDto> {
        this.assertAvailable()
        const budget = this.scenario === 'budget-reached' ? 2_000 : 614
        return {
            generatedAt: FIXTURE_NOW.toISOString(),
            publicWorker: {
                status: 'available',
                label: '运行正常',
                detail: '模拟健康检查通过',
            },
            d1: { status: 'available', label: '可查询', detail: '模拟 D1 可查询' },
            r2:
                this.scenario === 'no-sample'
                    ? { status: 'degraded', label: '暂无样本', detail: '没有登记模拟样本' }
                    : { status: 'available', label: '可读取', detail: '模拟样本可读取' },
            storeErrorContext: true,
            r2Budget: { used: budget, limit: 2_000 },
            limits: {
                samplesPerGroup: 3,
                dailyActiveRetentionDays: 35,
                errorReportsRetentionDays: 30,
                r2RetentionDays: 30,
                errorAggregatesRetentionDays: 365,
            },
            latestDataDay: '2026-08-14',
            cron: {
                status: 'available',
                label: '数据最新',
                detail: '最近聚合日期：2026-08-14（UTC）',
            },
            accountUsage: {
                status: 'unavailable',
                label: '不可用',
                detail: '尚未配置 Analytics API',
            },
        }
    }
}
