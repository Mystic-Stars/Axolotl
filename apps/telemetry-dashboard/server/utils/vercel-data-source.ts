import { GetObjectCommand, S3Client } from '@aws-sdk/client-s3'

import type {
    TelemetryDatabase,
    TelemetryObject,
    TelemetryObjectStore,
    TelemetryQueryResult,
    TelemetryStatement,
} from './d1-admin-api'

interface D1ApiResponse<T> {
    success: boolean
    errors?: Array<{ message?: string }>
    result?: Array<{ results?: T[]; success?: boolean }>
}

interface RemoteSettings {
    accountId: string
    databaseId: string
    apiToken: string
    r2AccessKeyId: string
    r2SecretAccessKey: string
    r2BucketName: string
}

function setting(name: string): string {
    return String(process.env[name] ?? '').trim()
}

function remoteSettings(): RemoteSettings | null {
    if (process.env.NODE_ENV !== 'production') return null
    const settings = {
        accountId: setting('CLOUDFLARE_ACCOUNT_ID'),
        databaseId: setting('CLOUDFLARE_D1_DATABASE_ID'),
        apiToken: setting('CLOUDFLARE_API_TOKEN'),
        r2AccessKeyId: setting('CLOUDFLARE_R2_ACCESS_KEY_ID'),
        r2SecretAccessKey: setting('CLOUDFLARE_R2_SECRET_ACCESS_KEY'),
        r2BucketName: setting('CLOUDFLARE_R2_BUCKET_NAME'),
    }
    return Object.values(settings).every(Boolean) ? settings : null
}

function assertReadOnlySql(sql: string): void {
    if (!/^\s*(SELECT|WITH)\b/i.test(sql)) throw new Error('Telemetry database is read-only')
}

class RemoteD1Statement implements TelemetryStatement {
    constructor(
        private readonly database: RemoteD1Database,
        private readonly sql: string,
        private readonly params: unknown[] = [],
    ) {}

    bind(...values: unknown[]): TelemetryStatement {
        return new RemoteD1Statement(this.database, this.sql, values)
    }

    async first<T = Record<string, unknown>>(): Promise<T | null> {
        const result = await this.all<T>()
        return result.results[0] ?? null
    }

    all<T = Record<string, unknown>>(): Promise<TelemetryQueryResult<T>> {
        return this.database.query<T>(this.sql, this.params)
    }
}

class RemoteD1Database implements TelemetryDatabase {
    constructor(private readonly settings: RemoteSettings) {}

    prepare(sql: string): TelemetryStatement {
        return new RemoteD1Statement(this, sql)
    }

    async batch(statements: TelemetryStatement[]): Promise<TelemetryQueryResult[]> {
        return Promise.all(statements.map((statement) => statement.all()))
    }

    async query<T>(sql: string, params: unknown[]): Promise<TelemetryQueryResult<T>> {
        assertReadOnlySql(sql)
        const response = await fetch(
            `https://api.cloudflare.com/client/v4/accounts/${encodeURIComponent(this.settings.accountId)}/d1/database/${encodeURIComponent(this.settings.databaseId)}/query`,
            {
                method: 'POST',
                headers: {
                    authorization: `Bearer ${this.settings.apiToken}`,
                    'content-type': 'application/json',
                },
                body: JSON.stringify({ sql, params }),
                signal: AbortSignal.timeout(10_000),
            },
        )
        const body = (await response.json()) as D1ApiResponse<T>
        const result = body.result?.[0]
        if (!response.ok || !body.success || !result?.success || !result.results) {
            throw new Error(body.errors?.[0]?.message || 'Cloudflare D1 query failed')
        }
        return { results: result.results }
    }
}

class RemoteR2Store implements TelemetryObjectStore {
    private readonly client: S3Client

    constructor(private readonly settings: RemoteSettings) {
        this.client = new S3Client({
            region: 'auto',
            endpoint: `https://${settings.accountId}.r2.cloudflarestorage.com`,
            credentials: {
                accessKeyId: settings.r2AccessKeyId,
                secretAccessKey: settings.r2SecretAccessKey,
            },
        })
    }

    async get(
        key: string,
        options?: { range?: { offset: number; length: number } },
    ): Promise<TelemetryObject | null> {
        const range = options?.range
        try {
            const object = await this.client.send(
                new GetObjectCommand({
                    Bucket: this.settings.r2BucketName,
                    Key: key,
                    Range: range
                        ? `bytes=${range.offset}-${range.offset + range.length - 1}`
                        : undefined,
                }),
            )
            if (!object.Body) return null
            return {
                body: object.Body.transformToWebStream(),
                httpMetadata: { contentEncoding: object.ContentEncoding },
            }
        } catch (error) {
            const status = (error as { $metadata?: { httpStatusCode?: number } }).$metadata
                ?.httpStatusCode
            if (status === 404) return null
            throw error
        }
    }
}

let source: { db: TelemetryDatabase; r2: TelemetryObjectStore } | null | undefined

export function remoteTelemetryDataSource(): {
    db: TelemetryDatabase
    r2: TelemetryObjectStore
} | null {
    if (source !== undefined) return source
    const settings = remoteSettings()
    source = settings
        ? { db: new RemoteD1Database(settings), r2: new RemoteR2Store(settings) }
        : null
    return source
}
