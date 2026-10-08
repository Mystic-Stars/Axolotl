import { ref } from 'vue'

import type { LogLevel, LogLine } from '../types'

export type FilterPredicate = (line: LogLine) => boolean

export type ConditionalLevel = 'debug' | 'trace'

export function useConsoleFilters() {
    const activeFilters = ref<Set<LogLevel>>(new Set(['error', 'warn', 'info']))

    function toggleFilter(level: LogLevel) {
        const next = new Set(activeFilters.value)
        if (next.has(level)) {
            next.delete(level)
        } else {
            next.add(level)
        }
        activeFilters.value = next
    }

    function buildFilterPredicate(): FilterPredicate | null {
        if (activeFilters.value.size === 0) return () => false
        const allowed = activeFilters.value
        return (line: LogLine) => {
            return allowed.has(line.level ?? 'info')
        }
    }

    return { activeFilters, toggleFilter, buildFilterPredicate }
}
