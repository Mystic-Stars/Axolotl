import type { H3Event } from 'h3'

import {
    ADMIN_RANGES,
    type AdminRange,
    type ErrorSort,
    type SortDirection,
} from '../../shared/types/telemetry'
import { AdminApiError } from './errors'

const ERROR_SORTS = ['lastSeen', 'firstSeen', 'occurrences', 'installations'] as const
const SORT_DIRECTIONS = ['asc', 'desc'] as const

export interface ErrorQuery {
    range: AdminRange
    page: number
    pageSize: number
    search: string
    version: string | null
    platform: string | null
    errorType: string | null
    hasSample: boolean | null
    sort: ErrorSort
    direction: SortDirection
}

function single(value: unknown): string | undefined {
    return Array.isArray(value) ? String(value[0]) : value == null ? undefined : String(value)
}

function integer(value: unknown, fallback: number, minimum: number, maximum: number): number {
    const raw = single(value)
    if (raw === undefined || raw === '') return fallback
    if (!/^\d+$/.test(raw))
        throw new AdminApiError(400, 'invalid_query', 'Invalid query parameters')
    const parsed = Number(raw)
    if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
        throw new AdminApiError(400, 'invalid_query', 'Invalid query parameters')
    }
    return parsed
}

function limited(value: unknown, maximum: number): string | null {
    const parsed = single(value)?.trim() ?? ''
    if (!parsed) return null
    if (parsed.length > maximum) {
        throw new AdminApiError(400, 'invalid_query', 'Invalid query parameters')
    }
    return parsed
}

export function parseRange(value: unknown): AdminRange {
    const parsed = single(value) ?? '30d'
    if (!ADMIN_RANGES.includes(parsed as AdminRange)) {
        throw new AdminApiError(400, 'invalid_range', 'Range must be 7d, 30d, 90d, or 365d')
    }
    return parsed as AdminRange
}

export function parseErrorQuery(query: Record<string, unknown>): ErrorQuery {
    const sort = single(query.sort) ?? 'lastSeen'
    const direction = single(query.direction) ?? 'desc'
    if (
        !ERROR_SORTS.includes(sort as ErrorSort) ||
        !SORT_DIRECTIONS.includes(direction as SortDirection)
    ) {
        throw new AdminApiError(400, 'invalid_query', 'Invalid query parameters')
    }
    const sample = single(query.hasSample)
    if (sample !== undefined && sample !== 'true' && sample !== 'false') {
        throw new AdminApiError(400, 'invalid_query', 'Invalid query parameters')
    }
    return {
        range: parseRange(query.range),
        page: integer(query.page, 1, 1, 100_000),
        pageSize: integer(query.pageSize, 25, 1, 100),
        search: limited(query.search, 120) ?? '',
        version: limited(query.version, 64),
        platform: limited(query.platform, 32),
        errorType: limited(query.errorType, 128),
        hasSample: sample === undefined ? null : sample === 'true',
        sort: sort as ErrorSort,
        direction: direction as SortDirection,
    }
}

export function getQueryRecord(event: H3Event): Record<string, unknown> {
    const url = new URL(event.node.req.url ?? '/', 'http://localhost')
    return Object.fromEntries(url.searchParams.entries())
}

export function rangeDays(range: AdminRange): number {
    return Number.parseInt(range, 10)
}

export function startDay(range: AdminRange, now = new Date()): string {
    const date = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()))
    date.setUTCDate(date.getUTCDate() - rangeDays(range) + 1)
    return date.toISOString().slice(0, 10)
}
