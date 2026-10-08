import { describe, expect, it } from 'vitest'

import { parseErrorQuery, parseRange } from '../../server/utils/validation'

describe('admin query validation', () => {
    it('allows only known UTC ranges', () => {
        expect(parseRange('7d')).toBe('7d')
        expect(parseRange(undefined)).toBe('30d')
        expect(() => parseRange('31d')).toThrowError('Range must be 7d, 30d, 90d, or 365d')
    })

    it('bounds page size, search length, and sort fields', () => {
        expect(parseErrorQuery({ page: '2', pageSize: '100', sort: 'occurrences' })).toMatchObject({
            page: 2,
            pageSize: 100,
            sort: 'occurrences',
        })
        expect(() => parseErrorQuery({ pageSize: '101' })).toThrowError('Invalid query parameters')
        expect(() => parseErrorQuery({ search: 'x'.repeat(121) })).toThrowError(
            'Invalid query parameters',
        )
        expect(() => parseErrorQuery({ sort: 'object_key' })).toThrowError(
            'Invalid query parameters',
        )
    })
})
