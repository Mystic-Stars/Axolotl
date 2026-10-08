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
import type { ErrorQuery } from './validation'

export interface TelemetryAdminApi {
    overview(range: AdminRange): Promise<OverviewDto>
    activity(range: AdminRange): Promise<ActivityDto>
    distributions(range: AdminRange): Promise<DistributionsDto>
    errors(query: ErrorQuery): Promise<ErrorsPageDto>
    errorDetail(fingerprint: string): Promise<ErrorDetailDto | null>
    errorSample(fingerprint: string): Promise<ErrorSampleDto | null>
    system(): Promise<SystemDto>
}
